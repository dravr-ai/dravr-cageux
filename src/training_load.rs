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
/// against; form is then [`FormBand::InsufficientHistory`] rather than banded
/// on absolute TSB, which means opposite things at CTL 40 and CTL 120.
///
/// This is a chronic-base guard, not a divide-by-zero guard. Because
/// `tsb == ctl - atl`, [`FormBand::DeepFatigue`] is exactly `atl > 1.3 * ctl` —
/// a ratio a beginner clears with one ordinary hard week. At 1.0 the guard let a
/// CTL-10 athlete at ATL 14 band as deepest fatigue and collect an overtraining
/// warning, which inverted the goal of banding on form at all. Below roughly 20
/// the ratio is dominated by single sessions and says nothing about form.
const MIN_CTL_FOR_RELATIVE_FORM: f64 = 20.0;

/// Form (TSB as % of CTL) below which the athlete is in the deepest fatigue band.
pub(crate) const DEEP_FATIGUE_FORM_PCT: f64 = -30.0;

/// Form (TSB as % of CTL) below which several recovery days are warranted — the
/// deep end of the deepest band. Shared with the recovery calculator so the rest
/// prescription and its stated reason cannot disagree about how deep is deep.
pub(crate) const SEVERE_FATIGUE_FORM_PCT: f64 = -50.0;

/// Form (TSB as % of CTL) below which the athlete is at the deep end of the
/// productive zone — a heavy block, not an emergency.
const HEAVY_BLOCK_FORM_PCT: f64 = -20.0;

/// Form (TSB as % of CTL) below which the athlete carries the normal fatigue
/// of a productive training block.
const PRODUCTIVE_FORM_PCT: f64 = -10.0;

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
    /// see [`FormBand`] for the band definitions.
    #[must_use]
    pub const fn calculate_tsb(ctl: f64, atl: f64) -> f64 {
        ctl - atl
    }

    /// Check whether the athlete's load pattern warrants caution
    ///
    /// Flagged factors are descriptive magnitude statements about the load
    /// pattern, never injury predictions (fixed-threshold injury prediction
    /// is not supported by the literature — Impellizzeri et al., 2020).
    ///
    /// Severity grades the *depth* of one axis rather than counting restatements
    /// of it. Acute-vs-chronic load and form are not independent observations:
    /// because `tsb == ctl - atl`, "ATL more than 30% above CTL" and
    /// [`FormBand::DeepFatigue`] are the same inequality. Listing both made every
    /// athlete past 1.3 collect two "corroborating" factors, forced
    /// [`RiskLevel::High`], and left [`RiskLevel::Moderate`] unreachable for
    /// anyone with a chronic base. One observation, stated once, graded by band.
    #[must_use]
    pub fn check_overtraining_risk(training_load: &TrainingLoad) -> OvertrainingRisk {
        let form_pct = FormBand::form_pct(training_load.tsb, training_load.ctl);

        let (risk_level, risk_factors) = match FormBand::from_form_pct(form_pct) {
            FormBand::DeepFatigue => (
                RiskLevel::High,
                vec![format!(
                    "Acute load is carrying form to {:.0}% of chronic fitness, past the -30% band",
                    form_pct.unwrap_or_default()
                )],
            ),
            FormBand::HeavyBlock => (
                RiskLevel::Moderate,
                vec![format!(
                    "Form at {:.0}% of chronic fitness - the deep end of a productive block",
                    form_pct.unwrap_or_default()
                )],
            ),
            // No chronic base to judge against makes no claim: at a near-zero CTL
            // a single session swings the ratio wildly, and inventing a risk level
            // from it is how beginners collected warnings for an ordinary week.
            _ => (RiskLevel::Low, Vec::new()),
        };

        OvertrainingRisk {
            risk_level,
            risk_factors,
        }
    }

    /// Calculate recommended recovery days from form (TSB as % of CTL)
    ///
    /// Everything at or above the deep-fatigue edge (-30% of CTL) is normal
    /// training and gets no recovery prescription; days are only recommended
    /// once form drops past it. Returns 0 when there is no chronic base to
    /// normalize against — an athlete whose form cannot be judged is not
    /// handed a rest prescription derived from a number that means nothing.
    #[must_use]
    pub fn recommend_recovery_days(tsb: f64, ctl: f64) -> u32 {
        /// Form (% of CTL) below which a couple of recovery days are warranted.
        const TWO_DAY_FATIGUE_FORM_PCT: f64 = -40.0;

        let Some(form_pct) = FormBand::form_pct(tsb, ctl) else {
            return 0;
        };
        if form_pct < SEVERE_FATIGUE_FORM_PCT {
            return 3;
        }
        if form_pct < TWO_DAY_FATIGUE_FORM_PCT {
            return 2;
        }
        if form_pct < DEEP_FATIGUE_FORM_PCT {
            return 1;
        }
        0
    }
}

