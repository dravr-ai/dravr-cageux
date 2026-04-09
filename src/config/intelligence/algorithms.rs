// ABOUTME: Algorithm selection configuration for fitness calculations
// ABOUTME: Configures TSS, MaxHR, FTP, LTHR, and VO2max algorithm implementations
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

//! Algorithm Selection Configuration
//!
//! Configures which algorithm implementation to use for various fitness calculations.
//! Each algorithm type uses enum dispatch for type-safe selection with minimal runtime overhead.
//!
//! # Algorithm Types
//!
//! - **TSS**: Training Stress Score calculation (`avg_power`, `normalized_power`, `hybrid`)
//! - **`MaxHR`**: Maximum heart rate estimation (`fox`, `tanaka`, `nes`, `gulati`)
//! - **FTP**: Functional Threshold Power estimation
//! - **LTHR**: Lactate Threshold Heart Rate estimation
//! - **`VO2max`**: Maximum oxygen uptake estimation
//!
//! # Configuration Methods
//!
//! 1. Environment variables (highest priority):
//!    ```bash
//!    export PIERRE_TSS_ALGORITHM=normalized_power
//!    export PIERRE_MAXHR_ALGORITHM=tanaka
//!    ```
//!
//! 2. Default values (if env vars not set)

use serde::{Deserialize, Serialize};

/// Algorithm Selection Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmConfig {
    /// TSS calculation algorithm: `avg_power`, `normalized_power`, or `hybrid`
    #[serde(default = "default_tss_algorithm")]
    pub tss: String,

    /// Max HR estimation algorithm: `fox`, `tanaka`, `nes`, or `gulati`
    #[serde(default = "default_maxhr_algorithm")]
    pub maxhr: String,

    /// FTP estimation algorithm: `20min_test`, `from_vo2max`, `ramp_test`, etc.
    #[serde(default = "default_ftp_algorithm")]
    pub ftp: String,

    /// LTHR estimation algorithm: `from_maxhr`, `from_30min`, etc.
    #[serde(default = "default_lthr_algorithm")]
    pub lthr: String,

    /// `VO2max` estimation algorithm: `from_vdot`, `cooper_test`, etc.
    #[serde(default = "default_vo2max_algorithm")]
    pub vo2max: String,

    /// Configuration for the pace-based TSS fallback (used when no power/HR data)
    #[serde(default)]
    pub tss_fallback: TssFallbackConfig,
}

/// Configuration for pace-based TSS fallback estimation.
///
/// When neither power-based (FTP) nor HR-based (LTHR) TSS can be calculated,
/// the system falls back to pace-based estimation. These parameters control
/// the intensity factor mapping from pace ratio to TSS.
///
/// Reference: Coggan, A. (2003). Training and Racing Using a Power Meter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TssFallbackConfig {
    /// Intensity factor at the sport-specific baseline pace (e.g., 0.75 = moderate effort).
    /// Higher values produce higher TSS for the same pace.
    #[serde(default = "default_moderate_intensity_factor")]
    pub moderate_intensity_factor: f64,

    /// Minimum intensity factor clamp (prevents unrealistically low TSS for very slow paces)
    #[serde(default = "default_min_intensity_factor")]
    pub min_intensity_factor: f64,

    /// Maximum intensity factor clamp (caps TSS at race-effort levels)
    #[serde(default = "default_max_intensity_factor")]
    pub max_intensity_factor: f64,

    /// TSS normalization constant (standard value: 100).
    /// `TSS = hours x IF² x normalization_constant`
    #[serde(default = "default_tss_normalization")]
    pub normalization_constant: f64,
}

/// Moderate-effort intensity factor: at baseline pace, IF = 0.75
fn default_moderate_intensity_factor() -> f64 {
    0.75
}

/// Minimum IF clamp: very slow paces floor at IF = 0.5
fn default_min_intensity_factor() -> f64 {
    0.5
}

/// Maximum IF clamp: fastest paces cap at IF = 1.2 (above-threshold race effort)
fn default_max_intensity_factor() -> f64 {
    1.2
}

/// Standard TSS normalization: 1 hour at IF=1.0 produces 100 TSS
fn default_tss_normalization() -> f64 {
    100.0
}

impl Default for TssFallbackConfig {
    fn default() -> Self {
        Self {
            moderate_intensity_factor: default_moderate_intensity_factor(),
            min_intensity_factor: default_min_intensity_factor(),
            max_intensity_factor: default_max_intensity_factor(),
            normalization_constant: default_tss_normalization(),
        }
    }
}

/// Default TSS algorithm (`avg_power` for backwards compatibility)
fn default_tss_algorithm() -> String {
    "avg_power".to_owned()
}

/// Default Max HR algorithm (tanaka as most accurate)
fn default_maxhr_algorithm() -> String {
    "tanaka".to_owned()
}

/// Default FTP algorithm (`from_vo2max` as most accessible)
fn default_ftp_algorithm() -> String {
    "from_vo2max".to_owned()
}

/// Default LTHR algorithm (`from_maxhr` as most common)
fn default_lthr_algorithm() -> String {
    "from_maxhr".to_owned()
}

/// Default `VO2max` algorithm (`from_vdot` as most validated)
fn default_vo2max_algorithm() -> String {
    "from_vdot".to_owned()
}

impl Default for AlgorithmConfig {
    fn default() -> Self {
        Self {
            tss: default_tss_algorithm(),
            maxhr: default_maxhr_algorithm(),
            ftp: default_ftp_algorithm(),
            lthr: default_lthr_algorithm(),
            vo2max: default_vo2max_algorithm(),
            tss_fallback: TssFallbackConfig::default(),
        }
    }
}
