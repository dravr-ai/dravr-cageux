// ABOUTME: Health check endpoint returning server status
// ABOUTME: Simple GET /health handler for monitoring and readiness probes
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

use axum::http::StatusCode;
use axum::Json;
use serde_json::{json, Value};

/// Health check handler returning server status
pub async fn health_check() -> (StatusCode, Json<Value>) {
    (
        StatusCode::OK,
        Json(json!({
            "status": "ok",
            "service": "dravr-cageux",
            "version": env!("CARGO_PKG_VERSION")
        })),
    )
}
