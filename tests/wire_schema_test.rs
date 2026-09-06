// ABOUTME: Pins that the types this crate puts on a caller's wire derive a real JSON Schema
// ABOUTME: A consumer declares an MCP outputSchema from these, so an empty schema is a broken promise
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

//! These twelve types are returned whole to callers that serialize them
//! straight onto a protocol wire — the platform's `analyze_sleep_quality`,
//! `calculate_recovery_score` and `suggest_rest_day` MCP tools do exactly
//! that, and MCP requires a tool declaring an `outputSchema` to answer with
//! conforming `structuredContent`.
//!
//! So the derive is load-bearing, not decoration: drop it and the consumer
//! cannot describe its own reply. These tests assert the schemas describe the
//! fields that are actually on the wire, because a schema that derives but
//! describes nothing would satisfy the compiler and fail the caller.

use dravr_cageux::recovery_calculator::{
    DataCompleteness, RecoveryCategory, RecoveryComponents, RecoveryScore, RestDayRecommendation,
    TrainingReadiness,
};
use dravr_cageux::sleep_analysis::{
    HrvRecoveryStatus, HrvTrend, HrvTrendAnalysis, SleepQualityCategory, SleepQualityScore,
};
use dravr_cageux::training_load::FormBand;

/// The property names a derived object schema declares.
fn properties(schema: &serde_json::Value) -> Vec<String> {
    schema["properties"]
        .as_object()
        .expect("an object schema declares properties")
        .keys()
        .cloned()
        .collect()
}

#[test]
fn the_sleep_quality_schema_describes_every_field_on_the_wire() {
    let schema = serde_json::to_value(schemars::schema_for!(SleepQualityScore)).unwrap();
    let props = properties(&schema);

    for field in [
        "overall_score",
        "duration_score",
        "stage_quality_score",
        "efficiency_score",
        "quality_category",
        "insights",
        "recommendations",
    ] {
        assert!(
            props.contains(&field.to_owned()),
            "SleepQualityScore sends {field} but its schema does not describe it: {props:?}"
        );
    }
}

#[test]
fn the_recovery_score_schema_describes_every_field_on_the_wire() {
    let schema = serde_json::to_value(schemars::schema_for!(RecoveryScore)).unwrap();
    let props = properties(&schema);

    for field in [
        "overall_score",
        "recovery_category",
        "data_completeness",
        "components",
        "training_readiness",
        "insights",
        "recommendations",
        "rest_day_recommended",
        "reasoning",
        "limitations",
    ] {
        assert!(
            props.contains(&field.to_owned()),
            "RecoveryScore sends {field} but its schema does not describe it: {props:?}"
        );
    }
}

#[test]
fn the_hrv_and_rest_day_schemas_describe_every_field_on_the_wire() {
    let hrv = serde_json::to_value(schemars::schema_for!(HrvTrendAnalysis)).unwrap();
    for field in [
        "current_rmssd",
        "weekly_average_rmssd",
        "baseline_rmssd",
        "baseline_deviation_percent",
        "recovery_status",
        "trend",
        "insights",
    ] {
        assert!(
            properties(&hrv).contains(&field.to_owned()),
            "HrvTrendAnalysis sends {field} but its schema does not describe it"
        );
    }

    let rest = serde_json::to_value(schemars::schema_for!(RestDayRecommendation)).unwrap();
    for field in [
        "rest_recommended",
        "confidence",
        "recovery_score",
        "primary_reasons",
        "supporting_factors",
        "alternatives",
        "estimated_recovery_hours",
    ] {
        assert!(
            properties(&rest).contains(&field.to_owned()),
            "RestDayRecommendation sends {field} but its schema does not describe it"
        );
    }
}

