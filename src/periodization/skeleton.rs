// ABOUTME: Season skeleton — the phase sequence, taper, loading, drop order and strength column for one event class
// ABOUTME: Parses training_catalogue/skeletons/<id>.yaml and validates it against the python skeleton rules
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::vocab::{
    DaysRange, EventClass, FlavourFamily, HoursTier, LoadingPatterns, PhaseKind, Share,
    StrengthGoal, WorkoutPurpose,
};
use super::{
    check_ref_shapes, is_kebab_case, parse_error, unresolved_in, CatalogueError,
    CatalogueValidationError, Check, PurposeUse, UnresolvedReference, Violation,
};
use crate::models::SportType;

/// A season skeleton: how the weeks to a goal are cut into phases.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkeletonTemplate {
    /// Kebab-case identifier — the file stem.
    pub id: String,
    /// The goal events this skeleton serves.
    pub event_classes: Vec<EventClass>,
    /// The weekly-hours bands it is written for.
    pub hours_tiers: Vec<HoursTier>,
    /// Fewest weeks the skeleton can be compressed into.
    pub min_weeks: u8,
    /// The phases, in season order.
    pub phases: Vec<SkeletonPhase>,
    /// The taper rule; absent only on a no-race skeleton.
    #[serde(default)]
    pub taper: Option<TaperRule>,
    /// Loading pattern, typical and recovery-limited.
    pub loading_pattern: LoadingPatterns,
    /// How much a recovery week cuts.
    pub recovery_week_cut: Share,
    /// Which phases shrink first toward `min_weeks`; never taper or peak.
    #[serde(default)]
    pub drop_order: Vec<PhaseKind>,
    /// How a B race inside the season is handled.
    pub multi_peak: MultiPeakRule,
    /// The strength column per phase kind.
    #[serde(default)]
    pub strength: BTreeMap<PhaseKind, StrengthPhase>,
    /// The propositions behind it, as `evidence/sports_science/<category>/<slug>.md`.
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

/// One phase of a skeleton.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkeletonPhase {
    /// The phase kind.
    pub kind: PhaseKind,
    /// One English sentence for the coach model.
    pub purpose: String,
    /// How long the phase runs.
    pub length: PhaseLength,
    /// Weekly volume as a share of the season's peak.
    pub volume_share_of_peak: Share,
    /// A family the phase is trained under regardless of the season's flavour.
    #[serde(default)]
    pub flavour_override: Option<FlavourFamily>,
    /// The session purposes that define the phase.
    #[serde(default)]
    pub key_sessions: Vec<WorkoutPurpose>,
}

/// How long a phase runs — exactly one of three shapes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged, try_from = "PhaseLengthShape")]
pub enum PhaseLength {
    /// A fixed number of weeks.
    FixedWeeks {
        /// The weeks.
        fixed_weeks: u8,
    },
    /// A fixed number of days (a taper).
    FixedDays {
        /// The days.
        fixed_days: u8,
    },
    /// A share of the weeks to the goal, clamped.
    Share {
        /// The share of the weeks to the goal.
        share_of_weeks_to_goal: f32,
        /// Fewest weeks the phase gets.
        min_weeks: u8,
        /// Most weeks the phase gets.
        max_weeks: u8,
    },
}

/// The five keys a `length` mapping may carry, before the shape is decided.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PhaseLengthShape {
    fixed_weeks: Option<u8>,
    fixed_days: Option<u8>,
    share_of_weeks_to_goal: Option<f32>,
    min_weeks: Option<u8>,
    max_weeks: Option<u8>,
}

impl PhaseLengthShape {
    /// The keys present, sorted, for the rejection message.
    fn present_keys(&self) -> Vec<&'static str> {
        [
            ("fixed_days", self.fixed_days.is_some()),
            ("fixed_weeks", self.fixed_weeks.is_some()),
            ("max_weeks", self.max_weeks.is_some()),
            ("min_weeks", self.min_weeks.is_some()),
            (
                "share_of_weeks_to_goal",
                self.share_of_weeks_to_goal.is_some(),
            ),
        ]
        .into_iter()
        .filter_map(|(name, present)| present.then_some(name))
        .collect()
    }
}

impl TryFrom<PhaseLengthShape> for PhaseLength {
    type Error = String;

