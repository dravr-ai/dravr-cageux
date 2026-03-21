// ABOUTME: Constants module with domain-separated organization for sports science
// ABOUTME: Physiological thresholds, time conversions, and measurement constants
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

//! Constants module
//!
//! Organizes sports science constants by domain for better maintainability.

/// Unit conversion and measurement constants
pub mod units;

/// Activity and data limits
pub mod limits {
    /// Default activities fetch limit
    pub const DEFAULT_ACTIVITIES_LIMIT: usize = 20;
    /// Maximum activities that can be fetched in one request
    pub const MAX_ACTIVITIES_FETCH: usize = 100;
    /// Minutes per hour
    pub const MINUTES_PER_HOUR: u64 = 60;
    /// Seconds per minute
    pub const SECONDS_PER_MINUTE: u64 = 60;
    /// Maximum timeframe days
    pub const MAX_TIMEFRAME_DAYS: u32 = 365;
    /// Activity capacity hint for pre-allocation
    pub const ACTIVITY_CAPACITY_HINT: usize = 50;
    /// Meters per kilometer
    pub const METERS_PER_KILOMETER: f64 = 1000.0;
    /// Percentage multiplier (multiply by 100)
    pub const PERCENTAGE_MULTIPLIER: f64 = 100.0;
    /// Default confidence threshold for analysis
    pub const DEFAULT_CONFIDENCE_THRESHOLD: f64 = 0.85;
}

/// Time-related constants for duration calculations
pub mod time_constants {
    /// Seconds in one hour
    pub const SECONDS_PER_HOUR: u64 = 3600;
    /// Seconds in one minute
    pub const SECONDS_PER_MINUTE: u64 = 60;
    /// Seconds in one hour (f64 for floating-point calculations)
    pub const SECONDS_PER_HOUR_F64: f64 = 3600.0;
    /// Seconds in one day
    pub const SECONDS_PER_DAY: u64 = 86_400;
    /// Seconds in one week
    pub const SECONDS_PER_WEEK: u64 = 604_800;
    /// Seconds in one month (30 days)
    pub const SECONDS_PER_MONTH: u64 = 2_592_000;
    /// Seconds in one year (365 days)
    pub const SECONDS_PER_YEAR: u64 = 31_536_000;
    /// Minutes in one hour
    pub const MINUTES_PER_HOUR: u64 = 60;
    /// Hours in one day
    pub const HOURS_PER_DAY: u64 = 24;
    /// Days in one week
    pub const DAYS_PER_WEEK: u64 = 7;
    /// Days in one month (30-day approximation)
    pub const DAYS_PER_MONTH: u64 = 30;
    /// Days in one quarter (90 days)
    pub const DAYS_PER_QUARTER: u64 = 90;
    /// Days in one year
    pub const DAYS_PER_YEAR: u64 = 365;
}

/// Physiological constants for fitness analysis
pub mod physiology {
    /// Default resting heart rate (BPM)
    pub const DEFAULT_RESTING_HR: u32 = 60;
    /// Default maximum heart rate (BPM)
    pub const DEFAULT_MAX_HR: u32 = 190;

    /// Minimum good ground contact time in milliseconds
    pub const MIN_GOOD_GCT_MS: f64 = 180.0;
    /// Maximum good ground contact time in milliseconds
    pub const MAX_GOOD_GCT_MS: f64 = 250.0;
    /// Optimal ground contact time in milliseconds
    pub const OPTIMAL_GCT_MS: f64 = 215.0;

    /// Minimum stride length in meters (below this is likely walking)
    pub const MIN_STRIDE_LENGTH_M: f64 = 0.5;
    /// Maximum realistic stride length in meters
    pub const MAX_STRIDE_LENGTH_M: f64 = 3.0;

    /// Minimum cadence (steps/min) for running
    pub const MIN_RUNNING_CADENCE: u32 = 140;
    /// Optimal cadence (steps/min) for efficient running
    pub const OPTIMAL_RUNNING_CADENCE: u32 = 180;

    /// Maximum physiologically possible heart rate
    pub const MAX_PHYSIOLOGICAL_HR: u32 = 220;
    /// Minimum resting heart rate for a healthy adult
    pub const MIN_RESTING_HR: u32 = 30;
}

/// Goal management thresholds and constants
pub mod goal_management {
    /// Minimum weekly activities for beginner fitness level
    pub const BEGINNER_MIN_WEEKLY_ACTIVITIES: u32 = 1;
    /// Minimum weekly activities for intermediate fitness level
    pub const INTERMEDIATE_MIN_WEEKLY_ACTIVITIES: u32 = 3;
    /// Minimum weekly activities for advanced fitness level
    pub const ADVANCED_MIN_WEEKLY_ACTIVITIES: u32 = 5;
    /// Minimum weekly activities for elite fitness level
    pub const ELITE_MIN_WEEKLY_ACTIVITIES: u32 = 7;

    /// Maximum reasonable weekly training hours
    pub const MAX_WEEKLY_TRAINING_HOURS: f64 = 30.0;
    /// Minimum training history months for trend analysis
    pub const MIN_TRAINING_HISTORY_MONTHS: u32 = 3;
}
