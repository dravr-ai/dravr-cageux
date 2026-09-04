// ABOUTME: Tests for the periodization kernel — the four catalogue shapes parse, validate, and every invariant fires on a broken fixture
// ABOUTME: Also pins the RelativeIntensity grammar, WorkoutFilter semantics, inline defaults, the loading-pattern string form and evidence-ref parsing
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::str_to_string,
    clippy::too_many_lines
)]

use std::collections::BTreeSet;
use std::fmt::Display;

use dravr_cageux::models::SportType;
use dravr_cageux::periodization::{
    evidence_ref_parts, CatalogueError, CatalogueValidationError, Contraindication, EventClass,
    EvidenceTier, Flavour, FlavourFamily, HoursTier, InputDimension, IntensityDistribution,
    LoadingPattern, Measurement, Modifier, PhaseFit, PhaseKind, PhaseLength, ProgressionLever,
    ReadinessLevel, RelativeIntensity, SelectionTable, Sequencing, SkeletonTemplate, StrengthGoal,
    UnresolvedReference, WorkoutFilter, WorkoutParams, WorkoutPurpose, WorkoutStep,
    WorkoutTemplate,
};
use serde::Serialize;

// ============================================================================
// Fixtures — the §3.3–3.6 examples of the Phase 1 spec, verbatim
// ============================================================================

const FLAVOUR_YAML: &str = r#"id: polarized-classic
family: polarized
sequencing: linear
modifiers: []
evidence_tier: rct
caveat: null                              # required (non-null) when evidence_tier is grey or coach_judgement
tid_targets:                              # per phase kind; base and build required
  base:  { z1: { min: 0.80, max: 0.90 }, z2: { min: 0.00, max: 0.05 }, z3: { min: 0.10, max: 0.20 } }
  build: { z1: { min: 0.75, max: 0.85 }, z2: { min: 0.00, max: 0.05 }, z3: { min: 0.15, max: 0.20 } }
hard_sessions_per_week:
  min: 1
  max: 2                                  # the base cap (<= 5 sessions/week)
  recovery_limited_max: 1
  max_by_sessions_per_week:               # optional, ascending; each max > base max
    - { from_sessions: 6, max: 3 }
    - { from_sessions: 10, max: 4 }
min_spacing_hours_between_hard: { default: 48, recovery_limited: 72 }
session_mix:                              # u8 weights over purposes, per phase kind; base and build required
  base:  { endurance_long: 3, endurance: 4, vo2max_long: 1, neuromuscular: 1 }
  build: { endurance_long: 2, endurance: 3, vo2max_long: 2, race_specific: 1 }
prerequisites:
  min_hours_per_week: 5
  min_sessions_per_week: 4
  measurement: [[hr], [rpe]]              # outer any-of, inner all-of: lactate-guided = [[lactate], [power, hr]]
  min_training_age_years: 1
contraindications: [novice_first_season]
loading_pattern: { default: "3:1", recovery_limited: "2:1" }   # quoted strings
readiness_substitution:                   # the ladder as data; every level present
  p0: { purposes: [recovery], max_hard_sessions_per_week: 0 }
  p1: { purposes: [recovery, endurance, endurance_long, tempo, mobility], max_hard_sessions_per_week: 0 }
  p2: { purposes: [recovery, endurance, endurance_long, tempo, sweet_spot, threshold, vo2max_long, vo2max_short, neuromuscular, mobility, strength_maint], max_hard_sessions_per_week: 1 }
  p3: { purposes: [recovery, endurance, endurance_long, tempo, sweet_spot, threshold, vo2max_long, vo2max_short, neuromuscular, mobility, strength_maint, race_specific], max_hard_sessions_per_week: 2 }
max_weeks: null                           # capped flavours only (time-crunched: 12)
evidence_refs:
  - evidence/sports_science/training_prescription/seiler-2010-best-practice.md
"#;

const SKELETON_YAML: &str = r#"# A phase `length` is exactly one of: { fixed_weeks: N } | { fixed_days: N } | { share_of_weeks_to_goal: F, min_weeks: N, max_weeks: N }
id: marathon-linear
event_classes: [marathon]
hours_tiers: [from_4_to_6, from_6_to_10, over_10]
min_weeks: 12
phases:
  - kind: base
    purpose: "Aerobic base and durability: the long run grows while everything else stays easy."
    length: { share_of_weeks_to_goal: 0.40, min_weeks: 6, max_weeks: 16 }
    volume_share_of_peak: { min: 0.40, max: 0.60 }        # Haugen 2022 build-entry volume
    flavour_override: pyramidal                            # optional FlavourFamily
    key_sessions: [endurance_long, tempo, neuromuscular]
  - kind: build
    purpose: "Threshold and endurance while volume climbs to its peak."
    length: { share_of_weeks_to_goal: 0.25, min_weeks: 2, max_weeks: 8 }
    volume_share_of_peak: { min: 0.60, max: 1.00 }
    key_sessions: [endurance_long, threshold]
  - kind: specialty
    purpose: "Race specificity: long runs carry blocks at marathon pace."
    length: { share_of_weeks_to_goal: 0.20, min_weeks: 2, max_weeks: 8 }
    volume_share_of_peak: { min: 0.80, max: 0.90 }
    key_sessions: [race_specific, endurance_long]
  - kind: taper
    purpose: "…"
    length: { fixed_days: 14 }
    volume_share_of_peak: { min: 0.40, max: 0.60 }
    key_sessions: [race_specific, endurance]
taper: { days: { min: 14, max: 21 }, volume_cut: { min: 0.40, max: 0.60 }, keep_intensity: true, keep_frequency: true }   # absent on the no-race skeleton
loading_pattern: { default: "3:1", recovery_limited: "2:1" }
recovery_week_cut: { min: 0.25, max: 0.35 }
drop_order: [base, build, specialty]       # shrinks toward min_weeks in this order; never taper or peak
multi_peak: { b_race_mini_taper_days: { min: 3, max: 5 }, transition_weeks_after_a_race: 1 }
strength:
  base:  { goal: max, sessions_per_week: 2, purposes: [strength_max, plyometric] }
  build: { goal: maintain, sessions_per_week: 1, purposes: [strength_maint] }
evidence_refs: [evidence/sports_science/training_prescription/bosquet-2007-taper-meta-analysis.md]
"#;

const SELECTION_YAML: &str = r#"rows:
  - input: hours_tier
    value: under_4
    prefer: [ { id: hvlit-foundation, weight: 3 } ]
    exclude: [ { id: norwegian-threshold-density, reason: "needs eight or more sessions a week" } ]
    tier: rct
    evidence_refs: [evidence/sports_science/training_prescription/festa-2019-polarized-vs-threshold-recreational.md]
    note: "Frequency and consistency first; under four hours the flavour question is second-order."
"#;

const WORKOUT_TOML: &str = r#"id = "9adab247-24fa-54f3-92bd-47ad84357f4b"
slug = "vo2max_4x8"
name = "VO2max — 4 × 8 min"
sport = "ride"
duration_minutes = 65
intensity_distribution = "vo2max"
purpose = "vo2max_long"
sport_variants = ["ride", "run"]          # SportType snake_case; primary `sport` must be listed; empty = primary only
evidence_tier = "rct"
# caveat = "…"                            # required when evidence_tier is grey or coach_judgement
evidence_refs = ["evidence/sports_science/training_prescription/seiler-2013-interval-duration-4x8.md"]
[params]
reps = { min = 4, max = 5, default = 4 }
work_seconds = { min = 420, max = 600, default = 480 }
rest_seconds = { min = 90, max = 180, default = 120 }
rpe = { min = 7, max = 8 }                # endurance purposes only
intensity_label = "~90% HRmax"            # coach voice
[params.intensity]                        # one entry per sport in play; must parse with RelativeIntensity::parse
ride = "100-105%"
run = "Z5"
[progression]
order = ["add_rep", "lengthen_rep", "shorten_rest", "raise_intensity"]
max_weekly_step = 1
[fit]
phases = ["build", "peak"]
readiness_min = "p2"
max_per_week = 2
min_spacing_hours = 48
contraindications = ["novice_first_season", "acute_injury"]

[target_zones]
hr_pct_of_lt2 = [0.6, 0.7, 0.85, 0.95, 1.05]
power_pct_of_ftp = [0.55, 0.75, 0.9, 1.0, 1.05]

[[structure]]
label = "Warm-up"
duration_seconds = 900
target_zone = "Z1"

[[structure]]
label = "Controlled VO2max interval"
duration_seconds = 480.0
target_zone = "VO2max"
repeat = 4

[[structure]]
label = "Easy recovery"
duration_seconds = 120
target_zone = "Z1"
repeat = 4

[[structure]]
label = "Cool-down"
duration_seconds = 600
target_zone = "Z1"
"#;

const STRENGTH_TOML: &str = r#"id = "6b1e3ec3-d889-58ae-b6a8-33ff83b715ef"
slug = "strength_max"
name = "Maximal strength — 4 × 4 half-squat"
sport = "strength_training"
sport_variants = []
duration_minutes = 55
intensity_distribution = "polarized"
purpose = "strength_max"
evidence_tier = "meta_analysis"
evidence_refs = ["evidence/sports_science/training_prescription/storen-2008-maximal-strength-running-economy.md"]

