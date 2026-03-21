// ABOUTME: Unit tests for sports science algorithms (MaxHR, FTP, TRIMP, VDOT, LTHR)
// ABOUTME: Validates mathematical correctness and physiological plausibility
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

use dravr_cageux::algorithms::ftp::FtpAlgorithm;
use dravr_cageux::algorithms::lthr::LthrAlgorithm;
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
