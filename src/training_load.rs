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

/// CTL at or below this value carries no meaningful fitness base to normalize
/// against; the form bands are then applied to absolute TSB instead of TSB as
/// a percentage of CTL.
const MIN_CTL_FOR_RELATIVE_FORM: f64 = 1.0;

/// Form (TSB as % of CTL) below which the athlete is overreaching.
const OVERREACHING_FORM_PCT: f64 = -30.0;

/// Form (TSB as % of CTL) at which the freshness band starts.
const FRESH_FORM_PCT: f64 = 5.0;

/// Form (TSB as % of CTL) above which detraining risk begins.
const DETRAINING_FORM_PCT: f64 = 20.0;

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
        // Fail loud (Issue #1): the algorithm rejects unsorted input — in
        // practice reverse-chronological (newest-first) data, which Strava
        // returns. Propagate that error rather than silently zeroing, so a
        // caller that forgot to sort oldest-first finds out instead of getting
        // a misleading zero load. Production callers sort before calling.
        let algorithm = self.algorithm_config.training_load_algorithm();
        let ctl = algorithm.calculate_ctl(&tss_data)?;
        let atl = algorithm.calculate_atl(&tss_data)?;
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
    /// Interpretation is CTL-relative (form as a percentage of fitness) —
    /// see [`Self::interpret_tsb`] for the band definitions.
    #[must_use]
    pub const fn calculate_tsb(ctl: f64, atl: f64) -> f64 {
        ctl - atl
    }

    /// Express TSB as form: a percentage of CTL (`tsb / ctl * 100`).
    ///
    /// When CTL is at or below [`MIN_CTL_FOR_RELATIVE_FORM`], the raw TSB is
    /// returned so the form bands apply to absolute TSB instead.
    fn form_percentage(tsb: f64, ctl: f64) -> f64 {
        if ctl > MIN_CTL_FOR_RELATIVE_FORM {
            tsb / ctl * 100.0
        } else {
            tsb
        }
    }

    /// Interpret TSB relative to CTL and provide a training status.
    ///
    /// Form is expressed as a percentage of fitness (`form_pct = tsb / ctl *
    /// 100`), following the TrainingPeaks/Friel and intervals.icu convention,
    /// so the same TSB reads differently for a 40-CTL and a 100-CTL athlete:
    /// - form below -30% of CTL: overreaching (high fatigue, recovery needed)
    /// - form -30% to +5% of CTL: productive training zone
    /// - form +5% to +20% of CTL: fresh, ready to perform
    /// - form above +20% of CTL: risk of detraining
    ///
    /// When CTL is at or below 1.0, there is no meaningful fitness base to
    /// normalize against and the same band edges apply to absolute TSB.
    #[must_use]
    pub fn interpret_tsb(tsb: f64, ctl: f64) -> TrainingStatus {
        let form_pct = Self::form_percentage(tsb, ctl);
        if form_pct < OVERREACHING_FORM_PCT {
            TrainingStatus::Overreaching
        } else if form_pct < FRESH_FORM_PCT {
            TrainingStatus::Productive
        } else if form_pct <= DETRAINING_FORM_PCT {
            TrainingStatus::Fresh
        } else {
            TrainingStatus::Detraining
        }
    }

    /// Check whether the athlete's load pattern warrants caution
    ///
    /// Flagged factors are descriptive magnitude statements about the load
    /// pattern, never injury predictions (fixed-threshold injury prediction
    /// is not supported by the literature — Impellizzeri et al., 2020):
    /// - ATL more than 30% above CTL: rapid ramp in acute load
    /// - ATL more than 50% above CTL: acute load far above the chronic base
    /// - Form below -30% of CTL: deep negative form
    #[must_use]
    pub fn check_overtraining_risk(training_load: &TrainingLoad) -> OvertrainingRisk {
        /// Acute-to-chronic ratio above which the ramp in load is flagged.
        const ACUTE_RAMP_RATIO: f64 = 1.3;
        /// Acute-to-chronic ratio above which acute load is far beyond the base.
        const ACUTE_SPIKE_RATIO: f64 = 1.5;

        let mut risk_factors = Vec::new();

        if training_load.ctl > 0.0 && training_load.atl > training_load.ctl * ACUTE_RAMP_RATIO {
            risk_factors
                .push("Acute load more than 30% above chronic load (rapid ramp)".to_owned());
        }

        if training_load.ctl > 0.0 && training_load.atl > training_load.ctl * ACUTE_SPIKE_RATIO {
            risk_factors
                .push("Acute load more than 50% above chronic load (very rapid ramp)".to_owned());
        }

        if Self::form_percentage(training_load.tsb, training_load.ctl) < OVERREACHING_FORM_PCT {
            risk_factors.push("Form deeper than -30% of fitness".to_owned());
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

    /// Calculate recommended recovery days from form (TSB as % of CTL)
    ///
    /// Form between -30% and +5% of CTL is normal productive training and gets
    /// no recovery prescription; days are only recommended once form drops
    /// below the overreaching edge (-30%). When CTL is at or below 1.0, the
    /// same band edges apply to absolute TSB.
    #[must_use]
    pub fn recommend_recovery_days(tsb: f64, ctl: f64) -> u32 {
        /// Form (% of CTL) below which several recovery days are warranted.
        const SEVERE_FATIGUE_FORM_PCT: f64 = -50.0;
        /// Form (% of CTL) below which a couple of recovery days are warranted.
        const DEEP_FATIGUE_FORM_PCT: f64 = -40.0;

        let form_pct = Self::form_percentage(tsb, ctl);
        if form_pct < SEVERE_FATIGUE_FORM_PCT {
            return 3;
        }
        if form_pct < DEEP_FATIGUE_FORM_PCT {
            return 2;
        }
        if form_pct < OVERREACHING_FORM_PCT {
            return 1;
        }
        0
    }
}

/// Training status based on TSB
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrainingStatus {
    /// Form below -30% of CTL: overreaching, high fatigue
    Overreaching,
    /// Form -30% to +5% of CTL: productive training zone
    Productive,
    /// Form +5% to +20% of CTL: fresh, ready to perform
    Fresh,
    /// Form above +20% of CTL: risk of detraining
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
