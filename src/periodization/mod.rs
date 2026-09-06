// ABOUTME: Periodization kernel — the training catalogue's file-level types, their parsing and their validation
// ABOUTME: Flavours, season skeletons, the selection table and workout templates; pure, no I/O, no registry
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

//! # Periodization kernel
//!
//! The training catalogue is four file shapes, each with one type here:
//!
//! | Shape | File | Type |
//! |---|---|---|
//! | flavour | `flavours/<id>.yaml` | [`Flavour`](crate::periodization::Flavour) |
//! | skeleton | `skeletons/<id>.yaml` | [`SkeletonTemplate`](crate::periodization::SkeletonTemplate) |
//! | selection table | `selection.yaml` | [`SelectionTable`](crate::periodization::SelectionTable) |
//! | workout template | `workouts/<slug>.toml` | [`WorkoutTemplate`](crate::periodization::WorkoutTemplate) |
//!
//! Every type parses from its text (`from_yaml` / `from_toml`), validates
//! itself with a pure `validate`, and reports the first violation as a
//! [`CatalogueValidationError`](crate::periodization::CatalogueValidationError)
//! that names the file's id and the offending key. The rule list is the one
//! `scripts/check-training-catalogue.py` in dravr-contremaitre enforces;
//! each Rust check carries a comment naming its python rule so the two
//! lists stay one list.
//!
//! Rules that need more than one file — a purpose no workout carries, a
//! selection id with no flavour file, an evidence ref with no proposition
//! behind it — are the registry's, but the walks are the kernel's: the
//! registry supplies a predicate and gets back every reference it denied,
//! keyed to the exact field. `unresolved_references` on every file-level
//! type covers evidence refs;
//! [`Flavour::unresolved_purposes`](crate::periodization::Flavour::unresolved_purposes)
//! and
//! [`SkeletonTemplate::unresolved_purposes`](crate::periodization::SkeletonTemplate::unresolved_purposes)
//! cover the phase-aware carrier rule;
//! [`SelectionTable::unresolved_flavours`](crate::periodization::SelectionTable::unresolved_flavours)
//! covers flavour ids.
//! [`Flavour::purposes_used`](crate::periodization::Flavour::purposes_used),
//! [`SkeletonTemplate::purposes_used`](crate::periodization::SkeletonTemplate::purposes_used)
//! and [`SelectionTable::flavour_ids`](crate::periodization::SelectionTable::flavour_ids)
//! are flat views over the same walks, and
//! [`evidence_ref_parts`](crate::periodization::evidence_ref_parts) splits a
//! ref the way a proposition store is keyed.
//!
//! The links above are written as `crate::periodization::…` on purpose: the
//! `///` on `pub mod periodization` in `lib.rs` concatenates with this
//! header, and a bare link resolves in that outer scope.

use std::fmt;

use thiserror::Error;

/// Flavour — a training-intensity-distribution model as data.
pub mod flavour;
/// The closed intensity grammar a coach's zone labels resolve through.
pub mod intensity;
/// Laying a season skeleton onto a calendar.
pub mod layout;
/// The profile-to-flavour rule that reads the selection table.
pub mod rule;
/// The profile-to-flavour selection table.
pub mod selection;
/// Whole-number deserializers for integer fields that arrive as `480.0`.
pub mod serde_num;
/// Season skeleton — phases, taper, loading and strength per event class.
pub mod skeleton;
/// Vocabularies (the closed enums) and the shared value shapes.
pub mod vocab;
/// Workout template — the structured session with its parameter ranges.
pub mod workout;

