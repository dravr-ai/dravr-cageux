// ABOUTME: Tests that training-intensity recommendations use the athlete's own max heart rate
// ABOUTME: Pins that identical sessions classify differently by age and are unclassified without one
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::str_to_string
)]

use chrono::{Duration, Utc};
use dravr_cageux::config::intelligence::IntelligenceConfig;
use dravr_cageux::models::{
    Activity, ActivityBuilder, FitnessLevel, SportType, TimeAvailability, UserFitnessProfile,
    UserPreferences,
};
use dravr_cageux::recommendation_engine::{
    AdvancedRecommendationEngine, RecommendationEngineTrait,
};
use dravr_cageux::types::{RecommendationType, TrainingRecommendation};

/// Average heart rate of every session in this fixture.
///
/// It sits above 80% of a 70-year-old's Tanaka-predicted maximum (159 bpm, so a
/// 127 bpm threshold) and below 80% of a 25-year-old's (190.5 bpm, so 152 bpm).
/// The same sessions are therefore hard for one athlete and easy for the other,
/// which is exactly what a single hard-coded ceiling cannot express.
const SESSION_HEART_RATE: u32 = 148;

fn profile(age: Option<i32>) -> UserFitnessProfile {
    UserFitnessProfile {
        user_id: "athlete".to_owned(),
        age,
        gender: None,
        weight: Some(70.0),
        height: Some(175.0),
        fitness_level: FitnessLevel::Intermediate,
        primary_sports: vec!["run".to_owned()],
        training_history_months: 24,
        preferences: UserPreferences {
            preferred_units: "metric".to_owned(),
            training_focus: vec!["endurance".to_owned()],
            injury_history: vec![],
            time_availability: TimeAvailability {
                hours_per_week: 6.0,
                preferred_days: vec!["monday".to_owned()],
                preferred_duration_minutes: Some(60),
            },
        },
        seasonal_context: None,
    }
}

fn recent_runs() -> Vec<Activity> {
    let now = Utc::now();
    (0..8)
        .map(|i| {
            ActivityBuilder::new(
                format!("a{i}"),
                format!("run {i}"),
                SportType::Run,
                now - Duration::days(i * 2),
                3600,
                "synthetic",
            )
            .distance_meters(10_000.0)
            .average_heart_rate(SESSION_HEART_RATE)
            .build()
        })
        .collect()
}

async fn intensity_titles(age: Option<i32>) -> Vec<String> {
    let config = IntelligenceConfig::load().expect("default intelligence config");
    let engine = AdvancedRecommendationEngine::new(&config);
    let recommendations = engine
        .generate_recommendations(&profile(age), &recent_runs())
        .await
        .expect("recommendations");

    recommendations
        .iter()
        .filter(|r: &&TrainingRecommendation| {
            matches!(r.recommendation_type, RecommendationType::Intensity)
        })
        .map(|r| r.title.clone())
        .collect()
}

#[tokio::test]
async fn identical_sessions_are_hard_for_an_older_athlete_and_easy_for_a_younger_one() {
    let older = intensity_titles(Some(70)).await;
    let younger = intensity_titles(Some(25)).await;

    assert_eq!(
        older,
        vec!["Add More Easy Training".to_owned()],
        "148 bpm is above 80% of a 70-year-old's predicted max, so the block reads as hard"
    );
    assert_eq!(
        younger,
        vec!["Increase Training Intensity".to_owned()],
        "148 bpm is below 80% of a 25-year-old's predicted max, so the block reads as easy"
    );
}

#[tokio::test]
async fn an_athlete_with_no_age_gets_no_intensity_verdict() {
    // Without an age there is no defensible heart-rate ceiling, so the engine
    // reports no intensity balance rather than judging the sessions against an
    // invented maximum.
    let titles = intensity_titles(None).await;
    assert!(
        titles.is_empty(),
        "no ceiling is knowable, so no intensity verdict should be issued: {titles:?}"
    );
}
