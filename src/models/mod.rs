// ABOUTME: Data models for sports science intelligence analysis
// ABOUTME: Activity, sport types, time series, nutrition, sleep, recipes, and fitness profiles
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

//! Data models for the dravr-cageux intelligence engine.
//!
//! This module contains all the domain types used by the analysis algorithms,
//! including activity data, sport classifications, physiological profiles,
//! and nutrition/recipe models.

/// Fitness activity models with builder pattern
pub mod activity;
/// Fitness profile and training preferences
pub mod fitness_profile;
/// Maximum heart rate estimation algorithms
pub mod maxhr;
/// Nutrition data models
pub mod nutrition;
/// Recipe models for training-aware meal planning
pub mod recipes;
/// Sleep quality and recovery data
pub mod sleep;
/// Sport type classification
pub mod sport;

// Re-export primary types at module level
pub use activity::{
    Activity, ActivityBuilder, HeartRateZone, PowerZone, SegmentEffort, TimeSeriesData,
};
pub use fitness_profile::{FitnessLevel, TimeAvailability, UserFitnessProfile, UserPreferences};
pub use maxhr::MaxHrAlgorithm;
pub use sport::SportType;
