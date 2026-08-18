// ABOUTME: Tests for TSB interpretation, recovery-day recommendations, and overtraining risk
// ABOUTME: Validates CTL-relative form bands (% of CTL) and the insufficient-history state at low CTL
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::str_to_string
)]

use dravr_cageux::training_load::{FormBand, RiskLevel, TrainingLoad, TrainingLoadCalculator};

// ============================================================================
// FormBand::from_tsb — CTL-relative form bands
// ============================================================================

#[test]
fn form_band_elite_negative_tsb_is_a_heavy_block_not_an_emergency() {
    // CTL 100, TSB -25 → form -25%: the deep end of the productive zone
    assert_eq!(FormBand::from_tsb(-25.0, 100.0), FormBand::HeavyBlock);
}

#[test]
fn form_band_elite_form_below_minus_thirty_is_deep_fatigue() {
    // CTL 100, TSB -35 → form -35%
    assert_eq!(FormBand::from_tsb(-35.0, 100.0), FormBand::DeepFatigue);
}

#[test]
fn form_band_low_ctl_same_tsb_is_deep_fatigue() {
    // CTL 40, TSB -25 → form -62.5%: deep fatigue on a small fitness base
    assert_eq!(FormBand::from_tsb(-25.0, 40.0), FormBand::DeepFatigue);
}

#[test]
fn form_band_edges() {
    let ctl = 100.0;
    // form exactly -30% is the top of the deep-fatigue edge, so still a block
    assert_eq!(FormBand::from_tsb(-30.0, ctl), FormBand::HeavyBlock);
    assert_eq!(FormBand::from_tsb(-30.5, ctl), FormBand::DeepFatigue);
    // -20% opens the ordinary productive band
    assert_eq!(FormBand::from_tsb(-20.0, ctl), FormBand::Productive);
    // -10% opens the balanced band
    assert_eq!(FormBand::from_tsb(-10.0, ctl), FormBand::Balanced);
    assert_eq!(FormBand::from_tsb(4.9, ctl), FormBand::Balanced);
    // +5% opens the fresh band
    assert_eq!(FormBand::from_tsb(5.0, ctl), FormBand::Fresh);
    // +20% is the last fresh value
    assert_eq!(FormBand::from_tsb(20.0, ctl), FormBand::Fresh);
    // above +20% is detraining
    assert_eq!(FormBand::from_tsb(20.1, ctl), FormBand::Detraining);
}

#[test]
fn form_band_without_chronic_base_is_insufficient_history_not_absolute_tsb() {
    // The whole point of the CTL-relative rebanding: with no fitness base,
    // the honest answer is "cannot judge", never a band read off raw TSB.
    // -35 at CTL 0 is a beginner's first hard week, not an elite's crisis.
    for tsb in [-35.0, -25.0, 10.0, 25.0] {
        assert_eq!(
            FormBand::from_tsb(tsb, 0.0),
            FormBand::InsufficientHistory,
            "TSB {tsb} at CTL 0 must not be banded"
        );
    }
    // The guard is on CTL, not TSB: one unit of chronic load is still no base.
    assert_eq!(
        FormBand::from_tsb(-35.0, 1.0),
        FormBand::InsufficientHistory
    );
    // Just past the guard, banding resumes: -0.1 on CTL 1.5 is -6.7% form.
    assert_eq!(FormBand::from_tsb(-0.1, 1.5), FormBand::Balanced);
}

#[test]
fn form_pct_is_tsb_over_ctl_and_none_without_a_base() {
    // The Raph incident numbers: TSB -66 on CTL 85 is -77.6% of fitness
    let pct = FormBand::form_pct(-66.0, 85.0).expect("CTL 85 is normalizable");
    assert!((pct - (-77.647)).abs() < 0.01, "got {pct}");
    assert_eq!(FormBand::form_pct(-25.0, 100.0), Some(-25.0));
    assert_eq!(FormBand::form_pct(-10.0, 0.5), None);
}

#[test]
fn form_band_labels_are_descriptive_never_injury_claims() {
    for band in [
        FormBand::InsufficientHistory,
        FormBand::DeepFatigue,
        FormBand::HeavyBlock,
        FormBand::Productive,
        FormBand::Balanced,
        FormBand::Fresh,
        FormBand::Detraining,
    ] {
        let label = band.label();
        assert!(!label.is_empty(), "{band:?} has no label");
        for banned in ["injury", "risk", "danger"] {
            assert!(
                !label.to_lowercase().contains(banned),
                "{band:?} label carries {banned} framing: {label}"
            );
        }
    }
    assert_eq!(
        FormBand::HeavyBlock.label(),
        "heavy block - the deep end of the productive zone"
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
fn recovery_days_without_chronic_base_prescribes_nothing() {
    // No fitness base to judge form against, so no rest prescription is
    // derived from a TSB that cannot be interpreted.
    for tsb in [-55.0, -35.0, -20.0] {
        assert_eq!(
            TrainingLoadCalculator::recommend_recovery_days(tsb, 0.0),
            0,
            "TSB {tsb} at CTL 0 must not prescribe rest"
        );
    }
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
fn overtraining_risk_without_chronic_base_flags_nothing() {
    // Neither ratio factor can fire without a chronic base, and form is not
    // interpretable, so the honest result is no factors rather than a
    // fabricated one read off absolute TSB.
    let risk = TrainingLoadCalculator::check_overtraining_risk(&load(0.0, 80.0, -40.0));
    assert_eq!(risk.risk_level, RiskLevel::Low);
    assert!(risk.risk_factors.is_empty());
}
