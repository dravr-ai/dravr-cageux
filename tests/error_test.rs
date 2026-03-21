// ABOUTME: Unit tests for IntelligenceError structured error types
// ABOUTME: Validates error construction, display formatting, and conversions
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

use dravr_cageux::error::{IntelligenceError, IntelligenceResult};

#[test]
fn invalid_input_single_arg() {
    let err = IntelligenceError::invalid_input("Age must be positive");
    let msg = err.to_string();
    assert!(msg.contains("Age must be positive"), "got: {msg}");
    assert!(
        msg.contains("input"),
        "should default field to 'input': {msg}"
    );
}

#[test]
fn invalid_input_field_two_args() {
    let err = IntelligenceError::invalid_input_field("age", "must be between 1 and 120");
    let msg = err.to_string();
    assert!(msg.contains("age"), "got: {msg}");
    assert!(msg.contains("must be between 1 and 120"), "got: {msg}");
}

#[test]
fn insufficient_data_error() {
    let err = IntelligenceError::insufficient_data(7, 3);
    let msg = err.to_string();
    assert!(msg.contains('7'), "should contain required count: {msg}");
    assert!(msg.contains('3'), "should contain actual count: {msg}");
}

#[test]
fn value_out_of_range_error() {
    let err = IntelligenceError::out_of_range("heart_rate", 250.0, 30.0, 220.0);
    let msg = err.to_string();
    assert!(msg.contains("heart_rate"), "got: {msg}");
    assert!(msg.contains("250"), "got: {msg}");
}

#[test]
fn algorithm_failure_error() {
    let err = IntelligenceError::algorithm_failure("VDOT", "pace too slow for model");
    let msg = err.to_string();
    assert!(msg.contains("VDOT"), "got: {msg}");
    assert!(msg.contains("pace too slow"), "got: {msg}");
}

#[test]
fn division_by_zero_error() {
    let err = IntelligenceError::division_by_zero("TSS calculation");
    let msg = err.to_string();
    assert!(msg.contains("TSS calculation"), "got: {msg}");
}

#[test]
fn configuration_error() {
    let err = IntelligenceError::configuration("missing training zones");
    let msg = err.to_string();
    assert!(msg.contains("missing training zones"), "got: {msg}");
}

#[test]
fn internal_error() {
    let err = IntelligenceError::internal("unexpected state");
    let msg = err.to_string();
    assert!(msg.contains("unexpected state"), "got: {msg}");
}

#[test]
fn result_type_works_with_question_mark() {
    fn fallible() -> IntelligenceResult<f64> {
        let value = 42.0_f64;
        if value < 0.0 {
            return Err(IntelligenceError::invalid_input("must be positive"));
        }
        Ok(value)
    }

    assert!((fallible().unwrap() - 42.0).abs() < f64::EPSILON);
}

#[test]
fn serialization_error_from_serde() {
    let result: Result<serde_json::Value, IntelligenceError> =
        serde_json::from_str("not json").map_err(IntelligenceError::from);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("serialization"), "got: {msg}");
}
