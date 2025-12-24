#![recursion_limit = "256"]
// Allow dead code - this binary is an MCP server that exposes only a subset of the library's features.
// Many library features (custom rulesets, direct analysis APIs, etc.) are intentionally available
// for integration use but not wired through MCP tools.
#![allow(dead_code)]

mod callgraph;
mod cfg;
mod chunking;
mod dfg;
mod embeddings;
mod extract;
mod git;
mod http_server;
mod hybrid_search;
mod incremental;
mod index;
mod lsp;
mod mcp;
mod metrics;
mod neural;
mod parser;
mod persist;
mod remote;
mod repo;
mod search;
mod security_config;
mod security_rules;
mod streaming;
mod supply_chain;
mod tool_handlers;
mod symbols;
mod taint;
mod type_inference;

use anyhow::Result;
use clap::Parser as ClapParser;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

#[derive(ClapParser, Debug)]
#[command(name = "narsil-mcp")]
#[command(version = "1.0.0")]
#[command(about = "Blazingly fast MCP server for code intelligence")]
struct Args {
    /// Paths to repositories or directories to index
    #[arg(short, long)]
    repos: Vec<PathBuf>,

    /// Path to persistent index storage
    #[arg(short, long, default_value = "~/.cache/narsil-mcp")]
    index_path: PathBuf,

    /// Enable verbose logging (to stderr)
    #[arg(short, long)]
    verbose: bool,

    /// Re-index all repositories on startup
    #[arg(long)]
    reindex: bool,

    /// Enable watch mode for incremental updates
    #[arg(short, long)]
    watch: bool,

    /// Enable call graph analysis (slower initial index)
    #[arg(long)]
    call_graph: bool,

    /// Enable git integration
    #[arg(long)]
    git: bool,

    /// Auto-discover repositories in a directory
    #[arg(long)]
    discover: Option<PathBuf>,

    /// Enable index persistence (save/load index to/from disk)
    #[arg(short, long)]
    persist: bool,

    /// Enable LSP integration for enhanced code intelligence (requires language servers installed)
    #[arg(long)]
    lsp: bool,

    /// Enable streaming responses for large result sets
    #[arg(long)]
    streaming: bool,

    /// Enable remote GitHub repository support (uses GITHUB_TOKEN env var for auth)
    #[arg(long)]
    remote: bool,

    /// Enable neural embeddings for semantic search (requires EMBEDDING_API_KEY, VOYAGE_API_KEY, or OPENAI_API_KEY)
    #[arg(long)]
    neural: bool,

    /// Neural embedding backend: "api" (default) or "onnx"
    #[arg(long, default_value = "api")]
    neural_backend: String,

    /// Neural embedding model name (e.g., "voyage-code-2", "text-embedding-3-small")
    #[arg(long)]
    neural_model: Option<String>,

    /// Enable HTTP server for visualization frontend
    #[arg(long)]
    http: bool,

    /// HTTP server port (default: 3000)
    #[arg(long, default_value = "3000")]
    http_port: u16,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging to stderr (stdout is for MCP protocol)
    let level = if args.verbose {
        Level::DEBUG
    } else {
        Level::INFO
    };
    let subscriber = FmtSubscriber::builder()
        .with_max_level(level)
        .with_writer(std::io::stderr)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting narsil-mcp v1.0.0");

    // Handle repository discovery if requested
    let mut repos = args.repos;
    if let Some(discover_path) = args.discover {
        info!("Discovering repositories in: {:?}", discover_path);
        let discovered = repo::discover_repos(&discover_path, 3)?;
        info!("Found {} repositories", discovered.len());
        repos.extend(discovered);
    }

    info!("Repos to index: {:?}", repos);
    info!(
        "Features: call_graph={}, git={}, watch={}, persist={}, lsp={}, streaming={}, remote={}, neural={}",
        args.call_graph, args.git, args.watch, args.persist, args.lsp, args.streaming, args.remote, args.neural
    );

