// ABOUTME: JSON-RPC 2.0 protocol types for MCP communication
// ABOUTME: Request/response types, tool definitions, and MCP protocol constants
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// MCP protocol version
pub const PROTOCOL_VERSION: &str = "2024-11-05";

/// Server name for MCP initialization
pub const SERVER_NAME: &str = "dravr-cageux";

/// Server version from Cargo manifest
pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// JSON-RPC 2.0 request
#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    /// Protocol version (must be "2.0")
    pub jsonrpc: String,
    /// Method name (e.g., "initialize", "tools/list", "tools/call")
    pub method: String,
    /// Optional parameters
    #[serde(default)]
    pub params: Option<Value>,
    /// Request identifier
    pub id: Option<Value>,
}

/// JSON-RPC 2.0 response
#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    /// Protocol version
    pub jsonrpc: String,
    /// Result (mutually exclusive with error)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// Error (mutually exclusive with result)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    /// Request identifier (echoed back)
    pub id: Option<Value>,
}

/// JSON-RPC 2.0 error object
#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    /// Error code
    pub code: i32,
    /// Error message
    pub message: String,
    /// Additional error data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// MCP tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Tool name (used in tools/call)
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// JSON Schema for input parameters
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

/// Result of a tool execution
#[derive(Debug, Serialize)]
pub struct CallToolResult {
    /// Content returned by the tool
    pub content: Vec<ToolContent>,
    /// Whether the tool execution resulted in an error
    #[serde(rename = "isError", skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

/// Content item in a tool result
#[derive(Debug, Serialize)]
pub struct ToolContent {
    /// Content type (always "text" for now)
    #[serde(rename = "type")]
    pub content_type: String,
    /// Text content
    pub text: String,
}

impl JsonRpcResponse {
    /// Create a success response
    #[must_use]
    pub fn success(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_owned(),
            result: Some(result),
            error: None,
            id,
        }
    }

    /// Create an error response
    #[must_use]
    pub fn error(id: Option<Value>, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".to_owned(),
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data: None,
            }),
            id,
        }
    }
}

impl CallToolResult {
    /// Create a successful text result
    #[must_use]
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent {
                content_type: "text".to_owned(),
                text: content.into(),
            }],
            is_error: None,
        }
    }

    /// Create an error result
    #[must_use]
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent {
                content_type: "text".to_owned(),
                text: message.into(),
            }],
            is_error: Some(true),
        }
    }
}