    /// python `check_phase_length`: the key set is exactly one of the
    /// three shapes, the offending keys named otherwise.
    fn try_from(shape: PhaseLengthShape) -> Result<Self, Self::Error> {
        match shape {
            PhaseLengthShape {
                fixed_weeks: Some(fixed_weeks),
                fixed_days: None,
                share_of_weeks_to_goal: None,
                min_weeks: None,
                max_weeks: None,
            } => Ok(Self::FixedWeeks { fixed_weeks }),
            PhaseLengthShape {
                fixed_weeks: None,
                fixed_days: Some(fixed_days),
                share_of_weeks_to_goal: None,
                min_weeks: None,
                max_weeks: None,
            } => Ok(Self::FixedDays { fixed_days }),
            PhaseLengthShape {
                fixed_weeks: None,
                fixed_days: None,
                share_of_weeks_to_goal: Some(share_of_weeks_to_goal),
                min_weeks: Some(min_weeks),
                max_weeks: Some(max_weeks),
            } => Ok(Self::Share {
                share_of_weeks_to_goal,
                min_weeks,
                max_weeks,
            }),
            other => Err(format!(
                "keys {:?} are not exactly one of {{fixed_weeks}}, {{fixed_days}}, {{share_of_weeks_to_goal, min_weeks, max_weeks}}",
                other.present_keys()
            )),
        }
    }
}

impl PhaseLength {
    /// python `check_phase_length` on the share shape: the share in
    /// `0..=1` and `min_weeks <= max_weeks`.
    fn check(&self, key: &str) -> Check {
        let Self::Share {
            share_of_weeks_to_goal,
            min_weeks,
            max_weeks,
        } = self
        else {
            return Ok(());
        };
        if !(0.0..=1.0).contains(share_of_weeks_to_goal) {
            return Err(Violation::new(
                format!("{key}.share_of_weeks_to_goal"),
                format!("share {share_of_weeks_to_goal} outside 0..=1"),
            ));
        }
        if min_weeks > max_weeks {
            return Err(Violation::new(
                key,
                format!("min_weeks {min_weeks} > max_weeks {max_weeks}"),
            ));
        }
        Ok(())
    }
}

/// The taper into the goal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaperRule {
    /// How long it runs.
    pub days: DaysRange,
    /// How much volume comes off.
    pub volume_cut: Share,
    /// Intensity is kept.
    pub keep_intensity: bool,
    /// Session count is kept.
    pub keep_frequency: bool,
}

impl TaperRule {
    /// python `check_skeleton` on `taper`: the day band ordered and the
    /// cut a valid share.
    fn check(&self) -> Check {
        self.days.check("taper.days")?;
        self.volume_cut.check("taper.volume_cut")
    }
}

/// How a B race inside the season is handled.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MultiPeakRule {
    /// The mini-taper before a B race.
    pub b_race_mini_taper_days: DaysRange,
    /// Transition weeks after an A race.
    pub transition_weeks_after_a_race: u8,
}

/// The strength work of one phase.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrengthPhase {
    /// What the strength work is for.
    pub goal: StrengthGoal,
    /// Sessions a week.
    pub sessions_per_week: u8,
    /// The strength purposes to draw from; all strength purposes.
    pub purposes: Vec<WorkoutPurpose>,
}

/// The carrier predicate of a skeleton walk.
///
/// The registry answers whether a workout of the purpose fits the phase
/// (`None` for any phase) and is written for the sport (`None` for any
/// sport) — see [`SkeletonTemplate::unresolved_purposes`].
pub type SkeletonCarrier<'a> =
    dyn Fn(Option<PhaseKind>, WorkoutPurpose, Option<&SportType>) -> bool + 'a;

/// Slack for the share sum, python's `1e-9` scaled to `f32` input.
const SHARE_SUM_TOLERANCE: f64 = 1e-6;

/// Days in a week, for the `fixed_days` contribution to the phase floor.
const DAYS_PER_WEEK: u32 = 7;

impl SkeletonPhase {
    /// The phase's own rules: its length shape and its volume share.
    fn check(&self, key: &str) -> Check {
        self.length.check(&format!("{key}.length"))?;
        self.volume_share_of_peak
            .check(&format!("{key}.volume_share_of_peak"))
    }
}

impl SkeletonTemplate {
    /// The shape name a parse error carries.
    const KIND: &'static str = "skeleton";

    /// Parse a skeleton YAML document and validate it.
    ///
    /// # Errors
    ///
    /// [`CatalogueError::Parse`] when the YAML does not deserialize;
    /// [`CatalogueError::Validation`] when a skeleton rule fails.
    pub fn from_yaml(text: &str) -> Result<Self, CatalogueError> {
        let skeleton: Self = serde_yaml::from_str(text).map_err(|e| parse_error(Self::KIND, e))?;
        skeleton.validate()?;
        Ok(skeleton)
    }

    /// The skeleton rules — python `check_skeleton`, one for one.
    ///
    /// # Errors
    ///
    /// The first broken rule, naming the id and the key.
    pub fn validate(&self) -> Result<(), CatalogueValidationError> {
        self.checks()
            .map_err(|violation| CatalogueValidationError::Skeleton {
                id: self.id.clone(),
                key: violation.key,
                message: violation.message,
            })
    }

