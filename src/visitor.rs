// ABOUTME: Visitor pattern for single-pass activity time series analysis
// ABOUTME: Enables efficient data processing without multiple iterations over streams
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

//! # Time Series Visitor Pattern
//!
//! Provides a visitor pattern for processing activity time series data in a single pass.
//! This reduces memory allocations and improves performance when multiple analyses
//! need to be performed on the same data.
//!
//! ## Example
//!
//! ```rust,no_run
//! use dravr_cageux::visitor::{TimeSeriesExt, TimeSeriesVisitor, StatsCollector};
//! use dravr_cageux::models::TimeSeriesData;
//!
//! // Create time series data
//! let time_series = TimeSeriesData {
//!     timestamps: vec![0, 1, 2],
//!     heart_rate: Some(vec![120, 130, 140]),
//!     power: None,
//!     cadence: None,
//!     speed: None,
//!     altitude: None,
//!     temperature: None,
//!     gps_coordinates: None,
//! };
//! let mut stats = StatsCollector::default();
//! time_series.accept(&mut stats);
//!
//! if let Some(avg) = stats.heart_rate.average() {
//!     println!("Average HR: {}", avg);
//! }
//! ```

use crate::models::TimeSeriesData;

/// Visitor trait for processing time series data streams in a single pass.
///
/// Implement this trait to create custom analyzers that process activity data
/// efficiently. Default implementations are no-ops, so you only need to override
/// the methods for data streams you care about.
///
/// The visitor methods receive both the value and the timestamp offset from
/// activity start (in seconds), enabling time-aware analysis.
pub trait TimeSeriesVisitor {
    /// Called before iteration begins. Use for initialization.
    fn start(&mut self) {}

    /// Visit a heart rate measurement.
    ///
    /// # Arguments
    /// * `bpm` - Heart rate in beats per minute
    /// * `timestamp` - Seconds from activity start
    #[allow(unused_variables)]
    fn visit_heart_rate(&mut self, bpm: u32, timestamp: u32) {}

    /// Visit a power measurement.
    ///
    /// # Arguments
    /// * `watts` - Power output in watts
    /// * `timestamp` - Seconds from activity start
    #[allow(unused_variables)]
    fn visit_power(&mut self, watts: u32, timestamp: u32) {}

    /// Visit a cadence measurement.
    ///
    /// # Arguments
    /// * `rpm` - Cadence in revolutions/steps per minute
    /// * `timestamp` - Seconds from activity start
    #[allow(unused_variables)]
    fn visit_cadence(&mut self, rpm: u32, timestamp: u32) {}

    /// Visit a speed measurement.
    ///
    /// # Arguments
    /// * `meters_per_sec` - Speed in meters per second
    /// * `timestamp` - Seconds from activity start
    #[allow(unused_variables)]
    fn visit_speed(&mut self, meters_per_sec: f32, timestamp: u32) {}

    /// Visit an altitude measurement.
    ///
    /// # Arguments
    /// * `meters` - Altitude in meters
    /// * `timestamp` - Seconds from activity start
    #[allow(unused_variables)]
    fn visit_altitude(&mut self, meters: f32, timestamp: u32) {}

    /// Visit a temperature measurement.
    ///
    /// # Arguments
    /// * `celsius` - Temperature in degrees Celsius
    /// * `timestamp` - Seconds from activity start
    #[allow(unused_variables)]
    fn visit_temperature(&mut self, celsius: f32, timestamp: u32) {}

    /// Visit a GPS coordinate.
    ///
    /// # Arguments
    /// * `lat` - Latitude in degrees
    /// * `lon` - Longitude in degrees
    /// * `timestamp` - Seconds from activity start
    #[allow(unused_variables)]
    fn visit_location(&mut self, lat: f64, lon: f64, timestamp: u32) {}

    /// Called after iteration completes. Use for finalization and cleanup.
    fn finish(&mut self) {}
}

/// Extension trait for `TimeSeriesData` to support the visitor pattern.
///
/// Enables single-pass iteration over time series data with custom analyzers.
pub trait TimeSeriesExt {
    /// Accept a visitor and iterate over all time series data in a single pass.
    fn accept<V: TimeSeriesVisitor>(&self, visitor: &mut V);

    /// Accept multiple visitors and iterate over all data in a single pass.
    fn accept_all(&self, visitors: &mut [&mut dyn TimeSeriesVisitor]);
}

