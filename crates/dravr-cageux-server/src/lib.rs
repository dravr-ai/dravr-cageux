// ABOUTME: Unified REST API + MCP server library for dravr-cageux
// ABOUTME: Re-exports router, health, auth, and state modules
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

//! # dravr-cageux-server
//!
//! Unified server exposing sports science intelligence via REST API and MCP.
//! Combines the core library's algorithms with HTTP endpoints and MCP protocol support.

/// Authentication middleware for bearer token validation
pub mod auth;
/// Health check endpoint
pub mod health;
/// Axum router combining REST and MCP routes
pub mod router;
