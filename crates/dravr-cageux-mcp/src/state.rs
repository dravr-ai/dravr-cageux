// ABOUTME: Shared server state for the MCP server
// ABOUTME: Thread-safe state container holding intelligence engine configuration
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

use std::sync::Arc;

/// Thread-safe shared state type alias
pub type SharedState = Arc<ServerState>;

/// Server state holding intelligence engine configuration
#[derive(Debug)]
pub struct ServerState {}

impl ServerState {
    /// Create a new server state
    #[must_use]
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for ServerState {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a new shared state instance
#[must_use]
pub fn create_shared_state() -> SharedState {
    Arc::new(ServerState::new())
}
