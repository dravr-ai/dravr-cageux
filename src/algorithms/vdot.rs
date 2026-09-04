// ABOUTME: VDOT (VO2max running) calculation algorithms with Daniels, Riegel, and hybrid methods
// ABOUTME: Implements Jack Daniels' VDOT methodology and Riegel's power-law race prediction formula
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

use crate::error::{IntelligenceError, IntelligenceResult};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// VDOT calculation algorithm selection
///
/// Different algorithms for calculating running performance metrics:
///
/// - `Daniels`: Jack Daniels' VDOT formula (VO2 = -4.60 + 0.182258xv + 0.000104xv²)
/// - `Riegel`: Power-law model (T2 = T1 x (D2/D1)^1.06)
/// - `Hybrid`: Auto-select based on race distance and conditions
///
/// # Scientific References
///
/// - Daniels, J. (2013). "Daniels' Running Formula" (3rd ed.). Human Kinetics.
/// - Riegel, P.S. (1981). "Athletic records and human endurance." *American Scientist*, 69(3), 285-290.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum VdotAlgorithm {
    /// Jack Daniels' VDOT formula
    ///
    /// Formula: VO2 = -4.60 + 0.182258 x velocity + 0.000104 x velocity²
    ///
    /// Where velocity is in meters per minute
    ///
    /// Pros: Physiologically accurate, accounts for running economy
    /// Cons: Requires velocity calculation, best for 5K-Marathon distances
    #[default]
    Daniels,

    /// Riegel power-law formula
    ///
    /// Formula: T2 = T1 x (D2/D1)^1.06
    ///
    /// Predicts time for distance D2 based on time T1 for distance D1
    ///
    /// Pros: Simple, works across all distances
    /// Cons: Less accurate for very short (<1 mile) or ultra distances
    Riegel {
        /// Exponent for power-law (default 1.06, can vary by athlete: 1.03-1.08)
        exponent: f64,
    },

    /// Hybrid: Auto-select best method based on distance and data
    ///
    /// Priority:
    /// 1. Daniels for 5K-Marathon range (optimal accuracy)
    /// 2. Riegel for ultra distances or when multiple race times available
    Hybrid,
}

/// Minimum velocity for VDOT calculation (m/min)
const MIN_VELOCITY: f64 = 100.0;

/// Maximum velocity for VDOT calculation (m/min)
const MAX_VELOCITY: f64 = 500.0;

/// Jack Daniels' VO2 formula coefficient for velocity squared term
const DANIELS_A: f64 = 0.000_104;

/// Jack Daniels' VO2 formula coefficient for velocity term
const DANIELS_B: f64 = 0.182_258;

/// Jack Daniels' VO2 formula constant term
const DANIELS_C: f64 = -4.60;

/// Asymptotic fraction of `VO2max` sustained over an arbitrarily long race
const PERCENT_MAX_ASYMPTOTE: f64 = 0.8;

/// Amplitude of the slow (endurance-fatigue) term of the %`VO2max` relation
const PERCENT_MAX_SLOW_AMPLITUDE: f64 = 0.189_439_3;

/// Decay rate per minute of the slow term of the %`VO2max` relation
const PERCENT_MAX_SLOW_RATE: f64 = 0.012_778;

/// Amplitude of the fast (anaerobic-contribution) term of the %`VO2max` relation
const PERCENT_MAX_FAST_AMPLITUDE: f64 = 0.298_955_8;

/// Decay rate per minute of the fast term of the %`VO2max` relation
const PERCENT_MAX_FAST_RATE: f64 = 0.193_260_5;

/// Bisection steps used to invert the VDOT relation into a race time.
///
/// Sixty halvings exhaust the mantissa of an `f64` bracket, so the result is
/// the closest representable time rather than a tolerance-limited estimate.
const PREDICTION_BISECTION_STEPS: u32 = 60;

