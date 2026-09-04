// ABOUTME: Performance prediction using VDOT and Riegel formulas for race time estimation
// ABOUTME: Implements Jack Daniels' VDOT methodology and Riegel's race time prediction formula
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

use crate::config::intelligence::{AlgorithmConfig, AlgorithmParamsConfig};
use crate::error::IntelligenceError;
use crate::models::Activity;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;

/// Standard race distances in meters
const DISTANCE_5K: f64 = 5_000.0;
const DISTANCE_10K: f64 = 10_000.0;
const DISTANCE_15K: f64 = 15_000.0;
const DISTANCE_HALF_MARATHON: f64 = 21_097.5;
const DISTANCE_MARATHON: f64 = 42_195.0;

/// Race predictions for standard distances
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RacePredictions {
    /// VDOT value (VO2 max adjusted for running economy)
    pub vdot: f64,
    /// Predicted race times in seconds for standard distances
    pub predictions: HashMap<String, f64>,
    /// Source activity used for calculation
    pub based_on_distance_meters: f64,
    /// Duration of source activity in seconds
    pub based_on_time_seconds: f64,
}

/// Performance prediction engine
pub struct PerformancePredictor;

impl PerformancePredictor {
    /// Calculate VDOT from race performance
    ///
    /// VDOT is Jack Daniels' VO2 max adjusted for running economy.
    /// The VDOT variant is resolved from `config` (default `daniels`).
    ///
    /// # Arguments
    /// * `distance_meters` - Race distance in meters
    /// * `time_seconds` - Race time in seconds
    /// * `config` - Algorithm configuration selecting the VDOT variant
    ///
    /// # Returns
    /// VDOT value (typically 30-85 for recreational to elite runners)
    ///
    /// # Errors
    /// Returns `IntelligenceError::InvalidInput` if time or distance is non-positive, or if velocity is outside valid range
    pub fn calculate_vdot(
        distance_meters: f64,
        time_seconds: f64,
        config: &AlgorithmConfig,
    ) -> Result<f64, IntelligenceError> {
        config
            .vdot_algorithm()
            .calculate_vdot(distance_meters, time_seconds)
    }

    /// Predict race time from a VDOT value.
    ///
    /// The VDOT variant is resolved from `config` (default `daniels`, which
    /// inverts Daniels' VO2/%`VO2max` relation; `riegel` applies the configured
    /// power-law exponent).
    ///
    /// # Arguments
    /// * `vdot` - VDOT value
    /// * `target_distance_meters` - Target race distance
    /// * `config` - Algorithm configuration selecting the VDOT variant
    ///
    /// # Returns
    /// Predicted race time in seconds
    ///
    /// # Errors
    /// Returns `IntelligenceError::InvalidInput` if VDOT is outside typical range (30-85)
    pub fn predict_time_vdot(
        vdot: f64,
        target_distance_meters: f64,
        config: &AlgorithmConfig,
    ) -> Result<f64, IntelligenceError> {
        config
            .vdot_algorithm()
            .predict_time(vdot, target_distance_meters)
    }

    /// Predict race time directly from a known performance using Riegel's power law.
    ///
    /// Riegel's formula: `Time2 = Time1 x (Distance2 / Distance1)^exponent`
    ///
    /// This is a simpler alternative to VDOT that works reasonably well
    /// for predicting times at different distances. The exponent is sourced
    /// from `params` (default 1.06) so it can be tuned via config/env.
    ///
    /// # Arguments
    /// * `known_distance` - Distance of known race in meters
    /// * `known_time` - Time of known race in seconds
    /// * `target_distance` - Target race distance in meters
    /// * `params` - Algorithm tuning parameters supplying the Riegel exponent
    ///
    /// # Errors
    /// Returns `IntelligenceError::InvalidInput` if any distance or time is non-positive
    pub fn predict_time_riegel(
        known_distance: f64,
        known_time: f64,
        target_distance: f64,
        params: &AlgorithmParamsConfig,
    ) -> Result<f64, IntelligenceError> {
        if known_distance <= 0.0 || known_time <= 0.0 || target_distance <= 0.0 {
            return Err(IntelligenceError::invalid_input(
                "All distances and times must be positive".to_owned(),
            ));
        }

        let distance_ratio = target_distance / known_distance;
        let predicted_time = known_time * distance_ratio.powf(params.vdot_riegel_exponent);

        Ok(predicted_time)
    }

