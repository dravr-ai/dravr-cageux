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

// ============================================================================
// Foundation modules
// ============================================================================

/// Server and intelligence configuration
pub mod config;
/// Domain constants for sports science calculations
pub mod constants;
/// Structured error types for the intelligence engine
pub mod error;
/// Data models (Activity, SportType, TimeSeriesData, etc.)
pub mod models;
/// Core intelligence types (`ActivityIntelligence`, `TrendDirection`, Goals, etc.)
pub mod types;

// ============================================================================
// Algorithm modules
// ============================================================================

/// Pluggable algorithms for FTP, LTHR, VO2max, TSS, TRIMP, VDOT, etc.
pub mod algorithms;
/// Physiological constants and thresholds
pub mod physiological_constants;

// ============================================================================
// Analysis modules
// ============================================================================

/// Advanced activity analysis with contextual insights
pub mod activity_analyzer;
/// Configuration for analysis parameters and thresholds
pub mod analysis_config;
/// Core single-activity analyzer
pub mod analyzer;
/// Insight generation and categorization
pub mod insights;
/// Performance metrics calculation
pub mod metrics;
/// Safe metrics extraction with validation
pub mod metrics_extractor;
/// Training pattern detection and analysis
pub mod pattern_detection;
/// Performance analysis engine (v1)
pub mod performance_analyzer;
/// Performance analyzer v2 with improved algorithms
pub mod performance_analyzer_v2;
/// Performance prediction and race time estimation
pub mod performance_prediction;
/// Statistical analysis and regression
pub mod statistical_analysis;
/// Training load calculation and monitoring
pub mod training_load;
/// Visitor pattern for single-pass activity time series analysis
pub mod visitor;

// ============================================================================
// Domain modules
// ============================================================================

/// Goal tracking and progress monitoring engine
pub mod goal_engine;
/// Nutrition needs calculation and meal planning
pub mod nutrition_calculator;
/// Recipe management with training-aware suggestions
pub mod recipes;
/// Training recommendation generation
pub mod recommendation_engine;
/// Recovery score calculation and recommendations
pub mod recovery_calculator;
/// Sleep quality analysis and HRV tracking
pub mod sleep_analysis;

// ============================================================================
// Social intelligence modules
// ============================================================================

/// In-memory cache for friend activity summaries
pub mod friend_activity_cache;
/// Adapts shared insights to user's training context
pub mod insight_adapter;

// ============================================================================
// Re-exports
// ============================================================================

pub use config::ServerConfig;
pub use error::{IntelligenceError, IntelligenceResult};
pub use models::{
    Activity, ActivityBuilder, FitnessLevel, HeartRateZone, MaxHrAlgorithm, PowerZone,
    SegmentEffort, SportType, TimeAvailability, TimeSeriesData, UserFitnessProfile,
    UserPreferences,
};
