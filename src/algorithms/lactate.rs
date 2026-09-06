// ABOUTME: Lactate step-test analysis — LT1 by log-log breakpoint, LT2 by Dmax, modified Dmax and the fixed 4.0 mmol OBLA
// ABOUTME: Pure arithmetic over the athlete's own stages; a construct the protocol cannot support is reported, never guessed
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

//! Graded lactate step test → LT1 / LT2 → the mmol band table.
//!
//! The athlete reports the stages of an incremental test — intensity as power
//! or running pace, blood lactate in mmol/L, heart rate when a strap was worn —
//! and [`LactateStepTest::analyze`] locates the two thresholds by the published
//! constructs, each reported under its own name because they do not coincide
//! (Jamnick et al. 2020):
//!
//! - **LT1 — log-log breakpoint** (Beaver, Wasserman & Whipp 1985): the
//!   intersection of the two regression lines that best split
//!   ln(lactate) against ln(intensity). The first rise above baseline.
//! - **LT2 — modified Dmax** (Bishop, Jenkins & Mackinnon 1998): the point on a
//!   third-order fit farthest from the chord joining the stage before the first
//!   rise greater than 0.4 mmol/L to the final stage.
//! - **LT2 — Dmax** (Cheng et al. 1992): the same construction with the chord
//!   from the first stage to the last.
//! - **LT2 — fixed 4.0 mmol/L OBLA** (Heck et al. 1985): linear interpolation of
//!   the first crossing of 4.0 mmol/L. A convention, not a physiological
//!   constant — Faude, Kindermann & Meyer (2009) is the reference critique, and
//!   well-trained athletes turn at 2.5–4.0 mmol/L (Seiler-Viken et al. 2025).
//!
//! Every method answers [`ThresholdOutcome::NotDeterminable`] with a reason when
//! the protocol cannot support it — lactate that never reaches 4.0, a curve
//! that never departs from its chord by as much as a meter can display,
//! segments that do not intersect — instead of substituting a rule of thumb.
//! Comparisons against the published thresholds carry the representation
//! error, so a reading a meter cannot distinguish cannot change the answer. The analysis
//! refuses fewer than four stages, more than [`MAX_STAGES`], or an intensity
//! that does not increase from stage to stage.
//!
//! # References
//!
//! - Beaver WL, Wasserman K, Whipp BJ. 1985. Improved detection of lactate
//!   threshold during exercise using a log-log transformation. *J Appl Physiol*
//!   59(6):1936–1940.
//! - Cheng B, Kuipers H, Snyder AC, Keizer HA, Jeukendrup A, Hesselink M. 1992.
//!   A new approach for the determination of ventilatory and lactate
//!   thresholds. *Int J Sports Med* 13(7):518–522.
//! - Bishop D, Jenkins DG, Mackinnon LT. 1998. The relationship between plasma
//!   lactate parameters, Wpeak and 1-h cycling performance in women. *Med Sci
//!   Sports Exerc* 30(8):1270–1275.
//! - Heck H, Mader A, Hess G, Mücke S, Müller R, Hollmann W. 1985. Justification
//!   of the 4-mmol/l lactate threshold. *Int J Sports Med* 6(3):117–130.
//! - Faude O, Kindermann W, Meyer T. 2009. Lactate threshold concepts: how valid
//!   are they? *Sports Med* 39(6):469–490.
//! - Jamnick NA, Pettitt RW, Granata C, Pyne DB, Bishop DJ. 2020. An examination
//!   and critique of current methods to determine exercise intensity. *Sports
//!   Med* 50(10):1729–1756.
//! - Seiler-Viken SA, Mentzoni F, Seiler S, Skarli S, Losnegard T. 2025. *Sci
//!   Rep* 15(1):34367.

use crate::error::{IntelligenceError, IntelligenceResult};
use serde::{Deserialize, Serialize};

/// The fewest stages any of the constructs can be fitted on: a cubic has four
/// coefficients, and the log-log split needs two points on each side.
pub const MIN_STAGES: usize = 4;

