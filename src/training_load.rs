// ABOUTME: Training load calculations including TSS, CTL, ATL, and TSB for fitness tracking
// ABOUTME: Implements exponential moving averages to track chronic and acute training loads
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

use crate::config::intelligence::AlgorithmConfig;
use crate::error::IntelligenceError;
use crate::metrics::MetricsCalculator;
use crate::models::Activity;
use serde::{Deserialize, Serialize};
use tracing::instrument;

/// TSS data point with timestamp — re-exported canonical type from the
/// algorithm layer so the training-load calculator and the
/// [`TrainingLoadAlgorithm`](crate::algorithms::TrainingLoadAlgorithm) enum
/// share a single definition.
pub use crate::algorithms::training_load::TssDataPoint;

/// Training load metrics for an athlete
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingLoad {
    /// Chronic Training Load (long-term smoothed TSS) - represents fitness.
    /// The smoothing method and window come from the configured training-load algorithm.
    pub ctl: f64,
    /// Acute Training Load (short-term smoothed TSS) - represents fatigue.
    /// The smoothing method and window come from the configured training-load algorithm.
    pub atl: f64,
    /// Training Stress Balance (CTL - ATL) - represents form/freshness
    pub tsb: f64,
    /// Individual TSS values with dates for visualization
    pub tss_history: Vec<TssDataPoint>,
}

/// Calculator for training load metrics.
///
/// Holds an [`AlgorithmConfig`] so it can resolve the configured training-load
/// smoothing algorithm (EMA/SMA/WMA/Kalman + CTL/ATL windows) via
/// [`AlgorithmConfig::training_load_algorithm`], and so the per-activity TSS it
/// computes honors the configured TSS algorithm too.
pub struct TrainingLoadCalculator {
    algorithm_config: AlgorithmConfig,
}

impl Default for TrainingLoadCalculator {
    fn default() -> Self {
        Self::new()
    }
}

impl TrainingLoadCalculator {
    /// Create a new training load calculator using default algorithm configuration
    /// (EMA with 42-day CTL / 7-day ATL windows).
    #[must_use]
    pub fn new() -> Self {
        Self {
            algorithm_config: AlgorithmConfig::default(),
        }
    }

    /// Create a training load calculator from an explicit algorithm configuration.
    ///
    /// The configured training-load variant and its CTL/ATL/Kalman parameters,
    /// as well as the TSS algorithm used for per-activity scoring, are sourced
    /// from `algorithm_config`.
    #[must_use]
    pub fn from_config(algorithm_config: AlgorithmConfig) -> Self {
        Self { algorithm_config }
    }

    /// Calculate TSS for a single activity using existing `MetricsCalculator`
    ///
    /// Returns the TSS value or an error if calculation fails
    ///
    /// # Errors
    /// Returns `IntelligenceError` if metrics calculation fails or TSS cannot be determined
    pub fn calculate_tss(
        &self,
        activity: &Activity,
        ftp: Option<f64>,
        lthr: Option<f64>,
        max_hr: Option<f64>,
        resting_hr: Option<f64>,
        weight_kg: Option<f64>,
    ) -> Result<f64, IntelligenceError> {
        let calculator = MetricsCalculator::new()
            .with_user_data(ftp, lthr, max_hr, resting_hr, weight_kg)
            .with_algorithm_config(self.algorithm_config.clone());

        let metrics = calculator.calculate_metrics(activity).map_err(|e| {
            IntelligenceError::internal(format!("Failed to calculate metrics: {e}"))
        })?;

        metrics.training_stress_score.ok_or_else(|| {
            IntelligenceError::internal("Unable to calculate TSS for activity".to_owned())
        })
    }

