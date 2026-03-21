// ABOUTME: Structured error types for the sports science intelligence engine
// ABOUTME: Defines IntelligenceError enum with domain-specific variants and Result alias
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

//! Error types for the dravr-cageux intelligence engine.
//!
//! All errors use structured variants rather than string-based ad-hoc errors.
//! The [`IntelligenceError`] enum covers domain-specific failure modes including
//! invalid input, insufficient data, and algorithm-specific errors.

use thiserror::Error;

/// Result type alias for intelligence operations.
pub type IntelligenceResult<T> = Result<T, IntelligenceError>;

/// Structured error type for the intelligence engine.
///
/// Each variant represents a distinct failure mode with typed fields
/// for programmatic error handling.
#[derive(Debug, Error)]
pub enum IntelligenceError {
    /// Input value is invalid for the requested operation.
    #[error("invalid input for field '{field}': {reason}")]
    InvalidInput {
        /// Name of the invalid field or parameter.
        field: String,
        /// Human-readable explanation of why the input is invalid.
        reason: String,
    },

    /// Not enough data points to perform the requested analysis.
    #[error("insufficient data: need at least {required} data points, got {actual}")]
    InsufficientData {
        /// Minimum number of data points required.
        required: usize,
        /// Actual number of data points provided.
        actual: usize,
    },

    /// A value is outside the valid domain range.
    #[error("value out of range for '{field}': {value} not in [{min}, {max}]")]
    ValueOutOfRange {
        /// Name of the field or parameter.
        field: String,
        /// The out-of-range value.
        value: f64,
        /// Minimum allowed value.
        min: f64,
        /// Maximum allowed value.
        max: f64,
    },

    /// An algorithm-specific calculation failed.
    #[error("algorithm '{algorithm}' failed: {reason}")]
    AlgorithmFailure {
        /// Name of the algorithm that failed.
        algorithm: String,
        /// Reason for the failure.
        reason: String,
    },

    /// Division by zero would occur in a calculation.
    #[error("division by zero in '{context}'")]
    DivisionByZero {
        /// Description of where the division by zero would occur.
        context: String,
    },

    /// Configuration is missing or invalid.
    #[error("configuration error: {reason}")]
    Configuration {
        /// Description of the configuration problem.
        reason: String,
    },

    /// Serialization or deserialization failed.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// An internal error that should not occur under normal conditions.
    #[error("internal error: {0}")]
    Internal(String),
}

impl IntelligenceError {
    /// Create an invalid input error.
    pub fn invalid_input(field: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::InvalidInput {
            field: field.into(),
            reason: reason.into(),
        }
    }

    /// Create an insufficient data error.
    pub fn insufficient_data(required: usize, actual: usize) -> Self {
        Self::InsufficientData { required, actual }
    }

    /// Create a value out of range error.
    pub fn out_of_range(field: impl Into<String>, value: f64, min: f64, max: f64) -> Self {
        Self::ValueOutOfRange {
            field: field.into(),
            value,
            min,
            max,
        }
    }

    /// Create an algorithm failure error.
    pub fn algorithm_failure(algorithm: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::AlgorithmFailure {
            algorithm: algorithm.into(),
            reason: reason.into(),
        }
    }

    /// Create a division by zero error.
    pub fn division_by_zero(context: impl Into<String>) -> Self {
        Self::DivisionByZero {
            context: context.into(),
        }
    }

    /// Create a configuration error.
    pub fn configuration(reason: impl Into<String>) -> Self {
        Self::Configuration {
            reason: reason.into(),
        }
    }

    /// Create an internal error.
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }
}
