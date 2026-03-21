// ABOUTME: Sports science intelligence engine for fitness analysis
// ABOUTME: VDOT, TSS, TRIMP, FTP, VO2max, recovery, nutrition, and performance analysis
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

#![deny(unsafe_code)]

//! # dravr-cageux
//!
//! Standalone sports science intelligence engine providing fitness analysis
//! algorithms, performance metrics, and training recommendations.
//!
//! ## Features
//!
//! - **Activity analysis** — single-activity and multi-activity insights
//! - **Performance tracking** — trend analysis, VDOT, FTP estimation
//! - **Training load** — TSS, TRIMP, acute/chronic training load balance
//! - **Recovery** — recovery score calculation with HRV integration
//! - **Sleep analysis** — sleep quality scoring and HRV trend detection
//! - **Nutrition** — TDEE, BMR, macronutrient planning, meal timing
//! - **Goal tracking** — progress monitoring and adjustment recommendations
//! - **Pattern detection** — overtraining signals, hard/easy balance
//!
//! ## Quick Start
//!
//! ```rust
//! use dravr_cageux::models::{ActivityBuilder, SportType};
//! use chrono::Utc;
//!
//! let activity = ActivityBuilder::new(
//!     "12345",
//!     "Morning Run",
//!     SportType::Run,
//!     Utc::now(),
//!     1800,
//!     "strava",
//! )
//! .distance_meters(5000.0)
//! .average_heart_rate(150)
//! .build();
//! ```

/// Server configuration loaded from environment variables
pub mod config;
/// Domain constants for sports science calculations
pub mod constants;
/// Structured error types for the intelligence engine
pub mod error;
/// Data models (Activity, SportType, TimeSeriesData, etc.)
pub mod models;

// Re-export primary types at crate root for convenience
pub use config::ServerConfig;
pub use error::{IntelligenceError, IntelligenceResult};
pub use models::{
    Activity, ActivityBuilder, FitnessLevel, HeartRateZone, MaxHrAlgorithm, PowerZone,
    SegmentEffort, SportType, TimeAvailability, TimeSeriesData, UserFitnessProfile,
    UserPreferences,
};
