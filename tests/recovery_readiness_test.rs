// ABOUTME: Content tests for the TSB-only recovery verdicts — which band prescribes rest,
// ABOUTME: and whether an ordinary mid-block athlete can still be told to stop.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use dravr_cageux::config::intelligence::sleep_recovery::SleepRecoveryConfig;
use dravr_cageux::recovery_calculator::RecoveryCalculator;
use dravr_cageux::recovery_calculator::TrainingReadiness;
use dravr_cageux::sleep_analysis::SleepQualityCategory;
use dravr_cageux::training_load::{FormBand, TrainingLoad};

/// Physically consistent load: TSB *is* CTL - ATL.
fn load(ctl: f64, atl: f64) -> TrainingLoad {
    TrainingLoad {
        ctl,
        atl,
        tsb: ctl - atl,
        tss_history: Vec::new(),
    }
}

fn readiness(ctl: f64, atl: f64) -> TrainingReadiness {
    let cfg = SleepRecoveryConfig::default();
    RecoveryCalculator::calculate_recovery_score_tsb_only(&load(ctl, atl), &cfg)
        .expect("tsb-only score is computable")
        .training_readiness
}

#[test]
fn deep_fatigue_prescribes_rest() {
    // Form -35% of CTL: past the deepest edge, so rest is the honest verdict.
    assert_eq!(
        FormBand::from_tsb(-35.0, 100.0),
        FormBand::DeepFatigue,
        "fixture must sit in the deepest band"
    );
    assert_eq!(readiness(100.0, 135.0), TrainingReadiness::RestNeeded);
}

#[test]
fn an_ordinary_block_is_not_told_to_stop() {
    // The case the form migration exists for: a CTL-150 athlete at TSB -20 is
    // at -13% of their own fitness — ordinary mid-block form, and the band
    // agrees. Rest must not be prescribed for it.
    let band = FormBand::from_tsb(-20.0, 150.0);
    assert_eq!(
        band,
        FormBand::Productive,
        "TSB -20 on CTL 150 is productive"
    );
    assert_ne!(
        readiness(150.0, 170.0),
        TrainingReadiness::RestNeeded,
        "an athlete inside the productive band was told to rest"
    );
}

#[test]
fn freshness_clears_quality_work() {
    // Form +10% of CTL with a strong score: ready for hard work.
    assert_eq!(FormBand::from_tsb(10.0, 100.0), FormBand::Fresh);
    assert_eq!(readiness(100.0, 90.0), TrainingReadiness::ReadyForHard);
}

#[test]
fn no_chronic_base_never_prescribes_rest_on_form() {
    // A beginner's hard week is not deep fatigue: with no chronic base the band
    // is InsufficientHistory, so nothing about *form* may order them to stop.
    assert_eq!(
        FormBand::from_tsb(-4.0, 10.0),
        FormBand::InsufficientHistory
    );
    assert_ne!(
        readiness(10.0, 14.0),
        TrainingReadiness::RestNeeded,
        "a beginner with no chronic base was prescribed rest"
    );
}

// ============================================================================
// Full mode (sleep/HRV present) — the path a WHOOP-connected athlete takes
// ============================================================================

#[test]
fn full_mode_ordinary_block_with_good_sleep_is_not_rest() {
    // Same athlete as above, now with sleep data. Form is -13% of CTL and sleep
    // is good, so nothing here warrants a stop order.
    let readiness = RecoveryCalculator::determine_training_readiness(
        88.0,
        &load(150.0, 170.0),
        SleepQualityCategory::Good,
        None,
        &SleepRecoveryConfig::default(),
    );
    assert_ne!(
        readiness,
        TrainingReadiness::RestNeeded,
        "a productive-band athlete sleeping well was ordered to rest"
    );
}

#[test]
fn full_mode_keeps_sleep_and_hrv_votes() {
    // Sleep and HRV are genuinely independent of load, so they still force rest
    // even when form is fine — this is what separates them from the load score.
    let cfg = SleepRecoveryConfig::default();
    assert_eq!(
        RecoveryCalculator::determine_training_readiness(
            88.0,
            &load(150.0, 170.0),
            SleepQualityCategory::Poor,
            None,
            &cfg,
        ),
        TrainingReadiness::RestNeeded,
        "poor sleep must still be able to prescribe rest"
    );
}

#[test]
fn full_mode_deep_fatigue_still_rests() {
    assert_eq!(
        RecoveryCalculator::determine_training_readiness(
            88.0,
            &load(100.0, 135.0),
            SleepQualityCategory::Good,
            None,
            &SleepRecoveryConfig::default(),
        ),
        TrainingReadiness::RestNeeded
    );
}
