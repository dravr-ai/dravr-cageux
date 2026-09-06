// ABOUTME: Laying a skeleton onto a calendar — backward from the A race, shrinking base first, never the taper
// ABOUTME: Every length rule the catalogue leaves open is decided here, once, and named in the doc comment
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

//! Turning a season skeleton into dated phases.
//!
//! The skeleton says what a season is made of; this says where the pieces
//! fall for one athlete with one calendar. It is laid **backward from the A
//! race**, because the taper is the only phase whose position is fixed by the
//! goal rather than by what precedes it.
//!
//! The catalogue leaves several length questions open, and guessing differently
//! in two places is how a plan stops adding up. Each is decided here and only
//! here:
//!
//! - A `Share` length multiplies the **whole** runway, not what is left after
//!   the phases already placed, because the shares in a skeleton are authored
//!   to sum to roughly one across the season.
//! - `FixedDays` becomes whole weeks, rounded up. A plan is written in weeks;
//!   a taper of ten days occupies two of them.
//! - When a phase carries its own length and the skeleton also states a
//!   `taper.days` band, the phase's length wins and is then clamped into the
//!   band. The band is the guard, not the source.
//! - The runway floor is the **greater** of `min_weeks` and the sum of every
//!   phase's own minimum. Both are real, and the larger binds.
//! - Shrinking walks `drop_order` in order, one week per visit, cycling until
//!   the season fits. Taper and peak are never shrunk, which is stricter than
//!   the brief and matches what the catalogue validator already enforces.
//!
//! B races are deliberately not an input. The skeleton's `b_race_mini_taper_days`
//! leaves too much unstated — which phase it cuts into, whether its days are
//! borrowed or added — so this takes the A races only, and a caller with a B
//! race cannot silently have it ignored.

use chrono::{Duration, NaiveDate};
use serde::{Deserialize, Serialize};

use super::skeleton::{PhaseLength, SkeletonPhase, SkeletonTemplate};
use super::vocab::{FlavourFamily, LoadingPattern, PhaseKind, Share, WorkoutPurpose};

/// Days in a training week.
const DAYS_PER_WEEK: i64 = 7;

/// One phase, placed on the calendar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LaidPhase {
    /// The phase kind.
    pub kind: PhaseKind,
    /// The Monday-agnostic start date: the day the phase begins.
    pub start: NaiveDate,
    /// Whole weeks.
    pub weeks: u8,
    /// The coach-facing sentence, from the skeleton.
    pub purpose: String,
    /// Weekly volume as a share of the season's peak.
    pub volume_share_of_peak: Share,
    /// A family this phase is trained under regardless of the season flavour.
    pub flavour_override: Option<FlavourFamily>,
    /// The session purposes that define the phase.
    pub key_sessions: Vec<WorkoutPurpose>,
    /// The load-to-recovery pattern this phase runs on.
    pub loading_pattern: LoadingPattern,
    /// The peak this phase is building toward.
    pub peak: NaiveDate,
}

impl LaidPhase {
    /// The day after the phase ends.
    #[must_use]
    pub fn end_exclusive(&self) -> NaiveDate {
        self.start + Duration::days(i64::from(self.weeks) * DAYS_PER_WEEK)
    }
}

/// What the layout could do with the runway it was given.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SeasonLayout {
    /// The season fits, with the phases in calendar order.
    Laid {
        /// The phases, earliest first.
        phases: Vec<LaidPhase>,
        /// Phase kinds shrunk below their authored length to make it fit.
        shrunk: Vec<PhaseKind>,
    },
    /// The runway is shorter than the skeleton's own floor, so nothing is laid.
    ///
    /// Compressing everything is the wrong answer to too little time: the
    /// verdict says so and lets the coach move the goal or pick another
    /// skeleton.
    NotEnoughRunway {
        /// Weeks the skeleton needs at its most compressed.
        needs_weeks: u8,
        /// Weeks actually available to the first peak.
        has_weeks: u32,
    },
}

