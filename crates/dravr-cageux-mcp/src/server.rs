// ABOUTME: MCP server routing JSON-RPC methods to handlers
// ABOUTME: Handles initialize, tools/list, tools/call, and ping methods
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

use serde_json::{json, Value};
use tracing::info;

use crate::protocol::{
    JsonRpcRequest, JsonRpcResponse, PROTOCOL_VERSION, SERVER_NAME, SERVER_VERSION,
};
use crate::state::SharedState;
use crate::tools::ToolRegistry;

/// MCP server that routes JSON-RPC requests to tool handlers
pub struct McpServer {
    state: SharedState,
    tools: ToolRegistry,
}

impl McpServer {
    /// Create a new MCP server with the given state and tools
    #[must_use]
    pub fn new(state: SharedState, tools: ToolRegistry) -> Self {
        Self { state, tools }
    }

    /// Handle a JSON-RPC request and return a response
    pub async fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        match request.method.as_str() {
            "initialize" => self.handle_initialize(request.id).await,
            "tools/list" => self.handle_tools_list(request.id),
            "tools/call" => self.handle_tools_call(request.id, request.params).await,
            "ping" => JsonRpcResponse::success(request.id, json!({})),
            _ => JsonRpcResponse::error(
                request.id,
                -32601,
                format!("Unknown method: {}", request.method),
            ),
        }
    }

    async fn handle_initialize(&self, id: Option<Value>) -> JsonRpcResponse {
        {
            let mut state = self.state.write().await;
            state.set_initialized();
        }

        info!("MCP server initialized: {SERVER_NAME} v{SERVER_VERSION}");

        JsonRpcResponse::success(
            id,
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": SERVER_NAME,
                    "version": SERVER_VERSION
                }
            }),
        )
    }

    fn handle_tools_list(&self, id: Option<Value>) -> JsonRpcResponse {
        let definitions = self.tools.list_definitions();
        JsonRpcResponse::success(
            id,
            json!({
                "tools": definitions
            }),
        )
    }

    async fn handle_tools_call(&self, id: Option<Value>, params: Option<Value>) -> JsonRpcResponse {
        let Some(params) = params else {
            return JsonRpcResponse::error(id, -32602, "Missing params for tools/call");
        };

        let Some(name) = params.get("name").and_then(Value::as_str) else {
            return JsonRpcResponse::error(id, -32602, "Missing 'name' in tools/call params");
        };
        let name = name.to_owned();

        let arguments = params
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));

        let result = self.tools.execute(&name, &self.state, arguments).await;
        JsonRpcResponse::success(id, serde_json::to_value(result).unwrap_or(json!(null)))
    }
}