/// The most stages a graded test is accepted with.
///
/// A step test is a handful of stages — four to eight is typical and the
/// longest ramp protocols run to about fifteen, because each stage costs the
/// athlete three to five minutes and a finger prick. Fifty is far past any
/// real protocol, and the ceiling matters: the log-log search fits a
/// regression pair at every split, so its cost grows with the square of the
/// stage count. Without a bound, a caller that sends tens of thousands of
/// stages spends minutes of CPU inside one call.
pub const MAX_STAGES: usize = 50;

/// The fixed blood-lactate concentration of the OBLA convention, in mmol/L.
pub const OBLA_MMOL: f64 = 4.0;

/// The stage-to-stage rise that marks the first lactate increase in the
/// modified Dmax construction (Bishop et al. 1998), in mmol/L.
pub const MODIFIED_DMAX_RISE_MMOL: f64 = 0.4;

/// The smallest departure from the chord the Dmax constructions will call a
/// threshold, in mmol/L.
///
/// A portable meter displays to 0.1 mmol/L, so a curve whose greatest
/// departure from its own chord is smaller than that is describing the
/// least-squares fit, not the athlete. Without this floor a submaximal test
/// that never approached LT2 still returns a "determined" threshold sitting
/// on a departure of a few thousandths of a mmol — and it can land below
/// LT1. Refusing is the honest answer, and matches what OBLA and modified
/// Dmax already do when the protocol cannot support them.
pub const DMAX_MIN_DEPARTURE_MMOL: f64 = 0.1;

/// Tolerance for comparing a measured lactate rise against
/// [`MODIFIED_DMAX_RISE_MMOL`].
///
/// Meters read to 0.1 mmol/L, so a rise of *exactly* 0.4 is the common case —
/// and binary64 does not agree with itself about it: `1.6 - 1.2` is
/// 0.400000000000000133 while `2.0 - 1.6` is 0.399999999999999911. Comparing
/// raw would let the athlete's particular decimal pair decide where the chord
/// starts, which moves LT2 and every power zone anchored on it. The published
/// criterion is a rise *greater than* 0.4, and on a 0.1-resolution meter that
/// means 0.5 or more, so the comparison is strict with the representation
/// error taken out.
const RISE_EPSILON_MMOL: f64 = 1e-9;

/// The lactate concentrations the band table is interpolated at, in mmol/L.
/// They span the LT1 band (1.0–2.0) and the LT2 band (2.5–4.0) reported by
/// Seiler-Viken et al. (2025).
pub const BAND_TABLE_MMOL: [f64; 7] = [1.0, 1.5, 2.0, 2.5, 3.0, 3.5, 4.0];

/// Blood lactate a portable meter can read, in mmol/L.
const LACTATE_RANGE_MMOL: (f64, f64) = (0.3, 25.0);
/// Heart rate a human can hold, in bpm.
const HEART_RATE_RANGE_BPM: (f64, f64) = (40.0, 220.0);
/// Stage power a human can hold, in watts.
const WATTS_RANGE: (f64, f64) = (1.0, 2500.0);
/// Stage running pace, in seconds per kilometre (1:40/km to 30:00/km).
const PACE_RANGE_SEC_PER_KM: (f64, f64) = (100.0, 1800.0);

/// A pivot smaller than this makes the normal equations singular.
const SINGULAR_PIVOT: f64 = 1e-12;

/// How the stage intensity is expressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LactateIntensityUnit {
    /// Cycling or running power, in watts. Higher is harder.
    Watts,
    /// Running pace, in seconds per kilometre. Lower is harder.
    SecondsPerKm,
}

impl LactateIntensityUnit {
    /// The unit's name as the schema and the reply spell it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Watts => "watts",
            Self::SecondsPerKm => "seconds_per_km",
        }
    }

    /// Map a stage intensity onto an axis that increases with effort: watts
    /// as they are, pace as speed in metres per second.
    fn effort(self, intensity: f64) -> f64 {
        match self {
            Self::Watts => intensity,
            Self::SecondsPerKm => 1000.0 / intensity,
        }
    }

    /// Map a point on the effort axis back to the unit the stages used.
    fn intensity(self, effort: f64) -> f64 {
        match self {
            Self::Watts => effort,
            Self::SecondsPerKm => 1000.0 / effort,
        }
    }

    const fn range(self) -> (f64, f64) {
        match self {
            Self::Watts => WATTS_RANGE,
            Self::SecondsPerKm => PACE_RANGE_SEC_PER_KM,
        }
    }
}