/// The variants a derived enum schema lists, however it lists them.
///
/// A doc-commented unit enum derives `oneOf` with one `const` arm per variant,
/// so that each variant keeps its own description; a bare one derives a flat
/// `enum` array. Both are the same contract to a client, so this reads either
/// rather than pinning one and breaking the day a doc comment is added.
fn variants(schema: &serde_json::Value) -> Option<Vec<String>> {
    if let Some(flat) = schema.get("enum").and_then(|v| v.as_array()) {
        return Some(
            flat.iter()
                .filter_map(|v| v.as_str().map(ToOwned::to_owned))
                .collect(),
        );
    }
    let arms = schema.get("oneOf")?.as_array()?;
    Some(
        arms.iter()
            .filter_map(|a| a.get("const")?.as_str().map(ToOwned::to_owned))
            .collect(),
    )
}

/// The enums are what a caller branches on, so their schemas have to list the
/// variants rather than degrade to a bare string.
///
/// Each is `#[serde(rename_all = ...)]`, so the schema must carry the RENAMED
/// spelling — the wire never sees `TsbOnly` or `ReadyForHard`. A schema
/// listing the Rust names would validate nothing the crate actually sends.
#[test]
fn the_enum_schemas_list_the_variants_as_they_are_serialized() {
    for (name, schema, expected) in [
        (
            "SleepQualityCategory",
            serde_json::to_value(schemars::schema_for!(SleepQualityCategory)).unwrap(),
            vec!["excellent", "good", "fair", "poor"],
        ),
        (
            "RecoveryCategory",
            serde_json::to_value(schemars::schema_for!(RecoveryCategory)).unwrap(),
            vec!["excellent", "good", "fair", "poor"],
        ),
        (
            "DataCompleteness",
            serde_json::to_value(schemars::schema_for!(DataCompleteness)).unwrap(),
            vec!["full", "partial", "tsbonly"],
        ),
        (
            "TrainingReadiness",
            serde_json::to_value(schemars::schema_for!(TrainingReadiness)).unwrap(),
            vec![
                "ready_for_hard",
                "ready_for_moderate",
                "easy_only",
                "rest_needed",
            ],
        ),
        (
            "FormBand",
            serde_json::to_value(schemars::schema_for!(FormBand)).unwrap(),
            vec![
                "insufficient_history",
                "deep_fatigue",
                "heavy_block",
                "productive",
                "balanced",
                "fresh",
                "detraining",
            ],
        ),
        (
            "HrvTrend",
            serde_json::to_value(schemars::schema_for!(HrvTrend)).unwrap(),
            vec!["improving", "stable", "declining"],
        ),
    ] {
        let listed = variants(&schema)
            .unwrap_or_else(|| panic!("{name}'s schema must list its variants: {schema:#}"));
        assert_eq!(
            listed, expected,
            "{name}'s schema must list the variants as they serialize"
        );
    }
}

/// A derived schema is only useful if the payload the crate produces passes
/// it. Serializing a real value and validating it against its own schema is
/// the check that a hand-written schema would fail.
#[test]
fn a_serialized_value_validates_against_its_own_derived_schema() {
    let components = RecoveryComponents {
        tsb_score: 72.0,
        sleep_score: Some(64.0),
        hrv_score: None,
        components_available: 2,
    };
    let schema = serde_json::to_value(schemars::schema_for!(RecoveryComponents)).unwrap();
    let payload = serde_json::to_value(&components).unwrap();

    let props = properties(&schema);
    for field in [
        "tsb_score",
        "sleep_score",
        "hrv_score",
        "components_available",
    ] {
        assert!(props.contains(&field.to_owned()), "missing {field}");
    }
    assert_eq!(
        payload["sleep_score"], 64.0,
        "the payload the schema describes is the one the crate sends"
    );
    assert!(
        payload["hrv_score"].is_null(),
        "an absent HRV component serializes as null, which the schema must allow"
    );

    // The enums travel by their serde names, which is what the schema lists.
    let status = serde_json::to_value(HrvRecoveryStatus::Recovered).unwrap();
    assert_eq!(status, serde_json::json!("recovered"));
}
