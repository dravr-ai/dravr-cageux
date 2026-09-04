// ABOUTME: Flavour — a training-intensity-distribution model as data: TID targets, hard-session caps, session mix, readiness ladder
// ABOUTME: Parses training_catalogue/flavours/<id>.yaml and validates it against the python flavour rules
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::vocab::{
    Contraindication, EvidenceTier, FlavourFamily, LoadingPatterns, Measurement, Modifier,
    PhaseKind, ReadinessLevel, Sequencing, TidTarget, WorkoutPurpose,
};
use super::{
    check_caveat, check_citation, check_ref_shapes, is_kebab_case, parse_error, unresolved_in,
    CatalogueError, CatalogueValidationError, Check, PurposeUse, UnresolvedReference, Violation,
};

/// A flavour: the intensity distribution, caps and substitutions a season
/// is trained under.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Flavour {
    /// Kebab-case identifier — the file stem, what skeletons and the
    /// selection table reference.
    pub id: String,
    /// The distribution family.
    pub family: FlavourFamily,
    /// How the phases are sequenced.
    pub sequencing: Sequencing,
    /// Modifiers laid over the flavour.
    #[serde(default)]
    pub modifiers: Vec<Modifier>,
    /// Strength of the evidence behind the flavour.
    pub evidence_tier: EvidenceTier,
    /// What the evidence does not cover — required at grey or coach judgement.
    #[serde(default)]
    pub caveat: Option<String>,
    /// TID target per phase kind; `base` and `build` are required.
    pub tid_targets: BTreeMap<PhaseKind, TidTarget>,
    /// The hard-session cap and its tiers.
    pub hard_sessions_per_week: HardSessionCap,
    /// Hours between two hard sessions.
    pub min_spacing_hours_between_hard: SpacingHours,
    /// Purpose weights per phase kind; `base` and `build` are required.
    pub session_mix: BTreeMap<PhaseKind, BTreeMap<WorkoutPurpose, u8>>,
    /// What an athlete needs before this flavour fits.
    pub prerequisites: FlavourPrerequisites,
    /// Athlete states the flavour is not for.
    #[serde(default)]
    pub contraindications: Vec<Contraindication>,
    /// Loading pattern, typical and recovery-limited.
    pub loading_pattern: LoadingPatterns,
    /// The readiness ladder as data; every level present.
    pub readiness_substitution: BTreeMap<ReadinessLevel, ReadinessRule>,
    /// A cap on how long the flavour may run, for capped flavours.
    #[serde(default)]
    pub max_weeks: Option<u8>,
    /// The propositions behind it, as `evidence/sports_science/<category>/<slug>.md`.
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

/// The hard-session cap: a base cap for a low session count, and tiers
/// that raise it as the athlete trains more often.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HardSessionCap {
    /// Fewest hard sessions the flavour works with.
    pub min: u8,
    /// The base cap.
    pub max: u8,
    /// The cap for a recovery-limited athlete.
    pub recovery_limited_max: u8,
    /// Higher caps unlocked by session count, ascending in `from_sessions`.
    #[serde(default)]
    pub max_by_sessions_per_week: Vec<SessionsTierCap>,
}

/// One tier of the hard-session cap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionsTierCap {
    /// The weekly session count from which this cap applies.
    pub from_sessions: u8,
    /// The cap at that count; above the base cap.
    pub max: u8,
}

/// Hours between two hard sessions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpacingHours {
    /// For most athletes.
    pub default: u16,
    /// For a recovery-limited athlete.
    pub recovery_limited: u16,
}

/// What an athlete needs before a flavour fits.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlavourPrerequisites {
    /// Weekly training hours.
    pub min_hours_per_week: f32,
    /// Weekly sessions.
    pub min_sessions_per_week: u8,
    /// Measurement the flavour is steered by: outer any-of, inner all-of
    /// (`[[lactate], [power, hr]]`).
    pub measurement: Vec<Vec<Measurement>>,
    /// Years of structured training.
    pub min_training_age_years: f32,
}

/// What a readiness level allows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadinessRule {
    /// The purposes that may be scheduled at this level.
    pub purposes: Vec<WorkoutPurpose>,
    /// Hard sessions allowed in the week.
    pub max_hard_sessions_per_week: u8,
}

