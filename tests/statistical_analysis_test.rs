// ABOUTME: Content tests for the Student's t p-value behind trend significance — tabulated
// ABOUTME: critical values at small df, convergence to the normal, and the verdicts it drives.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chrono::{Duration, Utc};
use dravr_cageux::statistical_analysis::{RegressionResult, StatisticalAnalyzer};
use dravr_cageux::types::{TrendDataPoint, TrendDirection};
use std::f64::consts::FRAC_2_PI;

/// Tabulated critical values are rounded to three decimals, which moves the
/// p-value at the table entry by a few 1e-5; the tolerance leaves room for that
/// and nothing else. A normal approximation misses these entries by 1e-2.
const TABLE_TOLERANCE: f64 = 5e-4;

/// Slope threshold the platform's trends tool passes to `determine_trend_direction`.
const SLOPE_THRESHOLD: f64 = 0.01;

fn weekly_points(values: &[f64]) -> Vec<TrendDataPoint> {
    let start = Utc::now();
    values
        .iter()
        .enumerate()
        .map(|(week, &value)| TrendDataPoint {
            date: start + Duration::weeks(week.try_into().expect("small fixture index")),
            value,
            smoothed_value: None,
        })
        .collect()
}

fn regression(values: &[f64]) -> RegressionResult {
    StatisticalAnalyzer::linear_regression(&weekly_points(values)).expect("fixture regresses")
}

fn assert_p(df: usize, t: f64, expected: f64) {
    let p = StatisticalAnalyzer::student_t_two_tailed_p_value(t, df);
    assert!(
        (p - expected).abs() < TABLE_TOLERANCE,
        "df={df} t={t}: p={p}, expected {expected}"
    );
}

#[test]
fn two_tailed_p_matches_the_five_percent_table_at_small_df() {
    // t_{0.975, df} from the standard table: the two-tailed p at each is 0.05.
    assert_p(1, 12.706, 0.05);
    assert_p(2, 4.303, 0.05);
    assert_p(5, 2.571, 0.05);
    assert_p(10, 2.228, 0.05);
    assert_p(30, 2.042, 0.05);
}

#[test]
fn two_tailed_p_matches_the_one_percent_and_one_per_mille_tables() {
    assert_p(1, 63.657, 0.01);
    assert_p(5, 4.032, 0.01);
    assert_p(10, 3.169, 0.01);
    assert_p(30, 2.750, 0.01);
    assert_p(5, 6.869, 0.001);
}

#[test]
fn two_tailed_p_converges_on_the_normal_at_large_df() {
    // Two-tailed p at t = 1.96 (the normal's 0.975 quantile) for growing df,
    // against the reference values 2·(1 − F_df(1.96)); the normal gives 0.0499958.
    let p_normal = 0.049_995_790_3;
    let references = [
        (1_000, 0.050_273_184_956),
        (10_000, 0.050_023_520_232),
        (100_000, 0.049_998_563_194),
    ];
    let mut previous_gap = f64::INFINITY;
    for (df, expected) in references {
        let p = StatisticalAnalyzer::student_t_two_tailed_p_value(1.96, df);
        assert!(
            (p - expected).abs() < 1e-9,
            "df={df}: p={p}, expected {expected}"
        );
        let gap = (p - p_normal).abs();
        assert!(gap < previous_gap, "df={df}: gap {gap} did not shrink");
        previous_gap = gap;
    }
    assert!(previous_gap < 5e-6);

    // The heavier tail at small df shows in the same statistic.
    let p_small = StatisticalAnalyzer::student_t_two_tailed_p_value(1.96, 5);
    assert!(
        (p_small - 0.107_287_952_5).abs() < 1e-9,
        "df=5 t=1.96: p={p_small}"
    );
}

#[test]
fn two_tailed_p_is_exact_in_closed_form_at_df_one_and_two() {
    // df=1 is the Cauchy distribution: p = 1 - (2/π)·atan(t).
    let t: f64 = 3.0;
    let cauchy = FRAC_2_PI.mul_add(-t.atan(), 1.0);
    let p1 = StatisticalAnalyzer::student_t_two_tailed_p_value(t, 1);
    assert!(
        (p1 - cauchy).abs() < 1e-12,
        "df=1 t={t}: p={p1} vs {cauchy}"
    );

    // df=2: p = 1 - t / √(2 + t²).
    let closed = 1.0 - t / t.mul_add(t, 2.0).sqrt();
    let p2 = StatisticalAnalyzer::student_t_two_tailed_p_value(t, 2);
    assert!(
        (p2 - closed).abs() < 1e-12,
        "df=2 t={t}: p={p2} vs {closed}"
    );
}

