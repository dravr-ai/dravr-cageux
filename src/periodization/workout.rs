// ABOUTME: Workout template — the structured session with its purpose, parameter ranges, progression levers and phase fit
// ABOUTME: Parses training_catalogue/workouts/<slug>.toml and validates it against the python workout rules plus the intensity grammar
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

use std::collections::HashMap;
use std::fmt;
use std::slice;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use toml::{Table, Value as TomlValue};
use uuid::Uuid;

use super::intensity::RelativeIntensity;
use super::serde_num::whole_u32;
use super::vocab::{
    vocab_enum, Contraindication, EvidenceTier, ParamRange, PhaseKind, ProgressionLever,
    ReadinessLevel, RpeRange, WorkoutPurpose,
};
use super::{
    check_caveat, check_citation, check_ref_shapes, parse_error, unresolved_in, CatalogueError,
    CatalogueValidationError, Check, UnresolvedReference, Violation,
};
use crate::models::SportType;

vocab_enum! {
    /// Intensity distribution model for a workout (Seiler-influenced).
    IntensityDistribution {
        /// Predominantly low intensity (>80 % Z1+Z2).
        Polarized => "polarized",
        /// Mostly threshold work (Z3 dominant).
        Threshold => "threshold",
        /// `VO2max`-dominant (Z4-Z5).
        Vo2max => "vo2max",
        /// Recovery (Z1 only).
        Recovery => "recovery",
        /// Mixed pyramid distribution.
        Pyramid => "pyramid",
    }
}

/// Per-session fuelling target carried by a planned day.
///
/// Mirrors `$defs.FuelingProtocol` in dravr-contremaitre's
/// `structured-workout.schema.json`, which the ultra and heat builder coaches
/// already emit on every long session.
///
/// One deliberate divergence from that schema: `sodium_mg_per_h` is optional
/// here. The schema requires it, which pushes a coach with no sweat estimate
/// into inventing one — and sodium is where an invented number does the most
/// harm. It is an estimated sweat *loss*, never a prescribed intake.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FuelingProtocol {
    /// Carbohydrate target in grams per hour.
    pub carbs_g_per_h: f32,
    /// Fluid target in millilitres per hour.
    pub fluid_ml_per_h: f32,
    /// Estimated sodium loss in milligrams per hour, when the athlete has a
    /// sweat measurement behind it. Absent means unknown, not zero.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sodium_mg_per_h: Option<f32>,
    /// Carbohydrate source when the rate depends on it — "glucose:fructose
    /// 1:0.8". A rate above 60 g/h is only reachable with multiple
    /// transportable carbohydrates, so this is what makes such a rate honest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub carb_source: Option<String>,
}

impl FuelingProtocol {
    /// One-line summary for a prompt, a calendar note or a plan card.
    ///
    /// Sodium is worded as a loss because that is what it is. Naming it an
    /// intake target would invert the evidence: hyponatremia is driven by
    /// fluid volume above sweat rate, and sodium supplementation does not
    /// prevent it.
    #[must_use]
    pub fn summary(&self) -> String {
        let mut parts = vec![
            format!("{:.0} g/h carbs", self.carbs_g_per_h),
            format!("{:.0} ml/h fluid", self.fluid_ml_per_h),
        ];
        if let Some(sodium) = self.sodium_mg_per_h {
            parts.push(format!("~{sodium:.0} mg/h sodium lost"));
        }
        if let Some(source) = &self.carb_source {
            parts.push(source.clone());
        }
        parts.join(" · ")
    }
}

/// Single step inside a structured workout (warmup, interval, recovery, cool-down).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkoutStep {
    /// Human-readable label ("Warm-up", "Interval", "Recovery", "Cool-down").
    pub label: String,
    /// Duration in seconds (use the lower bound for distance-based intervals).
    #[serde(deserialize_with = "whole_u32")]
    pub duration_seconds: u32,
    /// Optional distance in metres for distance-based intervals.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distance_meters: Option<f64>,
    /// Target zone label (`"Z1"`, `"Z2"`, `"Threshold"`, `"VO2max"`, `"Recovery"`).
    pub target_zone: String,
    /// Optional integer repeat count when this step is part of a set
    /// (e.g. `repeat = 4` for 4×8 min threshold). Defaults to 1.
    #[serde(default = "default_repeat", deserialize_with = "whole_u32")]
    pub repeat: u32,
    /// Optional free-form note for the coach to surface in the prescription.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

const fn default_repeat() -> u32 {
    1
}

