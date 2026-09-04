// ABOUTME: The closed intensity grammar — a coach's zone label resolved to a zone, a heart-rate zone, sweet spot or a percent band
// ABOUTME: Kept relative so a provider resolves it against the athlete's own zones and an FTP retest never invalidates a pushed session
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

use serde::{Deserialize, Serialize};

/// An intensity label relative to the athlete's thresholds.
///
/// This is the closed grammar Dravr understands, in the coach's own
/// vocabulary, when it turns a step's `target_zone` (or a plan day's
/// `intensity`) into a calendar target.
///
/// Kept relative on purpose: a provider resolves `Zone(2)` against the
/// athlete's own zones, so an FTP retest never invalidates a pushed session.
/// A label outside this grammar is not an error; it is prose, and the entry
/// goes out timed and un-targeted rather than with a target the coach never
/// stated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelativeIntensity {
    /// Training zone 1–7 in the sport's primary target family.
    Zone(u8),
    /// Training zone 1–7, explicitly on heart rate.
    HeartRateZone(u8),
    /// Sweet spot, the 88–94 % FTP band.
    SweetSpot,
    /// A percentage band of threshold; `high == low` for a single value.
    Percent {
        /// Lower bound, percent of threshold.
        low: u16,
        /// Upper bound, percent of threshold.
        high: u16,
    },
}

impl RelativeIntensity {
    /// Highest zone number the grammar admits.
    const MAX_ZONE: u8 = 7;
    /// Highest percentage the grammar admits (sprint work sits above 200 %).
    const MAX_PERCENT: u16 = 300;

    /// Parse a coach-vocabulary label: `Z2`, `zone 4`, `Z2 HR`, `Z2 pace`,
    /// `tempo`, `threshold`, `VO2max`, `sweet spot`, `75%`, `88-93% FTP`. Case
    /// and surrounding whitespace do not matter, and an en dash between the
    /// bounds of a band reads as the hyphen. Returns `None` for anything else.
    #[must_use]
    pub fn parse(label: &str) -> Option<Self> {
        let lowered = label.trim().to_lowercase().replace('\u{2013}', "-");
        let label = lowered.strip_prefix("zone ").unwrap_or(&lowered).trim();
        if let Some(rest) = label.strip_suffix(" hr") {
            return Self::zone_number(rest.trim()).map(Self::HeartRateZone);
        }
        // "Z2 pace" names the family the sport already decides, so the
        // suffix is dropped and the zone or band stays relative.
        let label = label.strip_suffix(" pace").unwrap_or(label).trim();
        if matches!(label, "sweet spot" | "sweetspot" | "sweet-spot") {
            return Some(Self::SweetSpot);
        }
        if let Some(zone) = Self::zone_number(label) {
            return Some(Self::Zone(zone));
        }
        Self::percent_band(label)
    }

    /// `z2` / `2` after a `zone ` prefix, or a named zone, → its number.
    fn zone_number(label: &str) -> Option<u8> {
        if let Some(digits) = label.strip_prefix('z') {
            let n: u8 = digits.trim().parse().ok()?;
            return (1..=Self::MAX_ZONE).contains(&n).then_some(n);
        }
        if let Ok(n) = label.parse::<u8>() {
            return (1..=Self::MAX_ZONE).contains(&n).then_some(n);
        }
        match label {
            "recovery" | "active recovery" => Some(1),
            "endurance" | "aerobic" => Some(2),
            "tempo" => Some(3),
            "threshold" | "lactate threshold" | "ftp" => Some(4),
            "vo2max" | "vo2 max" | "vo2" => Some(5),
            "anaerobic" | "anaerobic capacity" => Some(6),
            "neuromuscular" | "sprint" => Some(7),
            _ => None,
        }
    }

    /// `NN%` / `NN-MM%` / `NN-MM% FTP` → the band; anything else `None`.
    fn percent_band(label: &str) -> Option<Self> {
        let body = label
            .strip_suffix(" ftp")
            .or_else(|| label.strip_suffix("ftp"))
            .unwrap_or(label)
            .trim()
            .strip_suffix('%')?
            .trim();
        let (low, high) = match body.split_once('-') {
            Some((low, high)) => (low.trim(), Some(high.trim())),
            None => (body, None),
        };
        let low: u16 = low.parse().ok()?;
        if !(1..=Self::MAX_PERCENT).contains(&low) {
            return None;
        }
        let high = match high {
            Some(high) => high.parse::<u16>().ok()?,
            None => low,
        };
        (low..=Self::MAX_PERCENT)
            .contains(&high)
            .then_some(Self::Percent { low, high })
    }
}
