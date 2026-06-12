// ABOUTME: Tests for the seasonality module — season detection, sport-season matrix, and alternatives
// ABOUTME: Validates hemisphere detection, season mapping, and seasonal sport recommendations
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::str_to_string
)]

use dravr_cageux::models::SportType;
use dravr_cageux::seasonality::{
    build_seasonal_context, hemisphere_from_latitude, recommended_sports_for_season,
    season_from_month, seasonal_alternatives, sport_seasonal_suitability,
    sport_suitability_at_location, Hemisphere, Season, SeasonalSuitability,
};

// ============================================================================
// Hemisphere detection
// ============================================================================

#[test]
fn test_northern_hemisphere_from_positive_latitude() {
    assert_eq!(hemisphere_from_latitude(45.87), Hemisphere::Northern); // Prevost, Quebec
    assert_eq!(hemisphere_from_latitude(48.86), Hemisphere::Northern); // Paris
    assert_eq!(hemisphere_from_latitude(90.0), Hemisphere::Northern); // North Pole
    assert_eq!(hemisphere_from_latitude(24.0), Hemisphere::Northern); // Just above tropic
}

#[test]
fn test_southern_hemisphere_from_negative_latitude() {
    assert_eq!(hemisphere_from_latitude(-33.87), Hemisphere::Southern); // Sydney
    assert_eq!(hemisphere_from_latitude(-45.03), Hemisphere::Southern); // Queenstown NZ
    assert_eq!(hemisphere_from_latitude(-90.0), Hemisphere::Southern); // South Pole
    assert_eq!(hemisphere_from_latitude(-24.0), Hemisphere::Southern); // Just below tropic
}

#[test]
fn test_tropical_zone_near_equator() {
    assert_eq!(hemisphere_from_latitude(0.0), Hemisphere::Tropical); // Equator
    assert_eq!(hemisphere_from_latitude(10.0), Hemisphere::Tropical); // Costa Rica
    assert_eq!(hemisphere_from_latitude(-10.0), Hemisphere::Tropical); // Tanzania
    assert_eq!(hemisphere_from_latitude(23.5), Hemisphere::Tropical); // Tropic of Cancer
    assert_eq!(hemisphere_from_latitude(-23.5), Hemisphere::Tropical); // Tropic of Capricorn
}

// ============================================================================
// Season detection
// ============================================================================

#[test]
fn test_northern_hemisphere_seasons() {
    assert_eq!(season_from_month(Hemisphere::Northern, 1), Season::Winter);
    assert_eq!(season_from_month(Hemisphere::Northern, 2), Season::Winter);
    assert_eq!(season_from_month(Hemisphere::Northern, 3), Season::Spring);
    assert_eq!(season_from_month(Hemisphere::Northern, 6), Season::Summer);
    assert_eq!(season_from_month(Hemisphere::Northern, 9), Season::Fall);
    assert_eq!(season_from_month(Hemisphere::Northern, 12), Season::Winter);
}

#[test]
fn test_southern_hemisphere_seasons_inverted() {
    assert_eq!(season_from_month(Hemisphere::Southern, 1), Season::Summer); // Jan = summer
    assert_eq!(season_from_month(Hemisphere::Southern, 6), Season::Winter); // Jun = winter
    assert_eq!(season_from_month(Hemisphere::Southern, 9), Season::Spring); // Sep = spring
    assert_eq!(season_from_month(Hemisphere::Southern, 12), Season::Summer); // Dec = summer
}

#[test]
fn test_tropical_seasons_follow_calendar() {
    assert_eq!(season_from_month(Hemisphere::Tropical, 1), Season::Winter);
    assert_eq!(season_from_month(Hemisphere::Tropical, 4), Season::Spring);
    assert_eq!(season_from_month(Hemisphere::Tropical, 7), Season::Summer);
    assert_eq!(season_from_month(Hemisphere::Tropical, 10), Season::Fall);
}

// ============================================================================
// Build seasonal context
// ============================================================================

