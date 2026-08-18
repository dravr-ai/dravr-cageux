// ABOUTME: Tests for TSB interpretation, recovery-day recommendations, and overtraining risk
// ABOUTME: Validates CTL-relative form bands (% of CTL) and the absolute-TSB fallback at low CTL
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::str_to_string
)]

use dravr_cageux::training_load::{
    RiskLevel, TrainingLoad, TrainingLoadCalculator, TrainingStatus,
};

// ============================================================================
// interpret_tsb — CTL-relative form bands
// ============================================================================

#[test]
fn interpret_tsb_elite_negative_tsb_is_productive() {
    // CTL 100, TSB -25 → form -25%: inside the -30%..+5% productive band
    assert_eq!(
        TrainingLoadCalculator::interpret_tsb(-25.0, 100.0),
        TrainingStatus::Productive
    );
}

#[test]
fn interpret_tsb_elite_form_below_minus_thirty_is_overreaching() {
    // CTL 100, TSB -35 → form -35%
    assert_eq!(
        TrainingLoadCalculator::interpret_tsb(-35.0, 100.0),
        TrainingStatus::Overreaching
    );
}

#[test]
fn interpret_tsb_low_ctl_same_tsb_is_overreaching() {
    // CTL 40, TSB -25 → form -62.5%: deep overreaching on a small fitness base
    assert_eq!(
        TrainingLoadCalculator::interpret_tsb(-25.0, 40.0),
        TrainingStatus::Overreaching
    );
}

#[test]
fn interpret_tsb_band_edges() {
    let ctl = 100.0;
    // form exactly -30% falls in the productive band
    assert_eq!(
        TrainingLoadCalculator::interpret_tsb(-30.0, ctl),
        TrainingStatus::Productive
    );
    // just below +5% is still productive
    assert_eq!(
        TrainingLoadCalculator::interpret_tsb(4.9, ctl),
        TrainingStatus::Productive
    );
    // +5% opens the fresh band
    assert_eq!(
        TrainingLoadCalculator::interpret_tsb(5.0, ctl),
        TrainingStatus::Fresh
    );
    // +20% is the last fresh value
    assert_eq!(
        TrainingLoadCalculator::interpret_tsb(20.0, ctl),
        TrainingStatus::Fresh
    );
    // above +20% is detraining
    assert_eq!(
        TrainingLoadCalculator::interpret_tsb(20.1, ctl),
        TrainingStatus::Detraining
    );
}

#[test]
fn interpret_tsb_zero_ctl_applies_bands_to_absolute_tsb() {
    assert_eq!(
        TrainingLoadCalculator::interpret_tsb(-35.0, 0.0),
        TrainingStatus::Overreaching
    );
    assert_eq!(
        TrainingLoadCalculator::interpret_tsb(-25.0, 0.0),
        TrainingStatus::Productive
    );
    assert_eq!(
        TrainingLoadCalculator::interpret_tsb(10.0, 0.0),
        TrainingStatus::Fresh
    );
    assert_eq!(
        TrainingLoadCalculator::interpret_tsb(25.0, 0.0),
        TrainingStatus::Detraining
    );
}

// ============================================================================
// recommend_recovery_days — form-relative prescription
// ============================================================================

#[test]
fn recovery_days_elite_productive_form_needs_none() {
    // CTL 100, TSB -25 → form -25%: normal productive training, no rest days
    assert_eq!(
        TrainingLoadCalculator::recommend_recovery_days(-25.0, 100.0),
        0
    );
}

#[test]
fn recovery_days_form_minus_thirty_five_percent_needs_one() {
    assert_eq!(
        TrainingLoadCalculator::recommend_recovery_days(-35.0, 100.0),
        1
    );
}

#[test]
fn recovery_days_form_minus_sixty_two_percent_needs_three() {
    // CTL 40, TSB -25 → form -62.5%
    assert_eq!(
        TrainingLoadCalculator::recommend_recovery_days(-25.0, 40.0),
        3
    );
}

