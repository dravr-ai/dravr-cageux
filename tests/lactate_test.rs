// ABOUTME: Content tests for the lactate step-test analysis — exact OBLA interpolation, Dmax against an independent scan, log-log against a constructed breakpoint
// ABOUTME: Pins every refusal path so a sparse or disordered protocol is rejected by name instead of estimated
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use dravr_cageux::algorithms::lactate::{
    LactateIntensityUnit, LactateStage, LactateStepTest, LactateThresholdMethod,
    LactateThresholdPoint, LactateThresholds, ThresholdOutcome, BAND_TABLE_MMOL, MIN_STAGES,
    OBLA_MMOL,
};
use dravr_cageux::error::IntelligenceError;

fn stage(intensity: f64, lactate_mmol: f64, heart_rate: Option<f64>) -> LactateStage {
    LactateStage {
        intensity,
        lactate_mmol,
        heart_rate,
    }
}

/// A six-stage cycling test whose lactate crosses 4.0 mmol/L a quarter of the
/// way between 250 W (3.6) and 275 W (5.2), with heart rate on every stage.
fn cycling_test() -> LactateStepTest {
    LactateStepTest {
        unit: LactateIntensityUnit::Watts,
        stages: vec![
            stage(150.0, 1.0, Some(120.0)),
            stage(175.0, 1.2, Some(130.0)),
            stage(200.0, 1.6, Some(140.0)),
            stage(225.0, 2.4, Some(150.0)),
            stage(250.0, 3.6, Some(160.0)),
            stage(275.0, 5.2, Some(170.0)),
        ],
    }
}

fn determined(outcome: &ThresholdOutcome) -> LactateThresholdPoint {
    match outcome {
        ThresholdOutcome::Determined(point) => *point,
        ThresholdOutcome::NotDeterminable { reason } => {
            panic!("expected a determined threshold, got: {reason}")
        }
    }
}

fn reason(outcome: &ThresholdOutcome) -> &str {
    match outcome {
        ThresholdOutcome::NotDeterminable { reason } => reason,
        ThresholdOutcome::Determined(point) => {
            panic!("expected not determinable, got {point:?}")
        }
    }
}

fn approx(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected} ± {tolerance}, got {actual}"
    );
}

// ============================================================================
// OBLA 4.0 mmol/L — exact linear interpolation
// ============================================================================

#[test]
fn obla_interpolates_the_first_crossing_of_four_mmol_exactly() {
    let result = cycling_test().analyze().unwrap();
    let point = determined(&result.lt2_obla_4mmol);
    // 3.6 → 5.2 crosses 4.0 at fraction 0.4 / 1.6 = 0.25 of the 250 → 275 W stage.
    approx(point.intensity, 256.25, 1e-9);
    approx(point.lactate_mmol, OBLA_MMOL, 1e-12);
    approx(point.heart_rate.unwrap(), 162.5, 1e-9);
    assert_eq!(result.stage_count, 6);
    assert_eq!(result.unit, LactateIntensityUnit::Watts);
}

#[test]
fn obla_is_not_determinable_when_lactate_never_reaches_four() {
    let test = LactateStepTest {
        unit: LactateIntensityUnit::Watts,
        stages: vec![
            stage(150.0, 1.0, None),
            stage(180.0, 1.3, None),
            stage(210.0, 1.9, None),
            stage(240.0, 2.8, None),
        ],
    };
    let result = test.analyze().unwrap();
    let why = reason(&result.lt2_obla_4mmol);
    assert!(why.contains("2.8"), "reason names the peak: {why}");
    assert!(why.contains("4.0"), "reason names the convention: {why}");
}

#[test]
fn obla_is_not_determinable_when_the_first_stage_is_already_above_four() {
    let test = LactateStepTest {
        unit: LactateIntensityUnit::Watts,
        stages: vec![
            stage(250.0, 4.2, None),
            stage(275.0, 5.0, None),
            stage(300.0, 6.5, None),
            stage(325.0, 8.9, None),
        ],
    };
    let result = test.analyze().unwrap();
    assert!(reason(&result.lt2_obla_4mmol).contains("first stage"));
}

