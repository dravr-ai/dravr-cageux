// ABOUTME: Configuration module for dravr-cageux
// ABOUTME: Server config (env vars) and intelligence analysis configuration
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

/// Intelligence module configuration (recommendations, performance, goals, etc.)
pub mod intelligence;
/// Server configuration loaded from environment variables
pub mod server;

pub use intelligence::IntelligenceConfig;
pub use server::ServerConfig;