/// Lay `skeleton` onto the calendar for an athlete whose season starts `today`.
///
/// `peaks` are the A races in date order; the first is what the season is laid
/// backward from. Each later peak gets a transition after the one before it,
/// then the phases run again toward it.
///
/// `recovery_limited` selects the loading arm, which is the only thing the
/// athlete's recovery speed changes about the shape of a season.
#[must_use]
pub fn build_skeleton(
    skeleton: &SkeletonTemplate,
    peaks: &[NaiveDate],
    today: NaiveDate,
    recovery_limited: bool,
) -> SeasonLayout {
    let Some(first_peak) = peaks.first().copied() else {
        return SeasonLayout::NotEnoughRunway {
            needs_weeks: skeleton.min_weeks,
            has_weeks: 0,
        };
    };
    let runway = weeks_between(today, first_peak);
    let floor = runway_floor(skeleton);
    if runway < u32::from(floor) {
        return SeasonLayout::NotEnoughRunway {
            needs_weeks: floor,
            has_weeks: runway,
        };
    }

    let loading = if recovery_limited {
        skeleton.loading_pattern.recovery_limited
    } else {
        skeleton.loading_pattern.default
    };

    let mut lengths = authored_lengths(skeleton, runway);
    let shrunk = shrink_to_fit(skeleton, &mut lengths, runway);
    grow_to_fill(skeleton, &mut lengths, runway);

    let mut phases = Vec::new();
    // Whatever the authored phases cannot hold becomes general preparation, so
    // the athlete has a plan from today rather than a wait. `PhaseKind::Prep`
    // is the vocabulary's own word for it; no SkeletonPhase describes one, so
    // its sentence is written here.
    let spent = total_weeks(&lengths);
    if runway > spent {
        let lead = u8::try_from(runway - spent).unwrap_or(u8::MAX);
        phases.push(LaidPhase {
            kind: PhaseKind::Prep,
            start: today,
            weeks: lead,
            purpose: "General preparation: consistent easy volume and movement quality, before the base begins.".to_owned(),
            volume_share_of_peak: Share { min: 0.30, max: 0.50 },
            flavour_override: None,
            key_sessions: vec![
                WorkoutPurpose::Endurance,
                WorkoutPurpose::Mobility,
                WorkoutPurpose::StrengthAa,
            ],
            loading_pattern: loading,
            peak: first_peak,
        });
    }
    let mut cursor = place_block(skeleton, &lengths, first_peak, loading, &mut phases);

    // Later peaks: a transition after the race just run, then the same block
    // again toward the next one. The transition has no SkeletonPhase to copy,
    // so its sentence is written here.
    for peak in peaks.iter().skip(1).copied() {
        let transition_weeks = skeleton.multi_peak.transition_weeks_after_a_race;
        if transition_weeks > 0 {
            phases.push(LaidPhase {
                kind: PhaseKind::Transition,
                start: cursor,
                weeks: transition_weeks,
                purpose: "Absorb the race just run: easy movement, no structure, before the next block begins.".to_owned(),
                volume_share_of_peak: Share { min: 0.30, max: 0.50 },
                flavour_override: None,
                key_sessions: vec![WorkoutPurpose::Recovery, WorkoutPurpose::Endurance],
                loading_pattern: loading,
                peak,
            });
            cursor += Duration::days(i64::from(transition_weeks) * DAYS_PER_WEEK);
        }
        let block_runway = weeks_between(cursor, peak);
        let mut lengths = authored_lengths(skeleton, block_runway);
        let _ = shrink_to_fit(skeleton, &mut lengths, block_runway);
        cursor = place_block(skeleton, &lengths, peak, loading, &mut phases);
    }

    SeasonLayout::Laid { phases, shrunk }
}

/// Place one skeleton's phases so the last of them ends on `peak`, appending
/// them in calendar order and returning the day after the block.
fn place_block(
    skeleton: &SkeletonTemplate,
    lengths: &[u8],
    peak: NaiveDate,
    loading: LoadingPattern,
    out: &mut Vec<LaidPhase>,
) -> NaiveDate {
    let total: i64 = lengths.iter().map(|w| i64::from(*w)).sum();
    let mut start = peak - Duration::days(total * DAYS_PER_WEEK);
    let block_start = out.len();
    for (phase, weeks) in skeleton.phases.iter().zip(lengths) {
        if *weeks == 0 {
            continue;
        }
        out.push(LaidPhase {
            kind: phase.kind,
            start,
            weeks: *weeks,
            purpose: phase.purpose.clone(),
            volume_share_of_peak: phase.volume_share_of_peak.clone(),
            flavour_override: phase.flavour_override,
            key_sessions: phase.key_sessions.clone(),
            loading_pattern: loading,
            peak,
        });
        start += Duration::days(i64::from(*weeks) * DAYS_PER_WEEK);
    }
    debug_assert!(out.len() >= block_start, "a block appends in order");
    start
}

/// Whole weeks from `from` to `to`, rounded down, never negative.
fn weeks_between(from: NaiveDate, to: NaiveDate) -> u32 {
    let days = (to - from).num_days();
    if days <= 0 {
        return 0;
    }
    u32::try_from(days / DAYS_PER_WEEK).unwrap_or(u32::MAX)
}

/// The length each phase is authored to take on this runway, before shrinking.
fn authored_lengths(skeleton: &SkeletonTemplate, runway: u32) -> Vec<u8> {
    skeleton
        .phases
        .iter()
        .map(|p| resolve_length(&p.length, runway))
        .collect()
}