/// Descriptive band for an athlete's form, expressed relative to their own
/// chronic load (`form_pct = tsb / ctl * 100`) per the TrainingPeaks/Friel and
/// intervals.icu convention.
///
/// The bands state the magnitude of fatigue relative to fitness. They are not
/// injury predictions: fixed-threshold injury prediction from load ratios is
/// not supported by the literature (Impellizzeri et al., 2020). This is the
/// single form vocabulary — every surface that bands form derives it from
/// here rather than comparing raw TSB to a constant, because the same TSB is
/// a normal build week at CTL 120 and the deepest fatigue at CTL 40.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormBand {
    /// No chronic base to normalize against — form is not interpretable.
    InsufficientHistory,
    /// Form below -30% of CTL: the deepest fatigue band.
    DeepFatigue,
    /// Form -30% to -20% of CTL: the deep end of the productive zone.
    HeavyBlock,
    /// Form -20% to -10% of CTL: the normal fatigue of a productive block.
    Productive,
    /// Form -10% to +5% of CTL: neither fatigued nor peaked.
    Balanced,
    /// Form +5% to +20% of CTL: fresh, ready for quality work or racing.
    Fresh,
    /// Form above +20% of CTL: risk of detraining.
    Detraining,
}

impl FormBand {
    /// Express TSB as form: a percentage of CTL (`tsb / ctl * 100`).
    ///
    /// `None` when CTL is at or below [`MIN_CTL_FOR_RELATIVE_FORM`]. A raw TSB
    /// with no chronic base to scale it is not interpretable as form, and
    /// banding it on the absolute number would read a beginner's first hard
    /// week as an elite's deepest fatigue. Callers report the absence as
    /// insufficient history; they never substitute absolute-TSB thresholds.
    #[must_use]
    pub fn form_pct(tsb: f64, ctl: f64) -> Option<f64> {
        if ctl > MIN_CTL_FOR_RELATIVE_FORM {
            Some(tsb / ctl * 100.0)
        } else {
            None
        }
    }

    /// Band an athlete's form from their TSB and CTL.
    #[must_use]
    pub fn from_tsb(tsb: f64, ctl: f64) -> Self {
        Self::from_form_pct(Self::form_pct(tsb, ctl))
    }

    /// Band an already-computed form percentage.
    #[must_use]
    pub fn from_form_pct(form_pct: Option<f64>) -> Self {
        let Some(pct) = form_pct else {
            return Self::InsufficientHistory;
        };
        if pct < DEEP_FATIGUE_FORM_PCT {
            Self::DeepFatigue
        } else if pct < HEAVY_BLOCK_FORM_PCT {
            Self::HeavyBlock
        } else if pct < PRODUCTIVE_FORM_PCT {
            Self::Productive
        } else if pct < FRESH_FORM_PCT {
            Self::Balanced
        } else if pct <= DETRAINING_FORM_PCT {
            Self::Fresh
        } else {
            Self::Detraining
        }
    }

    /// Descriptive one-line reading of the band, for surfaces that hand the
    /// athlete's state to a coach or a language model. Never risk or injury
    /// language — the band describes fatigue relative to fitness.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::InsufficientHistory => "insufficient chronic history to judge form",
            Self::DeepFatigue => "deep fatigue - form far below this athlete's own fitness",
            Self::HeavyBlock => "heavy block - the deep end of the productive zone",
            Self::Productive => "productive - building fitness under normal training fatigue",
            Self::Balanced => "balanced - neither fatigued nor peaked",
            Self::Fresh => "fresh - ready for quality work or racing",
            Self::Detraining => "very fresh - possibly detraining",
        }
    }
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