impl VdotAlgorithm {
    /// Calculate VDOT from race performance
    ///
    /// # Arguments
    ///
    /// * `distance_meters` - Race distance in meters
    /// * `time_seconds` - Race time in seconds
    ///
    /// # Returns
    ///
    /// VDOT value (typically 30-85 for recreational to elite runners)
    ///
    /// # Errors
    ///
    /// Returns `IntelligenceError::InvalidInput` if:
    /// - Time or distance is non-positive
    /// - Velocity is outside valid range (100-500 m/min)
    /// - VDOT is outside typical range (30-85)
    ///
    /// # Example
    ///
    /// ```rust
    /// use dravr_cageux::algorithms::VdotAlgorithm;
    /// # use dravr_cageux::error::{IntelligenceError, IntelligenceResult};
    /// # fn example() -> IntelligenceResult<()> {
    /// let algorithm = VdotAlgorithm::Daniels;
    /// let vdot = algorithm.calculate_vdot(5000.0, 1200.0)?; // 5K in 20:00
    /// # Ok(())
    /// # }
    /// ```
    pub fn calculate_vdot(
        &self,
        distance_meters: f64,
        time_seconds: f64,
    ) -> IntelligenceResult<f64> {
        if time_seconds <= 0.0 {
            return Err(IntelligenceError::invalid_input(
                "Time must be positive".to_owned(),
            ));
        }

        if distance_meters <= 0.0 {
            return Err(IntelligenceError::invalid_input(
                "Distance must be positive".to_owned(),
            ));
        }

        match self {
            Self::Daniels => Self::calculate_daniels(distance_meters, time_seconds),
            Self::Riegel { exponent } => {
                Self::calculate_riegel_vdot(distance_meters, time_seconds, *exponent)
            }
            Self::Hybrid => Self::calculate_hybrid(distance_meters, time_seconds),
        }
    }

    /// Predict race time for target distance given VDOT
    ///
    /// # Arguments
    ///
    /// * `vdot` - VDOT value
    /// * `target_distance_meters` - Target race distance
    ///
    /// # Returns
    ///
    /// Predicted race time in seconds
    ///
    /// # Errors
    ///
    /// Returns `IntelligenceError::InvalidInput` if VDOT is outside typical range (30-85)
    pub fn predict_time(&self, vdot: f64, target_distance_meters: f64) -> IntelligenceResult<f64> {
        if !(30.0..=85.0).contains(&vdot) {
            return Err(IntelligenceError::invalid_input(format!(
                "VDOT {vdot:.1} is outside typical range (30-85)"
            )));
        }

        match self {
            Self::Daniels | Self::Hybrid => {
                Self::predict_time_daniels(vdot, target_distance_meters)
            }
            Self::Riegel { exponent } => {
                Self::predict_time_riegel(vdot, target_distance_meters, *exponent)
            }
        }
    }

    /// Calculate VDOT using Daniels formula
    fn calculate_daniels(distance_meters: f64, time_seconds: f64) -> IntelligenceResult<f64> {
        // Convert to velocity in meters per minute
        let velocity = (distance_meters / time_seconds) * 60.0;

        if !(MIN_VELOCITY..=MAX_VELOCITY).contains(&velocity) {
            return Err(IntelligenceError::invalid_input(format!(
                "Velocity {velocity:.1} m/min is outside valid range ({MIN_VELOCITY}-{MAX_VELOCITY})"
            )));
        }

        // VO2 = -4.60 + 0.182258xv + 0.000104xv²
        let vo2 = (DANIELS_A * velocity).mul_add(velocity, DANIELS_B.mul_add(velocity, DANIELS_C));

        // Calculate percent-max adjustment based on race duration
        let percent_used = Self::calculate_percent_max_adjustment(time_seconds);
        let vdot = vo2 / percent_used;

        Ok(vdot)
    }

    /// Fraction of `VO2max` sustained over a race of the given duration
    ///
    /// Daniels & Gilbert's continuous relation, with `t` in minutes:
    ///
    /// `%max = 0.8 + 0.1894393 x e^(-0.012778 t) + 0.2989558 x e^(-0.1932605 t)`
    ///
    /// A short race runs above `VO2max` — the anaerobic contribution carries the
    /// fraction past 1.0 — while a long one settles toward the 0.8 asymptote as
    /// fatigue accumulates. Dividing the race `VO2` by this fraction is what
    /// turns a single performance into a VDOT.
    ///
    /// Reference: Daniels, J. (2013). "Daniels' Running Formula" (3rd ed.).
    fn calculate_percent_max_adjustment(time_seconds: f64) -> f64 {
        let time_minutes = time_seconds / 60.0;

        PERCENT_MAX_SLOW_AMPLITUDE.mul_add(
            (-PERCENT_MAX_SLOW_RATE * time_minutes).exp(),
            PERCENT_MAX_FAST_AMPLITUDE.mul_add(
                (-PERCENT_MAX_FAST_RATE * time_minutes).exp(),
                PERCENT_MAX_ASYMPTOTE,
            ),
        )
    }

    /// Calculate VDOT using Riegel power-law formula
    ///
    /// Uses reference distance (10K) to compute equivalent `VO2max`
    fn calculate_riegel_vdot(
        distance_meters: f64,
        time_seconds: f64,
        exponent: f64,
    ) -> IntelligenceResult<f64> {
        // Convert to 10K equivalent time
        const REFERENCE_DISTANCE: f64 = 10_000.0;
        let time_10k_equivalent =
            time_seconds * (REFERENCE_DISTANCE / distance_meters).powf(exponent);

        // Use Daniels formula for 10K to get VDOT
        Self::calculate_daniels(REFERENCE_DISTANCE, time_10k_equivalent)
    }