impl WorkoutStep {
    /// Seconds a sequence of steps asks of the athlete: each step's duration
    /// times its repeat count, summed. `u64` so a payload past every bound
    /// still totals without wrapping; the callers bound it.
    #[must_use]
    pub fn total_seconds(steps: &[Self]) -> u64 {
        steps
            .iter()
            .map(|step| u64::from(step.duration_seconds) * u64::from(step.repeat))
            .sum()
    }
}

/// Per-template target zone overlay (applied on top of the user's own
/// `HrZoneSet` / `PowerZoneSet` when prescribing).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkoutTargetZones {
    /// Optional HR percentages (of LT2) for the workout's zones, in
    /// ascending order matching `Z1..Z5`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hr_pct_of_lt2: Option<[f64; 5]>,
    /// Optional power percentages (of FTP) for the workout's zones, in
    /// ascending order matching `Z1..Z5`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub power_pct_of_ftp: Option<[f64; 5]>,
}

/// The parameter ranges a coach fills in when instantiating a template.
///
/// Every field is optional so `{}` is the honest default of a user-authored
/// row; the catalogue rules decide which ones a purpose requires.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkoutParams {
    /// Sets, for strength and plyometric work.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sets: Option<ParamRange>,
    /// Reps per set, or interval count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reps: Option<ParamRange>,
    /// Ground contacts, for plyometrics.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contacts: Option<ParamRange>,
    /// Work interval length in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_seconds: Option<ParamRange>,
    /// Rest between work intervals in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rest_seconds: Option<ParamRange>,
    /// Whole-session duration in minutes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_minutes: Option<ParamRange>,
    /// Perceived exertion band — endurance purposes only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rpe: Option<RpeRange>,
    /// Load prescription in free text (`"85-90% 1RM"`) — strength purposes only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load: Option<String>,
    /// One intensity anchor per sport in play, each in the
    /// [`RelativeIntensity`] grammar. Keyed by a named sport only: a
    /// [`SportType::Other`] key serializes as a map, which JSON refuses as a
    /// map key, so [`WorkoutTemplate::validate_catalogue`] is the gate that
    /// keeps the JSON path total — it rejects an `Other` key before any row
    /// write.
    pub intensity: HashMap<SportType, String>,
    /// The anchor in the coach's own voice (`"~90% HRmax"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intensity_label: Option<String>,
}

impl WorkoutParams {
    /// The range fields, with the key each is reported under.
    fn ranges(&self) -> [(&'static str, Option<&ParamRange>); 6] {
        [
            ("sets", self.sets.as_ref()),
            ("reps", self.reps.as_ref()),
            ("contacts", self.contacts.as_ref()),
            ("work_seconds", self.work_seconds.as_ref()),
            ("rest_seconds", self.rest_seconds.as_ref()),
            ("duration_minutes", self.duration_minutes.as_ref()),
        ]
    }

    /// python `check_workout` on `params`: every range ordered, RPE in
    /// bounds, and (Rust only) no `Other` sport among the anchor keys.
    fn check(&self) -> Check {
        for (name, range) in self.ranges() {
            if let Some(range) = range {
                range.check(&format!("params.{name}"))?;
            }
        }
        if let Some(rpe) = &self.rpe {
            rpe.check("params.rpe")?;
        }
        let stray = self
            .intensity
            .keys()
            .filter_map(|sport| match sport {
                SportType::Other(name) => Some(name),
                _ => None,
            })
            .min();
        stray.map_or(Ok(()), |name| {
            Err(Violation::new(
                format!("params.intensity.{name}"),
                format!("{name:?} is not a catalogue sport"),
            ))
        })
    }
}

/// How a template grows week over week.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Progression {
    /// The levers to pull, in order.
    pub order: Vec<ProgressionLever>,
    /// How many levers may move in one week.
    pub max_weekly_step: u8,
}

impl Default for Progression {
    fn default() -> Self {
        Self {
            order: Vec::new(),
            max_weekly_step: 1,
        }
    }
}

/// Where in the season, and in what state, a template fits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PhaseFit {
    /// The phase kinds the template belongs in; empty means any.
    pub phases: Vec<PhaseKind>,
    /// The lowest readiness level the template may be scheduled at.
    pub readiness_min: ReadinessLevel,
    /// How many times a week at most.
    pub max_per_week: u8,
    /// Hours to keep between two instances.
    pub min_spacing_hours: u16,
    /// Athlete states the template is not for.
    pub contraindications: Vec<Contraindication>,
}

