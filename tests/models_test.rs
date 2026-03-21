// ABOUTME: Unit tests for data models (Activity, SportType, builder pattern)
// ABOUTME: Validates model construction, serialization, and type conversions
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

use chrono::Utc;
use dravr_cageux::models::{Activity, ActivityBuilder, SportType, TimeSeriesData};

#[test]
fn activity_builder_required_fields() {
    let activity = ActivityBuilder::new(
        "123",
        "Morning Run",
        SportType::Run,
        Utc::now(),
        1800,
        "strava",
    )
    .build();

    assert_eq!(activity.id(), "123");
    assert_eq!(activity.name(), "Morning Run");
    assert_eq!(activity.sport_type(), &SportType::Run);
    assert_eq!(activity.duration_seconds(), 1800);
    assert_eq!(activity.provider(), "strava");
    assert!(activity.distance_meters().is_none());
}

#[test]
fn activity_builder_optional_fields() {
    let activity = ActivityBuilder::new(
        "456",
        "Hill Ride",
        SportType::Ride,
        Utc::now(),
        3600,
        "garmin",
    )
    .distance_meters(40_000.0)
    .elevation_gain(800.0)
    .average_heart_rate(155)
    .max_heart_rate(178)
    .average_power(250)
    .max_power(500)
    .calories(900)
    .average_cadence(90)
    .ftp(280)
    .city("Montreal".to_owned())
    .country("Canada".to_owned())
    .build();

    assert_eq!(activity.distance_meters(), Some(40_000.0));
    assert_eq!(activity.elevation_gain(), Some(800.0));
    assert_eq!(activity.average_heart_rate(), Some(155));
    assert_eq!(activity.max_heart_rate(), Some(178));
    assert_eq!(activity.average_power(), Some(250));
    assert_eq!(activity.max_power(), Some(500));
    assert_eq!(activity.calories(), Some(900));
    assert_eq!(activity.average_cadence(), Some(90));
    assert_eq!(activity.ftp(), Some(280));
    assert_eq!(activity.city(), Some("Montreal"));
    assert_eq!(activity.country(), Some("Canada"));
}

#[test]
fn activity_serialization_roundtrip() {
    let activity = ActivityBuilder::new("789", "Swim", SportType::Swim, Utc::now(), 2400, "whoop")
        .distance_meters(1500.0)
        .build();

    let json = serde_json::to_string(&activity).unwrap();
    let deserialized: Activity = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.id(), "789");
    assert_eq!(deserialized.name(), "Swim");
    assert_eq!(deserialized.distance_meters(), Some(1500.0));
}

#[test]
fn sport_type_serialization() {
    let run = SportType::Run;
    let json = serde_json::to_string(&run).unwrap();
    assert_eq!(json, "\"run\"");

    let deserialized: SportType = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, SportType::Run);
}

#[test]
fn sport_type_from_provider_string() {
    assert_eq!(SportType::from_provider_string("Run", None), SportType::Run);
    assert_eq!(
        SportType::from_provider_string("Ride", None),
        SportType::Ride
    );
    assert_eq!(
        SportType::from_provider_string("MountainBikeRide", None),
        SportType::MountainBike
    );
    assert_eq!(
        SportType::from_provider_string("VirtualRide", None),
        SportType::VirtualRide
    );
}

#[test]
fn sport_type_other_variant() {
    let sport = SportType::from_provider_string("UnknownSport", None);
    assert!(matches!(sport, SportType::Other(_)));
}

#[test]
fn time_series_data_with_heart_rate() {
    let ts = TimeSeriesData {
        timestamps: vec![0, 1, 2, 3, 4],
        heart_rate: Some(vec![120, 130, 140, 150, 145]),
        power: None,
        cadence: None,
        speed: None,
        altitude: None,
        temperature: None,
        gps_coordinates: None,
    };

    assert_eq!(ts.timestamps.len(), 5);
    assert_eq!(ts.heart_rate.as_ref().unwrap().len(), 5);
    assert!(ts.power.is_none());
}

#[test]
fn activity_default_has_reasonable_values() {
    let activity = Activity::default();
    assert_eq!(activity.duration_seconds(), 1800);
    assert_eq!(activity.distance_meters(), Some(5000.0));
    assert_eq!(activity.sport_type(), &SportType::Run);
}