    /// Predict race time by inverting the Daniels VDOT relation
    ///
    /// [`Self::calculate_daniels`] maps a race to
    /// `vo2(velocity) / percent_max(time)`. Holding the distance fixed, that
    /// expression falls monotonically as the time rises, so the prediction is
    /// the time at which it equals the supplied VDOT. Bisecting over the
    /// velocity domain the VO2 polynomial is defined on
    /// (`MIN_VELOCITY`-`MAX_VELOCITY`) makes the prediction the exact inverse
    /// of the calculation, rather than a second model that could disagree
    /// with it.
    fn predict_time_daniels(vdot: f64, target_distance_meters: f64) -> IntelligenceResult<f64> {
        // Fastest and slowest race times the velocity domain admits.
        let mut fastest = target_distance_meters * 60.0 / MAX_VELOCITY;
        let mut slowest = target_distance_meters * 60.0 / MIN_VELOCITY;

        let vdot_at = |time_seconds: f64| -> f64 {
            let velocity = (target_distance_meters / time_seconds) * 60.0;
            let vo2 =
                (DANIELS_A * velocity).mul_add(velocity, DANIELS_B.mul_add(velocity, DANIELS_C));
            vo2 / Self::calculate_percent_max_adjustment(time_seconds)
        };

        if vdot_at(fastest) < vdot || vdot_at(slowest) > vdot {
            return Err(IntelligenceError::invalid_input(format!(
                "VDOT {vdot:.1} has no {target_distance_meters:.0} m race time within the model's velocity range ({MIN_VELOCITY}-{MAX_VELOCITY} m/min)"
            )));
        }

        for _ in 0..PREDICTION_BISECTION_STEPS {
            let midpoint = f64::midpoint(fastest, slowest);
            if vdot_at(midpoint) > vdot {
                fastest = midpoint;
            } else {
                slowest = midpoint;
            }
        }

        Ok(f64::midpoint(fastest, slowest))
    }

    /// Predict time using Riegel power-law formula
    fn predict_time_riegel(
        vdot: f64,
        target_distance_meters: f64,
        exponent: f64,
    ) -> IntelligenceResult<f64> {
        // Use 10K as reference
        const REFERENCE_DISTANCE: f64 = 10_000.0;

        // Get 10K time from VDOT
        let time_10k = Self::predict_time_daniels(vdot, REFERENCE_DISTANCE)?;

        // Apply Riegel formula: T2 = T1 x (D2/D1)^exponent
        let predicted_time =
            time_10k * (target_distance_meters / REFERENCE_DISTANCE).powf(exponent);

        Ok(predicted_time)
    }

    /// Hybrid: Auto-select best method
    fn calculate_hybrid(distance_meters: f64, time_seconds: f64) -> IntelligenceResult<f64> {
        // Use Daniels for typical race distances (5K-Marathon)
        if (5_000.0..=42_195.0).contains(&distance_meters) {
            Self::calculate_daniels(distance_meters, time_seconds)
        } else {
            // Use Riegel for ultra distances
            Self::calculate_riegel_vdot(distance_meters, time_seconds, 1.06)
        }
    }

    /// Get algorithm name
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Daniels => "daniels",
            Self::Riegel { .. } => "riegel",
            Self::Hybrid => "hybrid",
        }
    }

    /// Get algorithm description
    #[must_use]
    pub fn description(&self) -> String {
        match self {
            Self::Daniels => {
                "Jack Daniels VDOT (VO2 = -4.60 + 0.182258xv + 0.000104xv²)".to_owned()
            }
            Self::Riegel { exponent } => {
                format!("Riegel power-law (T2 = T1 x (D2/D1)^{exponent:.2})")
            }
            Self::Hybrid => "Hybrid VDOT (Daniels for 5K-Marathon, Riegel for ultra)".to_owned(),
        }
    }

    /// Get the formula as a string
    #[must_use]
    pub const fn formula(&self) -> &'static str {
        match self {
            Self::Daniels => "VO2 = -4.60 + 0.182258xv + 0.000104xv²",
            Self::Riegel { .. } => "T2 = T1 x (D2/D1)^exponent",
            Self::Hybrid => "Auto-select: Daniels (5K-Marathon) or Riegel (ultra)",
        }
    }
}

impl FromStr for VdotAlgorithm {
    type Err = IntelligenceError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "daniels" => Ok(Self::Daniels),
            "riegel" => Ok(Self::Riegel { exponent: 1.06 }),
            "hybrid" => Ok(Self::Hybrid),
            other => Err(IntelligenceError::invalid_input(format!(
                "Unknown VDOT algorithm: '{other}'. Valid options: daniels, riegel, hybrid"
            ))),
        }
    }
}
