// ABOUTME: The profile-to-flavour rule — eligibility before ranking, and every exclusion carrying a reason
// ABOUTME: Fixtures are shaped like the real catalogue rows they stand for, so each test affords its own question
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

//! What is asserted here is the rule's mechanics against fixtures it can hold
//! in one screen. The four profile scenarios the plan sets as acceptance
//! criteria are content tests over the *real* catalogue, which lives in
//! dravr-contremaitre rather than in this crate, so they belong to the platform
//! side where the seeded registry can be read.

use std::collections::BTreeSet;

use dravr_cageux::periodization::{
    select_flavour, Confidence, Flavour, FlavourInputs, HoursTier, InjuryLoad, IntervalExperience,
    Measurement, RecoverySpeed, SelectionTable, SportMix, TrainingAge,
};

/// A flavour with the knobs a test needs, everything else held constant.
fn flavour(
    id: &str,
    min_hours: f32,
    min_sessions: u8,
    measurement: &str,
    contraindications: &str,
) -> Flavour {
    let yaml = format!(
        r#"id: {id}
family: polarized
sequencing: linear
modifiers: []
evidence_tier: rct
caveat: null
tid_targets:
  base:  {{ z1: {{ min: 0.80, max: 0.90 }}, z2: {{ min: 0.00, max: 0.05 }}, z3: {{ min: 0.10, max: 0.20 }} }}
  build: {{ z1: {{ min: 0.75, max: 0.85 }}, z2: {{ min: 0.00, max: 0.05 }}, z3: {{ min: 0.15, max: 0.20 }} }}
hard_sessions_per_week:
  min: 1
  max: 2
  recovery_limited_max: 1
min_spacing_hours_between_hard: {{ default: 48, recovery_limited: 72 }}
session_mix:
  base:  {{ endurance_long: 3, endurance: 4 }}
  build: {{ endurance_long: 2, endurance: 3 }}
prerequisites:
  min_hours_per_week: {min_hours}
  min_sessions_per_week: {min_sessions}
  measurement: {measurement}
  min_training_age_years: 1
contraindications: {contraindications}
loading_pattern: {{ default: "3:1", recovery_limited: "2:1" }}
readiness_substitution:
  p0: {{ purposes: [recovery], max_hard_sessions_per_week: 0 }}
  p1: {{ purposes: [recovery, endurance], max_hard_sessions_per_week: 0 }}
  p2: {{ purposes: [recovery, endurance, threshold], max_hard_sessions_per_week: 1 }}
  p3: {{ purposes: [recovery, endurance, threshold, vo2max_long], max_hard_sessions_per_week: 2 }}
max_weeks: null
evidence_refs:
  - evidence/sports_science/training_prescription/seiler-2010-best-practice.md
"#
    );
    Flavour::from_yaml(&yaml).unwrap_or_else(|e| panic!("fixture flavour {id} parses: {e}"))
}

/// An athlete who clears every prerequisite in the fixtures below.
fn athlete() -> FlavourInputs {
    FlavourInputs {
        hours_per_week: 8.0,
        sessions_per_week: 6,
        training_age_years: 4.0,
        training_age: TrainingAge::Trained,
        event_class: None,
        weeks_to_goal: None,
        measurements: BTreeSet::from([Measurement::Hr]),
        recovery_speed: RecoverySpeed::Typical,
        injury_load: InjuryLoad::None,
        interval_experience: IntervalExperience::TwoSeasons,
        sport_mix: SportMix::Running,
        season_phase: None,
        coach_preference: None,
    }
}

fn table(yaml: &str) -> SelectionTable {
    SelectionTable::from_yaml(yaml).expect("fixture selection table parses")
}

// ---------------------------------------------------------------------------
// Bands — the edges the data never states
// ---------------------------------------------------------------------------

#[test]
fn the_hours_bands_are_inclusive_floors() {
    let at = |h: f32| {
        let mut a = athlete();
        a.hours_per_week = h;
        a.hours_tier()
    };
    assert_eq!(at(3.9), HoursTier::Under4);
    assert_eq!(
        at(4.0),
        HoursTier::From4To6,
        "4.0 is the floor of from_4_to_6"
    );
    assert_eq!(at(5.9), HoursTier::From4To6);
    assert_eq!(
        at(6.0),
        HoursTier::From6To10,
        "6.0 is the floor of from_6_to_10"
    );
    assert_eq!(at(9.9), HoursTier::From6To10);
    assert_eq!(at(10.0), HoursTier::Over10, "10.0 is the floor of over_10");
}

// ---------------------------------------------------------------------------
// A missing device is effort, not a missing dimension
// ---------------------------------------------------------------------------

#[test]
fn an_athlete_with_no_device_is_read_as_steering_by_effort() {
    let mut a = athlete();
    a.measurements = BTreeSet::new();
    assert_eq!(
        a.effective_measurements(),
        BTreeSet::from([Measurement::Rpe])
    );
}