[params]
sets = { min = 3, max = 4, default = 4 }
reps = { min = 4, max = 6, default = 4 }
rest_seconds = { min = 120, max = 180, default = 150 }
load = "85-90% 1RM or heavier; 4 × 4RM is the canonical dose"

[progression]
order = ["add_load"]
max_weekly_step = 1

[fit]
phases = ["base"]
readiness_min = "p2"
max_per_week = 3
min_spacing_hours = 48
contraindications = ["acute_injury", "no_lifting_history"]

[target_zones]

[[structure]]
label = "Half-squat 4 × 4RM"
duration_seconds = 900
target_zone = "Strength"
"#;

// ============================================================================
// Helpers
// ============================================================================

fn flavour() -> Flavour {
    Flavour::from_yaml(FLAVOUR_YAML).expect("the §3.3 flavour example parses and validates")
}

fn skeleton() -> SkeletonTemplate {
    SkeletonTemplate::from_yaml(SKELETON_YAML)
        .expect("the §3.4 skeleton example parses and validates")
}

fn workout() -> WorkoutTemplate {
    WorkoutTemplate::from_toml(WORKOUT_TOML).expect("the §3.6 workout example parses and validates")
}

/// A fixture with one substring swapped — the way every broken fixture is
/// made, so the test names the single edit that breaks it.
fn edited(fixture: &str, from: &str, to: &str) -> String {
    assert!(
        fixture.contains(from),
        "the fixture no longer carries {from:?}; the edit would be a no-op"
    );
    fixture.replacen(from, to, 1)
}

/// The validation error a text produces, with the key and message it names.
fn violation(result: Result<impl Sized, CatalogueError>) -> (String, String) {
    match result {
        Err(CatalogueError::Validation(error)) => match error {
            CatalogueValidationError::Flavour { key, message, .. }
            | CatalogueValidationError::Skeleton { key, message, .. }
            | CatalogueValidationError::Selection { key, message }
            | CatalogueValidationError::Workout { key, message, .. } => (key, message),
        },
        Err(CatalogueError::Parse { message, .. }) => {
            panic!("expected a validation error, got a parse error: {message}")
        }
        Ok(_) => panic!("expected a validation error, the fixture validated"),
    }
}

/// The parse error message a text produces.
fn parse_message(result: Result<impl Sized, CatalogueError>) -> String {
    match result {
        Err(CatalogueError::Parse { message, .. }) => message,
        Err(CatalogueError::Validation(error)) => {
            panic!("expected a parse error, got a validation error: {error}")
        }
        Ok(_) => panic!("expected a parse error, the fixture parsed"),
    }
}

fn serde_names_match<T: Serialize + Display + Copy>(all: &[T]) {
    for value in all {
        let json = serde_json::to_string(value).unwrap();
        assert_eq!(
            json,
            format!("\"{value}\""),
            "serde name of {value} drifts from as_str"
        );
    }
}

// ============================================================================
// The spec examples parse and validate
// ============================================================================

#[test]
fn the_flavour_example_parses_with_every_field() {
    let flavour = flavour();
    assert_eq!(flavour.id, "polarized-classic");
    assert_eq!(flavour.family, FlavourFamily::Polarized);
    assert_eq!(flavour.sequencing, Sequencing::Linear);
    assert!(flavour.modifiers.is_empty());
    assert_eq!(flavour.evidence_tier, EvidenceTier::Rct);
    assert_eq!(flavour.caveat, None);
    let base = &flavour.tid_targets[&PhaseKind::Base];
    assert!((base.z1.min - 0.80).abs() < 1e-6 && (base.z1.max - 0.90).abs() < 1e-6);
    assert!((base.z3.min - 0.10).abs() < 1e-6);
    assert_eq!(flavour.tid_targets.len(), 2);
    let cap = &flavour.hard_sessions_per_week;
    assert_eq!((cap.min, cap.max, cap.recovery_limited_max), (1, 2, 1));
    assert_eq!(cap.max_by_sessions_per_week.len(), 2);
    assert_eq!(cap.max_by_sessions_per_week[1].from_sessions, 10);
    assert_eq!(cap.max_by_sessions_per_week[1].max, 4);
    assert_eq!(flavour.min_spacing_hours_between_hard.default, 48);
    assert_eq!(flavour.min_spacing_hours_between_hard.recovery_limited, 72);
    assert_eq!(
        flavour.session_mix[&PhaseKind::Base][&WorkoutPurpose::EnduranceLong],
        3
    );
    assert_eq!(
        flavour.session_mix[&PhaseKind::Build][&WorkoutPurpose::RaceSpecific],
        1
    );
    assert!((flavour.prerequisites.min_hours_per_week - 5.0).abs() < f32::EPSILON);
    assert_eq!(flavour.prerequisites.min_sessions_per_week, 4);
    assert_eq!(
        flavour.prerequisites.measurement,
        vec![vec![Measurement::Hr], vec![Measurement::Rpe]]
    );
    assert_eq!(
        flavour.contraindications,
        vec![Contraindication::NoviceFirstSeason]
    );
    assert_eq!(flavour.loading_pattern.default.to_string(), "3:1");
    assert_eq!(flavour.loading_pattern.recovery_limited.to_string(), "2:1");
}

#[test]
fn the_flavour_example_parses_its_ladder_and_evidence() {
    let flavour = flavour();
    let p2 = &flavour.readiness_substitution[&ReadinessLevel::P2];
    assert_eq!(p2.max_hard_sessions_per_week, 1);
    assert_eq!(p2.purposes.len(), 11);
    assert_eq!(flavour.max_weeks, None);
    assert_eq!(
        flavour.evidence_refs,
        vec!["evidence/sports_science/training_prescription/seiler-2010-best-practice.md"]
    );
    assert!(flavour
        .purposes_used()
        .contains(&WorkoutPurpose::RaceSpecific));
    assert_eq!(flavour.purposes_used().len(), 12);
}

#[test]
fn the_skeleton_example_parses_with_every_field() {
    let skeleton = skeleton();
    assert_eq!(skeleton.id, "marathon-linear");
    assert_eq!(skeleton.event_classes, vec![EventClass::Marathon]);
    assert_eq!(
        skeleton.hours_tiers,
        vec![HoursTier::From4To6, HoursTier::From6To10, HoursTier::Over10]
    );
    assert_eq!(skeleton.min_weeks, 12);
    assert_eq!(skeleton.phases.len(), 4);
    let base = &skeleton.phases[0];
    assert_eq!(base.kind, PhaseKind::Base);
    assert!(base.purpose.starts_with("Aerobic base"));
    match &base.length {
        PhaseLength::Share {
            share_of_weeks_to_goal,
            min_weeks,
            max_weeks,
        } => {
            assert!((share_of_weeks_to_goal - 0.40).abs() < 1e-6);
            assert_eq!((*min_weeks, *max_weeks), (6, 16));
        }
        other => panic!("base length is a share, got {other:?}"),
    }
    assert_eq!(base.flavour_override, Some(FlavourFamily::Pyramidal));
    assert_eq!(
        base.key_sessions,
        vec![
            WorkoutPurpose::EnduranceLong,
            WorkoutPurpose::Tempo,
            WorkoutPurpose::Neuromuscular
        ]
    );
    assert_eq!(
        skeleton.phases[3].length,
        PhaseLength::FixedDays { fixed_days: 14 }
    );
}

#[test]
fn the_skeleton_example_parses_its_taper_loading_and_strength() {
    let skeleton = skeleton();
    let taper = skeleton.taper.as_ref().unwrap();
    assert_eq!((taper.days.min, taper.days.max), (14, 21));
    assert!(taper.keep_intensity && taper.keep_frequency);
    assert_eq!(skeleton.loading_pattern.default.load_weeks, 3);
    assert!((skeleton.recovery_week_cut.max - 0.35).abs() < 1e-6);
    assert_eq!(
        skeleton.drop_order,
        vec![PhaseKind::Base, PhaseKind::Build, PhaseKind::Specialty]
    );
    assert_eq!(skeleton.multi_peak.b_race_mini_taper_days.max, 5);
    assert_eq!(skeleton.multi_peak.transition_weeks_after_a_race, 1);
    let strength_base = &skeleton.strength[&PhaseKind::Base];
    assert_eq!(strength_base.goal, StrengthGoal::Max);
    assert_eq!(strength_base.sessions_per_week, 2);
    assert_eq!(
        strength_base.purposes,
        vec![WorkoutPurpose::StrengthMax, WorkoutPurpose::Plyometric]
    );
    let used = skeleton.purposes_used();
    assert!(used.contains(&WorkoutPurpose::StrengthMaint));
    assert!(used.contains(&WorkoutPurpose::RaceSpecific));
    assert_eq!(used.len(), 9);
}

