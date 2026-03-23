// ABOUTME: Axum router combining REST API endpoints with MCP handler
// ABOUTME: Routes for health check, MCP protocol, and future intelligence endpoints
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

use std::sync::Arc;

use axum::Router;

use dravr_cageux_mcp::state::ServerState;
use dravr_cageux_mcp::McpServer;

use crate::health::health_check;

/// Build the application router with all routes
pub fn build_router(mcp_server: Arc<McpServer<ServerState>>) -> Router {
    let mcp_routes = dravr_tronc::mcp::transport::http::mcp_router(mcp_server);

    Router::new()
        .route("/health", axum::routing::get(health_check))
        .merge(mcp_routes)
}