impl PhaseFit {
    /// python `MIN_SPACING_FOR_QUALITY`: a quality session (readiness p2 or
    /// p3) never repeats inside a day.
    pub const MIN_SPACING_FOR_QUALITY: u16 = 24;

    /// python `check_workout` on `fit`: phases named, and spacing at least
    /// a day for a quality template.
    fn check(&self) -> Check {
        if self.phases.is_empty() {
            return Err(Violation::new(
                "fit.phases",
                "empty; name the phase kinds the template fits",
            ));
        }
        let quality = matches!(self.readiness_min, ReadinessLevel::P2 | ReadinessLevel::P3);
        if quality && self.min_spacing_hours < Self::MIN_SPACING_FOR_QUALITY {
            return Err(Violation::new(
                "fit.min_spacing_hours",
                format!(
                    "{} < {} while readiness_min is {}",
                    self.min_spacing_hours,
                    Self::MIN_SPACING_FOR_QUALITY,
                    self.readiness_min
                ),
            ));
        }
        Ok(())
    }
}

impl Default for PhaseFit {
    fn default() -> Self {
        Self {
            phases: Vec::new(),
            readiness_min: ReadinessLevel::P2,
            max_per_week: 7,
            min_spacing_hours: 0,
            contraindications: Vec::new(),
        }
    }
}

/// Endurance workout template — declarative, repeatable structured session.
///
/// The catalogue bank (`training_catalogue/workouts/*.toml` in the platform,
/// `training/workouts/*.toml` in contremaitre) deserializes into this struct
/// via [`WorkoutTemplate::from_toml`]; the platform's
/// `TrainingCatalogueRegistry` holds the parsed set. User-authored rows live
/// in the `workout_templates` database table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkoutTemplate {
    /// Stable identifier (UUID for DB rows, deterministic UUID for catalogue templates).
    pub id: Uuid,
    /// Tenant scope for user-authored templates; `None` for catalogue templates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<Uuid>,
    /// User scope for user-authored templates; `None` for catalogue templates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<Uuid>,
    /// URL-safe slug (`long_run_z2`, `threshold_4x8`, …) — unique per scope.
    pub slug: String,
    /// Human-readable workout name.
    pub name: String,
    /// Sport this template applies to.
    pub sport: SportType,
    /// Total expected duration in minutes (sum of step durations).
    pub duration_minutes: u32,
    /// High-level intensity distribution for downstream coaching cues.
    pub intensity_distribution: IntensityDistribution,
    /// What the session is for.
    pub purpose: WorkoutPurpose,
    /// Every sport the template is written for; the primary `sport` must be
    /// listed when this is non-empty, and empty means the primary alone.
    #[serde(default)]
    pub sport_variants: Vec<SportType>,
    /// Strength of the evidence behind the template. Defaults to coach
    /// judgement so a user-authored row is honest.
    #[serde(default = "EvidenceTier::coach_judgement")]
    pub evidence_tier: EvidenceTier,
    /// What the evidence does not cover — required at grey or coach judgement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caveat: Option<String>,
    /// Ordered list of steps that make up the workout.
    pub structure: Vec<WorkoutStep>,
    /// Per-template target-zone overlay.
    pub target_zones: WorkoutTargetZones,
    /// Parameter ranges the coach fills in.
    #[serde(default)]
    pub params: WorkoutParams,
    /// How the template grows.
    #[serde(default)]
    pub progression: Progression,
    /// Where it fits.
    #[serde(default)]
    pub fit: PhaseFit,
    /// The propositions behind it, as `evidence/sports_science/<category>/<slug>.md`.
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    /// Whether this template is one of the catalogue templates (read-only).
    #[serde(default)]
    pub is_compiled_in: bool,
    /// Last-modified timestamp.
    #[serde(default = "Utc::now")]
    pub updated_at: DateTime<Utc>,
}

impl EvidenceTier {
    /// The serde default of a template's `evidence_tier`.
    const fn coach_judgement() -> Self {
        Self::CoachJudgement
    }
}

/// The serde name of a sport — what a catalogue file writes and what an
/// error names.
fn sport_key(sport: &SportType) -> String {
    match sport {
        SportType::Other(name) => name.clone(),
        named => serde_json::to_value(named)
            .ok()
            .and_then(|value| JsonValue::as_str(&value).map(str::to_owned))
            .unwrap_or_else(|| named.display_name().to_owned()),
    }
}

impl WorkoutTemplate {
    /// The shape name a parse error carries.
    const KIND: &'static str = "workout template";

