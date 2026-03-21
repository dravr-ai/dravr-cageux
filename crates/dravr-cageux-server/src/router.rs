// ABOUTME: Axum router combining REST API endpoints with MCP handler
// ABOUTME: Routes for health check, MCP protocol, and future intelligence endpoints
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use axum::Router;

use dravr_cageux_mcp::protocol::{JsonRpcRequest, JsonRpcResponse};
use dravr_cageux_mcp::server::McpServer;

use crate::health::health_check;

/// Shared application state for route handlers
type AppState = Arc<McpServer>;

/// Build the application router with all routes
pub fn build_router(mcp_server: Arc<McpServer>) -> Router {
    Router::new()
        .route("/health", axum::routing::get(health_check))
        .route("/mcp", axum::routing::post(handle_mcp))
        .with_state(mcp_server)
}

async fn handle_mcp(
    State(server): State<AppState>,
    Json(request): Json<JsonRpcRequest>,
) -> Json<JsonRpcResponse> {
    let response = server.handle_request(request).await;
    Json(response)
}
