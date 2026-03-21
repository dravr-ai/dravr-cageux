// ABOUTME: Bearer token authentication middleware for REST API
// ABOUTME: Validates CAGEUX_API_TOKEN from request Authorization header
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::Response;

/// Bearer token authentication middleware.
///
/// If `expected_token` is `None`, all requests are allowed (auth disabled).
/// Otherwise, validates the `Authorization: Bearer <token>` header.
pub async fn auth_middleware(
    request: Request,
    next: Next,
    expected_token: Option<String>,
) -> Result<Response, StatusCode> {
    let Some(token) = expected_token else {
        // Auth disabled — pass through
        return Ok(next.run(request).await);
    };

    let auth_header = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok());

    match auth_header {
        Some(header) if header.strip_prefix("Bearer ").is_some_and(|t| t == token) => {
            Ok(next.run(request).await)
        }
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}