    /// Parse a catalogue TOML document and validate it.
    ///
    /// The parsed template is marked `is_compiled_in` — the flag means
    /// "catalogue, read-only" — and a file that writes the flag itself is
    /// refused (python: `is_compiled_in` never written in a catalogue file).
    ///
    /// The text is parsed twice on purpose: once as a plain table to look
    /// for the flag before the typed shape can absorb it, then typed from
    /// the source text so a bad value is reported with its line and column
    /// and the offending line — a table-to-typed conversion keeps only the
    /// key path.
    ///
    /// # Errors
    ///
    /// [`CatalogueError::Parse`] when the TOML does not deserialize;
    /// [`CatalogueError::Validation`] when a catalogue rule fails.
    pub fn from_toml(text: &str) -> Result<Self, CatalogueError> {
        let table: Table = text.parse().map_err(|e| parse_error(Self::KIND, e))?;
        if table.contains_key("is_compiled_in") {
            let slug = table
                .get("slug")
                .and_then(TomlValue::as_str)
                .unwrap_or_default()
                .to_owned();
            return Err(CatalogueValidationError::Workout {
                slug,
                key: "is_compiled_in".to_owned(),
                message: "never written in a catalogue file; the parser sets it".to_owned(),
            }
            .into());
        }
        let mut template: Self = toml::from_str(text).map_err(|e| parse_error(Self::KIND, e))?;
        template.is_compiled_in = true;
        template.validate_catalogue()?;
        Ok(template)
    }

    /// The catalogue rules: python `check_workout`, plus the two residuals
    /// only Rust checks — every intensity anchor parses with
    /// [`RelativeIntensity::parse`], and no `SportType::Other` anywhere.
    ///
    /// # Errors
    ///
    /// The first broken rule, naming the slug and the key.
    pub fn validate_catalogue(&self) -> Result<(), CatalogueValidationError> {
        self.checks()
            .map_err(|violation| CatalogueValidationError::Workout {
                slug: self.slug.clone(),
                key: violation.key,
                message: violation.message,
            })
    }

    fn checks(&self) -> Check {
        self.check_sports()?;
        check_ref_shapes("", &self.evidence_refs)?;
        check_citation("", &self.evidence_refs, self.evidence_tier, "evidence_tier")?;
        check_caveat(self.evidence_tier, self.caveat.as_deref())?;
        self.params.check()?;
        self.check_purpose_shape()?;
        self.check_anchors()?;
        self.check_progression()?;
        self.fit.check()
    }

