// ABOUTME: Unit tests for sports science algorithms (MaxHR, FTP, TRIMP, VDOT, LTHR)
// ABOUTME: Validates mathematical correctness and physiological plausibility
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::str_to_string
)]

use dravr_cageux::algorithms::ftp::FtpAlgorithm;
use dravr_cageux::algorithms::lthr::LthrAlgorithm;
use dravr_cageux::algorithms::training_load::{TrainingLoadAlgorithm, TssDataPoint};
use dravr_cageux::algorithms::trimp::TrimpAlgorithm;
use dravr_cageux::algorithms::vdot::VdotAlgorithm;
use dravr_cageux::algorithms::vo2max::Vo2maxAlgorithm;
use dravr_cageux::models::MaxHrAlgorithm;

// ============================================================================
// MaxHR Algorithm Tests
// ============================================================================

#[test]
fn maxhr_tanaka_age_40() {
    let result = MaxHrAlgorithm::Tanaka.estimate(40, None).unwrap();
    assert!((result - 180.0).abs() < 0.1);
}

#[test]
fn maxhr_fox_age_30() {
    let result = MaxHrAlgorithm::Fox.estimate(30, None).unwrap();
    assert!((result - 190.0).abs() < 0.1);
}

#[test]
fn maxhr_nes_age_50() {
    let result = MaxHrAlgorithm::Nes.estimate(50, None).unwrap();
    assert!((result - 179.0).abs() < 0.1);
}

#[test]
fn maxhr_gulati_female_age_35() {
    let result = MaxHrAlgorithm::Gulati.estimate(35, Some("female")).unwrap();
    assert!((result - 175.2).abs() < 0.1);
}

#[test]
fn maxhr_gulati_male_falls_back_to_tanaka() {
    let gulati_male = MaxHrAlgorithm::Gulati.estimate(40, Some("male")).unwrap();
    let tanaka = MaxHrAlgorithm::Tanaka.estimate(40, None).unwrap();
    assert!((gulati_male - tanaka).abs() < 0.1);
}

#[test]
fn maxhr_rejects_age_zero() {
    assert!(MaxHrAlgorithm::Tanaka.estimate(0, None).is_err());
}

#[test]
fn maxhr_rejects_age_over_120() {
    assert!(MaxHrAlgorithm::Tanaka.estimate(121, None).is_err());
}

#[test]
fn maxhr_from_str_parsing() {
    assert_eq!(
        "tanaka".parse::<MaxHrAlgorithm>().unwrap(),
        MaxHrAlgorithm::Tanaka
    );
    assert_eq!(
        "fox".parse::<MaxHrAlgorithm>().unwrap(),
        MaxHrAlgorithm::Fox
    );
    assert!("invalid".parse::<MaxHrAlgorithm>().is_err());
}

// ============================================================================
// FTP Algorithm Tests (data in enum variants, estimate_ftp takes no args)
// ============================================================================

#[test]
fn ftp_twenty_minute_test() {
    let algo = FtpAlgorithm::From20MinTest {
        avg_power_20min: 300.0,
    };
    let result = algo.estimate_ftp().unwrap();
    // 300 * 0.95 = 285
    assert!((result - 285.0).abs() < 0.1, "FTP 20min: {result}");
}

#[test]
fn ftp_ramp_test() {
    let algo = FtpAlgorithm::FromRampTest {
        max_1min_power: 400.0,
    };
    let result = algo.estimate_ftp().unwrap();
    // 400 * 0.75 = 300
    assert!((result - 300.0).abs() < 0.1, "FTP ramp: {result}");
}

// ============================================================================
// TRIMP Algorithm Tests
// ============================================================================

#[test]
fn trimp_bannister_male_basic() {
    // calculate(avg_hr: u32, duration_minutes: f64, max_hr: u32, resting_hr: Option<u32>, gender: Option<&str>)
    let result = TrimpAlgorithm::BannisterMale
        .calculate(150, 60.0, 190, Some(60), Some("male"))
        .unwrap();
    assert!(result > 50.0, "TRIMP: {result}");
    assert!(result < 300.0, "TRIMP: {result}");
}

#[test]
fn trimp_rejects_resting_above_max() {
    // resting_hr (200) > max_hr (190) should error
    let result = TrimpAlgorithm::BannisterMale.calculate(150, 60.0, 190, Some(200), None);
    assert!(result.is_err());
}

// ============================================================================
// VDOT Algorithm Tests (calculate_vdot takes distance and time)
// ============================================================================

#[test]
fn vdot_daniels_5k_20min() {
    let result = VdotAlgorithm::Daniels
        .calculate_vdot(5000.0, 1200.0)
        .unwrap();
    assert!(result > 35.0, "VDOT: {result}");
    assert!(result < 55.0, "VDOT: {result}");
}

#[test]
fn vdot_rejects_zero_distance() {
    assert!(VdotAlgorithm::Daniels.calculate_vdot(0.0, 1200.0).is_err());
}

