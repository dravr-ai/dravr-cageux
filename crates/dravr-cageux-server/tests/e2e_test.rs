// ABOUTME: E2E tests for MCP and REST server endpoints
// ABOUTME: Tests health check, MCP initialize, tools/list, and tools/call via HTTP
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tokio::sync::RwLock;
use tower::ServiceExt;

use dravr_cageux_mcp::state::ServerState;
use dravr_cageux_mcp::{build_tool_registry, McpServer};
use dravr_cageux_server::router::build_router;

fn create_test_app() -> axum::Router {
    let state = Arc::new(RwLock::new(ServerState::new()));
    let tools = build_tool_registry();
    let mcp_server = Arc::new(McpServer::new(
        "dravr-cageux",
        env!("CARGO_PKG_VERSION"),
        tools,
        state,
    ));
    build_router(mcp_server)
}

#[tokio::test]
async fn health_check_returns_ok() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "ok");
    assert_eq!(json["service"], "dravr-cageux");
}

#[tokio::test]
async fn mcp_initialize_returns_capabilities() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&json!({
                        "jsonrpc": "2.0",
                        "method": "initialize",
                        "params": {
                            "protocolVersion": "2024-11-05",
                            "capabilities": {},
                            "clientInfo": { "name": "test-client" }
                        },
                        "id": 1
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["jsonrpc"], "2.0");
    assert!(json["result"]["capabilities"]["tools"].is_object());
    assert_eq!(json["result"]["serverInfo"]["name"], "dravr-cageux");
    assert_eq!(json["id"], 1);
}

#[tokio::test]
async fn mcp_tools_list_returns_array() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&json!({
                        "jsonrpc": "2.0",
                        "method": "tools/list",
                        "id": 2
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["jsonrpc"], "2.0");
    assert!(json["result"]["tools"].is_array());
    assert_eq!(json["id"], 2);
}

#[tokio::test]
async fn mcp_unknown_method_returns_error() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&json!({
                        "jsonrpc": "2.0",
                        "method": "nonexistent/method",
                        "id": 3
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json["error"].is_object());
    assert_eq!(json["error"]["code"], -32601);
    assert_eq!(json["id"], 3);
}

#[tokio::test]
async fn mcp_ping_returns_empty_result() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&json!({
                        "jsonrpc": "2.0",
                        "method": "ping",
                        "id": 4
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json["result"].is_object());
    assert_eq!(json["id"], 4);
}

#[tokio::test]
async fn mcp_tools_call_unknown_tool_returns_error() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&json!({
                        "jsonrpc": "2.0",
                        "method": "tools/call",
                        "params": {
                            "name": "nonexistent_tool",
                            "arguments": {}
                        },
                        "id": 5
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    // The result contains an error from the tool registry
    let result = &json["result"];
    assert!(result["isError"] == true);
}
