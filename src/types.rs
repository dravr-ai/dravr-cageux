// ABOUTME: Core intelligence types for activity analysis, trends, goals, and recommendations
// ABOUTME: Structs and enums used across the intelligence engine modules
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

use chrono::{DateTime, Utc};
pub use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::insights::Insight;
pub use crate::metrics::AdvancedMetrics;
use crate::physiological_constants::fitness_score_thresholds::{
    EXCELLENT_PERFORMANCE_THRESHOLD, GOOD_PERFORMANCE_THRESHOLD, MODERATE_PERFORMANCE_THRESHOLD,
};

// Re-export commonly needed items so modules can import from crate::types
pub use crate::metrics::MetricsCalculator;
pub use crate::models::fitness_profile::{FitnessLevel, UserFitnessProfile};
pub use crate::physiological_constants::fitness_score_thresholds::{
    FITNESS_IMPROVING_THRESHOLD, FITNESS_STABLE_THRESHOLD, MIN_STATISTICAL_SIGNIFICANCE_POINTS,
    SMALL_DATASET_REDUCTION_FACTOR, STATISTICAL_SIGNIFICANCE_THRESHOLD, STRENGTH_ENDURANCE_DIVISOR,
};

/// Activity intelligence summary with insights and analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityIntelligence {
    /// Natural language summary of the activity
    pub summary: String,

    /// Key insights extracted from the activity
    pub key_insights: Vec<Insight>,

    /// Performance metrics and indicators
    pub performance_indicators: PerformanceMetrics,

    /// Contextual factors affecting the activity
    pub contextual_factors: ContextualFactors,

    /// Timestamp when the analysis was generated
    pub generated_at: DateTime<Utc>,
}

/// Performance metrics derived from activity analysis
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PerformanceMetrics {
    /// Relative effort (1-10 scale)
    pub relative_effort: Option<f32>,

    /// Zone distribution (percentage in each zone)
    pub zone_distribution: Option<ZoneDistribution>,

    /// Personal records achieved
    pub personal_records: Vec<PersonalRecord>,

    /// Efficiency score (0-100)
    pub efficiency_score: Option<f32>,

    /// Comparison with recent activities
    pub trend_indicators: TrendIndicators,
}

/// Heart rate or power zone distribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneDistribution {
    /// Percentage of time in Zone 1 (Recovery, <60% max HR)
    pub zone1_recovery: f32,
    /// Percentage of time in Zone 2 (Endurance, 60-70% max HR)
    pub zone2_endurance: f32,
    /// Percentage of time in Zone 3 (Tempo, 70-80% max HR)
    pub zone3_tempo: f32,
    /// Percentage of time in Zone 4 (Threshold, 80-90% max HR)
    pub zone4_threshold: f32,
    /// Percentage of time in Zone 5 (VO2 Max, >90% max HR)
    pub zone5_vo2max: f32,
}

/// Personal record information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalRecord {
    /// Type of record (e.g., `fastest_5k`, `longest_run`)
    pub record_type: String,
    /// Record value
    pub value: f64,
    /// Unit of measurement (e.g., "seconds", "meters")
    pub unit: String,
    /// Previous best value before this record
    pub previous_best: Option<f64>,
    /// Improvement over previous best as percentage
    pub improvement_percentage: Option<f32>,
}

/// Trend indicators comparing to recent activities
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TrendIndicators {
    /// Trend in pace performance
    pub pace_trend: TrendDirection,
    /// Trend in effort levels
    pub effort_trend: TrendDirection,
    /// Trend in distance covered
    pub distance_trend: TrendDirection,
    /// Consistency score (0-100, higher is more consistent)
    pub consistency_score: f32,
}

/// Direction of a trend
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum TrendDirection {
    /// Performance is improving
    Improving,
    /// Performance is stable
    #[default]
    Stable,
    /// Performance is declining
    Declining,
}