/// One stage of a graded lactate test.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LactateStage {
    /// Stage intensity in the test's [`LactateIntensityUnit`].
    pub intensity: f64,
    /// Blood lactate sampled at the end of the stage, in mmol/L.
    pub lactate_mmol: f64,
    /// Heart rate at the end of the stage, in bpm, when a strap was worn.
    pub heart_rate: Option<f64>,
}

/// A graded lactate step test as the athlete reports it, stages in the order
/// they were run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LactateStepTest {
    /// How every stage's intensity is expressed.
    pub unit: LactateIntensityUnit,
    /// The stages, easiest first.
    pub stages: Vec<LactateStage>,
}

/// The published construct a threshold was located with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LactateThresholdMethod {
    /// LT1 as the intersection of two log-log regression segments.
    LogLog,
    /// LT2 as the point farthest from the first-to-last chord on a cubic fit.
    Dmax,
    /// LT2 as Dmax with the chord starting at the stage before the first rise
    /// greater than 0.4 mmol/L.
    ModifiedDmax,
    /// LT2 as the interpolated crossing of 4.0 mmol/L.
    Obla4,
}

impl LactateThresholdMethod {
    /// Which threshold the construct marks.
    #[must_use]
    pub const fn threshold(self) -> &'static str {
        match self {
            Self::LogLog => "LT1",
            Self::Dmax | Self::ModifiedDmax | Self::Obla4 => "LT2",
        }
    }

    /// The method's name as the schema and the reply spell it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LogLog => "log_log",
            Self::Dmax => "dmax",
            Self::ModifiedDmax => "modified_dmax",
            Self::Obla4 => "obla_4mmol",
        }
    }

    /// The paper the construct comes from.
    #[must_use]
    pub const fn reference(self) -> &'static str {
        match self {
            Self::LogLog => "Beaver, Wasserman & Whipp 1985, J Appl Physiol 59(6):1936-1940",
            Self::Dmax => "Cheng et al. 1992, Int J Sports Med 13(7):518-522",
            Self::ModifiedDmax => "Bishop, Jenkins & Mackinnon 1998, Med Sci Sports Exerc 30(8):1270-1275",
            Self::Obla4 => "Heck et al. 1985, Int J Sports Med 6(3):117-130; critique: Faude, Kindermann & Meyer 2009, Sports Med 39(6):469-490",
        }
    }
}

/// A threshold located on the intensity axis, with the lactate and heart
/// rate at that point.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LactateThresholdPoint {
    /// Intensity at the threshold, in the test's unit.
    pub intensity: f64,
    /// Blood lactate at the threshold, in mmol/L.
    pub lactate_mmol: f64,
    /// Heart rate at the threshold, interpolated between the two stages that
    /// bracket it, when both carried one.
    pub heart_rate: Option<f64>,
}

/// What one construct concluded from the stages.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum ThresholdOutcome {
    /// The construct located the threshold.
    Determined(LactateThresholdPoint),
    /// The protocol cannot support the construct; the reason names why.
    NotDeterminable {
        /// Why the construct could not be applied to these stages.
        reason: String,
    },
}

impl ThresholdOutcome {
    /// The located point, when there is one.
    #[must_use]
    pub const fn point(&self) -> Option<&LactateThresholdPoint> {
        match self {
            Self::Determined(point) => Some(point),
            Self::NotDeterminable { .. } => None,
        }
    }
}

/// The third-order least-squares fit of lactate against effort that the Dmax
/// constructions run on.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LactateCurveFit {
    /// Coefficients `c0..c3` of `lactate = c0 + c1·t + c2·t² + c3·t³`, where
    /// `t` is effort normalised to `[0, 1]` across the stages.
    pub coefficients: [f64; 4],
    /// Share of the lactate variance the cubic explains; 1.0 when the fit
    /// passes through every stage.
    pub r_squared: f64,
}

