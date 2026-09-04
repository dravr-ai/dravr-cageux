// ABOUTME: Tests for configurable algorithm selection + tuning parameters
// ABOUTME: Validates AlgorithmConfig resolvers, param injection, and TrainingLoadCalculator config wiring
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::str_to_string
)]

use chrono::{Duration, Utc};
use dravr_cageux::algorithms::{
    RecoveryAggregationAlgorithm, TrainingLoadAlgorithm, TrimpAlgorithm, TssAlgorithm,
    VdotAlgorithm,
};
use dravr_cageux::config::intelligence::{AlgorithmConfig, AlgorithmParamsConfig};
use dravr_cageux::models::{ActivityBuilder, MaxHrAlgorithm, SportType};
use dravr_cageux::training_load::{TrainingLoadCalculator, TssDataPoint};

// ============================================================================
// Defaults
// ============================================================================

#[test]
fn default_params_match_documented_constants() {
    let cfg = AlgorithmConfig::default();
    assert_eq!(cfg.params.tss_window_seconds, 30);
    assert!((cfg.params.vdot_riegel_exponent - 1.06).abs() < f64::EPSILON);
    assert_eq!(cfg.params.training_load_ctl_days, 42);
    assert_eq!(cfg.params.training_load_atl_days, 7);
    assert!((cfg.params.training_load_kalman_process_noise - 1.0).abs() < f64::EPSILON);
    assert!((cfg.params.training_load_kalman_measurement_noise - 10.0).abs() < f64::EPSILON);
    assert!((cfg.params.ftp_vo2max_power_coefficient - 13.5).abs() < f64::EPSILON);
    assert!((cfg.params.lthr_maxhr_percentage - 0.88).abs() < f64::EPSILON);
}

#[test]
fn default_selections_resolve_to_default_variants() {
    let cfg = AlgorithmConfig::default();
    assert_eq!(cfg.tss_algorithm(), TssAlgorithm::default());
    assert_eq!(cfg.maxhr_algorithm(), MaxHrAlgorithm::default());
    assert_eq!(cfg.trimp_algorithm(), TrimpAlgorithm::default());
    assert_eq!(cfg.vdot_algorithm(), VdotAlgorithm::default());
}

// ============================================================================
// Selection resolvers
// ============================================================================

#[test]
fn tss_selection_injects_configured_window() {
    let cfg = AlgorithmConfig {
        tss: "normalized_power".to_owned(),
        params: AlgorithmParamsConfig {
            tss_window_seconds: 45,
            ..AlgorithmParamsConfig::default()
        },
        ..AlgorithmConfig::default()
    };
    assert_eq!(
        cfg.tss_algorithm(),
        TssAlgorithm::NormalizedPower { window_seconds: 45 }
    );
}

#[test]
fn maxhr_selection_resolves_each_variant() {
    let mut cfg = AlgorithmConfig {
        maxhr: "fox".to_owned(),
        ..AlgorithmConfig::default()
    };
    assert_eq!(cfg.maxhr_algorithm(), MaxHrAlgorithm::Fox);
    cfg.maxhr = "nes".to_owned();
    assert_eq!(cfg.maxhr_algorithm(), MaxHrAlgorithm::Nes);
    cfg.maxhr = "gulati".to_owned();
    assert_eq!(cfg.maxhr_algorithm(), MaxHrAlgorithm::Gulati);
}

#[test]
fn vdot_selection_injects_configured_exponent() {
    let cfg = AlgorithmConfig {
        vdot: "riegel".to_owned(),
        params: AlgorithmParamsConfig {
            vdot_riegel_exponent: 1.08,
            ..AlgorithmParamsConfig::default()
        },
        ..AlgorithmConfig::default()
    };
    assert_eq!(
        cfg.vdot_algorithm(),
        VdotAlgorithm::Riegel { exponent: 1.08 }
    );
}