/// Contextual factors that might affect performance
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContextualFactors {
    /// Weather conditions during the activity
    pub weather: Option<WeatherConditions>,
    /// Location where the activity took place
    pub location: Option<LocationContext>,
    /// Time of day when activity occurred
    pub time_of_day: TimeOfDay,
    /// Number of days since last activity
    pub days_since_last_activity: Option<i32>,
    /// Weekly training load context
    pub weekly_load: Option<ContextualWeeklyLoad>,
}

/// Weather conditions during activity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherConditions {
    /// Temperature in degrees Celsius
    pub temperature_celsius: f32,
    /// Relative humidity as percentage
    pub humidity_percentage: Option<f32>,
    /// Wind speed in kilometers per hour
    pub wind_speed_kmh: Option<f32>,
    /// Weather conditions description (e.g., "sunny", "rainy", "cloudy")
    pub conditions: String,
}

/// Location context for the activity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationContext {
    /// City name
    pub city: Option<String>,
    /// State or region name
    pub region: Option<String>,
    /// Country name
    pub country: Option<String>,
    /// Trail or route name if applicable
    pub trail_name: Option<String>,
    /// Terrain type (e.g., "road", "trail", "track")
    pub terrain_type: Option<String>,
    /// Human-readable display name for the location
    pub display_name: String,
}

/// Time of day categorization
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TimeOfDay {
    /// Early morning (5-7 AM)
    EarlyMorning,
    /// Morning (7-11 AM)
    #[default]
    Morning,
    /// Midday (11 AM - 2 PM)
    Midday,
    /// Afternoon (2-6 PM)
    Afternoon,
    /// Evening (6-9 PM)
    Evening,
    /// Night (9 PM - 5 AM)
    Night,
}

/// Weekly training load summary for contextual factors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextualWeeklyLoad {
    /// Total distance covered this week in kilometers
    pub total_distance_km: f64,
    /// Total training duration this week in hours
    pub total_duration_hours: f64,
    /// Number of activities completed this week
    pub activity_count: i32,
    /// Trend in training load compared to previous weeks
    pub load_trend: TrendDirection,
}

impl ActivityIntelligence {
    /// Create a new activity intelligence analysis
    #[must_use]
    pub fn new(
        summary: String,
        insights: Vec<Insight>,
        performance: PerformanceMetrics,
        context: ContextualFactors,
    ) -> Self {
        Self {
            summary,
            key_insights: insights,
            performance_indicators: performance,
            contextual_factors: context,
            generated_at: Utc::now(),
        }
    }

    /// Create an empty `ActivityIntelligence` instance for default initialization
    #[must_use]
    pub fn create_empty() -> Self {
        Self {
            summary: "No analysis available".to_owned(),
            key_insights: vec![],
            performance_indicators: PerformanceMetrics {
                relative_effort: None,
                zone_distribution: None,
                personal_records: vec![],
                efficiency_score: None,
                trend_indicators: TrendIndicators {
                    pace_trend: TrendDirection::Stable,
                    effort_trend: TrendDirection::Stable,
                    distance_trend: TrendDirection::Stable,
                    consistency_score: 0.0,
                },
            },
            contextual_factors: ContextualFactors {
                weather: None,
                location: None,
                time_of_day: TimeOfDay::Morning,
                days_since_last_activity: None,
                weekly_load: None,
            },
            generated_at: Utc::now(),
        }
    }
}

// === ADVANCED ANALYTICS TYPES ===

/// Time frame for analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimeFrame {
    /// Last 7 days
    Week,
    /// Last 30 days
    Month,
    /// Last 90 days
    Quarter,
    /// Last 180 days
    SixMonths,
    /// Last 365 days
    Year,
    /// Custom date range
    Custom {
        /// Start of the time range
        start: DateTime<Utc>,
        /// End of the time range
        end: DateTime<Utc>,
    },
}

impl TimeFrame {
    /// Get the duration in days
    #[must_use]
    pub fn to_days(&self) -> i64 {
        match self {
            Self::Week => 7,
            Self::Month => 30,
            Self::Quarter => 90,
            Self::SixMonths => 180,
            Self::Year => 365,
            Self::Custom { start, end } => (*end - *start).num_days(),
        }
    }