#[test]
fn test_build_context_prevost_quebec_january() {
    let ctx = build_seasonal_context(45.87, 1);
    assert_eq!(ctx.hemisphere, Hemisphere::Northern);
    assert_eq!(ctx.season, Season::Winter);
    assert!((ctx.latitude - 45.87).abs() < f64::EPSILON);
    assert_eq!(ctx.month, 1);
}

#[test]
fn test_build_context_sydney_january() {
    let ctx = build_seasonal_context(-33.87, 1);
    assert_eq!(ctx.hemisphere, Hemisphere::Southern);
    assert_eq!(ctx.season, Season::Summer); // January in Southern Hemisphere = Summer
}

// ============================================================================
// Sport-season suitability
// ============================================================================

#[test]
fn test_cycling_inappropriate_in_winter() {
    assert_eq!(
        sport_seasonal_suitability(&SportType::Ride, &Season::Winter),
        SeasonalSuitability::Inappropriate
    );
    assert_eq!(
        sport_seasonal_suitability(&SportType::MountainBike, &Season::Winter),
        SeasonalSuitability::Inappropriate
    );
    assert_eq!(
        sport_seasonal_suitability(&SportType::GravelRide, &Season::Winter),
        SeasonalSuitability::Inappropriate
    );
}

#[test]
fn test_cycling_ideal_in_summer() {
    assert_eq!(
        sport_seasonal_suitability(&SportType::Ride, &Season::Summer),
        SeasonalSuitability::Ideal
    );
}

#[test]
fn test_xc_skiing_ideal_in_winter() {
    assert_eq!(
        sport_seasonal_suitability(&SportType::CrossCountrySkiing, &Season::Winter),
        SeasonalSuitability::Ideal
    );
}

#[test]
fn test_xc_skiing_inappropriate_in_summer() {
    assert_eq!(
        sport_seasonal_suitability(&SportType::CrossCountrySkiing, &Season::Summer),
        SeasonalSuitability::Inappropriate
    );
}

#[test]
fn test_indoor_sports_always_ideal() {
    for season in &[Season::Spring, Season::Summer, Season::Fall, Season::Winter] {
        assert_eq!(
            sport_seasonal_suitability(&SportType::VirtualRide, season),
            SeasonalSuitability::Ideal,
            "VirtualRide should be Ideal in {season:?}"
        );
        assert_eq!(
            sport_seasonal_suitability(&SportType::VirtualRun, season),
            SeasonalSuitability::Ideal,
            "VirtualRun should be Ideal in {season:?}"
        );
        assert_eq!(
            sport_seasonal_suitability(&SportType::Yoga, season),
            SeasonalSuitability::Ideal,
            "Yoga should be Ideal in {season:?}"
        );
        assert_eq!(
            sport_seasonal_suitability(&SportType::StrengthTraining, season),
            SeasonalSuitability::Ideal,
            "StrengthTraining should be Ideal in {season:?}"
        );
    }
}

#[test]
fn test_running_acceptable_in_winter() {
    assert_eq!(
        sport_seasonal_suitability(&SportType::Run, &Season::Winter),
        SeasonalSuitability::Acceptable
    );
}

#[test]
fn test_suitability_is_recommended() {
    assert!(SeasonalSuitability::Ideal.is_recommended());
    assert!(SeasonalSuitability::Acceptable.is_recommended());
    assert!(!SeasonalSuitability::Marginal.is_recommended());
    assert!(!SeasonalSuitability::Inappropriate.is_recommended());
}

// ============================================================================
// Tropical override
// ============================================================================

#[test]
fn test_tropical_cycling_always_ideal() {
    let ctx = build_seasonal_context(10.0, 1); // Costa Rica, January
    assert_eq!(ctx.hemisphere, Hemisphere::Tropical);
    assert_eq!(
        sport_suitability_at_location(&SportType::Ride, &ctx),
        SeasonalSuitability::Ideal
    );
}

#[test]
fn test_tropical_skiing_still_ideal() {
    // In tropics, even "winter sports" get Ideal because there's no cold
    // This is correct — it's up to the platform layer to check actual conditions
    let ctx = build_seasonal_context(5.0, 7); // Equator, July
    assert_eq!(
        sport_suitability_at_location(&SportType::CrossCountrySkiing, &ctx),
        SeasonalSuitability::Ideal
    );
}

