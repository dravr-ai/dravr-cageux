// ABOUTME: Unit tests for IntelligenceConfig YAML/JSON constructors and overlay merging
// ABOUTME: Validates layered loading (defaults → env → overlay) and round-trip serialization
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::str_to_string
)]

use dravr_cageux::config::intelligence::{
    DefaultStrategy, IntelligenceConfig, IntelligenceStrategy,
};

/// Tolerance for float equality assertions in this test module. The values
/// being compared are either round-tripped exactly (serialize → deserialize)
/// or produced by the same default constructor on both sides, so a tiny
/// epsilon is sufficient.
const FLOAT_EPS: f64 = 1e-9;

fn assert_close(actual: f64, expected: f64, label: &str) {
    assert!(
        (actual - expected).abs() < FLOAT_EPS,
        "{label}: expected {expected}, got {actual}"
    );
}

#[test]
fn load_returns_validated_defaults() {
    let config = IntelligenceConfig::load().expect("default config must validate");
    assert!(config.sleep_recovery.sleep_duration.adult_min_hours > 0.0);
    assert!(
        config.sleep_recovery.sleep_duration.adult_min_hours
            < config.sleep_recovery.sleep_duration.adult_max_hours
    );
}

#[test]
fn from_yaml_str_round_trips_defaults() {
    let original = IntelligenceConfig::load().expect("load default");
    let yaml = serde_yaml::to_string(&original).expect("serialize default to YAML");
    let parsed = IntelligenceConfig::from_yaml_str(&yaml).expect("parse YAML round-trip");

    assert_close(
        parsed.sleep_recovery.sleep_duration.adult_min_hours,
        original.sleep_recovery.sleep_duration.adult_min_hours,
        "sleep_duration.adult_min_hours round-trip",
    );
    assert_close(
        parsed.sleep_recovery.sleep_duration.adult_max_hours,
        original.sleep_recovery.sleep_duration.adult_max_hours,
        "sleep_duration.adult_max_hours round-trip",
    );
}

#[test]
fn from_json_str_round_trips_defaults() {
    let original = IntelligenceConfig::load().expect("load default");
    let json = serde_json::to_string(&original).expect("serialize default to JSON");
    let parsed = IntelligenceConfig::from_json_str(&json).expect("parse JSON round-trip");

    assert_close(
        parsed.nutrition.macronutrients.protein_min_g_per_kg,
        original.nutrition.macronutrients.protein_min_g_per_kg,
        "nutrition.protein_min_g_per_kg round-trip",
    );
}

#[test]
fn with_overlay_replaces_only_specified_fields() {
    let original = IntelligenceConfig::load().expect("load default");

    // Partial overlay touching only sleep duration thresholds.
    let overlay = r"
sleep_recovery:
  sleep_duration:
    adult_min_hours: 7.5
    adult_max_hours: 9.5
";

    let merged = IntelligenceConfig::with_overlay(overlay).expect("overlay must validate");

    // Overridden fields take the new values
    assert_close(
        merged.sleep_recovery.sleep_duration.adult_min_hours,
        7.5,
        "overridden adult_min_hours",
    );
    assert_close(
        merged.sleep_recovery.sleep_duration.adult_max_hours,
        9.5,
        "overridden adult_max_hours",
    );

    // Unrelated nested fields remain at defaults
    assert_close(
        merged.nutrition.macronutrients.protein_min_g_per_kg,
        original.nutrition.macronutrients.protein_min_g_per_kg,
        "untouched protein_min_g_per_kg",
    );
    assert_close(
        merged.weather_analysis.temperature.ideal_min_celsius.into(),
        original
            .weather_analysis
            .temperature
            .ideal_min_celsius
            .into(),
        "untouched ideal_min_celsius",
    );
}

#[test]
fn with_overlay_rejects_invalid_ranges() {
    // adult_min_hours >= adult_max_hours violates validate()
    let overlay = r"
sleep_recovery:
  sleep_duration:
    adult_min_hours: 10.0
    adult_max_hours: 9.0
";

    let err = IntelligenceConfig::with_overlay(overlay)
        .expect_err("invalid overlay must be rejected by validate()");
    let msg = err.to_string();
    assert!(
        msg.contains("adult_min_hours") || msg.contains("Invalid") || msg.contains("range"),
        "expected validation error mentioning the invalid range, got: {msg}"
    );
}

#[test]
fn from_yaml_str_rejects_malformed_input() {
    let err = IntelligenceConfig::from_yaml_str("this: is: not: valid: yaml: ::")
        .expect_err("malformed YAML must fail to parse");
    assert!(err.to_string().contains("YAML") || err.to_string().contains("deserialize"));
}

#[test]
fn default_strategy_carries_injected_config() {
    let config = IntelligenceConfig::load().expect("load default");
    let strategy = DefaultStrategy::new(config.clone());

    // Strategy returns references into its embedded config snapshot
    assert_close(
        strategy.recommendation_thresholds().low_weekly_distance_km,
        config
            .recommendation_engine
            .thresholds
            .low_weekly_distance_km,
        "strategy mirrors recommendation thresholds",
    );
    assert_close(
        strategy
            .weather_config()
            .temperature
            .ideal_min_celsius
            .into(),
        config.weather_analysis.temperature.ideal_min_celsius.into(),
        "strategy mirrors weather config",
    );
}

#[test]
fn default_strategy_from_env_uses_layered_load() {
    let direct = DefaultStrategy::from_env().expect("from_env must succeed without overrides");
    let expected = IntelligenceConfig::load().expect("load default");

    assert_close(
        direct
            .config()
            .sleep_recovery
            .sleep_duration
            .adult_min_hours,
        expected.sleep_recovery.sleep_duration.adult_min_hours,
        "from_env mirrors load()",
    );
}
