// ABOUTME: Laying a skeleton onto a calendar — the runway cases the plan names as acceptance criteria
// ABOUTME: A short season drops base first and keeps the taper; too short refuses instead of compressing
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use chrono::NaiveDate;
use dravr_cageux::periodization::{
    build_skeleton, FlavourFamily, LaidPhase, PhaseKind, SeasonLayout, SkeletonTemplate,
};

/// The marathon skeleton from the spec: base, build, specialty, taper, with
/// base shrinking first and the taper fixed at fourteen days.
const SKELETON_YAML: &str = r#"id: marathon-linear
event_classes: [marathon]
hours_tiers: [from_4_to_6, from_6_to_10, over_10]
min_weeks: 12
phases:
  - kind: base
    purpose: "Aerobic base and durability."
    length: { share_of_weeks_to_goal: 0.40, min_weeks: 6, max_weeks: 16 }
    volume_share_of_peak: { min: 0.40, max: 0.60 }
    flavour_override: pyramidal
    key_sessions: [endurance_long, tempo, neuromuscular]
  - kind: build
    purpose: "Threshold and endurance while volume climbs."
    length: { share_of_weeks_to_goal: 0.25, min_weeks: 2, max_weeks: 8 }
    volume_share_of_peak: { min: 0.60, max: 1.00 }
    key_sessions: [endurance_long, threshold]
  - kind: specialty
    purpose: "Race specificity."
    length: { share_of_weeks_to_goal: 0.20, min_weeks: 2, max_weeks: 8 }
    volume_share_of_peak: { min: 0.80, max: 0.90 }
    key_sessions: [race_specific, endurance_long]
  - kind: taper
    purpose: "Volume down, sharpness kept."
    length: { fixed_days: 14 }
    volume_share_of_peak: { min: 0.40, max: 0.60 }
    key_sessions: [race_specific, endurance]
taper: { days: { min: 14, max: 21 }, volume_cut: { min: 0.40, max: 0.60 }, keep_intensity: true, keep_frequency: true }
loading_pattern: { default: "3:1", recovery_limited: "2:1" }
recovery_week_cut: { min: 0.25, max: 0.35 }
drop_order: [base, build, specialty]
multi_peak: { b_race_mini_taper_days: { min: 3, max: 5 }, transition_weeks_after_a_race: 2 }
strength: {}
evidence_refs:
  - evidence/sports_science/training_prescription/seiler-2010-best-practice.md
"#;

fn skeleton() -> SkeletonTemplate {
    SkeletonTemplate::from_yaml(SKELETON_YAML).expect("the spec skeleton parses")
}

fn d(s: &str) -> NaiveDate {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").expect("a date")
}

fn laid(layout: &SeasonLayout) -> &[LaidPhase] {
    match layout {
        SeasonLayout::Laid { phases, .. } => phases,
        SeasonLayout::NotEnoughRunway {
            needs_weeks,
            has_weeks,
        } => panic!("expected a season, got a refusal: needs {needs_weeks}, has {has_weeks}"),
    }
}

#[test]
fn a_full_runway_lays_base_build_specialty_taper_ending_on_the_race() {
    let race = d("2026-10-10");
    let layout = build_skeleton(&skeleton(), &[race], d("2026-04-25"), false);
    let phases = laid(&layout);

    assert_eq!(
        phases.iter().map(|p| p.kind).collect::<Vec<_>>(),
        vec![
            PhaseKind::Base,
            PhaseKind::Build,
            PhaseKind::Specialty,
            PhaseKind::Taper
        ],
        "season order, base first"
    );
    let last = phases.last().expect("a taper");
    assert_eq!(
        last.end_exclusive(),
        race,
        "the last phase ends on the race"
    );
    assert_eq!(last.weeks, 2, "fourteen days is two weeks");
    assert!(
        phases
            .windows(2)
            .all(|w| w[0].end_exclusive() == w[1].start),
        "phases abut with no gap and no overlap"
    );
    assert_eq!(
        phases[0].flavour_override,
        Some(FlavourFamily::Pyramidal),
        "the base carries its own family"
    );
}

#[test]
fn a_tight_runway_shrinks_in_drop_order_and_never_the_taper() {
    let race = d("2026-10-10");
    // Thirteen weeks: above the skeleton's floor of twelve, but the authored
    // lengths sum to fourteen, so exactly one week has to come out.
    let layout = build_skeleton(&skeleton(), &[race], d("2026-07-11"), false);
    let phases = laid(&layout);

    let taper = phases
        .iter()
        .find(|p| p.kind == PhaseKind::Taper)
        .expect("the taper survives");
    assert_eq!(taper.weeks, 2, "the taper is never shortened");
    assert_eq!(taper.end_exclusive(), race, "and it still ends on the race");

    let total: u32 = phases.iter().map(|p| u32::from(p.weeks)).sum();
    assert!(total <= 13, "the season fits the runway, got {total}");

    // `drop_order` is [base, build, specialty], but base's own floor is six
    // weeks and its 0.40 share only reaches six at this runway — so base is
    // already at its floor and build is the first phase with room to give.
    // The invariant is "the earliest phase in drop_order that is above its
    // floor", not "always base".
    match &layout {
        SeasonLayout::Laid { shrunk, .. } => {
            assert_eq!(
                shrunk,
                &vec![PhaseKind::Build],
                "build gives the week, because base has none to give"
            );
            assert!(
                !shrunk.contains(&PhaseKind::Taper),
                "the taper is never in the shrink set"
            );
        }
        SeasonLayout::NotEnoughRunway { .. } => panic!("thirteen weeks clears the floor of twelve"),
    }

    let base = phases
        .iter()
        .find(|p| p.kind == PhaseKind::Base)
        .expect("the base survives");
    assert_eq!(
        base.weeks, 6,
        "held at its own floor rather than cut below it"
    );
}