/// One row of the mmol band table: the intensity and heart rate at which the
/// measured curve first crossed a given lactate concentration.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LactateBandRow {
    /// The lactate concentration this row is interpolated at, in mmol/L.
    pub lactate_mmol: f64,
    /// Intensity at the first crossing, in the test's unit.
    pub intensity: f64,
    /// Heart rate at the crossing, when both bracketing stages carried one.
    pub heart_rate: Option<f64>,
}

/// Everything a step test yields, each threshold under the construct that
/// produced it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LactateThresholds {
    /// The unit every intensity below is expressed in.
    pub unit: LactateIntensityUnit,
    /// How many stages the analysis ran on.
    pub stage_count: usize,
    /// LT1 by the log-log breakpoint.
    pub lt1_log_log: ThresholdOutcome,
    /// LT2 by modified Dmax.
    pub lt2_modified_dmax: ThresholdOutcome,
    /// LT2 by Dmax.
    pub lt2_dmax: ThresholdOutcome,
    /// LT2 by the fixed 4.0 mmol/L convention.
    pub lt2_obla_4mmol: ThresholdOutcome,
    /// The cubic the Dmax constructions ran on.
    pub curve: LactateCurveFit,
    /// Intensity and heart rate at each concentration of [`BAND_TABLE_MMOL`]
    /// the stages actually crossed, in ascending lactate order.
    pub band_table: Vec<LactateBandRow>,
}

/// The stages after validation, on the effort axis.
struct EffortSeries {
    unit: LactateIntensityUnit,
    /// Effort per stage, strictly increasing.
    efforts: Vec<f64>,
    /// Lactate per stage, in mmol/L.
    lactates: Vec<f64>,
    /// Heart rate per stage, when reported.
    heart_rates: Vec<Option<f64>>,
}

impl EffortSeries {
    fn from_test(test: &LactateStepTest) -> IntelligenceResult<Self> {
        let actual = test.stages.len();
        if actual < MIN_STAGES {
            return Err(IntelligenceError::insufficient_data(MIN_STAGES, actual));
        }
        if actual > MAX_STAGES {
            return Err(IntelligenceError::invalid_input_field(
                "stages",
                format!(
                    "a graded step test runs at most {MAX_STAGES} stages; got {actual}. Send the test's own stages, not a time series"
                ),
            ));
        }
        let (min_intensity, max_intensity) = test.unit.range();
        let mut efforts = Vec::with_capacity(actual);
        let mut lactates = Vec::with_capacity(actual);
        let mut heart_rates = Vec::with_capacity(actual);
        for (index, stage) in test.stages.iter().enumerate() {
            let field = |name: &str| format!("stages[{index}].{name}");
            if !stage.intensity.is_finite()
                || stage.intensity < min_intensity
                || stage.intensity > max_intensity
            {
                return Err(IntelligenceError::out_of_range(
                    field("intensity"),
                    stage.intensity,
                    min_intensity,
                    max_intensity,
                ));
            }
            if !stage.lactate_mmol.is_finite()
                || stage.lactate_mmol < LACTATE_RANGE_MMOL.0
                || stage.lactate_mmol > LACTATE_RANGE_MMOL.1
            {
                return Err(IntelligenceError::out_of_range(
                    field("lactate_mmol"),
                    stage.lactate_mmol,
                    LACTATE_RANGE_MMOL.0,
                    LACTATE_RANGE_MMOL.1,
                ));
            }
            if let Some(heart_rate) = stage.heart_rate {
                if !heart_rate.is_finite()
                    || heart_rate < HEART_RATE_RANGE_BPM.0
                    || heart_rate > HEART_RATE_RANGE_BPM.1
                {
                    return Err(IntelligenceError::out_of_range(
                        field("heart_rate"),
                        heart_rate,
                        HEART_RATE_RANGE_BPM.0,
                        HEART_RATE_RANGE_BPM.1,
                    ));
                }
            }
            let effort = test.unit.effort(stage.intensity);
            if let Some(previous) = efforts.last().copied() {
                if effort <= previous {
                    let direction = match test.unit {
                        LactateIntensityUnit::Watts => "more watts",
                        LactateIntensityUnit::SecondsPerKm => "a faster pace",
                    };
                    return Err(IntelligenceError::invalid_input_field(
                        field("intensity"),
                        format!(
                            "every stage must be harder than the one before ({direction}); stage {} is not harder than stage {}",
                            index + 1,
                            index
                        ),
                    ));
                }
            }
            efforts.push(effort);
            lactates.push(stage.lactate_mmol);
            heart_rates.push(stage.heart_rate);
        }
        Ok(Self {
            unit: test.unit,
            efforts,
            lactates,
            heart_rates,
        })
    }