// ============================================================================
// Dmax and modified Dmax — against an independent numeric scan of a known cubic
// ============================================================================

/// Lactate that lies exactly on `1 + 0.5·t + 4·t³` for `t = (watts − 150) / 100`.
fn exact_cubic_test() -> LactateStepTest {
    let lactate = |t: f64| 4.0_f64.mul_add(t.powi(3), 0.5_f64.mul_add(t, 1.0));
    LactateStepTest {
        unit: LactateIntensityUnit::Watts,
        stages: (0..6)
            .map(|i| {
                let t = f64::from(i) * 0.2;
                stage(100.0_f64.mul_add(t, 150.0), lactate(t), None)
            })
            .collect(),
    }
}

/// The `t` in `(from, to)` where the known cubic sits farthest below its own
/// chord, found by brute-force scan — an independent route to the answer the
/// analytic solver must reproduce.
fn scan_farthest_below_chord(from: f64, to: f64) -> f64 {
    let curve = |t: f64| 4.0_f64.mul_add(t.powi(3), 0.5_f64.mul_add(t, 1.0));
    let slope = (curve(to) - curve(from)) / (to - from);
    let mut best = (from, f64::MIN);
    let steps = 200_000;
    for i in 1..steps {
        let t = (to - from).mul_add(f64::from(i) / f64::from(steps), from);
        let distance = slope.mul_add(t - from, curve(from)) - curve(t);
        if distance > best.1 {
            best = (t, distance);
        }
    }
    best.0
}

#[test]
fn the_cubic_fit_recovers_exact_cubic_data() {
    let result = exact_cubic_test().analyze().unwrap();
    let [c0, c1, c2, c3] = result.curve.coefficients;
    approx(c0, 1.0, 1e-6);
    approx(c1, 0.5, 1e-6);
    approx(c2, 0.0, 1e-6);
    approx(c3, 4.0, 1e-6);
    approx(result.curve.r_squared, 1.0, 1e-9);
}

#[test]
fn dmax_matches_the_independent_scan_of_the_known_curve() {
    let result = exact_cubic_test().analyze().unwrap();
    let point = determined(&result.lt2_dmax);
    let expected_t = scan_farthest_below_chord(0.0, 1.0);
    // Analytically f'(t) = 0.5 + 12 t² equals the chord slope 4.5 at t = 1/√3.
    approx(expected_t, 1.0 / 3.0_f64.sqrt(), 1e-4);
    approx(point.intensity, 100.0_f64.mul_add(expected_t, 150.0), 0.05);
    let expected_lactate = 4.0_f64.mul_add(expected_t.powi(3), 0.5_f64.mul_add(expected_t, 1.0));
    approx(point.lactate_mmol, expected_lactate, 1e-3);
    assert!(point.heart_rate.is_none(), "no strap was worn");
}

#[test]
fn modified_dmax_starts_its_chord_at_the_stage_before_the_first_rise_over_point_four() {
    let result = exact_cubic_test().analyze().unwrap();
    // Stage lactates: 1.000, 1.132, 1.456, 2.164, 3.448, 5.500 — the first
    // stage-to-stage rise above 0.4 is 1.456 → 2.164, so the chord starts at
    // the third stage (t = 0.4).
    let point = determined(&result.lt2_modified_dmax);
    let expected_t = scan_farthest_below_chord(0.4, 1.0);
    approx(point.intensity, 100.0_f64.mul_add(expected_t, 150.0), 0.05);
    let dmax_point = determined(&result.lt2_dmax);
    assert!(
        point.intensity > dmax_point.intensity,
        "modified Dmax sits to the right of Dmax on an accelerating curve: {} vs {}",
        point.intensity,
        dmax_point.intensity
    );
}