#[test]
fn a_lactate_flavour_is_refused_by_name_to_an_athlete_with_no_device() {
    let mut a = athlete();
    a.measurements = BTreeSet::new();
    let t = table(
        r#"rows:
  - input: measurement
    value: rpe
    prefer: []
    exclude:
      - { id: lactate-guided, reason: "needs a lactate meter, or power with heart rate" }
    tier: review
    evidence_refs: [evidence/sports_science/training_prescription/seiler-2010-best-practice.md]
"#,
    );
    let flavours = [flavour(
        "lactate-guided",
        10.0,
        8,
        "[[lactate], [power, hr]]",
        "[]",
    )];
    let verdict = select_flavour(&a, &t, &flavours);

    let out = verdict
        .exclusion("lactate-guided")
        .expect("the flavour is excluded, not merely out-scored");
    assert!(
        out.reasons
            .iter()
            .any(|r| r == "needs a lactate meter, or power with heart rate"),
        "the catalogue's own words reach the athlete: {:?}",
        out.reasons
    );
    assert!(verdict.ranked.is_empty(), "nothing eligible remains");
}

// ---------------------------------------------------------------------------
// Eligibility runs before ranking
// ---------------------------------------------------------------------------

#[test]
fn an_excluded_flavour_cannot_be_recovered_by_weight() {
    let t = table(
        r#"rows:
  - input: hours_tier
    value: from_6_to_10
    prefer:
      - { id: heavy, weight: 5 }
      - { id: light, weight: 1 }
    exclude:
      - { id: heavy, reason: "not for this athlete" }
    tier: rct
    evidence_refs: [evidence/sports_science/training_prescription/seiler-2010-best-practice.md]
"#,
    );
    let flavours = [
        flavour("heavy", 4.0, 4, "[[hr]]", "[]"),
        flavour("light", 4.0, 4, "[[hr]]", "[]"),
    ];
    let verdict = select_flavour(&athlete(), &t, &flavours);
    assert_eq!(
        verdict.top().map(|s| s.id.as_str()),
        Some("light"),
        "the 5-weight flavour is out; the 1-weight one wins"
    );
    assert!(verdict.exclusion("heavy").is_some());
}

#[test]
fn a_prerequisite_shortfall_states_the_number_the_athlete_missed() {
    let mut a = athlete();
    a.hours_per_week = 5.0;
    a.sessions_per_week = 3;
    let t = table(
        r"rows:
  - input: hours_tier
    value: from_4_to_6
    prefer:
      - { id: demanding, weight: 4 }
    exclude: []
    tier: rct
    evidence_refs: [evidence/sports_science/training_prescription/seiler-2010-best-practice.md]
",
    );
    let flavours = [flavour("demanding", 10.0, 8, "[[hr]]", "[]")];
    let verdict = select_flavour(&a, &t, &flavours);

    let out = verdict
        .exclusion("demanding")
        .expect("excluded on prerequisites");
    assert!(
        out.reasons
            .iter()
            .any(|r| r.contains("10") && r.contains("5.0")),
        "the hours shortfall names both numbers: {:?}",
        out.reasons
    );
    assert!(
        out.reasons
            .iter()
            .any(|r| r.contains('8') && r.contains('3')),
        "the session shortfall names both numbers: {:?}",
        out.reasons
    );
    assert!(verdict.ranked.is_empty());
}

#[test]
fn a_contraindication_the_profile_raises_removes_the_flavour() {
    let mut a = athlete();
    a.training_age = TrainingAge::Novice;
    let t = table(
        r"rows:
  - input: training_age
    value: novice
    prefer:
      - { id: sharp, weight: 4 }
    exclude: []
    tier: rct
    evidence_refs: [evidence/sports_science/training_prescription/seiler-2010-best-practice.md]
",
    );
    // The row prefers it and stays silent about the contraindication; the
    // flavour file is what refuses. A row that forgets cannot admit an athlete
    // the flavour itself forbids.
    let flavours = [flavour("sharp", 4.0, 4, "[[hr]]", "[novice_first_season]")];
    let verdict = select_flavour(&a, &t, &flavours);
    let out = verdict
        .exclusion("sharp")
        .expect("the flavour refuses a novice");
    assert!(
        out.reasons
            .iter()
            .any(|r| r.contains("novice_first_season")),
        "{:?}",
        out.reasons
    );
}

// ---------------------------------------------------------------------------
// The measurement branches are any-of over all-of
// ---------------------------------------------------------------------------