    fn len(&self) -> usize {
        self.efforts.len()
    }

    fn first_effort(&self) -> f64 {
        self.efforts.first().copied().unwrap_or_default()
    }

    fn last_effort(&self) -> f64 {
        self.efforts.last().copied().unwrap_or_default()
    }

    /// Effort normalised to `[0, 1]` across the stages, which keeps the cubic
    /// normal equations well conditioned at any wattage.
    fn normalised(&self, effort: f64) -> f64 {
        (effort - self.first_effort()) / (self.last_effort() - self.first_effort())
    }

    fn denormalised(&self, t: f64) -> f64 {
        (self.last_effort() - self.first_effort()).mul_add(t, self.first_effort())
    }

    /// Heart rate at an effort, interpolated between the two stages that
    /// bracket it when both carried one.
    fn heart_rate_at(&self, effort: f64) -> Option<f64> {
        let upper = self.efforts.iter().position(|&e| e >= effort)?;
        if upper == 0 {
            return self.heart_rates.first().copied().flatten();
        }
        let lower = upper - 1;
        let (low_hr, high_hr) = (self.heart_rates[lower]?, self.heart_rates[upper]?);
        let span = self.efforts[upper] - self.efforts[lower];
        let fraction = (effort - self.efforts[lower]) / span;
        Some((high_hr - low_hr).mul_add(fraction, low_hr))
    }

    /// Effort at which the measured stages first cross a lactate level,
    /// by linear interpolation between the two bracketing stages. `None` when
    /// the first stage already sits at or above the level, or no stage
    /// reaches it.
    fn first_crossing(&self, level: f64) -> Option<f64> {
        let upper =
            (1..self.len()).find(|&i| self.lactates[i - 1] < level && self.lactates[i] >= level)?;
        let lower = upper - 1;
        let rise = self.lactates[upper] - self.lactates[lower];
        let fraction = (level - self.lactates[lower]) / rise;
        Some((self.efforts[upper] - self.efforts[lower]).mul_add(fraction, self.efforts[lower]))
    }

    fn point_at(&self, effort: f64, lactate_mmol: f64) -> LactateThresholdPoint {
        LactateThresholdPoint {
            intensity: self.unit.intensity(effort),
            lactate_mmol,
            heart_rate: self.heart_rate_at(effort),
        }
    }
}

/// A cubic in normalised effort with Horner evaluation.
#[derive(Debug, Clone, Copy)]
struct Cubic([f64; 4]);

impl Cubic {
    fn value(self, t: f64) -> f64 {
        let [c0, c1, c2, c3] = self.0;
        c3.mul_add(t, c2).mul_add(t, c1).mul_add(t, c0)
    }

    /// The `t` in `(from, to)` at which the curve lies farthest below the
    /// chord joining its own values at `from` and `to` — where the curve's
    /// derivative `3·c3·t² + 2·c2·t + c1` equals the chord's slope, solved
    /// analytically. `None` when no interior point lies below the chord.
    fn farthest_below_chord(self, from: f64, to: f64) -> Option<f64> {
        let chord_slope = (self.value(to) - self.value(from)) / (to - from);
        let [_, c1, c2, c3] = self.0;
        let candidates = solve_quadratic(3.0 * c3, 2.0 * c2, c1 - chord_slope);
        let below_chord = |t: f64| {
            let on_chord = chord_slope.mul_add(t - from, self.value(from));
            on_chord - self.value(t)
        };
        candidates
            .into_iter()
            .flatten()
            .filter(|&t| t > from && t < to)
            // A departure the meter could not have displayed is fit noise, not
            // a threshold; see [`DMAX_MIN_DEPARTURE_MMOL`].
            .filter(|&t| below_chord(t) >= DMAX_MIN_DEPARTURE_MMOL)
            .max_by(|&a, &b| below_chord(a).total_cmp(&below_chord(b)))
    }
}

