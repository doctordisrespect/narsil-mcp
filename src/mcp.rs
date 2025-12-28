use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tracing::{debug, info};

use crate::config::{ClientInfo, ConfigLoader, ToolConfig, ToolFilter};
use crate::index::CodeIntelEngine;
use crate::tool_metadata::TOOL_METADATA;

// Re-export for internal use
pub use crate::tool_handlers::ToolRegistry;

/// MCP Protocol Version
const MCP_VERSION: &str = "2024-11-05";
const SERVER_NAME: &str = "narsil-mcp";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

impl JsonRpcResponse {
    fn success(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    fn error(id: Option<Value>, code: i32, message: &str) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.to_string(),
                data: None,
            }),
        }
    }
}

pub struct McpServer {
    engine: Arc<CodeIntelEngine>,
    tool_registry: ToolRegistry,
    config: ToolConfig,
    client_info: Arc<Mutex<Option<ClientInfo>>>,
}

impl McpServer {
    #[allow(dead_code)]
    pub fn new(engine: CodeIntelEngine) -> Self {
        let config = ConfigLoader::new().load().unwrap_or_else(|e| {
            eprintln!("Warning: Failed to load config: {}. Using defaults.", e);
            // Return default config by loading it again
            ConfigLoader::new().default_config.clone()
        });
        Self {
            engine: Arc::new(engine),
            tool_registry: ToolRegistry::new(),
            config,
            client_info: Arc::new(Mutex::new(None)),
        }
    }