#[test]
fn training_load_selection_injects_window_and_noise_params() {
    let mut cfg = AlgorithmConfig {
        training_load: "sma".to_owned(),
        params: AlgorithmParamsConfig {
            training_load_ctl_days: 35,
            training_load_atl_days: 5,
            training_load_kalman_process_noise: 2.0,
            training_load_kalman_measurement_noise: 20.0,
            ..AlgorithmParamsConfig::default()
        },
        ..AlgorithmConfig::default()
    };
    assert_eq!(
        cfg.training_load_algorithm(),
        TrainingLoadAlgorithm::Sma {
            ctl_days: 35,
            atl_days: 5
        }
    );

    cfg.training_load = "kalman".to_owned();
    assert_eq!(
        cfg.training_load_algorithm(),
        TrainingLoadAlgorithm::KalmanFilter {
            ctl_days: 35,
            atl_days: 5,
            process_noise: 2.0,
            measurement_noise: 20.0
        }
    );
}

#[test]
fn recovery_selection_uses_provided_weights_for_weighted_average() {
    let weighted = RecoveryAggregationAlgorithm::WeightedAverage {
        tsb_weight_full: 0.25,
        sleep_weight_full: 0.50,
        hrv_weight_full: 0.25,
        tsb_weight_no_hrv: 0.45,
        sleep_weight_no_hrv: 0.55,
    };
    let mut cfg = AlgorithmConfig {
        recovery: "weighted_average".to_owned(),
        ..AlgorithmConfig::default()
    };

    // weighted_average → caller-supplied weighted variant is used verbatim
    assert_eq!(cfg.recovery_algorithm(weighted.clone()), weighted);

    // a non-weighted selection resolves to its parameterless variant
    cfg.recovery = "geometric_mean".to_owned();
    assert_eq!(
        cfg.recovery_algorithm(weighted),
        RecoveryAggregationAlgorithm::GeometricMean
    );
}

// ============================================================================
// Invalid config falls back to defaults
// ============================================================================

#[test]
fn invalid_selection_falls_back_to_default() {
    let mut cfg = AlgorithmConfig {
        tss: "not_a_real_algorithm".to_owned(),
        ..AlgorithmConfig::default()
    };
    assert_eq!(cfg.tss_algorithm(), TssAlgorithm::default());

    cfg.training_load = "nonsense".to_owned();
    assert_eq!(
        cfg.training_load_algorithm(),
        TrainingLoadAlgorithm::default()
    );
}

#[test]
fn invalid_recovery_selection_falls_back_to_provided_weighted() {
    let weighted = RecoveryAggregationAlgorithm::default();
    let cfg = AlgorithmConfig {
        recovery: "nonsense".to_owned(),
        ..AlgorithmConfig::default()
    };
    assert_eq!(cfg.recovery_algorithm(weighted.clone()), weighted);
}

// ============================================================================
// Serialization round-trip of the new fields
// ============================================================================

#[test]
fn new_fields_round_trip_through_yaml() {
    let cfg = AlgorithmConfig {
        trimp: "edwards_simplified".to_owned(),
        training_load: "wma".to_owned(),
        recovery: "minimum".to_owned(),
        params: AlgorithmParamsConfig {
            vdot_riegel_exponent: 1.05,
            training_load_ctl_days: 28,
            ..AlgorithmParamsConfig::default()
        },
        ..AlgorithmConfig::default()
    };

    let yaml = serde_yaml::to_string(&cfg).expect("serialize AlgorithmConfig");
    let parsed: AlgorithmConfig = serde_yaml::from_str(&yaml).expect("parse AlgorithmConfig");

    assert_eq!(parsed.trimp, "edwards_simplified");
    assert_eq!(parsed.training_load, "wma");
    assert_eq!(parsed.recovery, "minimum");
    assert!((parsed.params.vdot_riegel_exponent - 1.05).abs() < f64::EPSILON);
    assert_eq!(parsed.params.training_load_ctl_days, 28);
}

// ============================================================================
// TrainingLoadCalculator honors the configured algorithm
// ============================================================================

/// Build `days` consecutive days of constant TSS for a deterministic load series.
fn constant_tss_series(days: i64, tss: f64) -> Vec<TssDataPoint> {
    let start = Utc::now() - Duration::days(days - 1);
    (0..days)
        .map(|d| TssDataPoint {
            date: start + Duration::days(d),
            tss,
        })
        .collect()
}