impl TimeSeriesExt for TimeSeriesData {
    /// Accept a visitor and iterate over all time series data in a single pass.
    ///
    /// This method iterates through the timestamps once, calling the appropriate
    /// visitor methods for each available data stream at each timestamp.
    ///
    /// # Arguments
    /// * `visitor` - A mutable reference to a type implementing `TimeSeriesVisitor`
    ///
    /// # Example
    ///
    /// See module-level documentation for a complete example.
    fn accept<V: TimeSeriesVisitor>(&self, visitor: &mut V) {
        visitor.start();

        for (idx, &timestamp) in self.timestamps.iter().enumerate() {
            // Visit heart rate if available at this index
            if let Some(hr_data) = &self.heart_rate {
                if let Some(&bpm) = hr_data.get(idx) {
                    visitor.visit_heart_rate(bpm, timestamp);
                }
            }

            // Visit power if available at this index
            if let Some(power_data) = &self.power {
                if let Some(&watts) = power_data.get(idx) {
                    visitor.visit_power(watts, timestamp);
                }
            }

            // Visit cadence if available at this index
            if let Some(cadence_data) = &self.cadence {
                if let Some(&rpm) = cadence_data.get(idx) {
                    visitor.visit_cadence(rpm, timestamp);
                }
            }

            // Visit speed if available at this index
            if let Some(speed_data) = &self.speed {
                if let Some(&speed) = speed_data.get(idx) {
                    visitor.visit_speed(speed, timestamp);
                }
            }

            // Visit altitude if available at this index
            if let Some(altitude_data) = &self.altitude {
                if let Some(&alt) = altitude_data.get(idx) {
                    visitor.visit_altitude(alt, timestamp);
                }
            }

            // Visit temperature if available at this index
            if let Some(temp_data) = &self.temperature {
                if let Some(&temp) = temp_data.get(idx) {
                    visitor.visit_temperature(temp, timestamp);
                }
            }

            // Visit GPS coordinates if available at this index
            if let Some(gps_data) = &self.gps_coordinates {
                if let Some(&(lat, lon)) = gps_data.get(idx) {
                    visitor.visit_location(lat, lon, timestamp);
                }
            }
        }

        visitor.finish();
    }

    /// Accept multiple visitors and iterate over all data in a single pass.
    ///
    /// This is more efficient than calling `accept` multiple times when you
    /// need to run several analyses on the same data.
    ///
    /// # Arguments
    /// * `visitors` - A slice of mutable visitor references
    fn accept_all(&self, visitors: &mut [&mut dyn TimeSeriesVisitor]) {
        for visitor in visitors.iter_mut() {
            visitor.start();
        }

        for (idx, &timestamp) in self.timestamps.iter().enumerate() {
            if let Some(hr_data) = &self.heart_rate {
                if let Some(&bpm) = hr_data.get(idx) {
                    for visitor in visitors.iter_mut() {
                        visitor.visit_heart_rate(bpm, timestamp);
                    }
                }
            }

            if let Some(power_data) = &self.power {
                if let Some(&watts) = power_data.get(idx) {
                    for visitor in visitors.iter_mut() {
                        visitor.visit_power(watts, timestamp);
                    }
                }
            }

            if let Some(cadence_data) = &self.cadence {
                if let Some(&rpm) = cadence_data.get(idx) {
                    for visitor in visitors.iter_mut() {
                        visitor.visit_cadence(rpm, timestamp);
                    }
                }
            }

            if let Some(speed_data) = &self.speed {
                if let Some(&speed) = speed_data.get(idx) {
                    for visitor in visitors.iter_mut() {
                        visitor.visit_speed(speed, timestamp);
                    }
                }
            }

            if let Some(altitude_data) = &self.altitude {
                if let Some(&alt) = altitude_data.get(idx) {
                    for visitor in visitors.iter_mut() {
                        visitor.visit_altitude(alt, timestamp);
                    }
                }
            }

            if let Some(temp_data) = &self.temperature {
                if let Some(&temp) = temp_data.get(idx) {
                    for visitor in visitors.iter_mut() {
                        visitor.visit_temperature(temp, timestamp);
                    }
                }
            }

            if let Some(gps_data) = &self.gps_coordinates {
                if let Some(&(lat, lon)) = gps_data.get(idx) {
                    for visitor in visitors.iter_mut() {
                        visitor.visit_location(lat, lon, timestamp);
                    }
                }
            }
        }

        for visitor in visitors.iter_mut() {
            visitor.finish();
        }
    }
}

// === Built-in Visitor Implementations ===

/// Statistics for a numeric stream (min, max, sum, count, average).
#[derive(Debug, Clone, Default)]
pub struct StreamStats {
    /// Minimum value observed
    pub min: Option<f64>,
    /// Maximum value observed
    pub max: Option<f64>,
    /// Sum of all values
    pub sum: f64,
    /// Number of data points
    pub count: u64,
}

