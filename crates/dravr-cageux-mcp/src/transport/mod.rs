// ABOUTME: Transport abstractions for MCP server communication
// ABOUTME: Supports stdio (for CLI integration) and HTTP (for REST API)
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

/// Stdio transport for MCP (reads JSON-RPC from stdin, writes to stdout)
pub mod stdio;

/// HTTP transport for MCP (Axum route handler)
pub mod http;

use std::sync::Arc;

use crate::server::McpServer;

/// Trait for MCP transport implementations
#[async_trait::async_trait]
pub trait McpTransport {
    /// Serve the MCP server using this transport
    async fn serve(&self, server: Arc<McpServer>) -> Result<(), Box<dyn std::error::Error>>;
}
