// ABOUTME: Unit tests for server configuration from environment variables
// ABOUTME: Validates defaults, env var parsing, and error handling
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::str_to_string
)]

use dravr_cageux::config::ServerConfig;

#[test]
fn server_config_default_values() {
    let config = ServerConfig::default();
    assert_eq!(config.host, "127.0.0.1");
    assert_eq!(config.port, 3100);
    assert_eq!(config.transport, "http");
    assert!(config.api_token.is_none());
}

#[test]
fn server_config_addr() {
    let config = ServerConfig {
        host: "0.0.0.0".to_owned(),
        port: 8080,
        transport: "http".to_owned(),
        api_token: None,
    };
    assert_eq!(config.addr(), "0.0.0.0:8080");
}