#[test]
fn two_tailed_p_is_symmetric_bounded_and_one_at_zero() {
    let plus = StatisticalAnalyzer::student_t_two_tailed_p_value(2.3, 7);
    let minus = StatisticalAnalyzer::student_t_two_tailed_p_value(-2.3, 7);
    assert!((plus - minus).abs() < 1e-15, "sign of t must not matter");

    for df in [1, 4, 9, 50] {
        let at_zero = StatisticalAnalyzer::student_t_two_tailed_p_value(0.0, df);
        assert!(
            (at_zero - 1.0).abs() < 1e-12,
            "df={df}: p at t=0 is {at_zero}"
        );
        let far = StatisticalAnalyzer::student_t_two_tailed_p_value(1e6, df);
        assert!((0.0..1e-5).contains(&far), "df={df}: p at t=1e6 is {far}");
    }

    // Zero degrees of freedom leave the residual variance unknown: no evidence.
    let p0 = StatisticalAnalyzer::student_t_two_tailed_p_value(5.0, 0);
    assert!((p0 - 1.0).abs() < f64::EPSILON);
}

#[test]
fn a_slope_just_under_the_small_sample_critical_value_reads_as_stable() {
    // Seven weekly volumes (km): slope 2.5 km/week, t = 2.549 on df=5 against a
    // critical value of 2.571. The true two-tailed p is 0.0514, above 0.05, so
    // the block is not significant; the normal approximation put it at 0.027.
    let result = regression(&[42.0, 44.0, 40.0, 48.0, 60.0, 54.0, 52.0]);
    assert_eq!(result.degrees_of_freedom, 5);
    assert!((result.slope - 2.5).abs() < 1e-9, "slope={}", result.slope);
    let p = result
        .p_value
        .expect("df>0 and residuals>0 yield a p-value");
    assert!((p - 0.051_36).abs() < 1e-4, "p={p}");

    assert_eq!(
        StatisticalAnalyzer::determine_trend_direction(&result, false, SLOPE_THRESHOLD),
        TrendDirection::Stable
    );
    assert_eq!(
        StatisticalAnalyzer::determine_trend_direction(&result, true, SLOPE_THRESHOLD),
        TrendDirection::Stable
    );
}

#[test]
fn a_clearly_significant_small_sample_slope_still_moves_the_verdict() {
    // Seven weekly values with t = 3.90 on df=5: p = 0.0114, significant at 0.05.
    let result = regression(&[46.0, 49.0, 48.0, 49.0, 54.0, 51.0, 54.0]);
    assert_eq!(result.degrees_of_freedom, 5);
    let p = result
        .p_value
        .expect("df>0 and residuals>0 yield a p-value");
    assert!((p - 0.011_41).abs() < 1e-4, "p={p}");
    assert!(result.slope > SLOPE_THRESHOLD);

    // Rising watts improve; rising pace (lower is better) declines.
    assert_eq!(
        StatisticalAnalyzer::determine_trend_direction(&result, false, SLOPE_THRESHOLD),
        TrendDirection::Improving
    );
    assert_eq!(
        StatisticalAnalyzer::determine_trend_direction(&result, true, SLOPE_THRESHOLD),
        TrendDirection::Declining
    );
}

#[test]
fn a_noisy_small_sample_slope_reads_as_stable_under_either_tail() {
    // t = 1.71 on df=5: p = 0.148, nowhere near significant.
    let result = regression(&[10.0, 12.0, 9.0, 14.0, 11.0, 15.0, 13.0]);
    let p = result
        .p_value
        .expect("df>0 and residuals>0 yield a p-value");
    assert!((p - 0.148_2).abs() < 1e-3, "p={p}");
    assert_eq!(
        StatisticalAnalyzer::determine_trend_direction(&result, false, SLOPE_THRESHOLD),
        TrendDirection::Stable
    );
}