impl StreamStats {
    /// Calculate the average value.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn average(&self) -> Option<f64> {
        if self.count > 0 {
            // Safe: Activity data never approaches 2^52 data points where precision loss matters
            Some(self.sum / self.count as f64)
        } else {
            None
        }
    }

    /// Update stats with a new value.
    fn update(&mut self, value: f64) {
        self.min = Some(self.min.map_or(value, |m| m.min(value)));
        self.max = Some(self.max.map_or(value, |m| m.max(value)));
        self.sum += value;
        self.count += 1;
    }
}

/// Collects basic statistics for all numeric streams in a single pass.
///
/// # Example
///
/// ```rust,no_run
/// use dravr_cageux::visitor::{TimeSeriesExt, TimeSeriesVisitor, StatsCollector};
/// use dravr_cageux::models::TimeSeriesData;
///
/// let time_series = TimeSeriesData {
///     timestamps: vec![0, 1, 2, 3, 4],
///     heart_rate: Some(vec![120, 130, 140, 135, 125]),
///     power: Some(vec![200, 220, 240, 230, 210]),
///     cadence: None,
///     speed: None,
///     altitude: None,
///     temperature: None,
///     gps_coordinates: None,
/// };
///
/// let mut stats = StatsCollector::default();
/// time_series.accept(&mut stats);
///
/// // Access statistics for each stream
/// if let Some(avg_hr) = stats.heart_rate.average() {
///     println!("Average HR: {:.1} bpm", avg_hr);
/// }
/// if let (Some(min), Some(max)) = (stats.power.min, stats.power.max) {
///     println!("Power range: {:.0}-{:.0} watts", min, max);
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct StatsCollector {
    /// Heart rate statistics
    pub heart_rate: StreamStats,
    /// Power statistics
    pub power: StreamStats,
    /// Cadence statistics
    pub cadence: StreamStats,
    /// Speed statistics
    pub speed: StreamStats,
    /// Altitude statistics
    pub altitude: StreamStats,
    /// Temperature statistics
    pub temperature: StreamStats,
}

impl TimeSeriesVisitor for StatsCollector {
    fn visit_heart_rate(&mut self, bpm: u32, _timestamp: u32) {
        self.heart_rate.update(f64::from(bpm));
    }

    fn visit_power(&mut self, watts: u32, _timestamp: u32) {
        self.power.update(f64::from(watts));
    }

    fn visit_cadence(&mut self, rpm: u32, _timestamp: u32) {
        self.cadence.update(f64::from(rpm));
    }

    fn visit_speed(&mut self, meters_per_sec: f32, _timestamp: u32) {
        self.speed.update(f64::from(meters_per_sec));
    }

    fn visit_altitude(&mut self, meters: f32, _timestamp: u32) {
        self.altitude.update(f64::from(meters));
    }

    fn visit_temperature(&mut self, celsius: f32, _timestamp: u32) {
        self.temperature.update(f64::from(celsius));
    }
}

/// Calculates normalized power using the 30-second rolling average method.
///
/// Normalized Power (NP) represents the metabolic cost of an activity,
/// accounting for the non-linear physiological response to varying power outputs.
///
/// # Example
///
/// ```rust,no_run
/// use dravr_cageux::visitor::{TimeSeriesExt, TimeSeriesVisitor, NormalizedPowerCalculator};
/// use dravr_cageux::models::TimeSeriesData;
///
/// // Create power data (at least 30 seconds needed for NP calculation)
/// let power_values: Vec<u32> = (0..60).map(|i| 200 + (i % 50)).collect();
/// let timestamps: Vec<u32> = (0..60).collect();
///
/// let time_series = TimeSeriesData {
///     timestamps,
///     heart_rate: None,
///     power: Some(power_values),
///     cadence: None,
///     speed: None,
///     altitude: None,
///     temperature: None,
///     gps_coordinates: None,
/// };
///
/// let mut np_calc = NormalizedPowerCalculator::default();
/// time_series.accept(&mut np_calc);
///
/// if let Some(np) = np_calc.normalized_power() {
///     println!("Normalized Power: {:.0} watts", np);
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct NormalizedPowerCalculator {
    /// Rolling window of last 30 power values
    window: Vec<f64>,
    /// Sum of 30-second average power^4 values
    sum_power4: f64,
    /// Count of 30-second averages calculated
    count: u64,
}

impl NormalizedPowerCalculator {
    /// Rolling window size (30 seconds)
    const WINDOW_SIZE: usize = 30;

    /// Get the calculated normalized power.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn normalized_power(&self) -> Option<f64> {
        if self.count == 0 {
            return None;
        }

        // Safe: Activity data never approaches 2^52 data points where precision loss matters
        let mean_power4 = self.sum_power4 / self.count as f64;
        Some(mean_power4.powf(0.25))
    }
}

