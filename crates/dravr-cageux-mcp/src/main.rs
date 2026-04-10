// ABOUTME: CLI entry point for the dravr-cageux MCP server
// ABOUTME: Supports stdio and HTTP transport modes via dravr-tronc shared infrastructure
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

use std::error::Error;
use std::process;
use std::sync::Arc;

use clap::Parser;
use tokio::sync::RwLock;
use tracing::info;

use dravr_cageux_mcp::state::ServerState;
use dravr_cageux_mcp::{build_tool_registry, McpServer};
use dravr_tronc::mcp::transport::{http, stdio};
use dravr_tronc::server::{cli::McpArgs, tracing_init};

#[derive(Parser)]
#[command(
    name = "dravr-cageux-mcp",
    version,
    about = "Sports science MCP server"
)]
struct Cli {
    #[command(flatten)]
    server: McpArgs,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let cli = Cli::parse();

    tracing_init::init(&cli.server.transport);

    let state = Arc::new(RwLock::new(ServerState::new()));
    let tools = build_tool_registry();
    let server = Arc::new(McpServer::new(
        "dravr-cageux-mcp",
        env!("CARGO_PKG_VERSION"),
        tools,
        state,
    ));

    info!(
        "Starting dravr-cageux MCP server (transport: {})",
        cli.server.transport
    );

    match cli.server.transport.as_str() {
        "stdio" => stdio::run(server).await?,
        "http" => {
            http::serve(server, &cli.server.host, cli.server.port).await?;
        }
        other => {
            eprintln!("Unknown transport: {other}. Use 'stdio' or 'http'.");
            process::exit(1);
        }
    }

    Ok(())
}
