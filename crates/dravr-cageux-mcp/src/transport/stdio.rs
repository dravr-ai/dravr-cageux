// ABOUTME: Stdio transport reading JSON-RPC from stdin and writing to stdout
// ABOUTME: Line-delimited JSON protocol for MCP CLI integration
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{error, info};

use crate::protocol::JsonRpcRequest;
use crate::server::McpServer;
use crate::transport::McpTransport;

/// Stdio transport for MCP communication
pub struct StdioTransport;

#[async_trait::async_trait]
impl McpTransport for StdioTransport {
    async fn serve(&self, server: Arc<McpServer>) -> Result<(), Box<dyn std::error::Error>> {
        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let reader = BufReader::new(stdin);
        let mut lines = reader.lines();

        info!("MCP stdio transport ready");

        while let Ok(Some(line)) = lines.next_line().await {
            let line = line.trim().to_owned();
            if line.is_empty() {
                continue;
            }

            let request: JsonRpcRequest = match serde_json::from_str(&line) {
                Ok(r) => r,
                Err(e) => {
                    error!("Failed to parse JSON-RPC request: {e}");
                    let error_response = serde_json::json!({
                        "jsonrpc": "2.0",
                        "error": { "code": -32700, "message": format!("Parse error: {e}") },
                        "id": null
                    });
                    let response_str = serde_json::to_string(&error_response)?;
                    stdout.write_all(response_str.as_bytes()).await?;
                    stdout.write_all(b"\n").await?;
                    stdout.flush().await?;
                    continue;
                }
            };

            let response = server.handle_request(request).await;
            let response_str = serde_json::to_string(&response)?;
            stdout.write_all(response_str.as_bytes()).await?;
            stdout.write_all(b"\n").await?;
            stdout.flush().await?;
        }

        info!("MCP stdio transport closed");
        Ok(())
    }
}