/// Real roots of `a·t² + b·t + c = 0`, degrading to the linear root when the
/// quadratic term vanishes.
fn solve_quadratic(a: f64, b: f64, c: f64) -> [Option<f64>; 2] {
    if a.abs() < SINGULAR_PIVOT {
        if b.abs() < SINGULAR_PIVOT {
            return [None, None];
        }
        return [Some(-c / b), None];
    }
    let discriminant = b.mul_add(b, -4.0 * a * c);
    if discriminant < 0.0 {
        return [None, None];
    }
    let root = discriminant.sqrt();
    [Some((-b - root) / (2.0 * a)), Some((-b + root) / (2.0 * a))]
}

/// Least-squares cubic through `(t, y)` by the normal equations, solved with
/// partial pivoting. `None` when the system is singular.
fn fit_cubic(ts: &[f64], ys: &[f64]) -> Option<Cubic> {
    let mut power_sums = [0.0_f64; 7];
    let mut moment_sums = [0.0_f64; 4];
    for (&t, &y) in ts.iter().zip(ys) {
        let mut power = 1.0;
        for (k, sum) in power_sums.iter_mut().enumerate() {
            *sum += power;
            if k < 4 {
                moment_sums[k] = power.mul_add(y, moment_sums[k]);
            }
            power *= t;
        }
    }
    let mut matrix = [[0.0_f64; 5]; 4];
    for (row, matrix_row) in matrix.iter_mut().enumerate() {
        for (column, cell) in matrix_row.iter_mut().take(4).enumerate() {
            *cell = power_sums[row + column];
        }
        matrix_row[4] = moment_sums[row];
    }
    solve_augmented(&mut matrix).map(Cubic)
}

/// Gaussian elimination with partial pivoting on a 4×5 augmented matrix.
fn solve_augmented(matrix: &mut [[f64; 5]; 4]) -> Option<[f64; 4]> {
    for pivot_index in 0..4 {
        let pivot_row = (pivot_index..4).max_by(|&a, &b| {
            matrix[a][pivot_index]
                .abs()
                .total_cmp(&matrix[b][pivot_index].abs())
        })?;
        if matrix[pivot_row][pivot_index].abs() < SINGULAR_PIVOT {
            return None;
        }
        matrix.swap(pivot_index, pivot_row);
        let pivot = matrix[pivot_index];
        for row in matrix.iter_mut().skip(pivot_index + 1) {
            let factor = row[pivot_index] / pivot[pivot_index];
            for (cell, &pivot_cell) in row.iter_mut().zip(pivot.iter()).skip(pivot_index) {
                *cell = (-factor).mul_add(pivot_cell, *cell);
            }
        }
    }
    let mut solution = [0.0_f64; 4];
    for row in (0..4).rev() {
        let mut residual = matrix[row][4];
        for column in (row + 1)..4 {
            residual = (-matrix[row][column]).mul_add(solution[column], residual);
        }
        solution[row] = residual / matrix[row][row];
    }
    Some(solution)
}

/// Share of variance explained by the fitted values; 0.0 when the
/// observations carry no variance to explain.
fn r_squared(observed: &[f64], fitted: impl Fn(usize) -> f64) -> f64 {
    let mean = observed.iter().sum::<f64>() / observed.len() as f64;
    let total: f64 = observed.iter().map(|y| (y - mean).powi(2)).sum();
    let residual: f64 = observed
        .iter()
        .enumerate()
        .map(|(i, y)| (y - fitted(i)).powi(2))
        .sum();
    if total > 0.0 {
        1.0 - residual / total
    } else {
        0.0
    }
}