    /// Get start date relative to now
    #[must_use]
    pub fn start_date(&self) -> DateTime<Utc> {
        match self {
            Self::Week => Utc::now() - chrono::Duration::days(7),
            Self::Month => Utc::now() - chrono::Duration::days(30),
            Self::Quarter => Utc::now() - chrono::Duration::days(90),
            Self::SixMonths => Utc::now() - chrono::Duration::days(180),
            Self::Year => Utc::now() - chrono::Duration::days(365),
            Self::Custom { start, .. } => *start,
        }
    }

    /// Get end date
    #[must_use]
    pub fn end_date(&self) -> DateTime<Utc> {
        match self {
            Self::Custom { end, .. } => *end,
            _ => Utc::now(),
        }
    }
}

/// Confidence level for insights and recommendations
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Confidence {
    /// Low confidence (25%)
    Low = 1,
    /// Medium confidence (50%)
    Medium = 2,
    /// High confidence (75%)
    High = 3,
    /// Very high confidence (95%)
    VeryHigh = 4,
}

impl Confidence {
    /// Convert confidence to a 0-1 score
    #[must_use]
    pub const fn as_score(&self) -> f64 {
        match self {
            Self::Low => 0.25,
            Self::Medium => 0.50,
            Self::High => 0.75,
            Self::VeryHigh => 0.95,
        }
    }

    /// Create confidence from a 0-1 score
    #[must_use]
    pub fn from_score(score: f64) -> Self {
        if score >= EXCELLENT_PERFORMANCE_THRESHOLD {
            Self::VeryHigh
        } else if score >= GOOD_PERFORMANCE_THRESHOLD {
            Self::High
        } else if score >= MODERATE_PERFORMANCE_THRESHOLD {
            Self::Medium
        } else {
            Self::Low
        }
    }
}

/// Enhanced activity insights with advanced analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityInsights {
    /// Unique identifier for the activity
    pub activity_id: String,
    /// Overall performance score (0-100)
    pub overall_score: f64,
    /// List of advanced insights discovered
    pub insights: Vec<AdvancedInsight>,
    /// Advanced performance metrics
    pub metrics: AdvancedMetrics,
    /// Actionable recommendations for improvement
    pub recommendations: Vec<String>,
    /// Detected anomalies in the activity data
    pub anomalies: Vec<Anomaly>,
}

/// Advanced insight with confidence and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedInsight {
    /// Type of insight (e.g., `pace_improvement`, `fatigue_warning`)
    pub insight_type: String,
    /// Human-readable insight message
    pub message: String,
    /// Confidence level in this insight
    pub confidence: Confidence,
    /// Severity/importance of the insight
    pub severity: InsightSeverity,
    /// Additional metadata for the insight
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Severity level for insights
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InsightSeverity {
    /// Informational insight
    Info,
    /// Warning that needs attention
    Warning,
    /// Critical issue requiring immediate action
    Critical,
}

/// Detected anomaly in activity data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    /// Type of anomaly detected
    pub anomaly_type: String,
    /// Description of the anomaly
    pub description: String,
    /// Severity of the anomaly
    pub severity: InsightSeverity,
    /// Confidence in the anomaly detection
    pub confidence: Confidence,
    /// Metric that shows the anomaly
    pub affected_metric: String,
    /// Expected value for the metric
    pub expected_value: Option<f64>,
    /// Actual observed value
    pub actual_value: Option<f64>,
}

/// Performance trend analysis results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendAnalysis {
    /// Time period analyzed
    pub timeframe: TimeFrame,
    /// Metric being analyzed
    pub metric: String,
    /// Direction of the trend
    pub trend_direction: TrendDirection,
    /// Strength of the trend (0-1, higher is stronger)
    pub trend_strength: f64,
    /// Statistical significance (p-value)
    pub statistical_significance: f64,
    /// Individual data points in the trend
    pub data_points: Vec<TrendDataPoint>,
    /// Insights derived from the trend
    pub insights: Vec<AdvancedInsight>,
}

/// Data point in a trend analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendDataPoint {
    /// Date of this data point
    pub date: DateTime<Utc>,
    /// Raw value at this point
    pub value: f64,
    /// Smoothed value (moving average) if available
    pub smoothed_value: Option<f64>,
}

