// ABOUTME: CLI entry point for the dravr-cageux MCP server
// ABOUTME: Supports stdio and HTTP transport modes via --transport flag
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

use std::sync::Arc;

use clap::Parser;
use tracing::info;

use dravr_cageux_mcp::state::create_shared_state;
use dravr_cageux_mcp::transport::http::HttpTransport;
use dravr_cageux_mcp::transport::stdio::StdioTransport;
use dravr_cageux_mcp::transport::McpTransport;
use dravr_cageux_mcp::{build_tool_registry, McpServer};

#[derive(Parser)]
#[command(
    name = "dravr-cageux-mcp",
    version,
    about = "Sports science MCP server"
)]
struct Cli {
    /// Transport mode: stdio or http
    #[arg(long, default_value = "stdio")]
    transport: String,

    /// HTTP host (only for http transport)
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// HTTP port (only for http transport)
    #[arg(long, default_value = "3100")]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();

    let state = create_shared_state();
    let tools = build_tool_registry();
    let server = Arc::new(McpServer::new(state, tools));

    info!(
        "Starting dravr-cageux MCP server (transport: {})",
        cli.transport
    );

    match cli.transport.as_str() {
        "stdio" => StdioTransport.serve(server).await?,
        "http" => {
            HttpTransport {
                host: cli.host,
                port: cli.port,
            }
            .serve(server)
            .await?;
        }
        other => {
            eprintln!("Unknown transport: {other}. Use 'stdio' or 'http'.");
            std::process::exit(1);
        }
    }

    Ok(())
}
