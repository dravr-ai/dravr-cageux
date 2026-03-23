// ABOUTME: Bearer token authentication middleware for REST API
// ABOUTME: Validates CAGEUX_API_TOKEN from request Authorization header via dravr-tronc
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;

/// Bearer token authentication middleware.
///
/// Reads `CAGEUX_API_TOKEN` from environment. If unset or empty, all requests
/// pass through (auth disabled). Otherwise, validates the `Authorization: Bearer <token>` header.
pub async fn require_auth(request: Request, next: Next) -> Response {
    dravr_tronc::server::auth::require_auth("CAGEUX_API_TOKEN", request, next).await
}
