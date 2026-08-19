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
    // The guard is a chronic-base guard, not a divide-by-zero guard. Because
    // DeepFatigue is exactly `atl > 1.3 * ctl`, a beginner clears it with one
    // ordinary hard week — CTL 10 / ATL 14 is form -40% — and used to collect an
    // overtraining warning and a critical flag for it.
    assert_eq!(
        FormBand::from_tsb(-4.0, 10.0),
        FormBand::InsufficientHistory,
        "a CTL-10 beginner must not band as deepest fatigue for one hard week"
    );
    assert_eq!(
        FormBand::from_tsb(-35.0, 19.9),
        FormBand::InsufficientHistory,
        "just under the floor there is still no base to divide by"
    );
    // Just past the floor, banding resumes: -3 on CTL 20.1 is -14.9% form.
    assert_eq!(FormBand::from_tsb(-3.0, 20.1), FormBand::Productive);
    // And an athlete with a real base is banded as before.
    assert_eq!(FormBand::from_tsb(-35.0, 100.0), FormBand::DeepFatigue);
}

#[test]
fn form_pct_is_tsb_over_ctl_and_none_without_a_base() {
    // The Raph incident numbers: TSB -66 on CTL 85 is -77.6% of fitness
    let pct = FormBand::form_pct(-66.0, 85.0).expect("CTL 85 is normalizable");
    assert!((pct - (-77.647)).abs() < 0.01, "got {pct}");
    assert_eq!(FormBand::form_pct(-25.0, 100.0), Some(-25.0));
    assert_eq!(FormBand::form_pct(-10.0, 0.5), None);
    assert_eq!(
        FormBand::form_pct(-4.0, 10.0),
        None,
        "below the chronic-base floor"
    );
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

/// Build a physically consistent load: TSB *is* CTL - ATL, so a fixture cannot
/// claim a form the acute/chronic ratio contradicts.
///
/// The previous helper took `tsb` as a free parameter, and every ramp test set
/// it to a value CTL - ATL could never produce (CTL 100 / ATL 135 was written as
/// TSB -20, not -35). That was the only way to make the acute-ratio factor and
/// the form factor look independent — which they are not: `tsb == ctl - atl`
/// makes "ATL 30% above CTL" and form below -30% the same inequality.
fn load(ctl: f64, atl: f64) -> TrainingLoad {
    TrainingLoad {
        ctl,
        atl,
        tsb: ctl - atl,
        tss_history: Vec::new(),
    }
}

#[test]
fn overtraining_risk_low_when_load_is_balanced() {
    let risk = TrainingLoadCalculator::check_overtraining_risk(&load(60.0, 55.0));
    assert_eq!(risk.risk_level, RiskLevel::Low);
    assert!(risk.risk_factors.is_empty());
}

#[test]
fn overtraining_risk_moderate_through_the_heavy_block() {
    // ATL 25% above CTL puts form at -25%: the deep end of a productive block.
    let risk = TrainingLoadCalculator::check_overtraining_risk(&load(100.0, 125.0));
    assert_eq!(risk.risk_level, RiskLevel::Moderate);
    assert_eq!(risk.risk_factors.len(), 1);
    assert!(
        risk.risk_factors[0].contains("-25% of chronic fitness"),
        "got {:?}",
        risk.risk_factors
    );
}

#[test]
fn overtraining_risk_high_past_the_deep_fatigue_band() {
    // ATL 35% above CTL is form -35%, past the -30% edge.
    let risk = TrainingLoadCalculator::check_overtraining_risk(&load(100.0, 135.0));
    assert_eq!(risk.risk_level, RiskLevel::High);
    assert_eq!(risk.risk_factors.len(), 1);
    assert!(
        risk.risk_factors[0].contains("-35% of chronic fitness"),
        "got {:?}",
        risk.risk_factors
    );
}

#[test]
fn overtraining_risk_states_one_observation_once() {
    // The regression guard. A very deep athlete is one observation, not three:
    // acute-ramp, acute-spike and deep-form were the same inequality restated,
    // which forced High on every athlete past 1.3 and made Moderate unreachable.
    let deep = TrainingLoadCalculator::check_overtraining_risk(&load(100.0, 160.0));
    assert_eq!(deep.risk_level, RiskLevel::High);
    assert_eq!(
        deep.risk_factors.len(),
        1,
        "one axis must yield one factor, got {:?}",
        deep.risk_factors
    );

    // And Moderate stays reachable, which it was not while the count decided severity.
    assert_eq!(
        TrainingLoadCalculator::check_overtraining_risk(&load(100.0, 122.0)).risk_level,
        RiskLevel::Moderate
    );
}

#[test]
fn overtraining_risk_without_chronic_base_flags_nothing() {
    // Form is not interpretable without a chronic base, so the honest result is
    // no factors rather than one fabricated from an absolute TSB.
    let risk = TrainingLoadCalculator::check_overtraining_risk(&load(0.0, 80.0));
    assert_eq!(risk.risk_level, RiskLevel::Low);
    assert!(risk.risk_factors.is_empty());
}