    /// Collect TSS data points from activities, logging any that are skipped
    fn collect_tss_data(
        &self,
        activities: &[Activity],
        ftp: Option<f64>,
        lthr: Option<f64>,
        max_hr: Option<f64>,
        resting_hr: Option<f64>,
        weight_kg: Option<f64>,
    ) -> Vec<TssDataPoint> {
        let mut tss_data: Vec<TssDataPoint> = Vec::with_capacity(activities.len());
        let mut skipped_count: usize = 0;
        for activity in activities {
            match self.calculate_tss(activity, ftp, lthr, max_hr, resting_hr, weight_kg) {
                Ok(tss) => {
                    tss_data.push(TssDataPoint {
                        date: activity.start_date(),
                        tss,
                    });
                }
                Err(e) => {
                    tracing::debug!(
                        activity_id = %activity.id(),
                        error = %e,
                        "Skipping activity in training load calculation — TSS unavailable"
                    );
                    skipped_count += 1;
                }
            }
        }

        if skipped_count > 0 {
            tracing::info!(
                included = tss_data.len(),
                skipped = skipped_count,
                "Training load calculated: {} activities included, {} skipped (no TSS data)",
                tss_data.len(),
                skipped_count
            );
        }

        tss_data
    }

    /// Calculate complete training load metrics (CTL, ATL, TSB) from activities
    ///
    /// Activities should be sorted by date (oldest first) for accurate EMA calculation.
    /// Activities without sufficient data for TSS estimation are excluded from the
    /// calculation and logged at debug level. An info-level summary is emitted when
    /// any activities are skipped.
    ///
    /// # Errors
    /// Returns `IntelligenceError` if no activities can be processed (empty input)
    #[instrument(
        skip(self, activities),
        fields(
            service = "training_load",
            operation = "calculate",
            activity_count = activities.len(),
        )
    )]
    pub fn calculate_training_load(
        &self,
        activities: &[Activity],
        ftp: Option<f64>,
        lthr: Option<f64>,
        max_hr: Option<f64>,
        resting_hr: Option<f64>,
        weight_kg: Option<f64>,
    ) -> Result<TrainingLoad, IntelligenceError> {
        if activities.is_empty() {
            return Ok(TrainingLoad {
                ctl: 0.0,
                atl: 0.0,
                tsb: 0.0,
                tss_history: Vec::new(),
            });
        }

        let tss_data = self.collect_tss_data(activities, ftp, lthr, max_hr, resting_hr, weight_kg);

        if tss_data.is_empty() {
            return Ok(TrainingLoad {
                ctl: 0.0,
                atl: 0.0,
                tsb: 0.0,
                tss_history: Vec::new(),
            });
        }

        // Resolve the configured training-load algorithm (default EMA 42/7) and
        // dispatch CTL/ATL through the single canonical implementation.
        //
        // A calculation error here means the input is unusable for smoothing —
        // in practice reverse-chronological (newest-first) data, which Strava
        // returns. Preserve the long-standing graceful contract: yield 0 rather
        // than erroring, so callers that forget to sort degrade to a zero load
        // instead of failing. Callers that need accuracy sort oldest-first.
        let algorithm = self.algorithm_config.training_load_algorithm();
        let ctl = algorithm.calculate_ctl(&tss_data).unwrap_or(0.0);
        let atl = algorithm.calculate_atl(&tss_data).unwrap_or(0.0);
        let tsb = Self::calculate_tsb(ctl, atl);

        Ok(TrainingLoad {
            ctl,
            atl,
            tsb,
            tss_history: tss_data,
        })
    }

    /// Calculate CTL (Chronic Training Load) - 42-day exponential moving average
    ///
    /// # Errors
    /// Returns `IntelligenceError` if training load calculation fails
    pub fn calculate_ctl(
        &self,
        activities: &[Activity],
        ftp: Option<f64>,
        lthr: Option<f64>,
        max_hr: Option<f64>,
        resting_hr: Option<f64>,
        weight_kg: Option<f64>,
    ) -> Result<f64, IntelligenceError> {
        let training_load =
            self.calculate_training_load(activities, ftp, lthr, max_hr, resting_hr, weight_kg)?;
        Ok(training_load.ctl)
    }

    /// Calculate ATL (Acute Training Load) - 7-day exponential moving average
    ///
    /// # Errors
    /// Returns `IntelligenceError` if training load calculation fails
    pub fn calculate_atl(
        &self,
        activities: &[Activity],
        ftp: Option<f64>,
        lthr: Option<f64>,
        max_hr: Option<f64>,
        resting_hr: Option<f64>,
        weight_kg: Option<f64>,
    ) -> Result<f64, IntelligenceError> {
        let training_load =
            self.calculate_training_load(activities, ftp, lthr, max_hr, resting_hr, weight_kg)?;
        Ok(training_load.atl)
    }

    /// Calculate TSB (Training Stress Balance) = CTL - ATL
    ///
    /// Interpretation:
    /// - TSB < -10: Overreaching (high fatigue, need recovery)
    /// - TSB -10 to 0: Productive training zone
    /// - TSB 0 to +10: Fresh, ready to perform
    /// - TSB > +10: Risk of detraining
    #[must_use]
    pub const fn calculate_tsb(ctl: f64, atl: f64) -> f64 {
        ctl - atl
    }

    /// Interpret TSB value and provide status
    #[must_use]
    pub fn interpret_tsb(tsb: f64) -> TrainingStatus {
        if tsb < -10.0 {
            TrainingStatus::Overreaching
        } else if tsb < 0.0 {
            TrainingStatus::Productive
        } else if tsb <= 10.0 {
            TrainingStatus::Fresh
        } else {
            TrainingStatus::Detraining
        }
    }

    /// Check if athlete is at risk of overtraining
    ///
    /// Warning conditions:
    /// - ATL > CTL x 1.3: Acute load spike
    /// - ATL > 150: Very high acute load
    /// - TSB < -10: Deep fatigue
    #[must_use]
    pub fn check_overtraining_risk(training_load: &TrainingLoad) -> OvertrainingRisk {
        let mut risk_factors = Vec::new();

        // Check for acute load spike
        if training_load.ctl > 0.0 && training_load.atl > training_load.ctl * 1.3 {
            risk_factors
                .push("Acute training load spike detected (>30% above chronic load)".to_owned());
        }

        // Check for very high acute load
        if training_load.atl > 150.0 {
            risk_factors.push("Very high acute training load (>150 TSS/day)".to_owned());
        }

        // Check for deep fatigue
        if training_load.tsb < -10.0 {
            risk_factors.push("Deep fatigue detected (TSB < -10) - recovery needed".to_owned());
        }

        let risk_level = if risk_factors.len() >= 2 {
            RiskLevel::High
        } else if risk_factors.len() == 1 {
            RiskLevel::Moderate
        } else {
            RiskLevel::Low
        };

        OvertrainingRisk {
            risk_level,
            risk_factors,
        }
    }

    /// Calculate recommended recovery days based on TSB
    #[must_use]
    pub fn recommend_recovery_days(tsb: f64) -> u32 {
        // Multi-level threshold function for recovery recommendations
        const VERY_DEEP_FATIGUE: f64 = -20.0;
        const DEEP_FATIGUE: f64 = -15.0;
        const MODERATE_FATIGUE: f64 = -10.0;
        const LIGHT_FATIGUE: f64 = 0.0;

        if tsb < VERY_DEEP_FATIGUE {
            return 5;
        }
        if tsb < DEEP_FATIGUE {
            return 3;
        }
        if tsb < MODERATE_FATIGUE {
            return 2;
        }
        if tsb < LIGHT_FATIGUE {
            return 1;
        }
        0
    }
}

/// Training status based on TSB
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrainingStatus {
    /// TSB < -10: Overreaching, high fatigue
    Overreaching,
    /// TSB -10 to 0: Productive training zone
    Productive,
    /// TSB 0 to +10: Fresh, ready to perform
    Fresh,
    /// TSB > +10: Risk of detraining
    Detraining,
}

/// Risk level for overtraining
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    /// Low risk of overtraining
    Low,
    /// Moderate risk - monitor closely
    Moderate,
    /// High risk - rest recommended
    High,
}

/// Overtraining risk assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OvertrainingRisk {
    /// Overall risk level
    pub risk_level: RiskLevel,
    /// Specific risk factors identified
    pub risk_factors: Vec<String>,
}