impl TimeSeriesVisitor for NormalizedPowerCalculator {
    #[allow(clippy::cast_precision_loss)]
    fn visit_power(&mut self, watts: u32, _timestamp: u32) {
        self.window.push(f64::from(watts));

        if self.window.len() >= Self::WINDOW_SIZE {
            // Safe: WINDOW_SIZE is 30, well below f64 precision limits
            let window_avg: f64 = self.window.iter().sum::<f64>() / Self::WINDOW_SIZE as f64;
            self.sum_power4 += window_avg.powi(4);
            self.count += 1;

            // Remove oldest value to maintain window size
            self.window.remove(0);
        }
    }
}

/// Detects cardiac decoupling (drift in HR:pace ratio).
///
/// Decoupling occurs when heart rate increases relative to pace over time,
/// indicating cardiovascular fatigue. A decoupling >5% suggests the activity
/// was too intense for the athlete's current aerobic fitness.
///
/// # Example
///
/// ```rust,no_run
/// use dravr_cageux::visitor::{TimeSeriesExt, TimeSeriesVisitor, DecouplingDetector};
/// use dravr_cageux::models::TimeSeriesData;
///
/// // Simulate HR drift: same speed but increasing heart rate over time
/// let timestamps: Vec<u32> = (0..40).collect();
/// let heart_rates: Vec<u32> = (0..40).map(|i| 140 + i / 2).collect(); // HR drifts up
/// let speeds: Vec<f32> = vec![3.5; 40]; // Constant pace
///
/// let time_series = TimeSeriesData {
///     timestamps,
///     heart_rate: Some(heart_rates),
///     power: None,
///     cadence: None,
///     speed: Some(speeds),
///     altitude: None,
///     temperature: None,
///     gps_coordinates: None,
/// };
///
/// let mut detector = DecouplingDetector::default();
/// time_series.accept(&mut detector);
///
/// if let Some(decoupling) = detector.decoupling_percentage() {
///     println!("Cardiac decoupling: {:.1}%", decoupling);
///     if decoupling > 5.0 {
///         println!("Warning: Activity may have been too intense");
///     }
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct DecouplingDetector {
    /// Current heart rate value waiting for paired speed
    current_hr: Option<f64>,
    /// Current speed value waiting for paired heart rate
    current_speed: Option<f64>,
    /// Accumulated data points with both HR and speed
    data_points: Vec<(f64, f64)>,
}

impl DecouplingDetector {
    /// Minimum data points required for reliable decoupling calculation
    const MIN_DATA_POINTS: usize = 20;

    /// Calculate decoupling percentage.
    ///
    /// Returns the percentage difference in efficiency (HR/speed ratio)
    /// between the first and second halves of the activity.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn decoupling_percentage(&self) -> Option<f64> {
        if self.data_points.len() < Self::MIN_DATA_POINTS {
            return None;
        }

        let midpoint = self.data_points.len() / 2;
        let first_half = &self.data_points[..midpoint];
        let second_half = &self.data_points[midpoint..];

        // Safe: Activity data never approaches 2^52 data points where precision loss matters
        // Calculate average efficiency (HR/speed) for each half
        let first_avg_hr: f64 =
            first_half.iter().map(|(hr, _)| hr).sum::<f64>() / first_half.len() as f64;
        let first_avg_speed: f64 =
            first_half.iter().map(|(_, s)| s).sum::<f64>() / first_half.len() as f64;

        let second_avg_hr: f64 =
            second_half.iter().map(|(hr, _)| hr).sum::<f64>() / second_half.len() as f64;
        let second_avg_speed: f64 =
            second_half.iter().map(|(_, s)| s).sum::<f64>() / second_half.len() as f64;

        // Avoid division by zero
        if first_avg_speed == 0.0 || second_avg_speed == 0.0 {
            return None;
        }

        let first_efficiency = first_avg_hr / first_avg_speed;
        let second_efficiency = second_avg_hr / second_avg_speed;

        if first_efficiency == 0.0 {
            return None;
        }

        // Decoupling is the percentage increase in HR/speed ratio
        Some((second_efficiency - first_efficiency) / first_efficiency * 100.0)
    }
}

impl TimeSeriesVisitor for DecouplingDetector {
    fn visit_heart_rate(&mut self, bpm: u32, _timestamp: u32) {
        self.current_hr = Some(f64::from(bpm));

        // If we have both HR and speed, record the data point
        if let (Some(hr), Some(speed)) = (self.current_hr, self.current_speed) {
            self.data_points.push((hr, speed));
            self.current_hr = None;
            self.current_speed = None;
        }
    }

    fn visit_speed(&mut self, meters_per_sec: f32, _timestamp: u32) {
        self.current_speed = Some(f64::from(meters_per_sec));

        // If we have both HR and speed, record the data point
        if let (Some(hr), Some(speed)) = (self.current_hr, self.current_speed) {
            self.data_points.push((hr, speed));
            self.current_hr = None;
            self.current_speed = None;
        }
    }
}