#[test]
fn the_selection_example_parses_with_every_field() {
    let table = SelectionTable::from_yaml(SELECTION_YAML)
        .expect("the §3.5 selection row parses and validates");
    assert_eq!(table.rows.len(), 1);
    let row = &table.rows[0];
    assert_eq!(row.input, InputDimension::HoursTier);
    assert_eq!(row.value, "under_4");
    assert_eq!(row.prefer[0].id, "hvlit-foundation");
    assert_eq!(row.prefer[0].weight, 3);
    assert_eq!(row.exclude[0].id, "norwegian-threshold-density");
    assert_eq!(row.exclude[0].reason, "needs eight or more sessions a week");
    assert_eq!(row.tier, EvidenceTier::Rct);
    assert_eq!(row.evidence_refs.len(), 1);
    assert!(row.note.as_deref().unwrap().starts_with("Frequency"));
    assert_eq!(
        table.flavour_ids(),
        BTreeSet::from(["hvlit-foundation", "norwegian-threshold-density"])
    );
}

#[test]
fn the_workout_example_parses_with_every_field() {
    let workout = workout();
    assert_eq!(workout.slug, "vo2max_4x8");
    assert_eq!(workout.sport, SportType::Ride);
    assert_eq!(
        workout.intensity_distribution,
        IntensityDistribution::Vo2max
    );
    assert_eq!(workout.purpose, WorkoutPurpose::Vo2maxLong);
    assert_eq!(
        workout.sport_variants,
        vec![SportType::Ride, SportType::Run]
    );
    assert_eq!(workout.evidence_tier, EvidenceTier::Rct);
    assert_eq!(workout.caveat, None);
    assert_eq!(workout.evidence_refs.len(), 1);
    let params = &workout.params;
    assert_eq!(params.reps.as_ref().unwrap().default, 4);
    assert_eq!(params.work_seconds.as_ref().unwrap().default, 480);
    assert_eq!(params.rest_seconds.as_ref().unwrap().max, 180);
    let rpe = params.rpe.as_ref().unwrap();
    assert_eq!((rpe.min, rpe.max), (7, 8));
    assert_eq!(params.intensity_label.as_deref(), Some("~90% HRmax"));
    assert_eq!(params.intensity[&SportType::Ride], "100-105%");
    assert_eq!(params.intensity[&SportType::Run], "Z5");
    assert_eq!(params.load, None);
}

#[test]
fn the_workout_example_parses_its_progression_fit_and_structure() {
    let workout = workout();
    assert_eq!(
        workout.progression.order,
        vec![
            ProgressionLever::AddRep,
            ProgressionLever::LengthenRep,
            ProgressionLever::ShortenRest,
            ProgressionLever::RaiseIntensity
        ]
    );
    assert_eq!(workout.progression.max_weekly_step, 1);
    assert_eq!(workout.fit.phases, vec![PhaseKind::Build, PhaseKind::Peak]);
    assert_eq!(workout.fit.readiness_min, ReadinessLevel::P2);
    assert_eq!(workout.fit.max_per_week, 2);
    assert_eq!(workout.fit.min_spacing_hours, 48);
    assert_eq!(
        workout.fit.contraindications,
        vec![
            Contraindication::NoviceFirstSeason,
            Contraindication::AcuteInjury
        ]
    );
    assert!(workout.is_compiled_in, "the parser marks a catalogue file");
    assert_eq!(workout.structure.len(), 4);
    assert_eq!(
        workout.structure[1].duration_seconds, 480,
        "480.0 is a whole number"
    );
    assert_eq!(WorkoutStep::total_seconds(&workout.structure), 3900);
    assert_eq!(
        workout.target_zones.power_pct_of_ftp.map(|z| z[4]),
        Some(1.05)
    );
}

#[test]
fn the_strength_example_parses_and_a_defaulted_row_reads_back_the_defaults() {
    let strength = WorkoutTemplate::from_toml(STRENGTH_TOML).unwrap();
    assert_eq!(strength.purpose, WorkoutPurpose::StrengthMax);
    assert!(strength
        .params
        .load
        .as_deref()
        .unwrap()
        .starts_with("85-90% 1RM"));
    assert_eq!(strength.params.rpe, None);
    assert!(strength.params.intensity.is_empty());

    let json = r#"{"id":"11111111-2222-3333-4444-555555555555","slug":"x","name":"x","sport":"run","duration_minutes":30,"intensity_distribution":"polarized","purpose":"endurance","structure":[],"target_zones":{}}"#;
    let row: WorkoutTemplate = serde_json::from_str(json).unwrap();
    assert_eq!(row.params, WorkoutParams::default());
    assert_eq!(row.fit, PhaseFit::default());
    assert_eq!(row.fit.readiness_min, ReadinessLevel::P2);
    assert_eq!(row.fit.max_per_week, 7);
    assert_eq!(row.progression.max_weekly_step, 1);
    assert_eq!(
        row.evidence_tier,
        EvidenceTier::CoachJudgement,
        "a user-authored row is honest about its evidence"
    );
    assert!(!row.is_compiled_in);
    let original = workout();
    let back: WorkoutTemplate =
        serde_json::from_str(&serde_json::to_string(&original).unwrap()).unwrap();
    assert_eq!(
        back, original,
        "the catalogue template round-trips through JSON"
    );
}

// ============================================================================
// Every §1 invariant fires on a broken fixture, naming the key
// ============================================================================