/// python `check_flavour`: `base` and `build` are present in a per-phase map.
fn check_required_phases<V>(key: &str, map: &BTreeMap<PhaseKind, V>) -> Check {
    [PhaseKind::Base, PhaseKind::Build]
        .into_iter()
        .find(|phase| !map.contains_key(phase))
        .map_or(Ok(()), |phase| {
            Err(Violation::new(
                format!("{key}.{phase}"),
                "missing; base and build are required",
            ))
        })
}

impl HardSessionCap {
    const KEY: &'static str = "hard_sessions_per_week";

    /// python `check_flavour` on `hard_sessions_per_week`: `min <= max`,
    /// the recovery-limited cap at or below it, tiers ascending and each
    /// above the base cap.
    fn check(&self) -> Check {
        if self.min > self.max {
            return Err(Violation::new(
                Self::KEY,
                format!("min {} > max {}", self.min, self.max),
            ));
        }
        if self.recovery_limited_max > self.max {
            return Err(Violation::new(
                format!("{}.recovery_limited_max", Self::KEY),
                format!("{} > max {}", self.recovery_limited_max, self.max),
            ));
        }
        let mut previous_from: Option<u8> = None;
        for (i, tier) in self.max_by_sessions_per_week.iter().enumerate() {
            let key = format!("{}.max_by_sessions_per_week[{i}]", Self::KEY);
            if let Some(previous) = previous_from.filter(|previous| tier.from_sessions <= *previous)
            {
                return Err(Violation::new(
                    format!("{key}.from_sessions"),
                    format!(
                        "{} not above the previous tier's {previous}; tiers must ascend",
                        tier.from_sessions
                    ),
                ));
            }
            previous_from = Some(tier.from_sessions);
            if tier.max <= self.max {
                return Err(Violation::new(
                    format!("{key}.max"),
                    format!("{} not above the base max {}", tier.max, self.max),
                ));
            }
        }
        Ok(())
    }
}

impl FlavourPrerequisites {
    /// python `check_flavour` on `prerequisites.measurement`: the outer
    /// any-of list and every inner all-of list are non-empty.
    fn check(&self) -> Check {
        if self.measurement.is_empty() {
            return Err(Violation::new(
                "prerequisites.measurement",
                "outer any-of list is empty",
            ));
        }
        self.measurement
            .iter()
            .position(Vec::is_empty)
            .map_or(Ok(()), |i| {
                Err(Violation::new(
                    format!("prerequisites.measurement[{i}]"),
                    "inner all-of list is empty",
                ))
            })
    }
}

impl Flavour {
    /// The shape name a parse error carries.
    const KIND: &'static str = "flavour";

    /// Parse a flavour YAML document and validate it.
    ///
    /// # Errors
    ///
    /// [`CatalogueError::Parse`] when the YAML does not deserialize;
    /// [`CatalogueError::Validation`] when a flavour rule fails.
    pub fn from_yaml(text: &str) -> Result<Self, CatalogueError> {
        let flavour: Self = serde_yaml::from_str(text).map_err(|e| parse_error(Self::KIND, e))?;
        flavour.validate()?;
        Ok(flavour)
    }

    /// The flavour rules — python `check_flavour`, one for one.
    ///
    /// # Errors
    ///
    /// The first broken rule, naming the id and the key.
    pub fn validate(&self) -> Result<(), CatalogueValidationError> {
        self.checks()
            .map_err(|violation| CatalogueValidationError::Flavour {
                id: self.id.clone(),
                key: violation.key,
                message: violation.message,
            })
    }

    fn checks(&self) -> Check {
        // python check_flavour: id is kebab-case (KEBAB_RE).
        if !is_kebab_case(&self.id) {
            return Err(Violation::new(
                "id",
                format!("{:?} is not kebab-case", self.id),
            ));
        }
        // python check_evidence: refs shaped, cited above grey, caveat at grey.
        check_ref_shapes("", &self.evidence_refs)?;
        check_citation("", &self.evidence_refs, self.evidence_tier, "evidence_tier")?;
        check_caveat(self.evidence_tier, self.caveat.as_deref())?;
        // python check_flavour: tid_targets carries base and build, each a
        // TID target whose shares sum sensibly (check_tid_target).
        check_required_phases("tid_targets", &self.tid_targets)?;
        for (phase, target) in &self.tid_targets {
            target.check(&format!("tid_targets.{phase}"))?;
        }
        self.hard_sessions_per_week.check()?;
        // python check_flavour: session_mix carries base and build.
        check_required_phases("session_mix", &self.session_mix)?;
        self.prerequisites.check()?;
        self.check_ladder()
    }