// ============================================================================
// Seasonal alternatives
// ============================================================================

#[test]
fn test_cycling_winter_alternatives() {
    let alts = seasonal_alternatives(&SportType::Ride, &Season::Winter);
    assert!(
        !alts.is_empty(),
        "Should suggest alternatives for cycling in winter"
    );
    assert!(
        alts.contains(&SportType::VirtualRide),
        "Should suggest indoor cycling"
    );
    assert!(
        alts.contains(&SportType::CrossCountrySkiing),
        "Should suggest XC skiing"
    );
}

#[test]
fn test_xc_skiing_summer_alternatives() {
    let alts = seasonal_alternatives(&SportType::CrossCountrySkiing, &Season::Summer);
    assert!(
        !alts.is_empty(),
        "Should suggest alternatives for XC skiing in summer"
    );
    assert!(
        alts.contains(&SportType::TrailRunning),
        "Should suggest trail running"
    );
}

#[test]
fn test_no_alternatives_for_appropriate_sport() {
    let alts = seasonal_alternatives(&SportType::Ride, &Season::Summer);
    assert!(
        alts.is_empty(),
        "No alternatives needed for cycling in summer"
    );
}

#[test]
fn test_no_alternatives_for_indoor_sport() {
    let alts = seasonal_alternatives(&SportType::VirtualRide, &Season::Winter);
    assert!(alts.is_empty(), "No alternatives needed for indoor cycling");
}

// ============================================================================
// Recommended sports per season
// ============================================================================

#[test]
fn test_winter_recommended_sports_include_skiing() {
    let sports = recommended_sports_for_season(&Season::Winter);
    assert!(
        sports.contains(&SportType::CrossCountrySkiing),
        "Winter should include XC skiing"
    );
    assert!(
        sports.contains(&SportType::AlpineSkiing),
        "Winter should include alpine skiing"
    );
    assert!(
        !sports.contains(&SportType::Ride),
        "Winter should NOT include outdoor cycling"
    );
}

#[test]
fn test_summer_recommended_sports_include_cycling() {
    let sports = recommended_sports_for_season(&Season::Summer);
    assert!(
        sports.contains(&SportType::Ride),
        "Summer should include cycling"
    );
    assert!(
        sports.contains(&SportType::Run),
        "Summer should include running"
    );
    assert!(
        !sports.contains(&SportType::AlpineSkiing),
        "Summer should NOT include alpine skiing"
    );
}

// ============================================================================
// The Petit Train du Nord scenario
// ============================================================================

#[test]
fn test_petit_train_du_nord_scenario() {
    // April in Prevost, Quebec — Pierre should NOT suggest cycling
    let ctx = build_seasonal_context(45.87, 4); // April
    assert_eq!(ctx.hemisphere, Hemisphere::Northern);
    assert_eq!(ctx.season, Season::Spring);

    // In spring, cycling is acceptable (not inappropriate)
    let cycling_suitability = sport_seasonal_suitability(&SportType::Ride, &Season::Spring);
    assert_eq!(cycling_suitability, SeasonalSuitability::Acceptable);

    // But in January (the real problem case), cycling is inappropriate
    let winter_ctx = build_seasonal_context(45.87, 1);
    assert_eq!(winter_ctx.season, Season::Winter);
    let winter_cycling = sport_seasonal_suitability(&SportType::Ride, &Season::Winter);
    assert_eq!(winter_cycling, SeasonalSuitability::Inappropriate);

    // And XC skiing on the same trail would be ideal
    let xc_skiing = sport_seasonal_suitability(&SportType::CrossCountrySkiing, &Season::Winter);
    assert_eq!(xc_skiing, SeasonalSuitability::Ideal);

    // Alternatives for cycling in winter include XC skiing
    let alts = seasonal_alternatives(&SportType::Ride, &Season::Winter);
    assert!(alts.contains(&SportType::CrossCountrySkiing));
    assert!(alts.contains(&SportType::VirtualRide));
}
