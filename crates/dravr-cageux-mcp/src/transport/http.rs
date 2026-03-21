// ABOUTME: HTTP transport exposing MCP as a POST /mcp endpoint
// ABOUTME: Axum handler wrapping JSON-RPC request/response over HTTP
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;

use crate::protocol::{JsonRpcRequest, JsonRpcResponse};
use crate::server::McpServer;
use crate::transport::McpTransport;

/// HTTP transport for MCP communication
pub struct HttpTransport {
    /// Host to bind to
    pub host: String,
    /// Port to listen on
    pub port: u16,
}

#[async_trait::async_trait]
impl McpTransport for HttpTransport {
    async fn serve(&self, server: Arc<McpServer>) -> Result<(), Box<dyn std::error::Error>> {
        let app = axum::Router::new()
            .route("/mcp", axum::routing::post(handle_mcp))
            .with_state(server);

        let listener =
            tokio::net::TcpListener::bind(format!("{}:{}", self.host, self.port)).await?;
        tracing::info!(
            "MCP HTTP transport listening on {}:{}",
            self.host,
            self.port
        );
        axum::serve(listener, app).await?;
        Ok(())
    }
}

async fn handle_mcp(
    State(server): State<Arc<McpServer>>,
    Json(request): Json<JsonRpcRequest>,
) -> Result<Json<JsonRpcResponse>, StatusCode> {
    let response = server.handle_request(request).await;
    Ok(Json(response))
}