pub use flavour::{
    Flavour, FlavourPrerequisites, HardSessionCap, ReadinessRule, SessionsTierCap, SpacingHours,
};
pub use intensity::RelativeIntensity;
pub use layout::{build_skeleton, LaidPhase, SeasonLayout};
pub use rule::{
    select_flavour, Confidence, ExcludedFlavour, FlavourInputs, FlavourVerdict, Reason,
    ScoredFlavour,
};
pub use selection::{FlavourExclusion, FlavourWeight, SelectionRow, SelectionTable};
pub use skeleton::{
    MultiPeakRule, PhaseLength, SkeletonCarrier, SkeletonPhase, SkeletonTemplate, StrengthPhase,
    TaperRule,
};
pub use vocab::{
    Contraindication, DaysRange, EventClass, EvidenceTier, FlavourFamily, HoursTier, InjuryLoad,
    InputDimension, IntervalExperience, LoadingPattern, LoadingPatternParseError, LoadingPatterns,
    Measurement, Modifier, ParamRange, PhaseKind, ProgressionLever, ReadinessLevel, RecoverySpeed,
    RpeRange, SeasonPhase, Sequencing, Share, SportMix, StrengthGoal, TidTarget, TrainingAge,
    WeeksToGoal, WorkoutPurpose,
};
pub use workout::{
    FuelingProtocol, IntensityDistribution, PhaseFit, Progression, WorkoutFilter, WorkoutParams,
    WorkoutStep, WorkoutTargetZones, WorkoutTemplate,
};

/// Why a catalogue text did not become a value: it did not parse, or it
/// parsed and broke an invariant.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CatalogueError {
    /// The text is not a well-formed document of the shape asked for —
    /// bad YAML or TOML, a missing field, a word outside its vocabulary.
    #[error("{kind} parse failed: {message}")]
    Parse {
        /// The shape that was being parsed (`flavour`, `skeleton`,
        /// `selection table`, `workout template`).
        kind: &'static str,
        /// The parser's own message, which names the field path and value.
        message: String,
    },
    /// The document parsed but one of the catalogue invariants failed.
    #[error(transparent)]
    Validation(#[from] CatalogueValidationError),
}

/// One broken invariant, naming the file by its id or slug and the key at
/// fault. The message is the python checker's, word for word where the
/// rule has one.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CatalogueValidationError {
    /// A flavour file.
    #[error("flavour '{id}': {key}: {message}")]
    Flavour {
        /// The flavour's `id`.
        id: String,
        /// The offending key, dotted, with `[i]` for list positions.
        key: String,
        /// What the rule found.
        message: String,
    },
    /// A skeleton file.
    #[error("skeleton '{id}': {key}: {message}")]
    Skeleton {
        /// The skeleton's `id`.
        id: String,
        /// The offending key.
        key: String,
        /// What the rule found.
        message: String,
    },
    /// The selection table.
    #[error("selection table: {key}: {message}")]
    Selection {
        /// The offending key, starting at `rows[i]`.
        key: String,
        /// What the rule found.
        message: String,
    },
    /// A workout template file.
    #[error("workout '{slug}': {key}: {message}")]
    Workout {
        /// The template's `slug`.
        slug: String,
        /// The offending key.
        key: String,
        /// What the rule found.
        message: String,
    },
}

/// A reference one file makes that no other file answers for: an
/// `evidence_refs` entry with no proposition, a purpose no workout carries
/// for the phase it is named in, a selection id with no flavour file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedReference {
    /// The file that makes it, as its error display names it
    /// (`flavour 'polarized-classic'`, `selection table`).
    pub owner: String,
    /// The key the reference sits under (`evidence_refs[2]`,
    /// `session_mix.build.race_specific`, `rows[4].prefer[0].id`).
    pub key: String,
    /// The reference as written — a path, a purpose, a flavour id.
    pub reference: String,
}

impl fmt::Display for UnresolvedReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {}: reference does not resolve: {}",
            self.owner, self.key, self.reference
        )
    }
}

/// Split an evidence ref into the `(category, slug)` a proposition store is
/// keyed by.
///
/// Only `evidence/sports_science/<category>/<slug>.md` resolves — that is
/// the whole proposition corpus — and `README.md` never does, so both come
/// back `None`.
///
/// ```
/// use dravr_cageux::periodization::evidence_ref_parts;
///
/// assert_eq!(
///     evidence_ref_parts("evidence/sports_science/recovery/kellmann-2018-recovery.md"),
///     Some(("recovery", "kellmann-2018-recovery"))
/// );
/// assert_eq!(evidence_ref_parts("evidence/sports_science/recovery/README.md"), None);
/// assert_eq!(evidence_ref_parts("docs/recovery/kellmann-2018-recovery.md"), None);
/// ```
#[must_use]
pub fn evidence_ref_parts(path: &str) -> Option<(&str, &str)> {
    let rest = path.strip_prefix("evidence/sports_science/")?;
    let (category, file) = rest.split_once('/')?;
    let slug = file.strip_suffix(".md")?;
    let well_formed =
        !category.is_empty() && !slug.is_empty() && slug != "README" && !slug.contains('/');
    well_formed.then_some((category, slug))
}