#[test]
fn modified_dmax_is_not_determinable_without_a_rise_over_point_four() {
    let test = LactateStepTest {
        unit: LactateIntensityUnit::Watts,
        stages: vec![
            stage(150.0, 1.0, None),
            stage(180.0, 1.3, None),
            stage(210.0, 1.6, None),
            stage(240.0, 1.9, None),
            stage(270.0, 2.2, None),
        ],
    };
    let result = test.analyze().unwrap();
    assert!(reason(&result.lt2_modified_dmax).contains("0.4"));
}

#[test]
fn dmax_is_not_determinable_on_a_straight_line() {
    let test = LactateStepTest {
        unit: LactateIntensityUnit::Watts,
        stages: vec![
            stage(150.0, 1.0, None),
            stage(180.0, 2.0, None),
            stage(210.0, 3.0, None),
            stage(240.0, 4.0, None),
            stage(270.0, 5.0, None),
        ],
    };
    let result = test.analyze().unwrap();
    assert!(
        reason(&result.lt2_dmax).contains("did not accelerate"),
        "a linear rise has no farthest point: {:?}",
        result.lt2_dmax
    );
}

// ============================================================================
// Log-log LT1 — against a constructed two-segment breakpoint
// ============================================================================

/// Lactate that follows one power law up to 200 W and a steeper one from
/// 300 W, so the log-log segments intersect at a known intensity.
fn two_segment_test() -> (LactateStepTest, f64, f64) {
    let left = |x: f64| 0.3 * (x - 200.0_f64.ln());
    let right = |x: f64| 1.5 * (x - 250.0_f64.ln());
    let stages = [100.0, 150.0, 200.0]
        .iter()
        .map(|&w: &f64| stage(w, left(w.ln()).exp(), None))
        .chain(
            [300.0, 400.0, 500.0]
                .iter()
                .map(|&w: &f64| stage(w, right(w.ln()).exp(), None)),
        )
        .collect();
    // Intersection: 0.3 (x − ln 200) = 1.5 (x − ln 250).
    let break_x = 1.5_f64.mul_add(250.0_f64.ln(), -0.3 * 200.0_f64.ln()) / 1.2;
    let expected_intensity = break_x.exp();
    let expected_lactate = left(break_x).exp();
    (
        LactateStepTest {
            unit: LactateIntensityUnit::Watts,
            stages,
        },
        expected_intensity,
        expected_lactate,
    )
}

#[test]
fn log_log_breakpoint_recovers_the_constructed_intersection() {
    let (test, expected_intensity, expected_lactate) = two_segment_test();
    let result = test.analyze().unwrap();
    let point = determined(&result.lt1_log_log);
    approx(point.intensity, expected_intensity, 1e-6);
    approx(point.lactate_mmol, expected_lactate, 1e-6);
    assert!(
        point.intensity > 200.0 && point.intensity < 300.0,
        "the breakpoint lies between the two segments: {}",
        point.intensity
    );
}

#[test]
fn log_log_is_not_determinable_when_lactate_flattens_instead_of_steepening() {
    let test = LactateStepTest {
        unit: LactateIntensityUnit::Watts,
        stages: vec![
            stage(100.0, 1.0, None),
            stage(150.0, 2.0, None),
            stage(200.0, 3.2, None),
            stage(250.0, 3.3, None),
            stage(300.0, 3.35, None),
            stage(350.0, 3.4, None),
        ],
    };
    let result = test.analyze().unwrap();
    assert!(reason(&result.lt1_log_log).contains("more steeply"));
}

// ============================================================================
// Band table and heart-rate interpolation
// ============================================================================