/// Fitness goal definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    /// Unique identifier for the goal
    pub id: String,
    /// User who owns this goal
    pub user_id: String,
    /// Goal title
    pub title: String,
    /// Detailed description of the goal
    pub description: String,
    /// Type and specifics of the goal
    pub goal_type: GoalType,
    /// Target value to achieve
    pub target_value: f64,
    /// Target completion date
    pub target_date: DateTime<Utc>,
    /// Current progress value
    pub current_value: f64,
    /// When the goal was created
    pub created_at: DateTime<Utc>,
    /// When the goal was last updated
    pub updated_at: DateTime<Utc>,
    /// Current status of the goal
    pub status: GoalStatus,
}

/// Type of fitness goal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GoalType {
    /// Distance goal (e.g., run 100km this month)
    Distance {
        /// Sport type
        sport: String,
        /// Time period for the goal
        timeframe: TimeFrame,
    },
    /// Time goal (e.g., run 5km in under 20 minutes)
    Time {
        /// Sport type
        sport: String,
        /// Target distance
        distance: f64,
    },
    /// Frequency goal (e.g., run 3 times per week)
    Frequency {
        /// Sport type
        sport: String,
        /// Target sessions per week
        sessions_per_week: i32,
    },
    /// Performance improvement goal
    Performance {
        /// Performance metric to improve
        metric: String,
        /// Target improvement percentage
        improvement_percent: f64,
    },
    /// Custom user-defined goal
    Custom {
        /// Custom metric name
        metric: String,
        /// Unit of measurement
        unit: String,
    },
}

/// Status of a goal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GoalStatus {
    /// Goal is active and in progress
    Active,
    /// Goal has been completed
    Completed,
    /// Goal is temporarily paused
    Paused,
    /// Goal was cancelled
    Cancelled,
}

/// Progress report for a goal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressReport {
    /// ID of the goal being reported on
    pub goal_id: String,
    /// Progress as a percentage (0-100)
    pub progress_percentage: f64,
    /// Estimated completion date based on current progress
    pub completion_date_estimate: Option<DateTime<Utc>>,
    /// Milestones that have been achieved
    pub milestones_achieved: Vec<Milestone>,
    /// Insights about goal progress
    pub insights: Vec<AdvancedInsight>,
    /// Recommendations for achieving the goal
    pub recommendations: Vec<String>,
    /// Whether the goal is on track for completion
    pub on_track: bool,
}

/// Milestone in goal progress
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    /// Name of the milestone
    pub name: String,
    /// Target value for this milestone
    pub target_value: f64,
    /// When the milestone was achieved (if achieved)
    pub achieved_date: Option<DateTime<Utc>>,
    /// Whether this milestone has been achieved
    pub achieved: bool,
}

/// Training recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingRecommendation {
    /// Type of recommendation
    pub recommendation_type: RecommendationType,
    /// Recommendation title
    pub title: String,
    /// Detailed description of the recommendation
    pub description: String,
    /// Priority level for acting on this recommendation
    pub priority: RecommendationPriority,
    /// Confidence in this recommendation
    pub confidence: Confidence,
    /// Explanation of why this recommendation is made
    pub rationale: String,
    /// Specific actionable steps to implement the recommendation
    pub actionable_steps: Vec<String>,
}

/// Type of training recommendation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RecommendationType {
    /// Recommendation about training intensity
    Intensity,
    /// Recommendation about training volume
    Volume,
    /// Recommendation about recovery and rest
    Recovery,
    /// Recommendation about technique and form
    Technique,
    /// Recommendation about nutrition and fueling
    Nutrition,
    /// Recommendation about equipment
    Equipment,
    /// Recommendation about training strategy
    Strategy,
}

/// Priority level for recommendations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecommendationPriority {
    /// Low priority, nice to have
    Low,
    /// Medium priority, should consider
    Medium,
    /// High priority, important to address
    High,
    /// Critical priority, urgent action needed
    Critical,
}