    // Build LSP config
    let mut lsp_config = lsp::LspConfig::default();
    if args.lsp {
        lsp_config.enabled = true;
        // Enable LSP for common languages
        for lang in [
            "rust",
            "python",
            "typescript",
            "javascript",
            "go",
            "c",
            "cpp",
            "java",
        ] {
            lsp_config.enabled_languages.insert(lang.to_string(), true);
        }
        info!(
            "LSP integration enabled for: {:?}",
            lsp_config.enabled_languages.keys().collect::<Vec<_>>()
        );
    }

    // Build streaming config
    let streaming_config = streaming::StreamingConfig {
        enabled: args.streaming,
        ..Default::default()
    };
    if args.streaming {
        info!(
            "Streaming responses enabled (threshold: {} items)",
            streaming_config.auto_stream_threshold
        );
    }

    // Build neural config
    let neural_config = neural::NeuralConfig {
        enabled: args.neural,
        backend: args.neural_backend.clone(),
        model_name: args.neural_model.clone(),
        ..Default::default()
    };
    if args.neural {
        info!(
            "Neural embeddings requested (backend={}, model={:?})",
            args.neural_backend,
            args.neural_model
        );
    }

    // Initialize the code intelligence engine with options
    let options = index::EngineOptions {
        git_enabled: args.git,
        call_graph_enabled: args.call_graph,
        persist_enabled: args.persist,
        watch_enabled: args.watch,
        streaming_config,
        lsp_config,
        neural_config,
    };
    let mut engine = index::CodeIntelEngine::with_options(args.index_path, repos, options).await?;

    // Initialize remote repository support if enabled
    if args.remote {
        match engine.init_remote_manager() {
            Ok(()) => info!("Remote repository support enabled"),
            Err(e) => warn!("Failed to initialize remote repository support: {}", e),
        }
    }

    let engine = Arc::new(engine);

    if args.reindex {
        info!("Re-indexing all repositories...");
        engine.reindex_all().await?;
    }

    // Start watch mode in background if enabled
    if args.watch {
        let watch_engine = Arc::clone(&engine);

        // Create a shutdown signal channel for graceful shutdown
        let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(1);

        tokio::spawn(async move {
            run_watch_mode(watch_engine, shutdown_rx).await;
        });

        // Store shutdown sender for potential cleanup (not used currently but available for future)
        drop(shutdown_tx);
    }

    // Start HTTP server if enabled
    if args.http {
        info!("Starting HTTP server on port {}", args.http_port);
        let http_server = http_server::HttpServer::new(Arc::clone(&engine), args.http_port);
        http_server.run().await?;
    } else {
        // Start the MCP server on stdio
        let server = mcp::McpServer::from_arc(engine);
        server.run().await?;
    }

    Ok(())
}

/// Run the file watcher in background using async event-driven approach
async fn run_watch_mode(
    engine: Arc<index::CodeIntelEngine>,
    mut shutdown: tokio::sync::broadcast::Receiver<()>,
) {
    info!("Starting async watch mode background task");

    let (_watcher, mut rx) = match engine.create_async_file_watcher() {
        Some((w, r)) => (w, r),
        None => {
            warn!("Failed to create async file watcher, watch mode disabled");
            return;
        }
    };

    loop {
        tokio::select! {
            // Receive batched file change events
            Some(changes) = rx.recv() => {
                if !changes.is_empty() {
                    info!("Detected {} file change(s)", changes.len());
                    match engine.process_file_changes(&changes).await {
                        Ok(count) => {
                            if count > 0 {
                                info!("Re-indexed {} file(s)", count);
                            }
                        }
                        Err(e) => {
                            warn!("Error processing file changes: {}", e);
                        }
                    }
                }
            }
            // Handle shutdown signal
            _ = shutdown.recv() => {
                info!("Watch mode shutting down");
                break;
            }
        }
    }
}