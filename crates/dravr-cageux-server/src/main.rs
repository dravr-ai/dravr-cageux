// ABOUTME: Unified CLI entry point for dravr-cageux server
// ABOUTME: Supports serve (REST+MCP), MCP stdio, and future CLI commands
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

use std::error::Error;
use std::sync::Arc;

use clap::{Parser, Subcommand};
use tokio::net::TcpListener;
use tracing::info;

use dravr_cageux::config::ServerConfig;
use dravr_cageux_mcp::state::ServerState;
use dravr_cageux_mcp::{build_tool_registry, McpServer};
use dravr_cageux_server::router::build_router;
use dravr_tronc::mcp::transport::stdio;
use dravr_tronc::server::tracing_init;

#[derive(Parser)]
#[command(
    name = "dravr-cageux-server",
    version,
    about = "Sports science intelligence server"
)]
struct Cli {
    /// Transport mode for MCP (when no subcommand)
    #[arg(long, default_value = "http")]
    transport: String,

    /// HTTP host
    #[arg(long)]
    host: Option<String>,

    /// HTTP port
    #[arg(long)]
    port: Option<u16>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Start the REST + MCP HTTP server
    Serve {
        /// HTTP host
        #[arg(long)]
        host: Option<String>,
        /// HTTP port
        #[arg(long)]
        port: Option<u16>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let cli = Cli::parse();

    tracing_init::init_with_notifications(&cli.transport);

    let config = ServerConfig::from_env().unwrap_or_default();

    let state = Arc::new(ServerState::new());
    let tools = build_tool_registry();
    let mcp_server = Arc::new(McpServer::new(
        "dravr-cageux",
        env!("CARGO_PKG_VERSION"),
        tools,
        state,
    ));

    match cli.command {
        Some(Command::Serve { host, port }) => {
            let host = host.unwrap_or(config.host);
            let port = port.unwrap_or(config.port);
            serve_http(mcp_server, &host, port).await?;
        }
        None => {
            let host = cli.host.unwrap_or(config.host);
            let port = cli.port.unwrap_or(config.port);
            if cli.transport == "stdio" {
                stdio::run(mcp_server).await?;
            } else {
                serve_http(mcp_server, &host, port).await?;
            }
        }
    }

    Ok(())
}

async fn serve_http(
    mcp_server: Arc<McpServer<ServerState>>,
    host: &str,
    port: u16,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let app = build_router(mcp_server);
    let addr = format!("{host}:{port}");
    let listener = TcpListener::bind(&addr).await?;

    info!("dravr-cageux server listening on {addr}");
    info!("  Health: GET http://{addr}/health");
    info!("  MCP:    POST http://{addr}/mcp");

    axum::serve(listener, app).await?;
    Ok(())
}