#[test]
fn an_unquoted_loading_pattern_is_the_number_181() {
    let text = edited(FLAVOUR_YAML, r#"default: "3:1""#, "default: 181");
    let message = parse_message(Flavour::from_yaml(&text));
    assert!(message.starts_with("loading_pattern.default:"), "{message}");
    assert!(
        message.contains(
            r#"quote the loading pattern ("3:1"): unquoted 3:1 is the number 181 in YAML 1.1"#
        ),
        "{message}"
    );
    let text = edited(
        SKELETON_YAML,
        r#"recovery_limited: "2:1""#,
        "recovery_limited: 121",
    );
    let message = parse_message(SkeletonTemplate::from_yaml(&text));
    assert!(
        message.starts_with("loading_pattern.recovery_limited:"),
        "{message}"
    );
    let text = edited(FLAVOUR_YAML, r#"default: "3:1""#, r#"default: "3-1""#);
    let message = parse_message(Flavour::from_yaml(&text));
    assert!(
        message.contains(r"does not match ^[1-9]\d*:[1-9]\d*$"),
        "{message}"
    );
}

#[test]
fn tid_share_sums_are_bounded() {
    let text = edited(
        FLAVOUR_YAML,
        "z3: { min: 0.10, max: 0.20 }",
        "z3: { min: 0.30, max: 0.40 }",
    );
    let (key, message) = violation(Flavour::from_yaml(&text));
    assert_eq!(key, "tid_targets.base");
    assert_eq!(message, "z1+z2+z3 min shares sum to 1.10, above 1.0");
    let text = edited(
        FLAVOUR_YAML,
        "z1: { min: 0.75, max: 0.85 }",
        "z1: { min: 0.50, max: 0.60 }",
    );
    let (key, message) = violation(Flavour::from_yaml(&text));
    assert_eq!(key, "tid_targets.build");
    assert_eq!(message, "z1+z2+z3 max shares sum to 0.85, below 1.0");
    let text = edited(
        FLAVOUR_YAML,
        "z2: { min: 0.00, max: 0.05 }",
        "z2: { min: 0.06, max: 0.05 }",
    );
    let (key, message) = violation(Flavour::from_yaml(&text));
    assert_eq!(key, "tid_targets.base.z2");
    assert_eq!(message, "min 0.06 > max 0.05");
}

#[test]
fn a_readiness_level_is_a_superset_of_the_level_below() {
    let text = edited(
        FLAVOUR_YAML,
        "p1: { purposes: [recovery, endurance",
        "p1: { purposes: [endurance",
    );
    let (key, message) = violation(Flavour::from_yaml(&text));
    assert_eq!(key, "readiness_substitution.p1.purposes");
    assert_eq!(
        message,
        "not a superset of the level below; missing recovery"
    );
    let text = edited(
        FLAVOUR_YAML,
        "max_hard_sessions_per_week: 2 }",
        "max_hard_sessions_per_week: 0 }",
    );
    let (key, message) = violation(Flavour::from_yaml(&text));
    assert_eq!(key, "readiness_substitution.p3.max_hard_sessions_per_week");
    assert_eq!(message, "0 decreases from 1 at the level below");
    let text = edited(
        FLAVOUR_YAML,
        "  p3: { purposes: [recovery, endurance, endurance_long, tempo, sweet_spot, threshold, vo2max_long, vo2max_short, neuromuscular, mobility, strength_maint, race_specific], max_hard_sessions_per_week: 2 }\n",
        "",
    );
    let (key, message) = violation(Flavour::from_yaml(&text));
    assert_eq!(key, "readiness_substitution.p3");
    assert_eq!(message, "missing readiness level");
    let text = edited(FLAVOUR_YAML, "  p3: {", "  p9: {");
    let message = parse_message(Flavour::from_yaml(&text));
    assert!(
        message.contains("unknown variant `p9`"),
        "a level outside the vocabulary is a parse error: {message}"
    );
}

#[test]
fn the_p0_ladder_names_no_quality_purpose() {
    let text = edited(
        FLAVOUR_YAML,
        "p0: { purposes: [recovery]",
        "p0: { purposes: [recovery, mobility, threshold]",
    );
    let (key, message) = violation(Flavour::from_yaml(&text));
    assert_eq!(key, "readiness_substitution.p0.purposes[2]");
    assert_eq!(
        message,
        "quality purpose \"threshold\" is not allowed at p0"
    );
    let text = edited(
        FLAVOUR_YAML,
        "p0: { purposes: [recovery]",
        "p0: { purposes: [recovery, mobility]",
    );
    let flavour = Flavour::from_yaml(&text).expect("mobility is not a quality purpose");
    assert_eq!(
        flavour.readiness_substitution[&ReadinessLevel::P0].purposes,
        vec![WorkoutPurpose::Recovery, WorkoutPurpose::Mobility]
    );
}

#[test]
fn a_flavour_needs_base_and_build_in_tid_targets_and_session_mix() {
    let text = edited(FLAVOUR_YAML, "  build: { z1", "  peak: { z1");
    let (key, message) = violation(Flavour::from_yaml(&text));
    assert_eq!(key, "tid_targets.build");
    assert_eq!(message, "missing; base and build are required");
    let text = edited(
        FLAVOUR_YAML,
        "  base:  { endurance_long: 3",
        "  peak:  { endurance_long: 3",
    );
    let (key, _) = violation(Flavour::from_yaml(&text));
    assert_eq!(key, "session_mix.base");
}

#[test]
fn hard_session_caps_are_ordered_and_tiers_ascend() {
    let text = edited(
        FLAVOUR_YAML,
        "recovery_limited_max: 1",
        "recovery_limited_max: 3",
    );
    let (key, message) = violation(Flavour::from_yaml(&text));
    assert_eq!(key, "hard_sessions_per_week.recovery_limited_max");
    assert_eq!(message, "3 > max 2");
    let text = edited(
        FLAVOUR_YAML,
        "from_sessions: 10, max: 4",
        "from_sessions: 6, max: 4",
    );
    let (key, message) = violation(Flavour::from_yaml(&text));
    assert_eq!(
        key,
        "hard_sessions_per_week.max_by_sessions_per_week[1].from_sessions"
    );
    assert_eq!(
        message,
        "6 not above the previous tier's 6; tiers must ascend"
    );
    let text = edited(
        FLAVOUR_YAML,
        "from_sessions: 6, max: 3",
        "from_sessions: 6, max: 2",
    );
    let (key, message) = violation(Flavour::from_yaml(&text));
    assert_eq!(
        key,
        "hard_sessions_per_week.max_by_sessions_per_week[0].max"
    );
    assert_eq!(message, "2 not above the base max 2");
    let text = edited(FLAVOUR_YAML, "  min: 1\n  max: 2", "  min: 3\n  max: 2");
    let (key, message) = violation(Flavour::from_yaml(&text));
    assert_eq!(key, "hard_sessions_per_week");
    assert_eq!(message, "min 3 > max 2");
}

#[test]
fn measurement_lists_are_non_empty_and_ids_are_kebab_case() {
    let text = edited(
        FLAVOUR_YAML,
        "measurement: [[hr], [rpe]]",
        "measurement: []",
    );
    let (key, message) = violation(Flavour::from_yaml(&text));
    assert_eq!(key, "prerequisites.measurement");
    assert_eq!(message, "outer any-of list is empty");
    let text = edited(
        FLAVOUR_YAML,
        "measurement: [[hr], [rpe]]",
        "measurement: [[hr], []]",
    );
    let (key, message) = violation(Flavour::from_yaml(&text));
    assert_eq!(key, "prerequisites.measurement[1]");
    assert_eq!(message, "inner all-of list is empty");
    let text = edited(
        FLAVOUR_YAML,
        "id: polarized-classic",
        "id: Polarized_Classic",
    );
    let (key, message) = violation(Flavour::from_yaml(&text));
    assert_eq!(key, "id");
    assert_eq!(message, "\"Polarized_Classic\" is not kebab-case");
}

#[test]
fn evidence_refs_may_not_be_empty_above_grey() {
    let text = edited(
        FLAVOUR_YAML,
        "evidence_refs:\n  - evidence/sports_science/training_prescription/seiler-2010-best-practice.md",
        "evidence_refs: []",
    );
    let (key, message) = violation(Flavour::from_yaml(&text));
    assert_eq!(key, "evidence_refs");
    assert_eq!(
        message,
        "empty while evidence_tier is rct; cite a proposition or tier it grey/coach_judgement"
    );
    let text = edited(
        WORKOUT_TOML,
        r#"evidence_refs = ["evidence/sports_science/training_prescription/seiler-2013-interval-duration-4x8.md"]"#,
        "evidence_refs = []",
    );
    let (key, _) = violation(WorkoutTemplate::from_toml(&text));
    assert_eq!(key, "evidence_refs");
    let text = edited(SELECTION_YAML, "evidence_refs: [evidence/sports_science/training_prescription/festa-2019-polarized-vs-threshold-recreational.md]", "evidence_refs: []");
    let (key, message) = violation(SelectionTable::from_yaml(&text));
    assert_eq!(key, "rows[0].evidence_refs");
    assert!(message.starts_with("empty while tier is rct"), "{message}");
    let text = edited(
        FLAVOUR_YAML,
        "evidence/sports_science/training_prescription/seiler-2010-best-practice.md",
        "evidence/sports_science/training_prescription/README.md",
    );
    let (key, message) = violation(Flavour::from_yaml(&text));
    assert_eq!(key, "evidence_refs[0]");
    assert!(
        message.ends_with("is not an evidence/sports_science/<category>/<slug>.md path"),
        "{message}"
    );
}

#[test]
fn grey_and_coach_judgement_need_a_caveat() {
    let text = edited(FLAVOUR_YAML, "evidence_tier: rct", "evidence_tier: grey");
    let (key, message) = violation(Flavour::from_yaml(&text));
    assert_eq!(key, "caveat");
    assert_eq!(message, "required when evidence_tier is grey");
    let text = edited(
        &text,
        "caveat: null",
        r#"caveat: "community practice with no peer-reviewed evidence base""#,
    );
    let flavour = Flavour::from_yaml(&text).unwrap();
    assert_eq!(flavour.evidence_tier, EvidenceTier::Grey);
    assert!(flavour.caveat.is_some());
    let text = edited(
        WORKOUT_TOML,
        r#"evidence_tier = "rct""#,
        r#"evidence_tier = "coach_judgement""#,
    );
    let (key, message) = violation(WorkoutTemplate::from_toml(&text));
    assert_eq!(key, "caveat");
    assert_eq!(message, "required when evidence_tier is coach_judgement");
    let text = edited(&text, r#"# caveat = "…""#, r#"caveat = "   ""#);
    let (key, _) = violation(WorkoutTemplate::from_toml(&text));
    assert_eq!(key, "caveat", "a blank caveat is no caveat");
}

#[test]
fn a_skeleton_taper_is_last_or_followed_only_by_race() {
    let late_base = "  - kind: base\n    purpose: \"late base\"\n    length: { fixed_weeks: 1 }\n    volume_share_of_peak: { min: 0.5, max: 0.6 }\n    key_sessions: [endurance]\ntaper: { days";
    let text = edited(SKELETON_YAML, "taper: { days", late_base);
    let (key, message) = violation(SkeletonTemplate::from_yaml(&text));
    assert_eq!(key, "phases[3]");
    assert_eq!(
        message,
        "taper must be last or followed only by race; followed by [\"base\"]"
    );
    let race = "  - kind: race\n    purpose: \"race week\"\n    length: { fixed_days: 7 }\n    volume_share_of_peak: { min: 0.3, max: 0.4 }\n    key_sessions: [race_specific]\ntaper: { days";
    let text = edited(SKELETON_YAML, "taper: { days", race);
    let text = edited(&text, "min_weeks: 12", "min_weeks: 13");
    let skeleton = SkeletonTemplate::from_yaml(&text).expect("race after taper is allowed");
    assert_eq!(skeleton.phases.len(), 5);
    assert_eq!(skeleton.phases[3].kind, PhaseKind::Taper);
    assert_eq!(skeleton.phases[4].kind, PhaseKind::Race);
    let text = edited(SKELETON_YAML, "  - kind: specialty", "  - kind: taper");
    let (key, message) = violation(SkeletonTemplate::from_yaml(&text));
    assert_eq!(key, "phases");
    assert_eq!(message, "more than one taper phase (2)");
}

#[test]
fn a_taper_phase_and_the_taper_rule_come_together() {
    let text = edited(SKELETON_YAML, "taper: { days: { min: 14, max: 21 }, volume_cut: { min: 0.40, max: 0.60 }, keep_intensity: true, keep_frequency: true }", "");
    let (key, message) = violation(SkeletonTemplate::from_yaml(&text));
    assert_eq!(key, "taper");
    assert_eq!(message, "a taper phase needs the top-level taper rule");
    let text = edited(SKELETON_YAML, "  - kind: taper", "  - kind: peak");
    let (key, message) = violation(SkeletonTemplate::from_yaml(&text));
    assert_eq!(key, "taper");
    assert_eq!(message, "a taper rule without a taper phase");
    let text = edited(&text, "taper: { days: { min: 14, max: 21 }, volume_cut: { min: 0.40, max: 0.60 }, keep_intensity: true, keep_frequency: true }", "");
    let (key, message) = violation(SkeletonTemplate::from_yaml(&text));
    assert_eq!(key, "phases");
    assert_eq!(
        message,
        "no taper phase while event_classes is not [no_race]"
    );
    let text = edited(
        &text,
        "event_classes: [marathon]",
        "event_classes: [no_race]",
    );
    let skeleton = SkeletonTemplate::from_yaml(&text).unwrap();
    assert!(
        skeleton.taper.is_none(),
        "the no-race skeleton has no taper"
    );
    let text = edited(
        SKELETON_YAML,
        "days: { min: 14, max: 21 }",
        "days: { min: 22, max: 21 }",
    );
    let (key, message) = violation(SkeletonTemplate::from_yaml(&text));
    assert_eq!(key, "taper.days");
    assert_eq!(message, "min 22 > max 21");
}

#[test]
fn a_phase_length_is_exactly_one_shape() {
    let text = edited(
        SKELETON_YAML,
        "length: { fixed_days: 14 }",
        "length: { fixed_days: 14, fixed_weeks: 2 }",
    );
    let message = parse_message(SkeletonTemplate::from_yaml(&text));
    assert!(
        message.contains(r#"keys ["fixed_days", "fixed_weeks"] are not exactly one of {fixed_weeks}, {fixed_days}, {share_of_weeks_to_goal, min_weeks, max_weeks}"#),
        "{message}"
    );
    let text = edited(
        SKELETON_YAML,
        "length: { share_of_weeks_to_goal: 0.40, min_weeks: 6, max_weeks: 16 }",
        "length: { share_of_weeks_to_goal: 0.40, min_weeks: 6 }",
    );
    let message = parse_message(SkeletonTemplate::from_yaml(&text));
    assert!(
        message.contains(r#"keys ["min_weeks", "share_of_weeks_to_goal"]"#),
        "{message}"
    );
    let text = edited(
        SKELETON_YAML,
        "length: { fixed_days: 14 }",
        "length: { fixed_days: 14, weeks: 2 }",
    );
    let message = parse_message(SkeletonTemplate::from_yaml(&text));
    assert!(message.contains("unknown field `weeks`"), "{message}");
    let text = edited(
        SKELETON_YAML,
        "share_of_weeks_to_goal: 0.40, min_weeks: 6, max_weeks: 16",
        "share_of_weeks_to_goal: 0.40, min_weeks: 17, max_weeks: 16",
    );
    let (key, message) = violation(SkeletonTemplate::from_yaml(&text));
    assert_eq!(key, "phases[0].length");
    assert_eq!(message, "min_weeks 17 > max_weeks 16");
}

#[test]
fn skeleton_lengths_fit_min_weeks_and_shares_sum_to_at_most_one() {
    let text = edited(SKELETON_YAML, "min_weeks: 12\n", "min_weeks: 11\n");
    let (key, message) = violation(SkeletonTemplate::from_yaml(&text));
    assert_eq!(key, "min_weeks");
    assert_eq!(
        message,
        "11 below the phase floor 12 (fixed weeks 0 + share min weeks 10 + fixed days 14 / 7 rounded up)"
    );
    let text = edited(
        SKELETON_YAML,
        "share_of_weeks_to_goal: 0.20",
        "share_of_weeks_to_goal: 0.40",
    );
    let (key, message) = violation(SkeletonTemplate::from_yaml(&text));
    assert_eq!(key, "phases");
    assert_eq!(message, "share_of_weeks_to_goal sums to 1.05, above 1.0");
    let text = edited(
        SKELETON_YAML,
        "phases:\n  - kind: base",
        "phases: []\nunused:\n  - kind: base",
    );
    let (key, message) = violation(SkeletonTemplate::from_yaml(&text));
    assert_eq!(key, "phases");
    assert_eq!(message, "no phases");
}

#[test]
fn drop_order_never_names_taper_or_peak_or_a_missing_phase() {
    let text = edited(
        SKELETON_YAML,
        "drop_order: [base, build, specialty]",
        "drop_order: [base, taper]",
    );
    let (key, message) = violation(SkeletonTemplate::from_yaml(&text));
    assert_eq!(key, "drop_order[1]");
    assert_eq!(message, "taper is never dropped");
    let text = edited(
        SKELETON_YAML,
        "drop_order: [base, build, specialty]",
        "drop_order: [prep]",
    );
    let (key, message) = violation(SkeletonTemplate::from_yaml(&text));
    assert_eq!(key, "drop_order[0]");
    assert_eq!(message, "the skeleton has no prep phase");
}

#[test]
fn strength_phases_carry_only_strength_purposes() {
    let text = edited(
        SKELETON_YAML,
        "purposes: [strength_maint] }",
        "purposes: [strength_maint, tempo] }",
    );
    let (key, message) = violation(SkeletonTemplate::from_yaml(&text));
    assert_eq!(key, "strength.build.purposes");
    assert_eq!(
        message,
        "\"tempo\" is not a strength purpose; allowed: mobility, plyometric, strength_aa, strength_maint, strength_max"
    );
    let text = edited(
        SKELETON_YAML,
        "purposes: [strength_max, plyometric] }",
        "purposes: [] }",
    );
    let (key, message) = violation(SkeletonTemplate::from_yaml(&text));
    assert_eq!(key, "strength.base.purposes");
    assert_eq!(message, "empty; name the strength purposes");
    let text = edited(
        SKELETON_YAML,
        "recovery_week_cut: { min: 0.25, max: 0.35 }",
        "recovery_week_cut: { min: 0.25, max: 1.35 }",
    );
    let (key, message) = violation(SkeletonTemplate::from_yaml(&text));
    assert_eq!(key, "recovery_week_cut.max");
    assert_eq!(message, "share 1.35 outside 0..=1");
}

#[test]
fn a_strength_template_prescribes_a_load_not_an_rpe() {
    // A template declared strength but shaped like an endurance one — an
    // RPE band and anchors, no load — fails on the load it lacks.
    let text = edited(
        WORKOUT_TOML,
        r#"purpose = "vo2max_long""#,
        r#"purpose = "strength_max""#,
    );
    let (key, message) = violation(WorkoutTemplate::from_toml(&text));
    assert_eq!(key, "params.load");
    assert_eq!(
        message,
        "required and non-empty for strength purpose strength_max"
    );
    let text = edited(&text, r"rpe = { min = 7, max = 8 }", r#"load = "85% 1RM""#);
    let (key, message) = violation(WorkoutTemplate::from_toml(&text));
    assert_eq!(key, "sport");
    assert_eq!(
        message,
        "strength purpose strength_max requires sport = \"strength_training\", got \"ride\""
    );
    let text = edited(
        STRENGTH_TOML,
        "sport_variants = []",
        r#"sport_variants = ["strength_training"]"#,
    );
    let (key, message) = violation(WorkoutTemplate::from_toml(&text));
    assert_eq!(key, "sport_variants");
    assert_eq!(message, "must be empty for strength purpose strength_max");
    let text = edited(
        STRENGTH_TOML,
        r#"load = "85-90% 1RM or heavier; 4 × 4RM is the canonical dose""#,
        r#"load = "  ""#,
    );
    let (key, _) = violation(WorkoutTemplate::from_toml(&text));
    assert_eq!(key, "params.load", "a blank load is no load");
}

#[test]
fn an_endurance_template_needs_an_rpe_and_an_anchor_per_sport_in_play() {
    let text = edited(WORKOUT_TOML, "run = \"Z5\"\n", "");
    let (key, message) = violation(WorkoutTemplate::from_toml(&text));
    assert_eq!(key, "params.intensity.run");
    assert_eq!(
        message,
        "missing or empty intensity anchor for a sport in play"
    );
    let text = edited(WORKOUT_TOML, "rpe = { min = 7, max = 8 }", "");
    let (key, message) = violation(WorkoutTemplate::from_toml(&text));
    assert_eq!(key, "params.rpe");
    assert_eq!(message, "required for endurance purpose vo2max_long");
    let text = edited(
        WORKOUT_TOML,
        r#"sport_variants = ["ride", "run"]"#,
        "sport_variants = []",
    );
    let workout = WorkoutTemplate::from_toml(&text).unwrap();
    assert!(
        workout.sport_variants.is_empty(),
        "the primary alone is in play"
    );
    let text = edited(
        WORKOUT_TOML,
        r#"sport_variants = ["ride", "run"]"#,
        r#"sport_variants = ["run"]"#,
    );
    let (key, message) = violation(WorkoutTemplate::from_toml(&text));
    assert_eq!(key, "sport_variants");
    assert_eq!(
        message,
        "primary sport \"ride\" is not listed among the variants"
    );
}

#[test]
fn every_anchor_is_in_the_intensity_grammar() {
    let text = edited(
        WORKOUT_TOML,
        r#"ride = "100-105%""#,
        r#"ride = "comfortably hard""#,
    );
    let (key, message) = violation(WorkoutTemplate::from_toml(&text));
    assert_eq!(key, "params.intensity.ride");
    assert!(
        message.starts_with("\"comfortably hard\" is not in the intensity grammar"),
        "{message}"
    );
    let text = edited(
        WORKOUT_TOML,
        "run = \"Z5\"\n",
        "run = \"Z5\"\nswim = \"250w\"\n",
    );
    let (key, _) = violation(WorkoutTemplate::from_toml(&text));
    assert_eq!(
        key, "params.intensity.swim",
        "an anchor outside the sports in play is still checked"
    );
}

#[test]
fn an_other_sport_is_refused_everywhere_it_can_appear() {
    let mut workout = workout();
    workout.sport = SportType::Other("unicycle".to_owned());
    let error = workout.validate_catalogue().unwrap_err();
    assert_eq!(
        error,
        CatalogueValidationError::Workout {
            slug: "vo2max_4x8".to_owned(),
            key: "sport".to_owned(),
            message: "\"unicycle\" is not a catalogue sport".to_owned(),
        }
    );
    assert_eq!(
        error.to_string(),
        "workout 'vo2max_4x8': sport: \"unicycle\" is not a catalogue sport"
    );
    let mut workout = self::workout();
    workout
        .sport_variants
        .push(SportType::Other("unicycle".to_owned()));
    let error = workout.validate_catalogue().unwrap_err();
    assert!(
        matches!(error, CatalogueValidationError::Workout { ref key, .. } if key == "sport_variants[2]")
    );
    let mut workout = self::workout();
    workout
        .params
        .intensity
        .insert(SportType::Other("unicycle".to_owned()), "Z2".to_owned());
    let error = workout.validate_catalogue().unwrap_err();
    assert!(
        matches!(error, CatalogueValidationError::Workout { ref key, .. } if key == "params.intensity.unicycle")
    );
    assert!(
        serde_json::to_string(&workout.params).is_err(),
        "an Other key is unserializable, so validate_catalogue is the gate"
    );
    assert!(
        serde_json::to_string(&self::workout().params).is_ok(),
        "named sport keys serialize"
    );
}

#[test]
fn a_quality_template_keeps_a_day_between_instances() {
    let text = edited(
        WORKOUT_TOML,
        "min_spacing_hours = 48",
        "min_spacing_hours = 12",
    );
    let (key, message) = violation(WorkoutTemplate::from_toml(&text));
    assert_eq!(key, "fit.min_spacing_hours");
    assert_eq!(message, "12 < 24 while readiness_min is p2");
    let text = edited(&text, r#"readiness_min = "p2""#, r#"readiness_min = "p1""#);
    let workout = WorkoutTemplate::from_toml(&text).unwrap();
    assert_eq!(
        workout.fit.min_spacing_hours, 12,
        "a p1 template may repeat inside a day"
    );
    let text = edited(WORKOUT_TOML, r#"phases = ["build", "peak"]"#, "phases = []");
    let (key, message) = violation(WorkoutTemplate::from_toml(&text));
    assert_eq!(key, "fit.phases");
    assert_eq!(message, "empty; name the phase kinds the template fits");
}

#[test]
fn ranges_are_ordered_and_reps_need_levers() {
    let text = edited(
        WORKOUT_TOML,
        "reps = { min = 4, max = 5, default = 4 }",
        "reps = { min = 4, max = 5, default = 6 }",
    );
    let (key, message) = violation(WorkoutTemplate::from_toml(&text));
    assert_eq!(key, "params.reps");
    assert_eq!(message, "default 6 > max 5");
    let text = edited(
        WORKOUT_TOML,
        "work_seconds = { min = 420, max = 600, default = 480 }",
        "work_seconds = { min = 500, max = 600, default = 480 }",
    );
    let (key, message) = violation(WorkoutTemplate::from_toml(&text));
    assert_eq!(key, "params.work_seconds");
    assert_eq!(message, "min 500 > default 480");
    let text = edited(
        WORKOUT_TOML,
        "rpe = { min = 7, max = 8 }",
        "rpe = { min = 7, max = 11 }",
    );
    let (key, message) = violation(WorkoutTemplate::from_toml(&text));
    assert_eq!(key, "params.rpe.max");
    assert_eq!(message, "RPE 11 outside 1..=10");
    let text = edited(
        WORKOUT_TOML,
        r#"order = ["add_rep", "lengthen_rep", "shorten_rest", "raise_intensity"]"#,
        "order = []",
    );
    let (key, message) = violation(WorkoutTemplate::from_toml(&text));
    assert_eq!(key, "progression.order");
    assert_eq!(
        message,
        "empty while params.reps is a range; name the levers that grow it"
    );
}

#[test]
fn a_catalogue_file_never_writes_is_compiled_in() {
    let text = edited(
        WORKOUT_TOML,
        r#"slug = "vo2max_4x8""#,
        "slug = \"vo2max_4x8\"\nis_compiled_in = true",
    );
    let (key, message) = violation(WorkoutTemplate::from_toml(&text));
    assert_eq!(key, "is_compiled_in");
    assert_eq!(
        message,
        "never written in a catalogue file; the parser sets it"
    );
    let text = edited(
        WORKOUT_TOML,
        "duration_seconds = 480.0",
        "duration_seconds = 480.5",
    );
    let message = parse_message(WorkoutTemplate::from_toml(&text));
    assert!(
        message.contains("expected a whole number between 0 and 4294967295, got 480.5"),
        "{message}"
    );
    let message = parse_message(WorkoutTemplate::from_toml("this is not toml = = ="));
    assert!(!message.is_empty());
}

#[test]
fn selection_rows_carry_allowed_values_once_and_say_something() {
    let text = edited(SELECTION_YAML, "value: under_4", "value: under_5");
    let (key, message) = violation(SelectionTable::from_yaml(&text));
    assert_eq!(key, "rows[0].value");
    assert_eq!(
        message,
        "\"under_5\" is not allowed for hours_tier; allowed: under_4, from_4_to_6, from_6_to_10, over_10"
    );
    let text = edited(SELECTION_YAML, "weight: 3", "weight: 6");
    let (key, message) = violation(SelectionTable::from_yaml(&text));
    assert_eq!(key, "rows[0].prefer[0].weight");
    assert_eq!(message, "6 outside 1..=5");
    let text = edited(
        SELECTION_YAML,
        "prefer: [ { id: hvlit-foundation, weight: 3 } ]",
        "prefer: null",
    );
    let text = edited(
        &text,
        r#"exclude: [ { id: norwegian-threshold-density, reason: "needs eight or more sessions a week" } ]"#,
        "exclude: []",
    );
    let (key, message) = violation(SelectionTable::from_yaml(&text));
    assert_eq!(key, "rows[0]");
    assert_eq!(
        message,
        "neither prefer nor exclude; a row must say something"
    );
    let twice = format!(
        "{SELECTION_YAML}{}",
        SELECTION_YAML.trim_start_matches("rows:\n")
    );
    let (key, message) = violation(SelectionTable::from_yaml(&twice));
    assert_eq!(key, "rows[1]");
    assert_eq!(message, "(hours_tier, under_4) appears twice");
    let text = edited(SELECTION_YAML, "input: hours_tier", "input: shoe_size");
    let message = parse_message(SelectionTable::from_yaml(&text));
    assert!(message.contains("unknown variant `shoe_size`"), "{message}");
}

#[test]
fn a_word_outside_its_vocabulary_is_a_parse_error_naming_the_field() {
    let text = edited(FLAVOUR_YAML, "family: polarized", "family: bogus");
    let message = parse_message(Flavour::from_yaml(&text));
    assert!(
        message.starts_with("family: unknown variant `bogus`"),
        "{message}"
    );
    let text = edited(SKELETON_YAML, "  - kind: base", "  - kind: warmup");
    let message = parse_message(SkeletonTemplate::from_yaml(&text));
    assert!(
        message.starts_with("phases[0].kind: unknown variant `warmup`"),
        "{message}"
    );
    let text = edited(
        WORKOUT_TOML,
        r#"order = ["add_rep","#,
        r#"order = ["add_more","#,
    );
    let message = parse_message(WorkoutTemplate::from_toml(&text));
    assert!(message.contains("unknown variant `add_more`"), "{message}");
    assert!(
        message.contains("line") && message.contains("add_more"),
        "a broken lever is reported with its line and the offending text: {message}"
    );
    let text = edited(WORKOUT_TOML, r#"purpose = "vo2max_long""#, "");
    let message = parse_message(WorkoutTemplate::from_toml(&text));
    assert!(message.contains("missing field `purpose`"), "{message}");
}

// ============================================================================
// Vocabularies
// ============================================================================

#[test]
fn every_vocabulary_serde_name_is_its_as_str() {
    serde_names_match(WorkoutPurpose::ALL);
    serde_names_match(PhaseKind::ALL);
    serde_names_match(ReadinessLevel::ALL);
    serde_names_match(FlavourFamily::ALL);
    serde_names_match(Sequencing::ALL);
    serde_names_match(Modifier::ALL);
    serde_names_match(Measurement::ALL);
    serde_names_match(Contraindication::ALL);
    serde_names_match(ProgressionLever::ALL);
    serde_names_match(EventClass::ALL);
    serde_names_match(HoursTier::ALL);
    serde_names_match(EvidenceTier::ALL);
    serde_names_match(StrengthGoal::ALL);
    serde_names_match(InputDimension::ALL);
    serde_names_match(IntensityDistribution::ALL);
    assert_eq!(WorkoutPurpose::ALL.len(), 17);
    assert_eq!(EventClass::ALL.len(), 15);
    assert_eq!(HoursTier::Under4.as_str(), "under_4");
    assert_eq!(EventClass::Run5k.to_string(), "run_5k");
    assert_eq!(HoursTier::From6To10.to_string(), "from_6_to_10");
    assert!(ReadinessLevel::P0 < ReadinessLevel::P3);
}

#[test]
fn purposes_split_into_strength_and_quality() {
    let strength: Vec<WorkoutPurpose> = WorkoutPurpose::ALL
        .iter()
        .copied()
        .filter(|purpose| purpose.is_strength())
        .collect();
    assert_eq!(
        strength,
        vec![
            WorkoutPurpose::StrengthAa,
            WorkoutPurpose::StrengthMax,
            WorkoutPurpose::StrengthMaint,
            WorkoutPurpose::Plyometric,
            WorkoutPurpose::Mobility
        ]
    );
    let easy: Vec<WorkoutPurpose> = WorkoutPurpose::ALL
        .iter()
        .copied()
        .filter(|purpose| !purpose.is_quality())
        .collect();
    assert_eq!(
        easy,
        vec![
            WorkoutPurpose::Recovery,
            WorkoutPurpose::Endurance,
            WorkoutPurpose::EnduranceLong,
            WorkoutPurpose::StrengthAa,
            WorkoutPurpose::StrengthMaint,
            WorkoutPurpose::Mobility
        ]
    );
    assert!(WorkoutPurpose::StrengthMax.is_quality());
    assert!(EvidenceTier::Rct.requires_citation());
    assert!(!EvidenceTier::Grey.requires_citation());
    assert!(!EvidenceTier::CoachJudgement.requires_citation());
}

#[test]
fn input_dimensions_know_their_allowed_values() {
    assert_eq!(
        InputDimension::HoursTier.allowed_values(),
        &["under_4", "from_4_to_6", "from_6_to_10", "over_10"]
    );
    let event_classes: Vec<&str> = EventClass::ALL.iter().map(|class| class.as_str()).collect();
    assert_eq!(
        InputDimension::EventClass.allowed_values(),
        event_classes.as_slice()
    );
    assert_eq!(
        InputDimension::Measurement.allowed_values(),
        &["lactate", "power", "pace", "hr", "rpe"]
    );
    assert_eq!(
        InputDimension::TrainingAge.allowed_values(),
        &["novice", "recreational", "trained", "elite"]
    );
    assert_eq!(
        InputDimension::WeeksToGoal.allowed_values(),
        &["under_8", "from_8_to_16", "over_16"]
    );
    assert_eq!(
        InputDimension::RecoverySpeed.allowed_values(),
        &["fast", "typical", "limited"]
    );
    assert_eq!(
        InputDimension::InjuryLoad.allowed_values(),
        &["none", "last_12_months"]
    );
    assert_eq!(
        InputDimension::IntervalExperience.allowed_values(),
        &["none", "some", "two_seasons"]
    );
    assert_eq!(
        InputDimension::SportMix.allowed_values(),
        &["running", "cycling", "triathlon", "swimming", "mixed"]
    );
    assert_eq!(
        InputDimension::SeasonPhase.allowed_values(),
        &["off_season", "base", "pre_competition", "competition"]
    );
}

// ============================================================================
// RelativeIntensity grammar
// ============================================================================

#[test]
fn the_intensity_grammar_is_closed() {
    use RelativeIntensity::{HeartRateZone, Percent, SweetSpot, Zone};
    let parsed = |s: &str| RelativeIntensity::parse(s);
    assert_eq!(parsed("Z2"), Some(Zone(2)));
    assert_eq!(parsed("Z5"), Some(Zone(5)));
    assert_eq!(parsed(" zone 4 "), Some(Zone(4)));
    assert_eq!(parsed("Z2 HR"), Some(HeartRateZone(2)));
    assert_eq!(parsed("Tempo"), Some(Zone(3)));
    assert_eq!(parsed("threshold"), Some(Zone(4)));
    assert_eq!(parsed("VO2max"), Some(Zone(5)));
    assert_eq!(parsed("sweet spot"), Some(SweetSpot));
    assert_eq!(parsed("75%"), Some(Percent { low: 75, high: 75 }));
    assert_eq!(
        parsed("100-105%"),
        Some(Percent {
            low: 100,
            high: 105
        })
    );
    assert_eq!(parsed("88-93% FTP"), Some(Percent { low: 88, high: 93 }));
    // A pace-family label names what the sport already decides, and an en
    // dash between a band's bounds is the hyphen a keyboard offered.
    assert_eq!(parsed("Z2 pace"), Some(Zone(2)));
    assert_eq!(parsed("zone 3 Pace"), Some(Zone(3)));
    assert_eq!(
        parsed("88\u{2013}93% FTP"),
        Some(Percent { low: 88, high: 93 })
    );
    // Outside the grammar: structure, inverted bands, absolute watts, prose.
    assert_eq!(parsed("3x8min @ 88-93% FTP"), None);
    assert_eq!(parsed("93-88%"), None);
    assert_eq!(parsed("250w"), None);
    assert_eq!(parsed("Z9"), None);
    assert_eq!(parsed("comfortably hard"), None);
    assert_eq!(parsed(""), None);
}

// ============================================================================
// WorkoutFilter, inline defaults, LoadingPattern, evidence refs
// ============================================================================

#[test]
fn a_filter_matches_on_purpose_phase_and_any_sport_in_play() {
    let workout = workout();
    let any = WorkoutFilter::default();
    assert!(any.matches(&workout));
    let filter = WorkoutFilter {
        purpose: Some(WorkoutPurpose::Vo2maxLong),
        phase: Some(PhaseKind::Peak),
        sport: Some(SportType::Run),
    };
    assert!(filter.matches(&workout), "a variant sport matches");
    let filter = WorkoutFilter {
        sport: Some(SportType::Swim),
        ..WorkoutFilter::default()
    };
    assert!(!filter.matches(&workout));
    let filter = WorkoutFilter {
        phase: Some(PhaseKind::Taper),
        ..WorkoutFilter::default()
    };
    assert!(!filter.matches(&workout));
    let mut anywhere = workout.clone();
    anywhere.fit.phases.clear();
    assert!(
        filter.matches(&anywhere),
        "empty fit.phases fits every phase"
    );
    let filter = WorkoutFilter {
        purpose: Some(WorkoutPurpose::Recovery),
        ..WorkoutFilter::default()
    };
    assert!(!filter.matches(&workout));
}

#[test]
fn inline_defaults_map_a_distribution_to_a_purpose_and_a_floor() {
    assert_eq!(
        WorkoutTemplate::inline_defaults(IntensityDistribution::Recovery),
        (WorkoutPurpose::Recovery, ReadinessLevel::P0)
    );
    assert_eq!(
        WorkoutTemplate::inline_defaults(IntensityDistribution::Polarized),
        (WorkoutPurpose::Endurance, ReadinessLevel::P1)
    );
    assert_eq!(
        WorkoutTemplate::inline_defaults(IntensityDistribution::Pyramid),
        (WorkoutPurpose::Tempo, ReadinessLevel::P1)
    );
    assert_eq!(
        WorkoutTemplate::inline_defaults(IntensityDistribution::Threshold),
        (WorkoutPurpose::Threshold, ReadinessLevel::P2)
    );
    assert_eq!(
        WorkoutTemplate::inline_defaults(IntensityDistribution::Vo2max),
        (WorkoutPurpose::Vo2maxLong, ReadinessLevel::P2)
    );
}

#[test]
fn a_loading_pattern_round_trips_through_its_string() {
    let pattern: LoadingPattern = "3:1".parse().unwrap();
    assert_eq!(pattern.load_weeks, 3);
    assert_eq!(pattern.recovery_weeks, 1);
    assert_eq!(pattern.to_string(), "3:1");
    assert_eq!(serde_json::to_string(&pattern).unwrap(), "\"3:1\"");
    let back: LoadingPattern = serde_json::from_str("\"3:1\"").unwrap();
    assert_eq!(back, pattern);
    let ten: LoadingPattern = "10:2".parse().unwrap();
    assert_eq!((ten.load_weeks, ten.recovery_weeks), (10, 2));
    for bad in ["0:1", "3", "a:b", "03:1", "3:", ":1", "3:1:1", ""] {
        let error = bad.parse::<LoadingPattern>().unwrap_err();
        assert_eq!(error.0, bad);
        assert!(error.to_string().contains("does not match"), "{bad}");
    }
    let error = serde_json::from_str::<LoadingPattern>("181").unwrap_err();
    assert!(error.to_string().contains("the number 181 in YAML 1.1"));
}

#[test]
fn evidence_refs_split_into_category_and_slug() {
    assert_eq!(
        evidence_ref_parts(
            "evidence/sports_science/training_prescription/seiler-2010-best-practice.md"
        ),
        Some(("training_prescription", "seiler-2010-best-practice"))
    );
    assert_eq!(
        evidence_ref_parts("evidence/sports_science/recovery/README.md"),
        None
    );
    assert_eq!(
        evidence_ref_parts("evidence/sports_science/README.md"),
        None
    );
    assert_eq!(evidence_ref_parts("evidence/recovery/x.md"), None);
    assert_eq!(
        evidence_ref_parts("evidence/sports_science/recovery/x.txt"),
        None
    );
    assert_eq!(
        evidence_ref_parts("evidence/sports_science/recovery/a/x.md"),
        None
    );
    assert_eq!(evidence_ref_parts("evidence/sports_science//x.md"), None);
    assert_eq!(evidence_ref_parts(""), None);
}

#[test]
fn unresolved_references_name_the_owner_the_key_and_the_ref() {
    let none_exist = |_: &str, _: &str| false;
    let all_exist = |_: &str, _: &str| true;
    assert!(flavour().unresolved_references(&all_exist).is_empty());
    assert_eq!(
        flavour().unresolved_references(&none_exist),
        vec![UnresolvedReference {
            owner: "flavour 'polarized-classic'".to_owned(),
            key: "evidence_refs[0]".to_owned(),
            reference: "evidence/sports_science/training_prescription/seiler-2010-best-practice.md"
                .to_owned(),
        }]
    );
    let only_taper = |category: &str, slug: &str| {
        category == "training_prescription" && slug == "bosquet-2007-taper-meta-analysis"
    };
    assert!(skeleton().unresolved_references(&only_taper).is_empty());
    let table = SelectionTable::from_yaml(SELECTION_YAML).unwrap();
    let unresolved = table.unresolved_references(&none_exist);
    assert_eq!(unresolved.len(), 1);
    assert_eq!(unresolved[0].owner, "selection table");
    assert_eq!(unresolved[0].key, "rows[0].evidence_refs[0]");
    assert_eq!(
        unresolved[0].to_string(),
        "selection table: rows[0].evidence_refs[0]: reference does not resolve: evidence/sports_science/training_prescription/festa-2019-polarized-vs-threshold-recreational.md"
    );
    let workout = workout();
    let unresolved = workout.unresolved_references(&none_exist);
    assert_eq!(unresolved[0].owner, "workout 'vo2max_4x8'");
}

#[test]
fn a_session_mix_purpose_needs_a_carrier_that_fits_its_phase() {
    let flavour = flavour();
    let owner = "flavour 'polarized-classic'";
    // The only race_specific carrier fits peak alone: the build mix names
    // it for build, and the p3 ladder names it for no phase at all.
    let peak_only = |phase: Option<PhaseKind>, purpose: WorkoutPurpose| {
        purpose != WorkoutPurpose::RaceSpecific || phase == Some(PhaseKind::Peak)
    };
    assert_eq!(
        flavour.unresolved_purposes(&peak_only),
        vec![
            UnresolvedReference {
                owner: owner.to_owned(),
                key: "session_mix.build.race_specific".to_owned(),
                reference: "race_specific".to_owned(),
            },
            UnresolvedReference {
                owner: owner.to_owned(),
                key: "readiness_substitution.p3.purposes[11]".to_owned(),
                reference: "race_specific".to_owned(),
            },
        ]
    );
    assert_eq!(
        flavour.unresolved_purposes(&peak_only)[0].to_string(),
        "flavour 'polarized-classic': session_mix.build.race_specific: reference does not resolve: race_specific"
    );
    let everywhere = |_: Option<PhaseKind>, _: WorkoutPurpose| true;
    assert!(flavour.unresolved_purposes(&everywhere).is_empty());
    // The registry's predicate: one WorkoutFilter match over its bank. The
    // spec bank carries vo2max_long for build and peak, and strength_max.
    let bank = [
        workout(),
        WorkoutTemplate::from_toml(STRENGTH_TOML).unwrap(),
    ];
    let carried = |phase: Option<PhaseKind>, purpose: WorkoutPurpose| {
        bank.iter().any(|template| {
            WorkoutFilter {
                purpose: Some(purpose),
                phase,
                sport: None,
            }
            .matches(template)
        })
    };
    let unresolved = flavour.unresolved_purposes(&carried);
    let keys: Vec<&str> = unresolved.iter().map(|entry| entry.key.as_str()).collect();
    assert!(
        keys.contains(&"session_mix.base.vo2max_long"),
        "vo2max_4x8 fits build and peak, not base: {keys:?}"
    );
    assert!(
        !keys.contains(&"session_mix.build.vo2max_long"),
        "vo2max_4x8 carries the build mix: {keys:?}"
    );
    assert!(
        !keys.contains(&"readiness_substitution.p2.purposes[6]"),
        "a ladder entry is carried from any phase: {keys:?}"
    );
    assert_eq!(unresolved.len(), 34, "{keys:?}");
}

#[test]
fn a_key_session_needs_a_carrier_that_fits_its_phase_and_sport() {
    let skeleton = skeleton();
    let owner = "skeleton 'marathon-linear'";
    // The only threshold carrier fits base alone: the build key session
    // goes uncarried.
    let base_only = |phase: Option<PhaseKind>, purpose: WorkoutPurpose, _: Option<&SportType>| {
        purpose != WorkoutPurpose::Threshold || phase == Some(PhaseKind::Base)
    };
    assert_eq!(
        skeleton.unresolved_purposes(&base_only),
        vec![UnresolvedReference {
            owner: owner.to_owned(),
            key: "phases[1].key_sessions[1]".to_owned(),
            reference: "threshold".to_owned(),
        }]
    );
    // The strength column is named for no phase and no sport.
    let phased = |phase: Option<PhaseKind>, _: WorkoutPurpose, sport: Option<&SportType>| {
        phase.is_some() && sport.is_none()
    };
    let keys: Vec<String> = skeleton
        .unresolved_purposes(&phased)
        .into_iter()
        .map(|entry| entry.key)
        .collect();
    assert_eq!(
        keys,
        vec![
            "strength.base.purposes[0]",
            "strength.base.purposes[1]",
            "strength.build.purposes[0]"
        ]
    );
    let everywhere = |_: Option<PhaseKind>, _: WorkoutPurpose, _: Option<&SportType>| true;
    assert!(skeleton.unresolved_purposes(&everywhere).is_empty());
    // An open-water skeleton asks for swim on every key session; a run-only
    // threshold carrier answers the marathon skeleton and not that one.
    let text = edited(
        SKELETON_YAML,
        "event_classes: [marathon]",
        "event_classes: [open_water_swim]",
    );
    let open_water = SkeletonTemplate::from_yaml(&text).unwrap();
    let text = edited(
        WORKOUT_TOML,
        r#"purpose = "vo2max_long""#,
        r#"purpose = "threshold""#,
    );
    let text = edited(&text, r#"sport = "ride""#, r#"sport = "run""#);
    let text = edited(
        &text,
        r#"sport_variants = ["ride", "run"]"#,
        r#"sport_variants = ["run"]"#,
    );
    let bank = [WorkoutTemplate::from_toml(&text).unwrap()];
    let carried = |phase: Option<PhaseKind>, purpose: WorkoutPurpose, sport: Option<&SportType>| {
        bank.iter().any(|template| {
            WorkoutFilter {
                purpose: Some(purpose),
                phase,
                sport: sport.cloned(),
            }
            .matches(template)
        })
    };
    let marathon_keys: Vec<String> = skeleton
        .unresolved_purposes(&carried)
        .into_iter()
        .map(|entry| entry.key)
        .collect();
    assert_eq!(marathon_keys.len(), 11, "{marathon_keys:?}");
    assert!(
        !marathon_keys.contains(&"phases[1].key_sessions[1]".to_owned()),
        "a run carrier answers a marathon key session: {marathon_keys:?}"
    );
    let open_water_unresolved = open_water.unresolved_purposes(&carried);
    assert_eq!(open_water_unresolved.len(), 12);
    let threshold = open_water_unresolved
        .iter()
        .find(|entry| entry.key == "phases[1].key_sessions[1]")
        .expect("a run-only carrier does not answer an open-water key session");
    assert_eq!(threshold.owner, owner);
    assert_eq!(threshold.reference, "threshold");
}

#[test]
fn a_selection_id_needs_a_flavour_file() {
    let table = SelectionTable::from_yaml(SELECTION_YAML).unwrap();
    let only_hvlit = |id: &str| id == "hvlit-foundation";
    let unresolved = table.unresolved_flavours(&only_hvlit);
    assert_eq!(
        unresolved,
        vec![UnresolvedReference {
            owner: "selection table".to_owned(),
            key: "rows[0].exclude[0].id".to_owned(),
            reference: "norwegian-threshold-density".to_owned(),
        }]
    );
    assert_eq!(
        unresolved[0].to_string(),
        "selection table: rows[0].exclude[0].id: reference does not resolve: norwegian-threshold-density"
    );
    let none = |_: &str| false;
    let keys: Vec<String> = table
        .unresolved_flavours(&none)
        .into_iter()
        .map(|entry| entry.key)
        .collect();
    assert_eq!(keys, vec!["rows[0].prefer[0].id", "rows[0].exclude[0].id"]);
    let all = |_: &str| true;
    assert!(table.unresolved_flavours(&all).is_empty());
}