#[test]
fn recovery_days_band_edges() {
    let ctl = 100.0;
    assert_eq!(
        TrainingLoadCalculator::recommend_recovery_days(-30.0, ctl),
        0
    );
    assert_eq!(
        TrainingLoadCalculator::recommend_recovery_days(-30.5, ctl),
        1
    );
    assert_eq!(
        TrainingLoadCalculator::recommend_recovery_days(-40.0, ctl),
        1
    );
    assert_eq!(
        TrainingLoadCalculator::recommend_recovery_days(-40.5, ctl),
        2
    );
    assert_eq!(
        TrainingLoadCalculator::recommend_recovery_days(-50.0, ctl),
        2
    );
    assert_eq!(
        TrainingLoadCalculator::recommend_recovery_days(-50.5, ctl),
        3
    );
}

#[test]
fn recovery_days_zero_ctl_applies_bands_to_absolute_tsb() {
    assert_eq!(
        TrainingLoadCalculator::recommend_recovery_days(-55.0, 0.0),
        3
    );
    assert_eq!(
        TrainingLoadCalculator::recommend_recovery_days(-35.0, 0.0),
        1
    );
    assert_eq!(
        TrainingLoadCalculator::recommend_recovery_days(-20.0, 0.0),
        0
    );
}

// ============================================================================
// check_overtraining_risk — descriptive load-pattern factors
// ============================================================================

fn load(ctl: f64, atl: f64, tsb: f64) -> TrainingLoad {
    TrainingLoad {
        ctl,
        atl,
        tsb,
        tss_history: Vec::new(),
    }
}

#[test]
fn overtraining_risk_low_when_load_is_balanced() {
    let risk = TrainingLoadCalculator::check_overtraining_risk(&load(60.0, 55.0, 5.0));
    assert_eq!(risk.risk_level, RiskLevel::Low);
    assert!(risk.risk_factors.is_empty());
}

#[test]
fn overtraining_risk_flags_acute_ramp_above_thirty_percent() {
    // ATL 35% above CTL; form held at -20% so only the ramp factor fires
    let risk = TrainingLoadCalculator::check_overtraining_risk(&load(100.0, 135.0, -20.0));
    assert_eq!(risk.risk_level, RiskLevel::Moderate);
    assert_eq!(risk.risk_factors.len(), 1);
    assert!(risk.risk_factors[0].contains("more than 30% above chronic load"));
}

#[test]
fn overtraining_risk_flags_acute_load_far_above_chronic() {
    // ATL 60% above CTL trips both ratio factors; form -25% stays in band
    let risk = TrainingLoadCalculator::check_overtraining_risk(&load(100.0, 160.0, -25.0));
    assert_eq!(risk.risk_level, RiskLevel::High);
    assert_eq!(risk.risk_factors.len(), 2);
    assert!(risk
        .risk_factors
        .iter()
        .any(|f| f.contains("more than 50% above chronic load")));
}

#[test]
fn overtraining_risk_flags_form_below_minus_thirty_percent() {
    // Ratio 1.1 is under both ramp thresholds; form -35% fires alone
    let risk = TrainingLoadCalculator::check_overtraining_risk(&load(100.0, 110.0, -35.0));
    assert_eq!(risk.risk_level, RiskLevel::Moderate);
    assert_eq!(risk.risk_factors.len(), 1);
    assert!(risk.risk_factors[0].contains("-30% of fitness"));
}

#[test]
fn overtraining_risk_zero_ctl_uses_absolute_tsb_for_form() {
    // No chronic base: ratio factors cannot fire; absolute TSB -40 is deep form
    let risk = TrainingLoadCalculator::check_overtraining_risk(&load(0.0, 80.0, -40.0));
    assert_eq!(risk.risk_level, RiskLevel::Moderate);
    assert_eq!(risk.risk_factors.len(), 1);
    assert!(risk.risk_factors[0].contains("-30% of fitness"));
}