#[test]
fn vdot_rejects_zero_time() {
    assert!(VdotAlgorithm::Daniels.calculate_vdot(5000.0, 0.0).is_err());
}

// ============================================================================
// VO2max Algorithm Tests (data in enum variants, estimate_vo2max takes no args)
// ============================================================================

#[test]
fn vo2max_cooper_test() {
    let algo = Vo2maxAlgorithm::CooperTest {
        distance_meters: 3000.0,
    };
    let result = algo.estimate_vo2max().unwrap();
    assert!(result > 30.0, "VO2max: {result}");
    assert!(result < 70.0, "VO2max: {result}");
}

// ============================================================================
// LTHR Algorithm Tests (data in enum variants, estimate_lthr takes no args)
// ============================================================================

#[test]
fn lthr_from_max_hr() {
    let algo = LthrAlgorithm::FromMaxHR {
        max_hr: 190.0,
        percentage: 0.85,
    };
    let result = algo.estimate_lthr().unwrap();
    assert!((result - 161.5).abs() < 1.0, "LTHR: {result}");
}

// ============================================================================
// VO2max — the published formulas, pinned to their sources
// ============================================================================

#[test]
fn vo2max_rockport_matches_kline_1987() {
    // Kline et al. (1987), worked by hand:
    //   132.853 - 0.0769(70) - 0.3877(40) + 6.315(1) - 3.2649(13) - 0.1565(140)
    // = 132.853 - 5.383 - 15.508 + 6.315 - 42.4437 - 21.91 = 53.92
    // A sign flip on the time or heart-rate term lands near 182, which is why
    // this asserts the value and not merely a physiological floor.
    let algo = Vo2maxAlgorithm::RockportWalk {
        weight_kg: 70.0,
        age: 40,
        gender: 1,
        time_seconds: 780.0,
        heart_rate: 140.0,
    };
    let result = algo.estimate_vo2max().unwrap();
    assert!((result - 53.92).abs() < 0.1, "Rockport VO2max: {result}");
}

#[test]
fn vo2max_rockport_falls_with_slower_walk_and_higher_hr() {
    let base = |secs: f64, hr: f64| Vo2maxAlgorithm::RockportWalk {
        weight_kg: 70.0,
        age: 40,
        gender: 1,
        time_seconds: secs,
        heart_rate: hr,
    };
    let fast = base(720.0, 130.0).estimate_vo2max().unwrap();
    let slow = base(900.0, 160.0).estimate_vo2max().unwrap();
    assert!(
        slow < fast,
        "a slower walk at a higher heart rate must estimate lower: fast={fast}, slow={slow}"
    );
}

#[test]
fn vo2max_from_vdot_is_already_ml_per_kg_per_min() {
    // VDOT is Daniels' economy-adjusted VO2max, already in ml/kg/min. The old
    // code multiplied by 3.5 (the MET factor) and reported 175 for a VDOT-50
    // runner — roughly double the highest value ever measured in a human.
    let result = Vo2maxAlgorithm::FromVdot { vdot: 50.0 }
        .estimate_vo2max()
        .unwrap();
    assert!((result - 50.0).abs() < f64::EPSILON, "VO2max: {result}");
}

// ============================================================================
// Training load — the chronic and acute estimates must actually differ
// ============================================================================

#[test]
fn kalman_chronic_and_acute_estimates_diverge() {
    // Both CTL and ATL used to dispatch to the same windowless call, so they
    // returned the identical number and TSB was identically zero — which bands
    // every athlete as balanced no matter what they have done. A ramp makes the
    // acute estimate ride above the chronic one.
    let now = chrono::Utc::now();
    let tss_data: Vec<TssDataPoint> = (0..42_i32)
        .map(|day| TssDataPoint {
            date: now - chrono::Duration::days(i64::from(41 - day)),
            tss: f64::from(day).mul_add(3.0, 40.0),
        })
        .collect();

    let algo = TrainingLoadAlgorithm::KalmanFilter {
        ctl_days: 42,
        atl_days: 7,
        process_noise: 1.0,
        measurement_noise: 10.0,
    };

    let ctl = algo.calculate_ctl(&tss_data).unwrap();
    let atl = algo.calculate_atl(&tss_data).unwrap();

    assert!(
        (ctl - atl).abs() > 1.0,
        "chronic and acute must not collapse to one value: ctl={ctl}, atl={atl}"
    );
    assert!(
        atl > ctl,
        "on a rising ramp the 7-day estimate must lead the 42-day one: ctl={ctl}, atl={atl}"
    );
}

#[test]
fn kalman_rejects_a_non_positive_window() {
    let now = chrono::Utc::now();
    let tss_data = vec![TssDataPoint {
        date: now,
        tss: 50.0,
    }];
    let algo = TrainingLoadAlgorithm::KalmanFilter {
        ctl_days: 0,
        atl_days: 7,
        process_noise: 1.0,
        measurement_noise: 10.0,
    };
    assert!(algo.calculate_ctl(&tss_data).is_err());
}
