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

use crate::algorithms::{
    MaxHrAlgorithm, RecoveryAggregationAlgorithm, TrainingLoadAlgorithm, TrimpAlgorithm,
    TssAlgorithm, VdotAlgorithm,
};
use serde::{Deserialize, Serialize};
use tracing::warn;

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

    /// TRIMP calculation algorithm: `bannister_male`, `bannister_female`,
    /// `edwards_simplified`, `lucia_banded`, or `hybrid`
    #[serde(default = "default_trimp_algorithm")]
    pub trimp: String,

    /// VDOT calculation algorithm: `daniels`, `riegel`, or `hybrid`
    #[serde(default = "default_vdot_algorithm")]
    pub vdot: String,

    /// Training load smoothing algorithm: `ema`, `sma`, `wma`, or `kalman`
    #[serde(default = "default_training_load_algorithm")]
    pub training_load: String,

    /// Recovery score aggregation algorithm: `weighted_average`, `geometric_mean`,
    /// `harmonic_mean`, `minimum`, or `bayesian`
    #[serde(default = "default_recovery_algorithm")]
    pub recovery: String,

    /// Configuration for the pace-based TSS fallback (used when no power/HR data)
    #[serde(default)]
    pub tss_fallback: TssFallbackConfig,

    /// Tuning parameters for the parameterized algorithm variants.
    ///
    /// These adjust the numeric knobs of a selected variant (e.g. the rolling
    /// window for normalized-power TSS) without changing which algorithm runs.
    /// Literature-defined formula constants (e.g. the Tanaka coefficient) are
    /// intentionally NOT configurable here — they define the algorithm itself.
    #[serde(default)]
    pub params: AlgorithmParamsConfig,
}

/// Tuning parameters for parameterized algorithm variants.
///
/// Each field maps to a struct-variant parameter that has a documented default
/// and a legitimate tuning range. Measured runtime inputs (e.g. an athlete's
/// 20-minute test power) are NOT configured here — they are supplied at call time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmParamsConfig {
    /// Rolling window in seconds for normalized-power TSS (standard: 30)
    #[serde(default = "default_tss_window_seconds")]
    pub tss_window_seconds: u32,

    /// Power-law exponent for the Riegel VDOT model (default 1.06, range 1.03-1.08)
    #[serde(default = "default_vdot_riegel_exponent")]
    pub vdot_riegel_exponent: f64,

    /// Chronic Training Load window in days (default 42 for fitness)
    #[serde(default = "default_ctl_days")]
    pub training_load_ctl_days: i64,

    /// Acute Training Load window in days (default 7 for fatigue)
    #[serde(default = "default_atl_days")]
    pub training_load_atl_days: i64,

    /// Kalman filter process noise (training-load variability, default 1.0)
    #[serde(default = "default_kalman_process_noise")]
    pub training_load_kalman_process_noise: f64,

    /// Kalman filter measurement noise (TSS measurement error, default 10.0)
    #[serde(default = "default_kalman_measurement_noise")]
    pub training_load_kalman_measurement_noise: f64,

    /// Power coefficient for VO2max-based FTP estimation (W per ml/kg/min, default 13.5)
    #[serde(default = "default_ftp_vo2max_power_coefficient")]
    pub ftp_vo2max_power_coefficient: f64,

    /// Percentage of `MaxHR` used by the from-maxhr LTHR estimate (default 0.88)
    #[serde(default = "default_lthr_maxhr_percentage")]
    pub lthr_maxhr_percentage: f64,
}

/// Default normalized-power TSS rolling window (30-second standard)
fn default_tss_window_seconds() -> u32 {
    30
}

/// Default Riegel power-law exponent
fn default_vdot_riegel_exponent() -> f64 {
    1.06
}

/// Default CTL window (42 days)
fn default_ctl_days() -> i64 {
    42
}

/// Default ATL window (7 days)
fn default_atl_days() -> i64 {
    7
}

/// Default Kalman process noise
fn default_kalman_process_noise() -> f64 {
    1.0
}

/// Default Kalman measurement noise
fn default_kalman_measurement_noise() -> f64 {
    10.0
}

/// Default VO2max-to-FTP power coefficient
fn default_ftp_vo2max_power_coefficient() -> f64 {
    13.5
}

/// Default from-maxhr LTHR percentage
fn default_lthr_maxhr_percentage() -> f64 {
    0.88
}