/// One phase's authored length in whole weeks.
fn resolve_length(length: &PhaseLength, runway: u32) -> u8 {
    match length {
        PhaseLength::FixedWeeks { fixed_weeks } => *fixed_weeks,
        PhaseLength::FixedDays { fixed_days } => {
            let weeks = i64::from(*fixed_days).div_euclid(DAYS_PER_WEEK)
                + i64::from(i64::from(*fixed_days).rem_euclid(DAYS_PER_WEEK) > 0);
            u8::try_from(weeks).unwrap_or(u8::MAX)
        }
        PhaseLength::Share {
            share_of_weeks_to_goal,
            min_weeks,
            max_weeks,
        } => {
            #[allow(clippy::cast_precision_loss)]
            let raw = share_of_weeks_to_goal * runway as f32;
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let rounded = raw.round().max(0.0) as u32;
            let clamped = rounded.clamp(u32::from(*min_weeks), u32::from(*max_weeks));
            u8::try_from(clamped).unwrap_or(u8::MAX)
        }
    }
}

/// The most weeks a phase may be grown to.
///
/// Only a `Share` phase states a ceiling; a fixed phase is fixed, which is why
/// a taper written as fourteen days never absorbs spare runway.
fn phase_ceiling(phase: &SkeletonPhase, runway: u32) -> u8 {
    match &phase.length {
        PhaseLength::Share { max_weeks, .. } => *max_weeks,
        other => resolve_length(other, runway),
    }
}

/// Spend spare runway on the phases that can hold it, in season order.
///
/// The authored shares sum to less than a season — 0.85 in the spec skeleton,
/// plus a fixed taper — so a long runway leaves weeks over. Left alone those
/// weeks fall *before* the season starts, and the athlete waits: at a
/// fifty-two week runway the marathon skeleton fills thirty-four and strands
/// eighteen. For a feature whose whole subject is the annual plan that is the
/// wrong answer, so the remainder is spent forward, each phase up to its own
/// authored ceiling, earliest first.
fn grow_to_fill(skeleton: &SkeletonTemplate, lengths: &mut [u8], runway: u32) {
    let mut guard = 0_u32;
    while total_weeks(lengths) < runway {
        let mut moved = false;
        for (at, phase) in skeleton.phases.iter().enumerate() {
            if lengths[at] < phase_ceiling(phase, runway) {
                lengths[at] += 1;
                moved = true;
                if total_weeks(lengths) >= runway {
                    return;
                }
            }
        }
        guard += 1;
        if !moved || guard > u32::from(u8::MAX) {
            return;
        }
    }
}

/// The fewest weeks a phase may be shrunk to.
///
/// A `Share` phase states its own floor. A fixed phase has none, so its
/// authored length *is* its floor — a taper written as fourteen days is not a
/// suggestion.
fn phase_floor(phase: &SkeletonPhase, runway: u32) -> u8 {
    match &phase.length {
        PhaseLength::Share { min_weeks, .. } => *min_weeks,
        other => resolve_length(other, runway),
    }
}

/// The shortest season this skeleton can honestly describe: the greater of its
/// declared `min_weeks` and the sum of its phases' own floors.
fn runway_floor(skeleton: &SkeletonTemplate) -> u8 {
    let summed: u32 = skeleton
        .phases
        .iter()
        .map(|p| u32::from(phase_floor(p, u32::from(skeleton.min_weeks))))
        .sum();
    u8::try_from(summed.max(u32::from(skeleton.min_weeks))).unwrap_or(u8::MAX)
}

/// Shrink toward the runway, one week at a time, in `drop_order`.
///
/// Returns the kinds actually shortened. Taper and peak are never touched: the
/// taper is the phase the whole season is for, and the catalogue's validator
/// protects both, so shrinking them here would contradict the data.
fn shrink_to_fit(skeleton: &SkeletonTemplate, lengths: &mut [u8], runway: u32) -> Vec<PhaseKind> {
    let mut shrunk = Vec::new();
    let mut guard = 0_u32;
    while total_weeks(lengths) > runway {
        let mut moved = false;
        for kind in &skeleton.drop_order {
            if matches!(kind, PhaseKind::Taper | PhaseKind::Peak) {
                continue;
            }
            let Some(at) = skeleton.phases.iter().position(|p| p.kind == *kind) else {
                continue;
            };
            let floor = phase_floor(&skeleton.phases[at], runway);
            if lengths[at] > floor {
                lengths[at] -= 1;
                moved = true;
                if !shrunk.contains(kind) {
                    shrunk.push(*kind);
                }
                if total_weeks(lengths) <= runway {
                    return shrunk;
                }
            }
        }
        guard += 1;
        if !moved || guard > u32::from(u8::MAX) {
            break;
        }
    }
    shrunk
}

fn total_weeks(lengths: &[u8]) -> u32 {
    lengths.iter().map(|w| u32::from(*w)).sum()
}