    fn checks(&self) -> Check {
        // python check_skeleton: id is kebab-case (KEBAB_RE).
        if !is_kebab_case(&self.id) {
            return Err(Violation::new(
                "id",
                format!("{:?} is not kebab-case", self.id),
            ));
        }
        // python check_evidence: refs shaped. A skeleton carries no tier,
        // so an empty list is allowed.
        check_ref_shapes("", &self.evidence_refs)?;
        // python check_skeleton: phases is non-empty.
        if self.phases.is_empty() {
            return Err(Violation::new("phases", "no phases"));
        }
        for (i, phase) in self.phases.iter().enumerate() {
            phase.check(&format!("phases[{i}]"))?;
        }
        self.check_taper()?;
        self.check_length_budget()?;
        // python check_share on recovery_week_cut.
        self.recovery_week_cut.check("recovery_week_cut")?;
        self.check_drop_order()?;
        // python check_days_range on multi_peak.b_race_mini_taper_days.
        self.multi_peak
            .b_race_mini_taper_days
            .check("multi_peak.b_race_mini_taper_days")?;
        self.check_strength()
    }

    /// Positions of the taper phases.
    fn taper_positions(&self) -> Vec<usize> {
        self.phases
            .iter()
            .enumerate()
            .filter(|(_, phase)| phase.kind == PhaseKind::Taper)
            .map(|(i, _)| i)
            .collect()
    }

    /// python `check_skeleton` on the taper: at most one taper phase, last
    /// or followed only by race; phase and top-level rule present together;
    /// a taper unless the skeleton is the no-race one; the rule well formed.
    fn check_taper(&self) -> Check {
        let positions = self.taper_positions();
        if positions.len() > 1 {
            return Err(Violation::new(
                "phases",
                format!("more than one taper phase ({})", positions.len()),
            ));
        }
        for &position in &positions {
            let trailing: Vec<&str> = self
                .phases
                .iter()
                .skip(position + 1)
                .filter(|phase| phase.kind != PhaseKind::Race)
                .map(|phase| phase.kind.as_str())
                .collect();
            if !trailing.is_empty() {
                return Err(Violation::new(
                    format!("phases[{position}]"),
                    format!(
                        "taper must be last or followed only by race; followed by {trailing:?}"
                    ),
                ));
            }
        }
        match (positions.is_empty(), &self.taper) {
            (false, None) => {
                return Err(Violation::new(
                    "taper",
                    "a taper phase needs the top-level taper rule",
                ))
            }
            (true, Some(_)) => {
                return Err(Violation::new(
                    "taper",
                    "a taper rule without a taper phase",
                ))
            }
            _ => {}
        }
        if positions.is_empty() && self.event_classes != [EventClass::NoRace] {
            return Err(Violation::new(
                "phases",
                "no taper phase while event_classes is not [no_race]",
            ));
        }
        self.taper.as_ref().map_or(Ok(()), TaperRule::check)
    }

    /// python `check_skeleton` on lengths: the shares sum to at most 1.0,
    /// and `min_weeks` covers the fixed weeks, the share minimums and the
    /// fixed days rounded up to weeks.
    fn check_length_budget(&self) -> Check {
        let mut fixed_weeks = 0u32;
        let mut fixed_days = 0u32;
        let mut share_min_weeks = 0u32;
        let mut share_sum = 0f64;
        for phase in &self.phases {
            match &phase.length {
                PhaseLength::FixedWeeks { fixed_weeks: weeks } => {
                    fixed_weeks += u32::from(*weeks);
                }
                PhaseLength::FixedDays { fixed_days: days } => fixed_days += u32::from(*days),
                PhaseLength::Share {
                    share_of_weeks_to_goal,
                    min_weeks,
                    ..
                } => {
                    share_sum += f64::from(*share_of_weeks_to_goal);
                    share_min_weeks += u32::from(*min_weeks);
                }
            }
        }
        if share_sum > 1.0 + SHARE_SUM_TOLERANCE {
            return Err(Violation::new(
                "phases",
                format!("share_of_weeks_to_goal sums to {share_sum:.2}, above 1.0"),
            ));
        }
        let floor = fixed_weeks + share_min_weeks + fixed_days.div_ceil(DAYS_PER_WEEK);
        if u32::from(self.min_weeks) < floor {
            return Err(Violation::new(
                "min_weeks",
                format!(
                    "{} below the phase floor {floor} (fixed weeks {fixed_weeks} + share min weeks {share_min_weeks} + fixed days {fixed_days} / 7 rounded up)",
                    self.min_weeks
                ),
            ));
        }
        Ok(())
    }