#[test]
fn a_runway_under_the_floor_refuses_instead_of_compressing() {
    let race = d("2026-10-10");
    let layout = build_skeleton(&skeleton(), &[race], d("2026-09-20"), false);
    match layout {
        SeasonLayout::NotEnoughRunway {
            needs_weeks,
            has_weeks,
        } => {
            assert!(has_weeks < u32::from(needs_weeks));
            assert!(
                needs_weeks >= 12,
                "the floor is at least the declared min_weeks"
            );
        }
        SeasonLayout::Laid { phases, .. } => {
            panic!("three weeks is not a marathon season: {:?}", phases.len())
        }
    }
}

#[test]
fn two_a_races_get_a_transition_between_them() {
    let first = d("2026-06-13");
    let second = d("2026-08-22"); // ten weeks later
    let layout = build_skeleton(&skeleton(), &[first, second], d("2026-01-10"), false);
    let phases = laid(&layout);

    let at = phases
        .iter()
        .position(|p| p.kind == PhaseKind::Transition)
        .expect("a transition after the first race");
    assert_eq!(phases[at].weeks, 2, "transition_weeks_after_a_race");
    assert_eq!(
        phases[at].start, first,
        "the transition begins the day the first race is run"
    );
    assert!(
        phases[..at].iter().all(|p| p.peak == first),
        "everything before it is aimed at the first race"
    );
    assert!(
        phases[at + 1..].iter().all(|p| p.peak == second),
        "everything after it is aimed at the second"
    );
    assert_eq!(
        phases.last().expect("a taper").end_exclusive(),
        second,
        "the season ends on the second race"
    );
}

#[test]
fn the_recovery_limited_arm_changes_the_loading_pattern() {
    let race = d("2026-10-10");
    let typical = build_skeleton(&skeleton(), &[race], d("2026-04-25"), false);
    let limited = build_skeleton(&skeleton(), &[race], d("2026-04-25"), true);
    assert_eq!(laid(&typical)[0].loading_pattern.to_string(), "3:1");
    assert_eq!(laid(&limited)[0].loading_pattern.to_string(), "2:1");
}

#[test]
fn a_season_with_no_race_is_a_refusal_rather_than_an_empty_plan() {
    let layout = build_skeleton(&skeleton(), &[], d("2026-04-25"), false);
    assert!(matches!(
        layout,
        SeasonLayout::NotEnoughRunway { has_weeks: 0, .. }
    ));
}

#[test]
fn a_long_runway_is_spent_forward_rather_than_left_as_a_wait() {
    // Fifty-two weeks: a full annual season, which is the whole subject of
    // this feature. The authored shares only reach thirty-four weeks, so the
    // remainder must be spent rather than stranded before the start.
    let race = d("2026-10-10");
    let today = race - chrono::Duration::weeks(52);
    let layout = build_skeleton(&skeleton(), &[race], today, false);
    let phases = laid(&layout);

    assert_eq!(
        phases.first().expect("a first phase").start,
        today,
        "the plan begins today, not after an eighteen-week wait"
    );
    assert_eq!(
        phases.last().expect("a taper").end_exclusive(),
        race,
        "and still ends on the race"
    );
    assert!(
        phases
            .windows(2)
            .all(|w| w[0].end_exclusive() == w[1].start),
        "with no gap anywhere in between"
    );

    let base = phases
        .iter()
        .find(|p| p.kind == PhaseKind::Base)
        .expect("a base");
    assert_eq!(
        base.weeks, 16,
        "the base is grown to its authored ceiling first"
    );

    // What the authored phases still cannot hold becomes general preparation.
    let prep = phases
        .iter()
        .find(|p| p.kind == PhaseKind::Prep)
        .expect("the remainder becomes prep, not a wait");
    assert_eq!(prep.start, today);
    assert!(prep.weeks > 0);

    let total: u32 = phases.iter().map(|p| u32::from(p.weeks)).sum();
    assert_eq!(total, 52, "every week of the runway is accounted for");
}

#[test]
fn the_taper_is_never_grown_to_absorb_spare_runway() {
    let race = d("2026-10-10");
    let layout = build_skeleton(
        &skeleton(),
        &[race],
        race - chrono::Duration::weeks(52),
        false,
    );
    let taper = laid(&layout)
        .iter()
        .find(|p| p.kind == PhaseKind::Taper)
        .expect("a taper")
        .clone();
    assert_eq!(
        taper.weeks, 2,
        "fourteen days is fourteen days however long the runway is"
    );
}