impl Default for AlgorithmParamsConfig {
    fn default() -> Self {
        Self {
            tss_window_seconds: default_tss_window_seconds(),
            vdot_riegel_exponent: default_vdot_riegel_exponent(),
            training_load_ctl_days: default_ctl_days(),
            training_load_atl_days: default_atl_days(),
            training_load_kalman_process_noise: default_kalman_process_noise(),
            training_load_kalman_measurement_noise: default_kalman_measurement_noise(),
            ftp_vo2max_power_coefficient: default_ftp_vo2max_power_coefficient(),
            lthr_maxhr_percentage: default_lthr_maxhr_percentage(),
        }
    }
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

/// Default TRIMP algorithm (`hybrid` auto-selects best method per available data)
fn default_trimp_algorithm() -> String {
    "hybrid".to_owned()
}

/// Default VDOT algorithm (`daniels` as most physiologically accurate)
fn default_vdot_algorithm() -> String {
    "daniels".to_owned()
}

/// Default training load algorithm (`ema` matches `TrainingPeaks` PMC)
fn default_training_load_algorithm() -> String {
    "ema".to_owned()
}

/// Default recovery aggregation algorithm (`weighted_average` as most common)
fn default_recovery_algorithm() -> String {
    "weighted_average".to_owned()
}

impl Default for AlgorithmConfig {
    fn default() -> Self {
        Self {
            tss: default_tss_algorithm(),
            maxhr: default_maxhr_algorithm(),
            ftp: default_ftp_algorithm(),
            lthr: default_lthr_algorithm(),
            vo2max: default_vo2max_algorithm(),
            trimp: default_trimp_algorithm(),
            vdot: default_vdot_algorithm(),
            training_load: default_training_load_algorithm(),
            recovery: default_recovery_algorithm(),
            tss_fallback: TssFallbackConfig::default(),
            params: AlgorithmParamsConfig::default(),
        }
    }
}

impl AlgorithmConfig {
    /// Resolve the configured TSS algorithm, injecting the configured rolling window.
    ///
    /// Falls back to [`TssAlgorithm::default`] (with a warning) on an invalid selection.
    #[must_use]
    pub fn tss_algorithm(&self) -> TssAlgorithm {
        match self.tss.parse::<TssAlgorithm>() {
            Ok(TssAlgorithm::NormalizedPower { .. }) => TssAlgorithm::NormalizedPower {
                window_seconds: self.params.tss_window_seconds,
            },
            Ok(algo) => algo,
            Err(e) => {
                warn!(config = %self.tss, error = %e, "Invalid TSS algorithm selection, using default");
                TssAlgorithm::default()
            }
        }
    }

    /// Resolve the configured Max HR estimation algorithm.
    ///
    /// Falls back to [`MaxHrAlgorithm::default`] (with a warning) on an invalid selection.
    #[must_use]
    pub fn maxhr_algorithm(&self) -> MaxHrAlgorithm {
        match self.maxhr.parse::<MaxHrAlgorithm>() {
            Ok(algo) => algo,
            Err(e) => {
                warn!(config = %self.maxhr, error = %e, "Invalid Max HR algorithm selection, using default");
                MaxHrAlgorithm::default()
            }
        }
    }

    /// Resolve the configured TRIMP algorithm.
    ///
    /// Falls back to [`TrimpAlgorithm::default`] (with a warning) on an invalid selection.
    #[must_use]
    pub fn trimp_algorithm(&self) -> TrimpAlgorithm {
        match self.trimp.parse::<TrimpAlgorithm>() {
            Ok(algo) => algo,
            Err(e) => {
                warn!(config = %self.trimp, error = %e, "Invalid TRIMP algorithm selection, using default");
                TrimpAlgorithm::default()
            }
        }
    }

    /// Resolve the configured VDOT algorithm, injecting the configured Riegel exponent.
    ///
    /// Falls back to [`VdotAlgorithm::default`] (with a warning) on an invalid selection.
    #[must_use]
    pub fn vdot_algorithm(&self) -> VdotAlgorithm {
        match self.vdot.parse::<VdotAlgorithm>() {
            Ok(VdotAlgorithm::Riegel { .. }) => VdotAlgorithm::Riegel {
                exponent: self.params.vdot_riegel_exponent,
            },
            Ok(algo) => algo,
            Err(e) => {
                warn!(config = %self.vdot, error = %e, "Invalid VDOT algorithm selection, using default");
                VdotAlgorithm::default()
            }
        }
    }

    /// Resolve the configured training-load smoothing algorithm, injecting the
    /// configured CTL/ATL windows and Kalman noise parameters.
    ///
    /// Falls back to [`TrainingLoadAlgorithm::default`] (with a warning) on an invalid selection.
    #[must_use]
    pub fn training_load_algorithm(&self) -> TrainingLoadAlgorithm {
        let p = &self.params;
        match self.training_load.parse::<TrainingLoadAlgorithm>() {
            Ok(TrainingLoadAlgorithm::Ema { .. }) => TrainingLoadAlgorithm::Ema {
                ctl_days: p.training_load_ctl_days,
                atl_days: p.training_load_atl_days,
            },
            Ok(TrainingLoadAlgorithm::Sma { .. }) => TrainingLoadAlgorithm::Sma {
                ctl_days: p.training_load_ctl_days,
                atl_days: p.training_load_atl_days,
            },
            Ok(TrainingLoadAlgorithm::Wma { .. }) => TrainingLoadAlgorithm::Wma {
                ctl_days: p.training_load_ctl_days,
                atl_days: p.training_load_atl_days,
            },
            Ok(TrainingLoadAlgorithm::KalmanFilter { .. }) => TrainingLoadAlgorithm::KalmanFilter {
                process_noise: p.training_load_kalman_process_noise,
                measurement_noise: p.training_load_kalman_measurement_noise,
            },
            Err(e) => {
                warn!(config = %self.training_load, error = %e, "Invalid training load algorithm selection, using default");
                TrainingLoadAlgorithm::default()
            }
        }
    }

    /// Resolve the configured recovery aggregation algorithm.
    ///
    /// `WeightedAverage` carries its weights from the caller's recovery-scoring
    /// configuration rather than this struct, so the `weighted` argument supplies
    /// the concrete weighted variant to use when `weighted_average` is selected
    /// (and as the fallback on an invalid selection).
    #[must_use]
    pub fn recovery_algorithm(
        &self,
        weighted: RecoveryAggregationAlgorithm,
    ) -> RecoveryAggregationAlgorithm {
        match self.recovery.parse::<RecoveryAggregationAlgorithm>() {
            Ok(RecoveryAggregationAlgorithm::WeightedAverage { .. }) => weighted,
            Ok(algo) => algo,
            Err(e) => {
                warn!(config = %self.recovery, error = %e, "Invalid recovery algorithm selection, using weighted average");
                weighted
            }
        }
    }
}