    /// python `check_flavour` on `readiness_substitution`: every level
    /// present, p0 naming no quality purpose (p0 is the block level, and a
    /// quality session is what it blocks — `NON_QUALITY_PURPOSES` in
    /// check-training-catalogue.py is the python side of
    /// [`WorkoutPurpose::is_quality`]), each level's purposes a superset of
    /// the level below, and the hard-session cap never decreasing with level.
    fn check_ladder(&self) -> Check {
        let mut below: Option<&ReadinessRule> = None;
        for level in ReadinessLevel::ALL {
            let key = format!("readiness_substitution.{level}");
            let rule = self
                .readiness_substitution
                .get(level)
                .ok_or_else(|| Violation::new(&key, "missing readiness level"))?;
            if *level == ReadinessLevel::P0 {
                let quality = rule
                    .purposes
                    .iter()
                    .position(|purpose| purpose.is_quality());
                if let Some(i) = quality {
                    return Err(Violation::new(
                        format!("{key}.purposes[{i}]"),
                        format!(
                            "quality purpose {:?} is not allowed at p0",
                            rule.purposes[i].as_str()
                        ),
                    ));
                }
            }
            if let Some(below) = below {
                let missing: BTreeSet<&str> = below
                    .purposes
                    .iter()
                    .filter(|purpose| !rule.purposes.contains(purpose))
                    .map(|purpose| purpose.as_str())
                    .collect();
                if !missing.is_empty() {
                    let missing: Vec<&str> = missing.into_iter().collect();
                    return Err(Violation::new(
                        format!("{key}.purposes"),
                        format!(
                            "not a superset of the level below; missing {}",
                            missing.join(", ")
                        ),
                    ));
                }
                if rule.max_hard_sessions_per_week < below.max_hard_sessions_per_week {
                    return Err(Violation::new(
                        format!("{key}.max_hard_sessions_per_week"),
                        format!(
                            "{} decreases from {} at the level below",
                            rule.max_hard_sessions_per_week, below.max_hard_sessions_per_week
                        ),
                    ));
                }
            }
            below = Some(rule);
        }
        Ok(())
    }

    /// Every place the flavour names a purpose, with the key it sits under
    /// and the phase it is named for: a session-mix entry is named for its
    /// phase, a ladder entry for none (it needs a carrier somewhere in the
    /// bank, not in one phase).
    fn purpose_uses(&self) -> impl Iterator<Item = PurposeUse> + '_ {
        let mix = self.session_mix.iter().flat_map(|(phase, weights)| {
            weights.keys().map(move |purpose| PurposeUse {
                key: format!("session_mix.{phase}.{purpose}"),
                phase: Some(*phase),
                purpose: *purpose,
            })
        });
        let ladder = self
            .readiness_substitution
            .iter()
            .flat_map(|(level, rule)| {
                rule.purposes
                    .iter()
                    .enumerate()
                    .map(move |(i, purpose)| PurposeUse {
                        key: format!("readiness_substitution.{level}.purposes[{i}]"),
                        phase: None,
                        purpose: *purpose,
                    })
            });
        mix.chain(ladder)
    }

    /// The carrier rule — python `check_carrier` on `session_mix`,
    /// `check_purposes` on the ladder: every purpose `carried(phase,
    /// purpose)` denies, keyed `session_mix.<phase>.<purpose>` with the phase
    /// it is named for, or `readiness_substitution.<level>.purposes[i]` with
    /// no phase. The registry answers with whether a workout of that purpose
    /// fits the phase (`fit.phases` empty or containing it) — one
    /// [`WorkoutFilter`](super::WorkoutFilter) match over its bank.
    pub fn unresolved_purposes(
        &self,
        carried: &dyn Fn(Option<PhaseKind>, WorkoutPurpose) -> bool,
    ) -> Vec<UnresolvedReference> {
        let owner = format!("flavour '{}'", self.id);
        self.purpose_uses()
            .filter(|entry| !carried(entry.phase, entry.purpose))
            .map(|entry| entry.into_unresolved(&owner))
            .collect()
    }

    /// Every purpose the flavour names — the flat view over
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
            &format!("flavour '{}'", self.id),
            "",
            &self.evidence_refs,
            exists,
        )
    }
}