#[test]
fn a_two_device_branch_needs_both_devices() {
    let t = table(
        r"rows:
  - input: measurement
    value: power
    prefer:
      - { id: lactate-guided, weight: 3 }
    exclude: []
    tier: rct
    evidence_refs: [evidence/sports_science/training_prescription/seiler-2010-best-practice.md]
",
    );
    let flavours = [flavour(
        "lactate-guided",
        4.0,
        4,
        "[[lactate], [power, hr]]",
        "[]",
    )];

    let mut power_only = athlete();
    power_only.measurements = BTreeSet::from([Measurement::Power]);
    assert!(
        select_flavour(&power_only, &t, &flavours)
            .exclusion("lactate-guided")
            .is_some(),
        "power alone does not satisfy the [power, hr] branch"
    );

    let mut both = athlete();
    both.measurements = BTreeSet::from([Measurement::Power, Measurement::Hr]);
    let verdict = select_flavour(&both, &t, &flavours);
    assert!(
        verdict.exclusion("lactate-guided").is_none(),
        "power with heart rate satisfies the branch: {:?}",
        verdict.excluded
    );
    assert_eq!(verdict.top().map(|s| s.id.as_str()), Some("lactate-guided"));
}

// ---------------------------------------------------------------------------
// Scoring, provenance and the coach's pin
// ---------------------------------------------------------------------------

#[test]
fn weights_sum_across_dimensions_and_each_contribution_is_traceable() {
    let mut a = athlete();
    a.sport_mix = SportMix::Running;
    let t = table(
        r#"rows:
  - input: hours_tier
    value: from_6_to_10
    prefer:
      - { id: alpha, weight: 2 }
      - { id: beta, weight: 4 }
    exclude: []
    tier: meta_analysis
    evidence_refs: [evidence/sports_science/training_prescription/seiler-2010-best-practice.md]
    note: "the hours row"
  - input: sport_mix
    value: running
    prefer:
      - { id: alpha, weight: 5 }
    exclude: []
    tier: rct
    evidence_refs: [evidence/sports_science/training_prescription/seiler-2010-best-practice.md]
"#,
    );
    let flavours = [
        flavour("alpha", 4.0, 4, "[[hr]]", "[]"),
        flavour("beta", 4.0, 4, "[[hr]]", "[]"),
    ];
    let verdict = select_flavour(&a, &t, &flavours);

    let top = verdict.top().expect("a winner");
    assert_eq!(top.id, "alpha");
    assert_eq!(top.score, 7, "2 from hours + 5 from sport mix");
    assert_eq!(top.reasons.len(), 2, "one reason per row that spoke");
    assert_eq!(top.reasons[0].weight, 5, "heaviest contribution first");
    assert!(
        top.reasons
            .iter()
            .any(|r| r.note.as_deref() == Some("the hours row")),
        "the row's coach-voice note travels with the reason"
    );
    assert_eq!(verdict.ranked[1].score, 4);
}

#[test]
fn a_coach_pin_outranks_the_table_but_only_when_the_athlete_can_run_it() {
    let t = table(
        r"rows:
  - input: hours_tier
    value: from_6_to_10
    prefer:
      - { id: table-choice, weight: 5 }
      - { id: house-style, weight: 1 }
    exclude: []
    tier: rct
    evidence_refs: [evidence/sports_science/training_prescription/seiler-2010-best-practice.md]
",
    );
    let flavours = [
        flavour("table-choice", 4.0, 4, "[[hr]]", "[]"),
        flavour("house-style", 4.0, 4, "[[hr]]", "[]"),
    ];

    let mut pinned = athlete();
    pinned.coach_preference = Some("house-style".to_owned());
    let verdict = select_flavour(&pinned, &t, &flavours);
    assert_eq!(verdict.top().map(|s| s.id.as_str()), Some("house-style"));
    assert_eq!(verdict.coach_pinned.as_deref(), Some("house-style"));

    // Pinning something the athlete cannot run does not resurrect it.
    let mut impossible = athlete();
    impossible.coach_preference = Some("unavailable".to_owned());
    let verdict = select_flavour(&impossible, &t, &flavours);
    assert_eq!(verdict.coach_pinned, None);
    assert_eq!(verdict.top().map(|s| s.id.as_str()), Some("table-choice"));
}

#[test]
fn a_one_weight_margin_is_not_confidence() {
    let close = table(
        r"rows:
  - input: hours_tier
    value: from_6_to_10
    prefer:
      - { id: alpha, weight: 3 }
      - { id: beta, weight: 2 }
    exclude: []
    tier: rct
    evidence_refs: [evidence/sports_science/training_prescription/seiler-2010-best-practice.md]
",
    );
    let flavours = [
        flavour("alpha", 4.0, 4, "[[hr]]", "[]"),
        flavour("beta", 4.0, 4, "[[hr]]", "[]"),
    ];
    assert_eq!(
        select_flavour(&athlete(), &close, &flavours).confidence,
        Confidence::Low,
        "a single weight between the top two is a tie in everything but arithmetic"
    );
}