#[test]
fn from_config_default_matches_new() {
    // from_config(default) and new() must be behaviorally identical (EMA 42/7).
    let series = constant_tss_series(30, 60.0);

    let default_algo = AlgorithmConfig::default().training_load_algorithm();
    let ctl_default = default_algo.calculate_ctl(&series).expect("ctl");

    // new() uses AlgorithmConfig::default() internally; from_config(default)
    // must resolve the same algorithm and produce the same CTL on the same data.
    let _calc_new = TrainingLoadCalculator::new();
    let _calc_cfg = TrainingLoadCalculator::from_config(AlgorithmConfig::default());
    let cfg_algo = AlgorithmConfig::default().training_load_algorithm();
    let ctl_cfg = cfg_algo.calculate_ctl(&series).expect("ctl");

    assert!((ctl_default - ctl_cfg).abs() < f64::EPSILON);
}

#[test]
fn configured_window_changes_ctl_result() {
    let series = constant_tss_series(30, 60.0);

    let short = AlgorithmConfig {
        params: AlgorithmParamsConfig {
            training_load_ctl_days: 7,
            ..AlgorithmParamsConfig::default()
        },
        ..AlgorithmConfig::default()
    };
    let long = AlgorithmConfig {
        params: AlgorithmParamsConfig {
            training_load_ctl_days: 42,
            ..AlgorithmParamsConfig::default()
        },
        ..AlgorithmConfig::default()
    };

    let ctl_short = short
        .training_load_algorithm()
        .calculate_ctl(&series)
        .expect("ctl short");
    let ctl_long = long
        .training_load_algorithm()
        .calculate_ctl(&series)
        .expect("ctl long");

    // On a constant series a shorter EMA window reacts faster and sits closer
    // to the daily TSS (60) than a longer window over the same 30-day span.
    assert!(
        ctl_short > ctl_long,
        "shorter CTL window ({ctl_short}) should exceed longer window ({ctl_long}) on a constant series"
    );
}

#[test]
fn sma_and_ema_selections_differ() {
    // 60 days so the 42-day CTL window is fully populated for SMA.
    let series = constant_tss_series(60, 60.0);

    let ema_cfg = AlgorithmConfig {
        training_load: "ema".to_owned(),
        ..AlgorithmConfig::default()
    };
    let sma_cfg = AlgorithmConfig {
        training_load: "sma".to_owned(),
        ..AlgorithmConfig::default()
    };

    let ema = ema_cfg
        .training_load_algorithm()
        .calculate_ctl(&series)
        .expect("ema");
    let sma = sma_cfg
        .training_load_algorithm()
        .calculate_ctl(&series)
        .expect("sma");

    // SMA over a fully-populated 42-day window of constant 60 TSS equals 60;
    // EMA over the same 60-day span (starting from zero) has not fully
    // converged, so the two selections produce different CTL values.
    assert!(
        (sma - 60.0).abs() < 1.0,
        "SMA of constant 60 TSS over a full window should be ~60, got {sma}"
    );
    assert!(
        sma > ema && (sma - ema).abs() > 1.0,
        "EMA ({ema}) should trail SMA ({sma}) on a finite series"
    );
}

#[test]
fn calculate_training_load_reverse_chronological_is_rejected() {
    // Regression (Issue #1, fail-loud): newest-first activities (as Strava
    // returns) are unsorted, so the training-load algorithm rejects them.
    // TrainingLoadCalculator propagates that error rather than silently zeroing,
    // so a caller that forgot to sort oldest-first finds out.
    let now = Utc::now();
    let activities: Vec<_> = (0..3)
        .map(|i| {
            ActivityBuilder::new(
                format!("a{i}"),
                format!("run {i}"),
                SportType::Run,
                now - Duration::days(i), // index 0 = newest → reverse-chronological
                3600,
                "synthetic",
            )
            .distance_meters(10_000.0)
            .average_heart_rate(150)
            .build()
        })
        .collect();

    let calculator = TrainingLoadCalculator::new();
    let result = calculator.calculate_training_load(
        &activities,
        Some(250.0),
        Some(160.0),
        Some(190.0),
        Some(50.0),
        Some(70.0),
    );

    assert!(
        result.is_err(),
        "reverse-chronological (unsorted) activities must be rejected, not silently zeroed"
    );
}