    /// Create an McpServer from an existing `Arc<CodeIntelEngine>`.
    /// This allows sharing the engine with other components like watch mode.
    pub fn from_arc(engine: Arc<CodeIntelEngine>) -> Self {
        let config = ConfigLoader::new().load().unwrap_or_else(|e| {
            eprintln!("Warning: Failed to load config: {}. Using defaults.", e);
            ConfigLoader::new().default_config.clone()
        });
        Self {
            engine,
            tool_registry: ToolRegistry::new(),
            config,
            client_info: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn run(&self) -> Result<()> {
        info!("MCP server starting on stdio");

        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let mut reader = tokio::io::BufReader::new(stdin);
        let mut line = String::new();

        loop {
            line.clear();
            let bytes_read = reader.read_line(&mut line).await?;

            if bytes_read == 0 {
                info!("EOF received, shutting down");
                break;
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            debug!("Received: {}", trimmed);

            let response = match serde_json::from_str::<JsonRpcRequest>(trimmed) {
                Ok(request) => {
                    // Check if this is a notification (no id field means no response expected)
                    // JSON-RPC 2.0: "The Server MUST NOT reply to a Notification"
                    if request.id.is_none() {
                        // This is a notification - handle it but don't respond
                        debug!("Handling notification: {}", request.method);
                        let _ = self.handle_request(request).await;
                        continue;
                    }
                    self.handle_request(request).await
                }
                Err(e) => {
                    // Parse error - try to extract ID from raw JSON for error response
                    // If we can't get an ID, log the error but don't respond (avoids id:null issues)
                    if let Ok(raw) = serde_json::from_str::<Value>(trimmed) {
                        if let Some(id) = raw.get("id").cloned() {
                            // We have an ID, we can respond with an error
                            if !id.is_null() {
                                JsonRpcResponse::error(
                                    Some(id),
                                    -32700,
                                    &format!("Parse error: {}", e),
                                )
                            } else {
                                // id is null - don't respond to avoid ZodError
                                debug!("Parse error with null id, not responding: {}", e);
                                continue;
                            }
                        } else {
                            // No ID field - this might be a malformed notification, don't respond
                            debug!("Parse error without id field, not responding: {}", e);
                            continue;
                        }
                    } else {
                        // Complete parse failure - can't respond without an ID
                        debug!("Complete parse error, not responding: {}", e);
                        continue;
                    }
                }
            };

            let response_str = serde_json::to_string(&response)? + "\n";
            debug!("Sending: {}", response_str.trim());
            stdout.write_all(response_str.as_bytes()).await?;
            stdout.flush().await?;
        }

        Ok(())
    }

    async fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let id = request.id.clone();

        match request.method.as_str() {
            // MCP Lifecycle
            "initialize" => self.handle_initialize(id, request.params),
            "initialized" => JsonRpcResponse::success(id, json!({})),

            // Tool listing and execution
            "tools/list" => self.handle_tools_list(id),
            "tools/call" => self.handle_tool_call(id, request.params).await,

            // Resource listing
            "resources/list" => self.handle_resources_list(id),
            "resources/read" => self.handle_resource_read(id, request.params).await,

            // Prompts
            "prompts/list" => self.handle_prompts_list(id),

            _ => {
                JsonRpcResponse::error(id, -32601, &format!("Method not found: {}", request.method))
            }
        }
    }

    fn handle_initialize(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        // Extract and store client info for editor detection
        if let Some(client_info_value) = params.get("clientInfo") {
            if let (Some(name), version) = (
                client_info_value.get("name").and_then(|v| v.as_str()),
                client_info_value
                    .get("version")
                    .and_then(|v| v.as_str())
                    .map(String::from),
            ) {
                let client = ClientInfo {
                    name: name.to_string(),
                    version,
                };
                info!("MCP client detected: {} {:?}", client.name, client.version);
                if let Ok(mut guard) = self.client_info.lock() {
                    *guard = Some(client);
                }
            }
        }

        JsonRpcResponse::success(
            id,
            json!({
                "protocolVersion": MCP_VERSION,
                "serverInfo": {
                    "name": SERVER_NAME,
                    "version": SERVER_VERSION
                },
                "capabilities": {
                    "tools": {},
                    "resources": {
                        "subscribe": false,
                        "listChanged": false
                    },
                    "prompts": {}
                }
            }),
        )
    }

    fn handle_tools_list(&self, id: Option<Value>) -> JsonRpcResponse {
        // Get client info for editor-specific filtering
        let client_info: Option<ClientInfo> =
            self.client_info.lock().ok().and_then(|guard| guard.clone());

        // Create tool filter with current config and engine options
        let filter = ToolFilter::new(self.config.clone(), self.engine.options(), client_info);

        // Get filtered list of enabled tools
        let enabled_tools = filter.get_enabled_tools();

        // Build tools array from metadata
        let tools: Vec<Value> = enabled_tools
            .iter()
            .filter_map(|tool_name| {
                TOOL_METADATA.get(tool_name).map(|meta| {
                    json!({
                        "name": meta.name,
                        "description": meta.description,
                        "inputSchema": meta.input_schema,
                    })
                })
            })
            .collect();

        info!(
            "Returning {} tools (filtered from {} total)",
            tools.len(),
            TOOL_METADATA.len()
        );

        JsonRpcResponse::success(
            id,
            json!({
                "tools": tools
            }),
        )
    }

    async fn handle_tool_call(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        let start_time = std::time::Instant::now();
        let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

        // Dispatch to tool registry
        let result: Result<String> = self
            .tool_registry
            .dispatch(tool_name, &self.engine, arguments)
            .await;

        // Record metrics and log execution time
        let elapsed = start_time.elapsed();
        self.engine.metrics.record_tool(tool_name, elapsed);
        tracing::info!(
            tool = tool_name,
            duration_ms = elapsed.as_millis(),
            success = result.is_ok(),
            "Tool execution completed"
        );

        match result {
            Ok(content) => JsonRpcResponse::success(
                id,
                json!({
                    "content": [{
                        "type": "text",
                        "text": content
                    }]
                }),
            ),
            Err(e) => JsonRpcResponse::error(id, -32000, &e.to_string()),
        }
    }

    // Legacy match statement removed - now using ToolRegistry
    // Original handle_tool_call was 679 lines with cyclomatic complexity 78
    // New handle_tool_call is ~30 lines with cyclomatic complexity < 5
    #[allow(dead_code)]
    fn _legacy_handle_tool_call_reference() {
        // Handlers moved to src/tool_handlers/
        // - repo.rs: list_repos, get_project_structure, get_file, etc.
        // - symbols.rs: find_symbols, get_symbol_definition, etc.
        // - search.rs: search_code, semantic_search, hybrid_search, etc.
        // - callgraph.rs: get_call_graph, get_callers, get_callees, etc.
        // - git.rs: get_blame, get_file_history, get_recent_changes, etc.
        // - lsp.rs: get_hover_info, get_type_info, go_to_definition
        // - remote.rs: add_remote_repo, list_remote_files, get_remote_file
        // - security.rs: scan_security, find_injection_vulnerabilities, etc.
        // - supply_chain.rs: generate_sbom, check_dependencies, etc.
        // - analysis.rs: get_control_flow, find_dead_code, get_data_flow, etc.
    }

    fn handle_resources_list(&self, id: Option<Value>) -> JsonRpcResponse {
        // Resources are exposed as the indexed repositories
        JsonRpcResponse::success(
            id,
            json!({
                "resources": []
            }),
        )
    }

    async fn handle_resource_read(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        let uri = params.get("uri").and_then(|v| v.as_str()).unwrap_or("");

        match self.engine.read_resource(uri).await {
            Ok(content) => JsonRpcResponse::success(
                id,
                json!({
                    "contents": [{
                        "uri": uri,
                        "mimeType": "text/plain",
                        "text": content
                    }]
                }),
            ),
            Err(e) => JsonRpcResponse::error(id, -32000, &e.to_string()),
        }
    }

    fn handle_prompts_list(&self, id: Option<Value>) -> JsonRpcResponse {
        JsonRpcResponse::success(
            id,
            json!({
                "prompts": [
                    {
                        "name": "explain_codebase",
                        "description": "Get an overview of a codebase's architecture and key components",
                        "arguments": [
                            {
                                "name": "repo",
                                "description": "Repository to explain",
                                "required": true
                            }
                        ]
                    },
                    {
                        "name": "find_implementation",
                        "description": "Find where a specific feature or algorithm is implemented",
                        "arguments": [
                            {
                                "name": "repo",
                                "description": "Repository to search",
                                "required": true
                            },
                            {
                                "name": "feature",
                                "description": "Feature or algorithm to find",
                                "required": true
                            }
                        ]
                    }
                ]
            }),
        )
    }
}
