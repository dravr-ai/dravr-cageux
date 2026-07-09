// ABOUTME: Unit tests for insight_adapter helpers (string truncation)
// ABOUTME: Validates char-safe truncation never panics on multibyte UTF-8 input
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::str_to_string
)]

use dravr_cageux::insight_adapter::truncate_string;

#[test]
fn truncate_string_keeps_short_input() {
    assert_eq!(truncate_string("short", 10), "short");
}

#[test]
fn truncate_string_truncates_ascii_with_ellipsis() {
    // 11 bytes > max_len 8 -> first 5 chars + "..." = 8 chars.
    assert_eq!(truncate_string("hello world", 8), "hello...");
}

#[test]
fn truncate_string_does_not_panic_on_multibyte_boundary() {
    // Each 'é' is 2 bytes: 20 bytes, 10 chars. max_len 10 forces a cut at
    // byte 7 (max_len - 3), which is NOT a char boundary. Byte-slicing here
    // would panic; char-based truncation must not.
    let input = "é".repeat(10);
    let result = truncate_string(&input, 10);

    assert!(result.ends_with("..."));
    // 7 kept chars + "..." — verify it is valid UTF-8 (String guarantees it).
    assert_eq!(result, format!("{}...", "é".repeat(7)));
}