    /// Generate predictions for standard race distances
    ///
    /// Given a single race performance, predicts times for 5K, 10K, 15K, Half, Marathon
    ///
    /// # Errors
    /// Returns `IntelligenceError::InvalidInput` if distance or time values are invalid for VDOT calculation
    pub fn generate_race_predictions(
        distance_meters: f64,
        time_seconds: f64,
        config: &AlgorithmConfig,
    ) -> Result<RacePredictions, IntelligenceError> {
        let vdot = Self::calculate_vdot(distance_meters, time_seconds, config)?;

        let mut predictions = HashMap::new();

        // Predict standard distances
        let distances = vec![
            ("5K", DISTANCE_5K),
            ("10K", DISTANCE_10K),
            ("15K", DISTANCE_15K),
            ("Half Marathon", DISTANCE_HALF_MARATHON),
            ("Marathon", DISTANCE_MARATHON),
        ];

        for (name, distance) in distances {
            if let Ok(predicted_time) = Self::predict_time_vdot(vdot, distance, config) {
                predictions.insert(name.to_owned(), predicted_time);
            }
        }

        Ok(RacePredictions {
            vdot,
            predictions,
            based_on_distance_meters: distance_meters,
            based_on_time_seconds: time_seconds,
        })
    }

    /// Generate predictions from a best performance activity
    ///
    /// # Errors
    /// Returns `IntelligenceError::InvalidInput` if activity lacks distance or duration data
    pub fn generate_predictions_from_activity(
        activity: &Activity,
        config: &AlgorithmConfig,
    ) -> Result<RacePredictions, IntelligenceError> {
        let distance = activity.distance_meters().ok_or_else(|| {
            IntelligenceError::invalid_input("Activity must have distance".to_owned())
        })?;

        let duration = activity.duration_seconds();

        #[allow(clippy::cast_precision_loss)]
        let duration_f64 = duration as f64;
        Self::generate_race_predictions(distance, duration_f64, config)
    }

    /// Find best performance from activities for race prediction
    ///
    /// Looks for fastest pace activities that are likely race efforts (>3km, <2 hours)
    #[must_use]
    pub fn find_best_performance(activities: &[Activity]) -> Option<&Activity> {
        activities
            .iter()
            .filter(|a| {
                // Filter for likely race efforts
                a.distance_meters().is_some_and(|distance| {
                    let duration = a.duration_seconds();
                    #[allow(clippy::cast_precision_loss)]
                    let duration_f64 = duration as f64;
                    // At least 3K distance, non-zero duration
                    duration > 0
                        && distance >= 3_000.0
                        // Less than 2 hours
                        && duration < 7_200
                        // Reasonable pace (faster than 8 min/km)
                        && (distance / duration_f64) > (1000.0 / 480.0)
                })
            })
            .max_by(|a, b| {
                // Find fastest pace (duration > 0 is guaranteed by filter above)
                #[allow(clippy::cast_precision_loss)]
                let pace_a = a.distance_meters().map_or(0.0, |d| {
                    let dur = a.duration_seconds().max(1) as f64;
                    d / dur
                });
                #[allow(clippy::cast_precision_loss)]
                let pace_b = b.distance_meters().map_or(0.0, |d| {
                    let dur = b.duration_seconds().max(1) as f64;
                    d / dur
                });
                pace_a.partial_cmp(&pace_b).unwrap_or(Ordering::Equal)
            })
    }

    /// Format time in seconds to human-readable format (HH:MM:SS)
    #[must_use]
    pub fn format_time(seconds: f64) -> String {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let total_seconds = seconds.round() as u32;
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let secs = total_seconds % 60;

        if hours > 0 {
            format!("{hours}:{minutes:02}:{secs:02}")
        } else {
            format!("{minutes}:{secs:02}")
        }
    }

    /// Format pace in min/km
    #[must_use]
    pub fn format_pace_per_km(meters_per_second: f64) -> String {
        if meters_per_second <= 0.0 {
            return "N/A".to_owned();
        }

        let seconds_per_km = 1000.0 / meters_per_second;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let minutes = (seconds_per_km / 60.0).floor() as u32;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let seconds = (seconds_per_km % 60.0).round() as u32;

        format!("{minutes}:{seconds:02}")
    }
}