#[test]
fn band_table_interpolates_each_crossed_level_and_skips_the_baseline() {
    let result = cycling_test().analyze().unwrap();
    // The first stage already sits at 1.0, so 1.0 has no crossing; the other
    // six levels are each crossed once.
    assert_eq!(result.band_table.len(), BAND_TABLE_MMOL.len() - 1);
    let row = |level: f64| {
        result
            .band_table
            .iter()
            .find(|r| (r.lactate_mmol - level).abs() < 1e-12)
            .copied()
            .unwrap()
    };
    assert!(result.band_table.iter().all(|r| r.lactate_mmol > 1.0));
    // 1.2 → 1.6 crosses 1.5 at 0.75 of 175 → 200 W.
    approx(row(1.5).intensity, 193.75, 1e-9);
    approx(row(1.5).heart_rate.unwrap(), 137.5, 1e-9);
    // 1.6 → 2.4 crosses 2.0 halfway through 200 → 225 W.
    approx(row(2.0).intensity, 212.5, 1e-9);
    approx(row(2.0).heart_rate.unwrap(), 145.0, 1e-9);
    // 2.4 → 3.6 crosses 3.0 halfway through 225 → 250 W.
    approx(row(3.0).intensity, 237.5, 1e-9);
    approx(row(4.0).intensity, 256.25, 1e-9);
    let levels: Vec<f64> = result.band_table.iter().map(|r| r.lactate_mmol).collect();
    assert!(
        levels.windows(2).all(|w| w[0] < w[1]),
        "ascending: {levels:?}"
    );
}

#[test]
fn heart_rate_is_omitted_when_a_bracketing_stage_has_none() {
    let mut test = cycling_test();
    test.stages[5].heart_rate = None;
    let result = test.analyze().unwrap();
    let obla = determined(&result.lt2_obla_4mmol);
    assert!(
        obla.heart_rate.is_none(),
        "250 → 275 W lost its upper heart rate"
    );
    let two = result
        .band_table
        .iter()
        .find(|r| (r.lactate_mmol - 2.0).abs() < 1e-12)
        .unwrap();
    approx(two.heart_rate.unwrap(), 145.0, 1e-9);
}

// ============================================================================
// Running pace — the effort axis is speed, the answers come back as pace
// ============================================================================

#[test]
fn pace_stages_are_analysed_on_the_speed_axis_and_reported_as_pace() {
    let test = LactateStepTest {
        unit: LactateIntensityUnit::SecondsPerKm,
        stages: vec![
            stage(360.0, 1.0, None),
            stage(340.0, 1.2, None),
            stage(320.0, 1.6, None),
            stage(300.0, 2.4, None),
            stage(280.0, 3.6, None),
            stage(260.0, 5.2, None),
        ],
    };
    let result = test.analyze().unwrap();
    assert_eq!(result.unit, LactateIntensityUnit::SecondsPerKm);
    let obla = determined(&result.lt2_obla_4mmol);
    // The crossing is a quarter of the way from 280 to 260 s/km on the speed
    // axis: 1000/280 → 1000/260.
    let expected_speed = (1000.0_f64 / 260.0 - 1000.0 / 280.0).mul_add(0.25, 1000.0 / 280.0);
    approx(obla.intensity, 1000.0 / expected_speed, 1e-9);
    assert!(
        obla.intensity < 280.0 && obla.intensity > 260.0,
        "pace at LT2 lies between the bracketing stages: {}",
        obla.intensity
    );
}

#[test]
fn a_pace_stage_that_is_slower_than_the_one_before_is_refused() {
    let test = LactateStepTest {
        unit: LactateIntensityUnit::SecondsPerKm,
        stages: vec![
            stage(360.0, 1.0, None),
            stage(340.0, 1.2, None),
            stage(345.0, 1.6, None),
            stage(300.0, 2.4, None),
        ],
    };
    match test.analyze() {
        Err(IntelligenceError::InvalidInput { field, reason }) => {
            assert_eq!(field, "stages[2].intensity");
            assert!(reason.contains("faster pace"), "{reason}");
        }
        other => panic!("expected InvalidInput, got {other:?}"),
    }
}

// ============================================================================
// Refusals
// ============================================================================

#[test]
fn fewer_than_four_stages_are_refused() {
    let mut test = cycling_test();
    test.stages.truncate(3);
    match test.analyze() {
        Err(IntelligenceError::InsufficientData { required, actual }) => {
            assert_eq!(required, MIN_STAGES);
            assert_eq!(actual, 3);
        }
        other => panic!("expected InsufficientData, got {other:?}"),
    }
}