/// A broken invariant before it is attributed to a file: the key and the
/// message. Each file-level type wraps it in its own
/// [`CatalogueValidationError`] variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Violation {
    pub(crate) key: String,
    pub(crate) message: String,
}

impl Violation {
    pub(crate) fn new(key: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            message: message.into(),
        }
    }
}

/// The outcome of one invariant check.
pub(crate) type Check = Result<(), Violation>;

/// One place a flavour or skeleton names a purpose: the key it sits under,
/// the phase it is named for (`None` when the entry needs a carrier
/// anywhere in the bank), and the purpose. The `unresolved_purposes` walks
/// yield these and hand the denied ones back as [`UnresolvedReference`]s.
pub(crate) struct PurposeUse {
    pub(crate) key: String,
    pub(crate) phase: Option<vocab::PhaseKind>,
    pub(crate) purpose: vocab::WorkoutPurpose,
}

impl PurposeUse {
    pub(crate) fn into_unresolved(self, owner: &str) -> UnresolvedReference {
        UnresolvedReference {
            owner: owner.to_owned(),
            key: self.key,
            reference: self.purpose.as_str().to_owned(),
        }
    }
}

/// Turn a serde parse failure into the kernel's error.
pub(crate) fn parse_error(kind: &'static str, error: impl fmt::Display) -> CatalogueError {
    CatalogueError::Parse {
        kind,
        message: error.to_string(),
    }
}

/// python `KEBAB_RE`: `^[a-z0-9]+(-[a-z0-9]+)*$`.
pub(crate) fn is_kebab_case(id: &str) -> bool {
    !id.is_empty()
        && id.split('-').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}

/// python `check_evidence`, the per-file half: every ref is shaped like a
/// proposition path (`evidence_refs[i]`).
pub(crate) fn check_ref_shapes(prefix: &str, refs: &[String]) -> Check {
    refs.iter()
        .enumerate()
        .find(|(_, reference)| evidence_ref_parts(reference).is_none())
        .map_or(Ok(()), |(i, reference)| {
            Err(Violation::new(
                format!("{prefix}evidence_refs[{i}]"),
                format!(
                    "{reference:?} is not an evidence/sports_science/<category>/<slug>.md path"
                ),
            ))
        })
}

/// python `check_evidence`, the tier half: a tier above grey needs a
/// citation (`evidence_refs`).
pub(crate) fn check_citation(
    prefix: &str,
    refs: &[String],
    tier: vocab::EvidenceTier,
    tier_key: &str,
) -> Check {
    if refs.is_empty() && tier.requires_citation() {
        return Err(Violation::new(
            format!("{prefix}evidence_refs"),
            format!("empty while {tier_key} is {tier}; cite a proposition or tier it grey/coach_judgement"),
        ));
    }
    Ok(())
}

/// python `check_evidence`, the caveat half, for the shapes that carry one:
/// grey or coach judgement needs a non-blank caveat.
pub(crate) fn check_caveat(tier: vocab::EvidenceTier, caveat: Option<&str>) -> Check {
    let stated = caveat.is_some_and(|text| !text.trim().is_empty());
    if !tier.requires_citation() && !stated {
        return Err(Violation::new(
            "caveat",
            format!("required when evidence_tier is {tier}"),
        ));
    }
    Ok(())
}

/// The cross-file half of python `check_evidence`: every ref the store
/// cannot answer for, in file order.
pub(crate) fn unresolved_in(
    owner: &str,
    prefix: &str,
    refs: &[String],
    exists: &dyn Fn(&str, &str) -> bool,
) -> Vec<UnresolvedReference> {
    refs.iter()
        .enumerate()
        .filter(|(_, reference)| {
            !evidence_ref_parts(reference).is_some_and(|(category, slug)| exists(category, slug))
        })
        .map(|(i, reference)| UnresolvedReference {
            owner: owner.to_owned(),
            key: format!("{prefix}evidence_refs[{i}]"),
            reference: reference.clone(),
        })
        .collect()
}
