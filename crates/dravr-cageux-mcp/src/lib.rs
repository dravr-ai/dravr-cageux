// ABOUTME: MCP server library exposing sports science intelligence via Model Context Protocol
// ABOUTME: JSON-RPC 2.0 protocol with stdio and HTTP transports via dravr-tronc shared infrastructure
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

//! # dravr-cageux-mcp
//!
//! MCP (Model Context Protocol) server for the dravr-cageux sports science engine.
//! Exposes intelligence algorithms as MCP tools via JSON-RPC 2.0.

/// Shared server state
pub mod state;
/// MCP tool definitions and registry
pub mod tools;

pub use dravr_tronc::McpServer;
pub use state::{ServerState, SharedState};
pub use tools::build_tool_registry;