#[test]
fn a_stage_that_is_not_harder_than_the_previous_is_refused_by_index() {
    let mut test = cycling_test();
    test.stages[3].intensity = 200.0;
    match test.analyze() {
        Err(IntelligenceError::InvalidInput { field, reason }) => {
            assert_eq!(field, "stages[3].intensity");
            assert!(reason.contains("more watts"), "{reason}");
            assert!(reason.contains("stage 4"), "{reason}");
        }
        other => panic!("expected InvalidInput, got {other:?}"),
    }
}

#[test]
fn lactate_outside_what_a_meter_reads_is_refused() {
    let mut test = cycling_test();
    test.stages[2].lactate_mmol = 30.0;
    match test.analyze() {
        Err(IntelligenceError::ValueOutOfRange { field, value, .. }) => {
            assert_eq!(field, "stages[2].lactate_mmol");
            approx(value, 30.0, 0.0);
        }
        other => panic!("expected ValueOutOfRange, got {other:?}"),
    }
}

#[test]
fn a_heart_rate_no_human_holds_is_refused() {
    let mut test = cycling_test();
    test.stages[1].heart_rate = Some(250.0);
    match test.analyze() {
        Err(IntelligenceError::ValueOutOfRange { field, .. }) => {
            assert_eq!(field, "stages[1].heart_rate");
        }
        other => panic!("expected ValueOutOfRange, got {other:?}"),
    }
}

#[test]
fn power_outside_the_human_range_is_refused() {
    let mut test = cycling_test();
    test.stages[5].intensity = 3000.0;
    match test.analyze() {
        Err(IntelligenceError::ValueOutOfRange { field, .. }) => {
            assert_eq!(field, "stages[5].intensity");
        }
        other => panic!("expected ValueOutOfRange, got {other:?}"),
    }
}

// ============================================================================
// Vocabulary and wire shape
// ============================================================================

#[test]
fn methods_name_their_threshold_and_paper() {
    assert_eq!(LactateThresholdMethod::LogLog.threshold(), "LT1");
    for method in [
        LactateThresholdMethod::Dmax,
        LactateThresholdMethod::ModifiedDmax,
        LactateThresholdMethod::Obla4,
    ] {
        assert_eq!(method.threshold(), "LT2");
    }
    assert!(LactateThresholdMethod::Obla4.reference().contains("Heck"));
    assert!(LactateThresholdMethod::Obla4.reference().contains("Faude"));
    assert!(LactateThresholdMethod::LogLog
        .reference()
        .contains("Beaver"));
    assert!(LactateThresholdMethod::ModifiedDmax
        .reference()
        .contains("Bishop"));
    assert!(LactateThresholdMethod::Dmax.reference().contains("Cheng"));
    assert_eq!(LactateThresholdMethod::Obla4.as_str(), "obla_4mmol");
}

#[test]
fn the_result_round_trips_through_json_with_tagged_outcomes() {
    let result = cycling_test().analyze().unwrap();
    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["unit"], "watts");
    assert_eq!(json["lt2_obla_4mmol"]["outcome"], "determined");
    approx(
        json["lt2_obla_4mmol"]["intensity"].as_f64().unwrap(),
        256.25,
        1e-9,
    );
    let back: LactateThresholds = serde_json::from_value(json).unwrap();
    assert_eq!(back, result);
    let flat = LactateStepTest {
        unit: LactateIntensityUnit::Watts,
        stages: vec![
            stage(150.0, 1.0, None),
            stage(180.0, 1.3, None),
            stage(210.0, 1.9, None),
            stage(240.0, 2.8, None),
        ],
    };
    let json = serde_json::to_value(flat.analyze().unwrap()).unwrap();
    assert_eq!(json["lt2_obla_4mmol"]["outcome"], "not_determinable");
    assert!(json["lt2_obla_4mmol"]["reason"]
        .as_str()
        .unwrap()
        .contains("2.8"));
}
