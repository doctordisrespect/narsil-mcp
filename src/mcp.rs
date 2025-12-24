use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tracing::{debug, info};

use crate::index::CodeIntelEngine;

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
}

impl McpServer {
    #[allow(dead_code)]
    pub fn new(engine: CodeIntelEngine) -> Self {
        Self {
            engine: Arc::new(engine),
            tool_registry: ToolRegistry::new(),
        }
    }

    /// Create an McpServer from an existing `Arc<CodeIntelEngine>`.
    /// This allows sharing the engine with other components like watch mode.
    pub fn from_arc(engine: Arc<CodeIntelEngine>) -> Self {
        Self {
            engine,
            tool_registry: ToolRegistry::new(),
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

    fn handle_initialize(&self, id: Option<Value>, _params: Value) -> JsonRpcResponse {
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
        JsonRpcResponse::success(
            id,
            json!({
                "tools": [
                    {
                        "name": "list_repos",
                        "description": "List all indexed repositories with metadata (path, language breakdown, file count)",
                        "inputSchema": {
                            "type": "object",
                            "properties": {},
                            "required": []
                        }
                    },
                    {
                        "name": "get_project_structure",
                        "description": "Get the directory structure and key files of a repository. Returns a tree view with file types and sizes.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name or path"
                                },
                                "max_depth": {
                                    "type": "integer",
                                    "description": "Maximum directory depth (default: 4)"
                                }
                            },
                            "required": ["repo"]
                        }
                    },
                    {
                        "name": "find_symbols",
                        "description": "Find data structures (structs, classes, enums, interfaces) and functions/methods in a repository. Supports filtering by type and name pattern.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name or path"
                                },
                                "symbol_type": {
                                    "type": "string",
                                    "enum": ["struct", "class", "enum", "interface", "function", "method", "trait", "type", "all"],
                                    "description": "Type of symbol to find (default: all)"
                                },
                                "pattern": {
                                    "type": "string",
                                    "description": "Glob or regex pattern to filter symbol names"
                                },
                                "file_pattern": {
                                    "type": "string",
                                    "description": "Glob pattern to filter files (e.g., '*.rs', 'src/**/*.py')"
                                }
                            },
                            "required": ["repo"]
                        }
                    },
                    {
                        "name": "get_symbol_definition",
                        "description": "Get the full definition of a symbol with surrounding context. Returns the source code with line numbers.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name or path"
                                },
                                "symbol": {
                                    "type": "string",
                                    "description": "Fully qualified symbol name (e.g., 'MyStruct', 'module::function')"
                                },
                                "context_lines": {
                                    "type": "integer",
                                    "description": "Number of context lines before/after (default: 5)"
                                }
                            },
                            "required": ["repo", "symbol"]
                        }
                    },
                    {
                        "name": "search_code",
                        "description": "Semantic and keyword search across code. Returns ranked excerpts with surrounding context.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name or path (optional, searches all if omitted)"
                                },
                                "query": {
                                    "type": "string",
                                    "description": "Search query - can be natural language or code pattern"
                                },
                                "file_pattern": {
                                    "type": "string",
                                    "description": "Glob pattern to filter files"
                                },
                                "max_results": {
                                    "type": "integer",
                                    "description": "Maximum results to return (default: 10)"
                                }
                            },
                            "required": ["query"]
                        }
                    },
                    {
                        "name": "get_file",
                        "description": "Get the contents of a specific file with optional line range",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name or path"
                                },
                                "path": {
                                    "type": "string",
                                    "description": "File path relative to repository root"
                                },
                                "start_line": {
                                    "type": "integer",
                                    "description": "Start line (1-indexed, optional)"
                                },
                                "end_line": {
                                    "type": "integer",
                                    "description": "End line (inclusive, optional)"
                                }
                            },
                            "required": ["repo", "path"]
                        }
                    },
                    {
                        "name": "find_references",
                        "description": "Find all references to a symbol across the codebase",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name or path"
                                },
                                "symbol": {
                                    "type": "string",
                                    "description": "Symbol name to find references for"
                                },
                                "include_definition": {
                                    "type": "boolean",
                                    "description": "Include the definition location (default: true)"
                                }
                            },
                            "required": ["repo", "symbol"]
                        }
                    },
                    {
                        "name": "get_dependencies",
                        "description": "Analyze dependencies and imports for a file or module",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name or path"
                                },
                                "path": {
                                    "type": "string",
                                    "description": "File or module path"
                                },
                                "direction": {
                                    "type": "string",
                                    "enum": ["imports", "imported_by", "both"],
                                    "description": "Direction of dependency analysis (default: both)"
                                }
                            },
                            "required": ["repo", "path"]
                        }
                    },
                    {
                        "name": "reindex",
                        "description": "Trigger re-indexing of a repository or all repositories",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository to reindex (optional, reindexes all if omitted)"
                                }
                            },
                            "required": []
                        }
                    },
                    {
                        "name": "semantic_search",
                        "description": "BM25-ranked semantic search with code-aware tokenization. Better than simple text search for natural language queries.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name (optional, searches all if omitted)"
                                },
                                "query": {
                                    "type": "string",
                                    "description": "Natural language or code search query"
                                },
                                "max_results": {
                                    "type": "integer",
                                    "description": "Maximum results to return (default: 10)"
                                },
                                "doc_type": {
                                    "type": "string",
                                    "enum": ["file", "function", "class", "struct", "method"],
                                    "description": "Filter by document type"
                                }
                            },
                            "required": ["query"]
                        }
                    },
                    {
                        "name": "get_call_graph",
                        "description": "Get the call graph for a repository or specific function. Requires --call-graph flag.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name or path"
                                },
                                "function": {
                                    "type": "string",
                                    "description": "Focus on specific function (optional)"
                                },
                                "depth": {
                                    "type": "integer",
                                    "description": "Maximum depth to traverse (default: 3)"
                                }
                            },
                            "required": ["repo"]
                        }
                    },
                    {
                        "name": "get_callers",
                        "description": "Find functions that call a given function. Requires --call-graph flag.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name or path"
                                },
                                "function": {
                                    "type": "string",
                                    "description": "Function name to find callers of"
                                },
                                "transitive": {
                                    "type": "boolean",
                                    "description": "Include transitive callers (default: false)"
                                },
                                "max_depth": {
                                    "type": "integer",
                                    "description": "Maximum depth for transitive analysis (default: 5)"
                                }
                            },
                            "required": ["repo", "function"]
                        }
                    },
                    {
                        "name": "get_callees",
                        "description": "Find functions called by a given function. Requires --call-graph flag.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name or path"
                                },
                                "function": {
                                    "type": "string",
                                    "description": "Function name to find callees of"
                                },
                                "transitive": {
                                    "type": "boolean",
                                    "description": "Include transitive callees (default: false)"
                                },
                                "max_depth": {
                                    "type": "integer",
                                    "description": "Maximum depth for transitive analysis (default: 5)"
                                }
                            },
                            "required": ["repo", "function"]
                        }
                    },
                    {
                        "name": "find_call_path",
                        "description": "Find the call path between two functions. Requires --call-graph flag.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name or path"
                                },
                                "from": {
                                    "type": "string",
                                    "description": "Source function name"
                                },
                                "to": {
                                    "type": "string",
                                    "description": "Target function name"
                                }
                            },
                            "required": ["repo", "from", "to"]
                        }
                    },
                    {
                        "name": "get_complexity",
                        "description": "Get complexity metrics (cyclomatic, cognitive) for a function. Requires --call-graph flag.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name or path"
                                },
                                "function": {
                                    "type": "string",
                                    "description": "Function name to analyze"
                                }
                            },
                            "required": ["repo", "function"]
                        }
                    },
                    {
                        "name": "get_function_hotspots",
                        "description": "Find highly connected functions (potential refactoring targets) based on call graph analysis. Requires --call-graph flag.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name or path"
                                },
                                "min_connections": {
                                    "type": "integer",
                                    "description": "Minimum total connections (incoming + outgoing) to be considered a hotspot (default: 5)"
                                }
                            },
                            "required": ["repo"]
                        }
                    },
                    {
                        "name": "get_blame",
                        "description": "Get git blame information for a file. Requires --git flag.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name or path"
                                },
                                "path": {
                                    "type": "string",
                                    "description": "File path relative to repository"
                                },
                                "start_line": {
                                    "type": "integer",
                                    "description": "Start line for blame range"
                                },
                                "end_line": {
                                    "type": "integer",
                                    "description": "End line for blame range"
                                }
                            },
                            "required": ["repo", "path"]
                        }
                    },
                    {
                        "name": "get_file_history",
                        "description": "Get git commit history for a file. Requires --git flag.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name or path"
                                },
                                "path": {
                                    "type": "string",
                                    "description": "File path relative to repository"
                                },
                                "max_commits": {
                                    "type": "integer",
                                    "description": "Maximum commits to return (default: 20)"
                                }
                            },
                            "required": ["repo", "path"]
                        }
                    },
                    {
                        "name": "get_recent_changes",
                        "description": "Get recent commits across the repository. Requires --git flag.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name or path"
                                },
                                "days": {
                                    "type": "integer",
                                    "description": "Number of days to look back (default: 7)"
                                }
                            },
                            "required": ["repo"]
                        }
                    },
                    {
                        "name": "get_hotspots",
                        "description": "Find code hotspots - files with high churn and complexity. Requires --git flag.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name or path"
                                },
                                "days": {
                                    "type": "integer",
                                    "description": "Number of days to analyze (default: 30)"
                                },
                                "min_complexity": {
                                    "type": "integer",
                                    "description": "Minimum cyclomatic complexity to report"
                                }
                            },
                            "required": ["repo"]
                        }
                    },
                    {
                        "name": "get_contributors",
                        "description": "Get contributors to a file or repository. Requires --git flag.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name or path"
                                },
                                "path": {
                                    "type": "string",
                                    "description": "File path (optional, shows repo contributors if omitted)"
                                }
                            },
                            "required": ["repo"]
                        }
                    },
                    {
                        "name": "get_commit_diff",
                        "description": "Get the diff for a specific commit. Requires --git flag.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name or path"
                                },
                                "commit": {
                                    "type": "string",
                                    "description": "Commit hash or reference (e.g., HEAD, branch name)"
                                },
                                "path": {
                                    "type": "string",
                                    "description": "Optional file path to filter the diff"
                                }
                            },
                            "required": ["repo", "commit"]
                        }
                    },
                    {
                        "name": "get_symbol_history",
                        "description": "Get commits that modified a specific symbol/function. Requires --git flag.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name or path"
                                },
                                "path": {
                                    "type": "string",
                                    "description": "File path relative to repository"
                                },
                                "symbol": {
                                    "type": "string",
                                    "description": "Symbol/function name to track"
                                },
                                "max_commits": {
                                    "type": "integer",
                                    "description": "Maximum commits to return (default: 10)"
                                }
                            },
                            "required": ["repo", "path", "symbol"]
                        }
                    },
                    {
                        "name": "get_branch_info",
                        "description": "Get current branch name and repository status. Requires --git flag.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name or path"
                                }
                            },
                            "required": ["repo"]
                        }
                    },
                    {
                        "name": "get_modified_files",
                        "description": "Get list of modified files in the working tree. Requires --git flag.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name or path"
                                }
                            },
                            "required": ["repo"]
                        }
                    },
                    {
                        "name": "discover_repos",
                        "description": "Auto-discover repositories in a directory by detecting VCS roots and project markers",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "path": {
                                    "type": "string",
                                    "description": "Base directory to search for repositories"
                                },
                                "max_depth": {
                                    "type": "integer",
                                    "description": "Maximum directory depth to search (default: 3)"
                                }
                            },
                            "required": ["path"]
                        }
                    },
                    {
                        "name": "validate_repo",
                        "description": "Validate that a path is a valid repository and can be indexed",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "path": {
                                    "type": "string",
                                    "description": "Path to validate as a repository"
                                }
                            },
                            "required": ["path"]
                        }
                    },
                    {
                        "name": "get_index_status",
                        "description": "Get status of the search index and enabled features. Shows which optional features are enabled (--git, --call-graph, --persist, --watch) and index statistics.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name (optional, shows all if omitted)"
                                }
                            },
                            "required": []
                        }
                    },
                    {
                        "name": "get_excerpt",
                        "description": "Extract code excerpts around specific lines with intelligent context expansion. Automatically expands to function/class boundaries when enabled.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name or path"
                                },
                                "path": {
                                    "type": "string",
                                    "description": "File path relative to repository root"
                                },
                                "lines": {
                                    "type": "array",
                                    "items": {
                                        "type": "integer"
                                    },
                                    "description": "Line numbers to extract around (1-indexed)"
                                },
                                "context_before": {
                                    "type": "integer",
                                    "description": "Lines of context before (default: 5)"
                                },
                                "context_after": {
                                    "type": "integer",
                                    "description": "Lines of context after (default: 5)"
                                },
                                "expand_to_scope": {
                                    "type": "boolean",
                                    "description": "Expand to function/class boundaries (default: true)"
                                },
                                "max_lines": {
                                    "type": "integer",
                                    "description": "Maximum lines per excerpt (default: 50)"
                                }
                            },
                            "required": ["repo", "path", "lines"]
                        }
                    },
                    {
                        "name": "get_hover_info",
                        "description": "Get hover information (type info, documentation) for a symbol at a specific position. Enhanced with LSP when available.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name or path"
                                },
                                "path": {
                                    "type": "string",
                                    "description": "File path relative to repository root"
                                },
                                "line": {
                                    "type": "integer",
                                    "description": "Line number (1-indexed)"
                                },
                                "character": {
                                    "type": "integer",
                                    "description": "Character position (0-indexed)"
                                }
                            },
                            "required": ["repo", "path", "line", "character"]
                        }
                    },
                    {
                        "name": "get_type_info",
                        "description": "Get precise type information for a symbol. Requires LSP to be enabled.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name or path"
                                },
                                "path": {
                                    "type": "string",
                                    "description": "File path relative to repository root"
                                },
                                "line": {
                                    "type": "integer",
                                    "description": "Line number (1-indexed)"
                                },
                                "character": {
                                    "type": "integer",
                                    "description": "Character position (0-indexed)"
                                }
                            },
                            "required": ["repo", "path", "line", "character"]
                        }
                    },
                    {
                        "name": "get_metrics",
                        "description": "Get performance metrics including tool execution times, indexing statistics, and server uptime",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "format": {
                                    "type": "string",
                                    "enum": ["markdown", "json"],
                                    "description": "Output format (default: markdown)"
                                }
                            },
                            "required": []
                        }
                    },
                    {
                        "name": "find_similar_code",
                        "description": "Find code similar to a given snippet using TF-IDF embeddings. Good for finding duplicate or related code patterns.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "query": {
                                    "type": "string",
                                    "description": "Code snippet to find similar code for"
                                },
                                "repo": {
                                    "type": "string",
                                    "description": "Repository to search in (optional, searches all if omitted)"
                                },
                                "max_results": {
                                    "type": "integer",
                                    "description": "Maximum results to return (default: 10)"
                                }
                            },
                            "required": ["query"]
                        }
                    },
                    {
                        "name": "find_similar_to_symbol",
                        "description": "Find code similar to a specific symbol (function, class, etc.). Useful for finding related implementations or potential duplicates.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name or path"
                                },
                                "symbol": {
                                    "type": "string",
                                    "description": "Symbol name to find similar code for"
                                },
                                "max_results": {
                                    "type": "integer",
                                    "description": "Maximum results to return (default: 10)"
                                }
                            },
                            "required": ["repo", "symbol"]
                        }
                    },
                    {
                        "name": "go_to_definition",
                        "description": "Find the definition location of a symbol at a specific position. Enhanced with LSP when available.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name or path"
                                },
                                "path": {
                                    "type": "string",
                                    "description": "File path relative to repository root"
                                },
                                "line": {
                                    "type": "integer",
                                    "description": "Line number (1-indexed)"
                                },
                                "character": {
                                    "type": "integer",
                                    "description": "Character position (0-indexed)"
                                }
                            },
                            "required": ["repo", "path", "line", "character"]
                        }
                    },
                    {
                        "name": "add_remote_repo",
                        "description": "Add a remote GitHub repository for indexing. Clones the repo to a temporary location.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "url": {
                                    "type": "string",
                                    "description": "GitHub URL (e.g., github.com/owner/repo or https://github.com/owner/repo)"
                                },
                                "sparse_paths": {
                                    "type": "array",
                                    "items": { "type": "string" },
                                    "description": "Optional: only clone these paths for efficiency"
                                }
                            },
                            "required": ["url"]
                        }
                    },
                    {
                        "name": "list_remote_files",
                        "description": "List files in a remote GitHub repository via API (no clone needed). Rate limited without GITHUB_TOKEN.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "url": {
                                    "type": "string",
                                    "description": "GitHub URL"
                                },
                                "path": {
                                    "type": "string",
                                    "description": "Optional subdirectory to list"
                                }
                            },
                            "required": ["url"]
                        }
                    },
                    {
                        "name": "get_remote_file",
                        "description": "Fetch a specific file from a remote GitHub repository via API (no clone needed).",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "url": {
                                    "type": "string",
                                    "description": "GitHub URL"
                                },
                                "path": {
                                    "type": "string",
                                    "description": "File path to fetch"
                                }
                            },
                            "required": ["url", "path"]
                        }
                    },
                    // Control Flow Graph (CFG) Tools
                    {
                        "name": "get_control_flow",
                        "description": "Get the control flow graph (CFG) for a function, showing basic blocks, branches, and loops.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name or path"
                                },
                                "path": {
                                    "type": "string",
                                    "description": "File path containing the function"
                                },
                                "function": {
                                    "type": "string",
                                    "description": "Function name to analyze"
                                }
                            },
                            "required": ["repo", "path", "function"]
                        }
                    },
                    {
                        "name": "find_dead_code",
                        "description": "Find unreachable code blocks in a function or file using control flow analysis.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name or path"
                                },
                                "path": {
                                    "type": "string",
                                    "description": "File path to analyze"
                                },
                                "function": {
                                    "type": "string",
                                    "description": "Optional: specific function to analyze"
                                }
                            },
                            "required": ["repo", "path"]
                        }
                    },
                    // Data Flow Graph (DFG) Tools
                    {
                        "name": "get_data_flow",
                        "description": "Get data flow analysis for a function, showing variable definitions and uses.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name or path"
                                },
                                "path": {
                                    "type": "string",
                                    "description": "File path containing the function"
                                },
                                "function": {
                                    "type": "string",
                                    "description": "Function name to analyze"
                                }
                            },
                            "required": ["repo", "path", "function"]
                        }
                    },
                    {
                        "name": "get_reaching_definitions",
                        "description": "Get reaching definitions analysis - which variable assignments reach each point in the code.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name or path"
                                },
                                "path": {
                                    "type": "string",
                                    "description": "File path containing the function"
                                },
                                "function": {
                                    "type": "string",
                                    "description": "Function name to analyze"
                                }
                            },
                            "required": ["repo", "path", "function"]
                        }
                    },
                    {
                        "name": "find_uninitialized",
                        "description": "Find variables that may be used before being initialized.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name or path"
                                },
                                "path": {
                                    "type": "string",
                                    "description": "File path to analyze"
                                },
                                "function": {
                                    "type": "string",
                                    "description": "Optional: specific function to analyze"
                                }
                            },
                            "required": ["repo", "path"]
                        }
                    },
                    {
                        "name": "find_dead_stores",
                        "description": "Find variable assignments that are never read (dead stores).",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name or path"
                                },
                                "path": {
                                    "type": "string",
                                    "description": "File path to analyze"
                                },
                                "function": {
                                    "type": "string",
                                    "description": "Optional: specific function to analyze"
                                }
                            },
                            "required": ["repo", "path"]
                        }
                    },
                    // Phase 2: Enhanced Search & Embeddings
                    {
                        "name": "hybrid_search",
                        "description": "Perform hybrid search combining BM25 keyword search with TF-IDF semantic similarity using Reciprocal Rank Fusion (RRF).",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "query": {
                                    "type": "string",
                                    "description": "Search query"
                                },
                                "repo": {
                                    "type": "string",
                                    "description": "Optional: limit to specific repository"
                                },
                                "max_results": {
                                    "type": "integer",
                                    "description": "Maximum results to return (default: 10)"
                                },
                                "mode": {
                                    "type": "string",
                                    "enum": ["hybrid", "bm25", "tfidf"],
                                    "description": "Search mode: hybrid (default), bm25 only, or tfidf only"
                                }
                            },
                            "required": ["query"]
                        }
                    },
                    {
                        "name": "search_chunks",
                        "description": "Search over AST-aware code chunks with symbol context.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "query": {
                                    "type": "string",
                                    "description": "Search query"
                                },
                                "repo": {
                                    "type": "string",
                                    "description": "Optional: limit to specific repository"
                                },
                                "chunk_type": {
                                    "type": "string",
                                    "enum": ["function", "method", "class", "trait", "module", "all"],
                                    "description": "Filter by chunk type"
                                },
                                "max_results": {
                                    "type": "integer",
                                    "description": "Maximum results to return (default: 10)"
                                }
                            },
                            "required": ["query"]
                        }
                    },
                    {
                        "name": "get_chunks",
                        "description": "Get AST-aware code chunks for a file with symbol context.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name or path"
                                },
                                "path": {
                                    "type": "string",
                                    "description": "File path"
                                },
                                "include_imports": {
                                    "type": "boolean",
                                    "description": "Include import statements in context (default: true)"
                                }
                            },
                            "required": ["repo", "path"]
                        }
                    },
                    {
                        "name": "get_chunk_stats",
                        "description": "Get statistics about code chunks in a repository.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name or path"
                                }
                            },
                            "required": ["repo"]
                        }
                    },
                    {
                        "name": "get_embedding_stats",
                        "description": "Get statistics about the embedding index.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {},
                            "required": []
                        }
                    },
                    // Phase 7: Neural Embeddings
                    {
                        "name": "neural_search",
                        "description": "Search code using neural semantic embeddings. Finds semantically similar code even with different variable names. Requires --neural flag and EMBEDDING_API_KEY.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "query": {
                                    "type": "string",
                                    "description": "Natural language or code query"
                                },
                                "repo": {
                                    "type": "string",
                                    "description": "Optional: limit to specific repository"
                                },
                                "max_results": {
                                    "type": "integer",
                                    "description": "Maximum results to return (default: 10)"
                                }
                            },
                            "required": ["query"]
                        }
                    },
                    {
                        "name": "find_semantic_clones",
                        "description": "Find semantically similar code (Type-3/4 clones) using neural embeddings. Detects code that does the same thing with different implementation.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name"
                                },
                                "path": {
                                    "type": "string",
                                    "description": "File path"
                                },
                                "function": {
                                    "type": "string",
                                    "description": "Function name to find clones of"
                                },
                                "threshold": {
                                    "type": "number",
                                    "description": "Similarity threshold 0-1 (default: 0.8)"
                                }
                            },
                            "required": ["repo", "path", "function"]
                        }
                    },
                    {
                        "name": "get_neural_stats",
                        "description": "Get statistics about the neural embedding index. Requires --neural flag.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {},
                            "required": []
                        }
                    },
                    // Phase 8: Type Inference
                    {
                        "name": "infer_types",
                        "description": "Infer types for variables in a Python/JavaScript/TypeScript function. Shows what types flow through the code without running external type checkers.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name"
                                },
                                "path": {
                                    "type": "string",
                                    "description": "File path relative to repository"
                                },
                                "function": {
                                    "type": "string",
                                    "description": "Function name to analyze"
                                }
                            },
                            "required": ["repo", "path", "function"]
                        }
                    },
                    {
                        "name": "check_type_errors",
                        "description": "Find potential type errors in Python/JavaScript/TypeScript code without running mypy/tsc. Detects type mismatches, undefined variables, etc.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name"
                                },
                                "path": {
                                    "type": "string",
                                    "description": "File path relative to repository"
                                }
                            },
                            "required": ["repo", "path"]
                        }
                    },
                    {
                        "name": "get_typed_taint_flow",
                        "description": "Enhanced taint analysis with type information. More precise than untyped taint tracking, combines data flow with type inference.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name"
                                },
                                "path": {
                                    "type": "string",
                                    "description": "File path relative to repository"
                                },
                                "source_line": {
                                    "type": "integer",
                                    "description": "Line number to trace from"
                                }
                            },
                            "required": ["repo", "path", "source_line"]
                        }
                    },
                    // Phase 3: Taint Analysis & Security
                    {
                        "name": "find_injection_vulnerabilities",
                        "description": "Find injection vulnerabilities (SQL injection, XSS, command injection, path traversal) using taint analysis.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name or path"
                                },
                                "path": {
                                    "type": "string",
                                    "description": "Optional: specific file to analyze"
                                },
                                "vulnerability_types": {
                                    "type": "array",
                                    "items": {
                                        "type": "string",
                                        "enum": ["sql", "xss", "command", "path", "all"]
                                    },
                                    "description": "Types of vulnerabilities to find (default: all)"
                                }
                            },
                            "required": ["repo"]
                        }
                    },
                    {
                        "name": "trace_taint",
                        "description": "Trace how tainted data flows from a source location through the code.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name or path"
                                },
                                "path": {
                                    "type": "string",
                                    "description": "File path containing the source"
                                },
                                "line": {
                                    "type": "integer",
                                    "description": "Line number of the taint source"
                                }
                            },
                            "required": ["repo", "path", "line"]
                        }
                    },
                    {
                        "name": "get_taint_sources",
                        "description": "List all identified taint sources (user inputs, file reads, network data) in the codebase.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name or path"
                                },
                                "path": {
                                    "type": "string",
                                    "description": "Optional: specific file to analyze"
                                },
                                "source_types": {
                                    "type": "array",
                                    "items": {
                                        "type": "string",
                                        "enum": ["user_input", "file_read", "database", "environment", "network", "all"]
                                    },
                                    "description": "Types of sources to find (default: all)"
                                }
                            },
                            "required": ["repo"]
                        }
                    },
                    {
                        "name": "get_security_summary",
                        "description": "Get a comprehensive security summary for a repository including vulnerability counts and risk assessment.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name or path"
                                }
                            },
                            "required": ["repo"]
                        }
                    },
                    // Phase 4: Security Rules Engine
                    {
                        "name": "scan_security",
                        "description": "Scan repository for security issues using the security rules engine. Detects vulnerabilities, secrets, crypto issues, and more.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name or path"
                                },
                                "path": {
                                    "type": "string",
                                    "description": "Optional specific file or directory path to scan"
                                },
                                "severity_threshold": {
                                    "type": "string",
                                    "enum": ["critical", "high", "medium", "low", "info"],
                                    "description": "Minimum severity level to report (default: low)"
                                },
                                "ruleset": {
                                    "type": "string",
                                    "description": "Optional ruleset to use (owasp, cwe, crypto, secrets, or path to custom YAML)"
                                },
                                "exclude_tests": {
                                    "type": "boolean",
                                    "description": "Exclude test files from scanning (default: true). Set to false to include test files."
                                },
                                "max_findings": {
                                    "type": "integer",
                                    "description": "Maximum number of findings to return. Useful for bounding output size on large codebases."
                                },
                                "offset": {
                                    "type": "integer",
                                    "description": "Skip this many findings before returning results. Use with max_findings for pagination."
                                }
                            },
                            "required": ["repo"]
                        }
                    },
                    {
                        "name": "check_owasp_top10",
                        "description": "Scan specifically for OWASP Top 10 2021 vulnerabilities including injection, broken auth, XSS, SSRF, etc.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name or path"
                                },
                                "path": {
                                    "type": "string",
                                    "description": "Optional specific file or directory path to scan"
                                }
                            },
                            "required": ["repo"]
                        }
                    },
                    {
                        "name": "check_cwe_top25",
                        "description": "Scan for CWE Top 25 Most Dangerous Software Weaknesses including buffer overflows, injection, improper input validation.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name or path"
                                },
                                "path": {
                                    "type": "string",
                                    "description": "Optional specific file or directory path to scan"
                                }
                            },
                            "required": ["repo"]
                        }
                    },
                    {
                        "name": "explain_vulnerability",
                        "description": "Get detailed explanation of a security vulnerability type including examples, references, and remediation guidance.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "rule_id": {
                                    "type": "string",
                                    "description": "Rule ID to explain (e.g., OWASP-A03-001, CWE-89-001)"
                                },
                                "cwe": {
                                    "type": "string",
                                    "description": "CWE ID to explain (e.g., CWE-89, CWE-79)"
                                }
                            }
                        }
                    },
                    {
                        "name": "suggest_fix",
                        "description": "Get suggested fixes for a specific security finding.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name or path"
                                },
                                "path": {
                                    "type": "string",
                                    "description": "File path containing the vulnerability"
                                },
                                "line": {
                                    "type": "integer",
                                    "description": "Line number of the vulnerability"
                                },
                                "rule_id": {
                                    "type": "string",
                                    "description": "Rule ID that detected the issue"
                                }
                            },
                            "required": ["repo", "path", "line"]
                        }
                    },
                    // Phase 5: Supply Chain Security
                    {
                        "name": "generate_sbom",
                        "description": "Generate a Software Bill of Materials (SBOM) for a project. Supports CycloneDX and SPDX formats. Parses Cargo.toml, package.json, requirements.txt, and go.mod.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name or path"
                                },
                                "format": {
                                    "type": "string",
                                    "enum": ["cyclonedx", "spdx", "json"],
                                    "description": "Output format (default: cyclonedx)"
                                },
                                "compact": {
                                    "type": "boolean",
                                    "description": "Output minified JSON without whitespace (~25% smaller). Useful for large projects. (default: false)"
                                }
                            },
                            "required": ["repo"]
                        }
                    },
                    {
                        "name": "check_dependencies",
                        "description": "Check project dependencies for known vulnerabilities using the OSV (Open Source Vulnerabilities) database. Returns CVE/GHSA IDs and recommended upgrades.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name or path"
                                },
                                "severity_threshold": {
                                    "type": "string",
                                    "enum": ["critical", "high", "medium", "low"],
                                    "description": "Minimum severity level to report (default: low)"
                                },
                                "include_dev": {
                                    "type": "boolean",
                                    "description": "Include dev dependencies (default: true)"
                                }
                            },
                            "required": ["repo"]
                        }
                    },
                    {
                        "name": "check_licenses",
                        "description": "Analyze dependency licenses for compliance issues. Detects copyleft licenses, unknown licenses, and license compatibility problems.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name or path"
                                },
                                "project_license": {
                                    "type": "string",
                                    "description": "SPDX identifier for your project's license (e.g., MIT, Apache-2.0)"
                                },
                                "fail_on_copyleft": {
                                    "type": "boolean",
                                    "description": "Treat copyleft licenses as issues (default: false)"
                                }
                            },
                            "required": ["repo"]
                        }
                    },
                    {
                        "name": "find_upgrade_path",
                        "description": "Find safe upgrade paths for vulnerable dependencies. Shows which versions fix known vulnerabilities and whether upgrades have breaking changes.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name or path"
                                },
                                "dependency": {
                                    "type": "string",
                                    "description": "Optional: specific dependency to check (checks all vulnerable deps if omitted)"
                                }
                            },
                            "required": ["repo"]
                        }
                    },
                    // Phase 6: Advanced Features
                    {
                        "name": "get_import_graph",
                        "description": "Build and analyze the import/dependency graph for a codebase. Shows which files import which other files, helps identify circular dependencies.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name"
                                },
                                "file": {
                                    "type": "string",
                                    "description": "Optional: focus on imports from/to a specific file"
                                },
                                "direction": {
                                    "type": "string",
                                    "enum": ["imports", "importers", "both"],
                                    "description": "Direction to show: what this file imports, what imports this file, or both (default: both)"
                                }
                            },
                            "required": ["repo"]
                        }
                    },
                    {
                        "name": "find_circular_imports",
                        "description": "Detect circular import dependencies in the codebase. Returns all cycles with the files involved.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name"
                                }
                            },
                            "required": ["repo"]
                        }
                    },
                    {
                        "name": "workspace_symbol_search",
                        "description": "Fuzzy search for symbols across the entire workspace. Uses trigram matching for typo-tolerant search.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "query": {
                                    "type": "string",
                                    "description": "Symbol name or partial name to search for"
                                },
                                "kind": {
                                    "type": "string",
                                    "enum": ["function", "class", "struct", "interface", "enum", "variable", "all"],
                                    "description": "Filter by symbol kind (default: all)"
                                },
                                "limit": {
                                    "type": "integer",
                                    "description": "Maximum results to return (default: 20)"
                                }
                            },
                            "required": ["query"]
                        }
                    },
                    {
                        "name": "get_incremental_status",
                        "description": "Get status of incremental indexing including Merkle tree root hash, file counts, and change statistics.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name"
                                }
                            },
                            "required": ["repo"]
                        }
                    },
                    {
                        "name": "find_symbol_usages",
                        "description": "Find all usages of a symbol across files, including imports and re-exports. Cross-language aware for JS/TS projects.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name"
                                },
                                "symbol": {
                                    "type": "string",
                                    "description": "Symbol name to find usages of"
                                },
                                "include_imports": {
                                    "type": "boolean",
                                    "description": "Include import statements (default: true)"
                                }
                            },
                            "required": ["repo", "symbol"]
                        }
                    },
                    {
                        "name": "get_export_map",
                        "description": "Get the export map for a file or module showing all exported symbols and their types.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "repo": {
                                    "type": "string",
                                    "description": "Repository name"
                                },
                                "path": {
                                    "type": "string",
                                    "description": "File path to get exports for"
                                }
                            },
                            "required": ["repo", "path"]
                        }
                    }
                ]
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