    /// python `check_skeleton` on `drop_order`: never taper or peak, and
    /// only phases the skeleton has.
    fn check_drop_order(&self) -> Check {
        for (i, kind) in self.drop_order.iter().enumerate() {
            let key = format!("drop_order[{i}]");
            if matches!(kind, PhaseKind::Taper | PhaseKind::Peak) {
                return Err(Violation::new(key, format!("{kind} is never dropped")));
            }
            if !self.phases.iter().any(|phase| phase.kind == *kind) {
                return Err(Violation::new(
                    key,
                    format!("the skeleton has no {kind} phase"),
                ));
            }
        }
        Ok(())
    }

    /// python `check_skeleton` on `strength`: every phase's purposes
    /// non-empty and all strength purposes.
    fn check_strength(&self) -> Check {
        for (phase, rule) in &self.strength {
            let key = format!("strength.{phase}.purposes");
            if rule.purposes.is_empty() {
                return Err(Violation::new(key, "empty; name the strength purposes"));
            }
            if let Some(stray) = rule.purposes.iter().find(|purpose| !purpose.is_strength()) {
                let allowed: BTreeSet<&str> = WorkoutPurpose::ALL
                    .iter()
                    .filter(|purpose| purpose.is_strength())
                    .map(|purpose| purpose.as_str())
                    .collect();
                let allowed: Vec<&str> = allowed.into_iter().collect();
                return Err(Violation::new(
                    key,
                    format!(
                        "{:?} is not a strength purpose; allowed: {}",
                        stray.as_str(),
                        allowed.join(", ")
                    ),
                ));
            }
        }
        Ok(())
    }

    /// Every place the skeleton names a purpose, with the key it sits under
    /// and the phase it is named for: a key session is named for its
    /// phase's kind, a strength-column entry for none (python walks the
    /// strength column through the phase-less `check_purposes`).
    fn purpose_uses(&self) -> impl Iterator<Item = PurposeUse> + '_ {
        let sessions = self.phases.iter().enumerate().flat_map(|(i, phase)| {
            phase
                .key_sessions
                .iter()
                .enumerate()
                .map(move |(j, purpose)| PurposeUse {
                    key: format!("phases[{i}].key_sessions[{j}]"),
                    phase: Some(phase.kind),
                    purpose: *purpose,
                })
        });
        let strength = self.strength.iter().flat_map(|(phase, rule)| {
            rule.purposes
                .iter()
                .enumerate()
                .map(move |(k, purpose)| PurposeUse {
                    key: format!("strength.{phase}.purposes[{k}]"),
                    phase: None,
                    purpose: *purpose,
                })
        });
        sessions.chain(strength)
    }

    /// The sport every key-session carrier must be written for: swim on
    /// the open-water skeleton, any sport otherwise (python `need_sport`).
    fn required_sport(&self) -> Option<SportType> {
        (self.event_classes == [EventClass::OpenWaterSwim]).then_some(SportType::Swim)
    }

    /// The carrier rule — python `check_carrier` on `key_sessions`,
    /// `check_purposes` on the strength column: every purpose
    /// `carried(phase, purpose, sport)` denies, keyed
    /// `phases[i].key_sessions[j]` with the phase's kind, or
    /// `strength.<phase>.purposes[k]` with no phase. `sport` is
    /// `Some(swim)` for a key session of an `[open_water_swim]` skeleton —
    /// its carrier must list swim as the primary sport or a variant — and
    /// `None` everywhere else. The registry answers with one
    /// [`WorkoutFilter`](super::WorkoutFilter) match over its bank.
    pub fn unresolved_purposes(&self, carried: &SkeletonCarrier<'_>) -> Vec<UnresolvedReference> {
        let owner = format!("skeleton '{}'", self.id);
        let required = self.required_sport();
        self.purpose_uses()
            .filter(|entry| {
                let sport = entry.phase.and(required.as_ref());
                !carried(entry.phase, entry.purpose, sport)
            })
            .map(|entry| entry.into_unresolved(&owner))
            .collect()
    }

    /// Every purpose the skeleton names — the flat view over
    /// [`Self::unresolved_purposes`]'s walk.
    #[must_use]
    pub fn purposes_used(&self) -> BTreeSet<WorkoutPurpose> {
        self.purpose_uses().map(|entry| entry.purpose).collect()
    }

    /// Every `evidence_refs` entry `exists(category, slug)` denies.
    pub fn unresolved_references(
        &self,
        exists: &dyn Fn(&str, &str) -> bool,
    ) -> Vec<UnresolvedReference> {
        unresolved_in(
            &format!("skeleton '{}'", self.id),
            "",
            &self.evidence_refs,
            exists,
        )
    }
}
