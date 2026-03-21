// ABOUTME: Server and engine configuration loaded from environment variables
// ABOUTME: Follows .envrc pattern with sensible defaults for all settings
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

//! Configuration types for the dravr-cageux server and intelligence engine.
//!
//! All configuration is loaded from environment variables (typically set via `.envrc`
//! and direnv). Each config struct provides a `from_env()` method that reads the
//! environment and falls back to sensible defaults.

use std::env;

use crate::error::{IntelligenceError, IntelligenceResult};

/// Server configuration for the REST API and MCP server.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Host address to bind to (default: 127.0.0.1)
    pub host: String,
    /// Port to listen on (default: 3100)
    pub port: u16,
    /// MCP transport mode: "stdio" or "http" (default: http)
    pub transport: String,
    /// Bearer token for REST API authentication (empty = no auth)
    pub api_token: Option<String>,
}

impl ServerConfig {
    /// Load server config from environment variables.
    ///
    /// | Variable | Default | Description |
    /// |----------|---------|-------------|
    /// | `CAGEUX_HOST` | `127.0.0.1` | Bind address |
    /// | `CAGEUX_PORT` | `3100` | Listen port |
    /// | `CAGEUX_TRANSPORT` | `http` | MCP transport (stdio/http) |
    /// | `CAGEUX_API_TOKEN` | *(none)* | Bearer token for auth |
    pub fn from_env() -> IntelligenceResult<Self> {
        let host = env::var("CAGEUX_HOST").unwrap_or_else(|_| "127.0.0.1".to_owned());
        let port_str = env::var("CAGEUX_PORT").unwrap_or_else(|_| "3100".to_owned());
        let port = port_str.parse::<u16>().map_err(|_| {
            IntelligenceError::configuration(format!(
                "CAGEUX_PORT must be a valid port number, got '{port_str}'"
            ))
        })?;
        let transport = env::var("CAGEUX_TRANSPORT").unwrap_or_else(|_| "http".to_owned());
        let api_token = env::var("CAGEUX_API_TOKEN").ok().filter(|t| !t.is_empty());

        Ok(Self {
            host,
            port,
            transport,
            api_token,
        })
    }

    /// Return the socket address string (host:port).
    #[must_use]
    pub fn addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_owned(),
            port: 3100,
            transport: "http".to_owned(),
            api_token: None,
        }
    }
}