/// Ordinary least-squares line through the points, as `(slope, intercept,
/// sum of squared residuals)`. `None` when every x is the same.
fn fit_line(xs: &[f64], ys: &[f64]) -> Option<(f64, f64, f64)> {
    let n = xs.len() as f64;
    let mean_x = xs.iter().sum::<f64>() / n;
    let mean_y = ys.iter().sum::<f64>() / n;
    let sxx: f64 = xs.iter().map(|x| (x - mean_x).powi(2)).sum();
    if sxx < SINGULAR_PIVOT {
        return None;
    }
    let sxy: f64 = xs
        .iter()
        .zip(ys)
        .map(|(x, y)| (x - mean_x) * (y - mean_y))
        .sum();
    let slope = sxy / sxx;
    let intercept = (-slope).mul_add(mean_x, mean_y);
    let sse: f64 = xs
        .iter()
        .zip(ys)
        .map(|(x, y)| (y - slope.mul_add(*x, intercept)).powi(2))
        .sum();
    Some((slope, intercept, sse))
}

impl LactateStepTest {
    /// Locate LT1 and LT2 by every construct the stages support, fit the cubic
    /// the Dmax constructions run on, and interpolate the mmol band table.
    ///
    /// # Errors
    ///
    /// - [`IntelligenceError::InsufficientData`] with fewer than
    ///   [`MIN_STAGES`] stages.
    /// - [`IntelligenceError::InvalidInput`] with more than [`MAX_STAGES`]
    ///   stages, which no real protocol reaches and whose cost is quadratic.
    /// - [`IntelligenceError::ValueOutOfRange`] for an intensity, lactate or
    ///   heart rate outside what a human and a portable meter produce.
    /// - [`IntelligenceError::InvalidInput`] when a stage is not harder than the
    ///   one before it.
    /// - [`IntelligenceError::AlgorithmFailure`] when the cubic normal equations
    ///   are singular, which four or more strictly increasing stages do not
    ///   produce.
    pub fn analyze(&self) -> IntelligenceResult<LactateThresholds> {
        let series = EffortSeries::from_test(self)?;
        let ts: Vec<f64> = series
            .efforts
            .iter()
            .map(|&e| series.normalised(e))
            .collect();
        let cubic = fit_cubic(&ts, &series.lactates).ok_or_else(|| {
            IntelligenceError::algorithm_failure(
                "lactate_cubic_fit",
                "the normal equations for the cubic fit are singular",
            )
        })?;
        let curve = LactateCurveFit {
            coefficients: cubic.0,
            r_squared: r_squared(&series.lactates, |i| cubic.value(ts[i])),
        };
        Ok(LactateThresholds {
            unit: series.unit,
            stage_count: series.len(),
            lt1_log_log: log_log_breakpoint(&series),
            lt2_modified_dmax: modified_dmax(&series, cubic, &ts),
            lt2_dmax: dmax(&series, cubic, 0, &ts),
            lt2_obla_4mmol: obla(&series),
            curve,
            band_table: band_table(&series),
        })
    }
}

/// LT2 as the interpolated first crossing of [`OBLA_MMOL`].
fn obla(series: &EffortSeries) -> ThresholdOutcome {
    if series
        .lactates
        .first()
        .is_some_and(|&first| first >= OBLA_MMOL)
    {
        return ThresholdOutcome::NotDeterminable {
            reason: format!(
                "the first stage already measured {:.1} mmol/L, at or above the {OBLA_MMOL:.1} mmol/L convention",
                series.lactates[0]
            ),
        };
    }
    series.first_crossing(OBLA_MMOL).map_or_else(
        || {
            let peak = series.lactates.iter().copied().fold(f64::MIN, f64::max);
            ThresholdOutcome::NotDeterminable {
                reason: format!(
                    "lactate peaked at {peak:.1} mmol/L, below the {OBLA_MMOL:.1} mmol/L convention"
                ),
            }
        },
        |effort| ThresholdOutcome::Determined(series.point_at(effort, OBLA_MMOL)),
    )
}