    /// The sports an intensity anchor is required for: the variants when
    /// there are any, else the primary sport.
    fn sports_in_play(&self) -> slice::Iter<'_, SportType> {
        if self.sport_variants.is_empty() {
            slice::from_ref(&self.sport).iter()
        } else {
            self.sport_variants.iter()
        }
    }

    /// Rust only: no `Other` sport; python `check_workout`: the primary
    /// sport is among the variants when there are any.
    fn check_sports(&self) -> Check {
        if let SportType::Other(name) = &self.sport {
            return Err(Violation::new(
                "sport",
                format!("{name:?} is not a catalogue sport"),
            ));
        }
        for (i, variant) in self.sport_variants.iter().enumerate() {
            if let SportType::Other(name) = variant {
                return Err(Violation::new(
                    format!("sport_variants[{i}]"),
                    format!("{name:?} is not a catalogue sport"),
                ));
            }
        }
        if !self.sport_variants.is_empty() && !self.sport_variants.contains(&self.sport) {
            return Err(Violation::new(
                "sport_variants",
                format!(
                    "primary sport {:?} is not listed among the variants",
                    sport_key(&self.sport)
                ),
            ));
        }
        Ok(())
    }

    /// python `check_workout`: a strength purpose prescribes a load on
    /// `strength_training` with no variants; an endurance purpose carries an
    /// RPE band and an anchor for every sport in play.
    fn check_purpose_shape(&self) -> Check {
        let purpose = self.purpose;
        if purpose.is_strength() {
            let load_stated = self
                .params
                .load
                .as_deref()
                .is_some_and(|load| !load.trim().is_empty());
            if !load_stated {
                return Err(Violation::new(
                    "params.load",
                    format!("required and non-empty for strength purpose {purpose}"),
                ));
            }
            if self.sport != SportType::StrengthTraining {
                return Err(Violation::new(
                    "sport",
                    format!(
                        "strength purpose {purpose} requires sport = \"strength_training\", got {:?}",
                        sport_key(&self.sport)
                    ),
                ));
            }
            if !self.sport_variants.is_empty() {
                return Err(Violation::new(
                    "sport_variants",
                    format!("must be empty for strength purpose {purpose}"),
                ));
            }
            return Ok(());
        }
        if self.params.rpe.is_none() {
            return Err(Violation::new(
                "params.rpe",
                format!("required for endurance purpose {purpose}"),
            ));
        }
        for sport in self.sports_in_play() {
            let stated = self
                .params
                .intensity
                .get(sport)
                .is_some_and(|anchor| !anchor.trim().is_empty());
            if !stated {
                return Err(Violation::new(
                    format!("params.intensity.{}", sport_key(sport)),
                    "missing or empty intensity anchor for a sport in play",
                ));
            }
        }
        Ok(())
    }

    /// Rust only: every anchor is in the [`RelativeIntensity`] grammar. The
    /// sports in play are walked first, in file order, then any other keys
    /// by name, so the report is deterministic.
    fn check_anchors(&self) -> Check {
        let mut sports: Vec<&SportType> = self.sports_in_play().collect();
        let mut others: Vec<&SportType> = self
            .params
            .intensity
            .keys()
            .filter(|sport| !sports.contains(sport))
            .collect();
        others.sort_by_key(|sport| sport_key(sport));
        sports.extend(others);
        for sport in sports {
            let Some(anchor) = self.params.intensity.get(sport) else {
                continue;
            };
            if RelativeIntensity::parse(anchor).is_none() {
                return Err(Violation::new(
                    format!("params.intensity.{}", sport_key(sport)),
                    format!(
                        "{anchor:?} is not in the intensity grammar (Z1-Z7, a zone name, sweet spot, NN-MM% FTP)"
                    ),
                ));
            }
        }
        Ok(())
    }

    /// python `check_workout`: a rep range needs the levers that grow it.
    fn check_progression(&self) -> Check {
        if self.params.reps.is_some() && self.progression.order.is_empty() {
            return Err(Violation::new(
                "progression.order",
                "empty while params.reps is a range; name the levers that grow it",
            ));
        }
        Ok(())
    }

    /// The purpose and readiness floor a session gets when it is authored
    /// inline — from a chat payload, say — with only an intensity
    /// distribution to go on.
    #[must_use]
    pub const fn inline_defaults(
        distribution: IntensityDistribution,
    ) -> (WorkoutPurpose, ReadinessLevel) {
        match distribution {
            IntensityDistribution::Recovery => (WorkoutPurpose::Recovery, ReadinessLevel::P0),
            IntensityDistribution::Polarized => (WorkoutPurpose::Endurance, ReadinessLevel::P1),
            IntensityDistribution::Pyramid => (WorkoutPurpose::Tempo, ReadinessLevel::P1),
            IntensityDistribution::Threshold => (WorkoutPurpose::Threshold, ReadinessLevel::P2),
            IntensityDistribution::Vo2max => (WorkoutPurpose::Vo2maxLong, ReadinessLevel::P2),
        }
    }

    /// Every `evidence_refs` entry `exists(category, slug)` denies.
    pub fn unresolved_references(
        &self,
        exists: &dyn Fn(&str, &str) -> bool,
    ) -> Vec<UnresolvedReference> {
        unresolved_in(
            &format!("workout '{}'", self.slug),
            "",
            &self.evidence_refs,
            exists,
        )
    }
}

/// What a caller asks the bank for: any combination of purpose, phase and
/// sport, each `None` meaning "any".
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkoutFilter {
    /// Only templates with this purpose.
    pub purpose: Option<WorkoutPurpose>,
    /// Only templates that fit this phase — a template with no `fit.phases`
    /// fits every phase.
    pub phase: Option<PhaseKind>,
    /// Only templates written for this sport, as the primary or a variant.
    pub sport: Option<SportType>,
}

impl WorkoutFilter {
    /// Whether `template` satisfies every stated criterion.
    #[must_use]
    pub fn matches(&self, template: &WorkoutTemplate) -> bool {
        let purpose_ok = self
            .purpose
            .is_none_or(|purpose| purpose == template.purpose);
        let phase_ok = self.phase.is_none_or(|phase| {
            template.fit.phases.is_empty() || template.fit.phases.contains(&phase)
        });
        let sport_ok = self.sport.as_ref().is_none_or(|sport| {
            template.sport == *sport || template.sport_variants.contains(sport)
        });
        purpose_ok && phase_ok && sport_ok
    }
}
