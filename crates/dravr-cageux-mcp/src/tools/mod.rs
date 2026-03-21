// ABOUTME: MCP tool trait and registry for sports science intelligence tools
// ABOUTME: Pluggable tool architecture with definition/execute dispatch pattern
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::protocol::{CallToolResult, ToolDefinition};
use crate::state::SharedState;

/// Trait for MCP tools that can be registered and executed
#[async_trait]
pub trait McpTool: Send + Sync {
    /// Return the tool definition (name, description, schema)
    fn definition(&self) -> ToolDefinition;

    /// Execute the tool with given arguments
    async fn execute(&self, state: &SharedState, arguments: Value) -> CallToolResult;
}

/// Registry of available MCP tools
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn McpTool>>,
}

impl ToolRegistry {
    /// Create a new empty registry
    #[must_use]
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Register a tool
    pub fn register(&mut self, tool: Arc<dyn McpTool>) {
        let name = tool.definition().name;
        self.tools.insert(name, tool);
    }

    /// List all tool definitions
    #[must_use]
    pub fn list_definitions(&self) -> Vec<ToolDefinition> {
        self.tools.values().map(|t| t.definition()).collect()
    }

    /// Execute a tool by name
    pub async fn execute(
        &self,
        name: &str,
        state: &SharedState,
        arguments: Value,
    ) -> CallToolResult {
        match self.tools.get(name) {
            Some(tool) => tool.execute(state, arguments).await,
            None => CallToolResult::error(format!("Unknown tool: {name}")),
        }
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Build the default tool registry with all available intelligence tools
#[must_use]
pub fn build_tool_registry() -> ToolRegistry {
    ToolRegistry::new()
}