/// LT2 as the point on the cubic farthest below the chord from the stage at
/// `from_index` to the last stage.
fn dmax(series: &EffortSeries, cubic: Cubic, from_index: usize, ts: &[f64]) -> ThresholdOutcome {
    let last = series.len() - 1;
    if from_index + 2 > last {
        return ThresholdOutcome::NotDeterminable {
            reason: format!(
                "only {} stages lie between the chord's start and the final stage; the construction needs an interior stage",
                last - from_index + 1
            ),
        };
    }
    cubic.farthest_below_chord(ts[from_index], ts[last]).map_or_else(
        || ThresholdOutcome::NotDeterminable {
            reason: format!(
                "the fitted curve never departs from its chord by the {DMAX_MIN_DEPARTURE_MMOL:.1} mmol/L a meter can display; lactate did not accelerate enough across the test to place this threshold"
            ),
        },
        |t| {
            let effort = series.denormalised(t);
            ThresholdOutcome::Determined(series.point_at(effort, cubic.value(t)))
        },
    )
}

/// LT2 by Dmax with the chord starting at the stage before the first rise
/// greater than [`MODIFIED_DMAX_RISE_MMOL`].
fn modified_dmax(series: &EffortSeries, cubic: Cubic, ts: &[f64]) -> ThresholdOutcome {
    let first_rise = (1..series.len()).find(|&i| {
        series.lactates[i] - series.lactates[i - 1] - MODIFIED_DMAX_RISE_MMOL > RISE_EPSILON_MMOL
    });
    first_rise.map_or_else(
        || ThresholdOutcome::NotDeterminable {
            reason: format!(
                "no stage-to-stage rise exceeded {MODIFIED_DMAX_RISE_MMOL:.1} mmol/L, so the chord has no start"
            ),
        },
        |rise_index| dmax(series, cubic, rise_index - 1, ts),
    )
}

/// LT1 as the intersection of the two regression lines that best split
/// ln(lactate) against ln(effort), with at least two stages on each side.
fn log_log_breakpoint(series: &EffortSeries) -> ThresholdOutcome {
    let xs: Vec<f64> = series.efforts.iter().map(|e| e.ln()).collect();
    let ys: Vec<f64> = series.lactates.iter().map(|l| l.ln()).collect();
    let best = (2..=series.len() - 2)
        .filter_map(|split| {
            let left = fit_line(&xs[..split], &ys[..split])?;
            let right = fit_line(&xs[split..], &ys[split..])?;
            Some((split, left, right, left.2 + right.2))
        })
        .min_by(|a, b| a.3.total_cmp(&b.3));
    let Some((split, (left_slope, left_intercept, _), (right_slope, right_intercept, _), _)) = best
    else {
        return ThresholdOutcome::NotDeterminable {
            reason: "the log-log segments could not be fitted".to_owned(),
        };
    };
    if right_slope <= left_slope {
        return ThresholdOutcome::NotDeterminable {
            reason: format!(
                "lactate does not rise more steeply after stage {split} than before it on the log-log axes"
            ),
        };
    }
    let break_x = (left_intercept - right_intercept) / (right_slope - left_slope);
    let (first_x, last_x) = (xs[0], xs[series.len() - 1]);
    if break_x < first_x || break_x > last_x {
        return ThresholdOutcome::NotDeterminable {
            reason: "the two log-log segments intersect outside the tested intensities".to_owned(),
        };
    }
    let effort = break_x.exp();
    let lactate = left_slope.mul_add(break_x, left_intercept).exp();
    ThresholdOutcome::Determined(series.point_at(effort, lactate))
}

/// The intensity and heart rate at each [`BAND_TABLE_MMOL`] level the stages
/// crossed.
fn band_table(series: &EffortSeries) -> Vec<LactateBandRow> {
    BAND_TABLE_MMOL
        .iter()
        .filter_map(|&level| {
            let effort = series.first_crossing(level)?;
            Some(LactateBandRow {
                lactate_mmol: level,
                intensity: series.unit.intensity(effort),
                heart_rate: series.heart_rate_at(effort),
            })
        })
        .collect()
}
