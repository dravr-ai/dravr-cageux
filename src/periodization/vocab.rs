// ABOUTME: The closed vocabularies of the training catalogue (purposes, phases, families, tiers …) and its shared value shapes
// ABOUTME: Mirrors the VOCAB dict of dravr-contremaitre's check-training-catalogue.py; change both or neither
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

use std::fmt;
use std::str::FromStr;

use serde::de::{Error as DeError, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

use super::{Check, Violation};

/// A closed vocabulary: a `Copy` enum whose serde name is its `as_str`,
/// with `ALL` in declaration order and `Display` through `as_str`.
///
/// Each variant's serde name is the literal, not `rename_all =
/// "snake_case"`: serde's snake-casing writes `Under4` as `under4` and
/// `Run5k` as `run5k`, and the catalogue writes `under_4` and `run_5k`.
macro_rules! vocab_enum {
    (
        $(#[$meta:meta])*
        $name:ident {
            $( $(#[$vmeta:meta])* $variant:ident => $text:literal ),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        pub enum $name {
            $( $(#[$vmeta])* #[serde(rename = $text)] $variant, )+
        }

        impl $name {
            /// Every value, in declaration order.
            pub const ALL: &[Self] = &[ $( Self::$variant, )+ ];

            /// The `snake_case` name — byte for byte what serde reads and writes.
            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self {
                    $( Self::$variant => $text, )+
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
}
pub(crate) use vocab_enum;

/// The `as_str` names of a vocabulary as a static array, for
/// [`InputDimension::allowed_values`].
macro_rules! vocab_names {
    ($name:ident, $ty:ident) => {
        const $name: [&str; $ty::ALL.len()] = {
            let mut out = [""; $ty::ALL.len()];
            let mut i = 0;
            while i < out.len() {
                out[i] = $ty::ALL[i].as_str();
                i += 1;
            }
            out
        };
    };
}

vocab_enum! {
    /// What a session is for — the 17-word vocabulary every `session_mix`,
    /// `key_sessions`, `readiness_substitution` and workout `purpose` draws
    /// from (python `VOCAB["purpose"]`).
    WorkoutPurpose {
        /// Easy movement that costs nothing.
        Recovery => "recovery",
        /// Steady aerobic work below LT1.
        Endurance => "endurance",
        /// The long session of the week.
        EnduranceLong => "endurance_long",
        /// Sustained work between LT1 and LT2.
        Tempo => "tempo",
        /// The 88–94 % FTP band, long reps.
        SweetSpot => "sweet_spot",
        /// Work at or around LT2.
        Threshold => "threshold",
        /// Long intervals at ~90 % of maximum (4 × 8 min).
        Vo2maxLong => "vo2max_long",
        /// Short intervals above threshold (30/15, 30/30).
        Vo2maxShort => "vo2max_short",
        /// Maximal efforts with full recovery.
        Sprint => "sprint",
        /// Strides and hill sprints: form and recruitment, not fatigue.
        Neuromuscular => "neuromuscular",
        /// Race-pace work the goal event asks for.
        RaceSpecific => "race_specific",
        /// Bike-to-run transition session.
        Brick => "brick",
        /// Anatomical adaptation — light load, high reps, movement quality.
        StrengthAa => "strength_aa",
        /// Maximal strength — heavy load, low reps, full recovery.
        StrengthMax => "strength_max",
        /// In-season strength maintenance.
        StrengthMaint => "strength_maint",
        /// Plyometrics — jumps and contacts.
        Plyometric => "plyometric",
        /// Mobility and core.
        Mobility => "mobility",
    }
}

impl WorkoutPurpose {
    /// A gym purpose: the template prescribes a load, not an intensity anchor.
    #[must_use]
    pub const fn is_strength(self) -> bool {
        matches!(
            self,
            Self::StrengthAa
                | Self::StrengthMax
                | Self::StrengthMaint
                | Self::Plyometric
                | Self::Mobility
        )
    }

    /// A quality session — one that counts against the hard-session cap.
    #[must_use]
    pub const fn is_quality(self) -> bool {
        !matches!(
            self,
            Self::Recovery
                | Self::Endurance
                | Self::EnduranceLong
                | Self::Mobility
                | Self::StrengthAa
                | Self::StrengthMaint
        )
    }
}

vocab_enum! {
    /// The kind of a season phase (python `VOCAB["phase_kind"]`).
    PhaseKind {
        /// General preparation before the base.
        Prep => "prep",
        /// Aerobic base.
        Base => "base",
        /// Build toward the event's demands.
        Build => "build",
        /// Race-specific specialty block.
        Specialty => "specialty",
        /// Peak — the sharpest weeks before the taper.
        Peak => "peak",
        /// Taper into the goal.
        Taper => "taper",
        /// Race week.
        Race => "race",
        /// Transition after a goal.
        Transition => "transition",
        /// Planned recovery.
        Recovery => "recovery",
    }
}

vocab_enum! {
    /// Readiness level, lowest first (python `VOCAB["readiness_level"]`).
    ///
    /// P0 blocks everything but recovery; P1 is caution — endurance and light
    /// tempo; P2 maintains — one quality session, never two within 48 h; P3
    /// builds — two quality sessions per microcycle.
    ReadinessLevel {
        /// Block: recovery only.
        P0 => "p0",
        /// Caution: endurance and light tempo.
        P1 => "p1",
        /// Maintain: one quality session.
        P2 => "p2",
        /// Build: two quality sessions per microcycle.
        P3 => "p3",
    }
}

vocab_enum! {
    /// The intensity-distribution family a flavour belongs to (python `VOCAB["family"]`).
    FlavourFamily {
        /// ~80 / ≤5 / 15–20.
        Polarized => "polarized",
        /// 70–80 / 15–20 / 5–10.
        Pyramidal => "pyramidal",
        /// Threshold-dense, sub-threshold singles.
        Threshold => "threshold",
        /// Lactate-guided double-threshold days.
        LactateGuided => "lactate_guided",
        /// Time-crunched, interval-dense.
        HiitDense => "hiit_dense",
        /// High-volume low-intensity: ≥90 / ≤8 / ≤3.
        Hvlit => "hvlit",
    }
}

vocab_enum! {
    /// How the phases are sequenced across the season (python `VOCAB["sequencing"]`).
    Sequencing {
        /// Traditional linear progression.
        Linear => "linear",
        /// Concentrated blocks.
        Block => "block",
        /// Reverse periodization.
        Reverse => "reverse",
        /// Pyramidal base moving to a polarized build.
        PyramidalToPolarized => "pyramidal_to_polarized",
        /// Several goals in one season.
        MultiPeak => "multi_peak",
    }
}

vocab_enum! {
    /// A modifier laid over a flavour (python `VOCAB["modifier"]`).
    Modifier {
        /// Heat acclimation block.
        HeatBlock => "heat_block",
        /// Strength emphasis.
        StrengthEmphasis => "strength_emphasis",
        /// Durability block.
        DurabilityBlock => "durability_block",
        /// Altitude block.
        Altitude => "altitude",
        /// Fuelling progression toward race intake.
        FuellingProgression => "fuelling_progression",
        /// Race simulation sessions.
        RaceSimulation => "race_simulation",
    }
}

vocab_enum! {
    /// What the athlete can measure intensity with (python `VOCAB["measurement"]`).
    Measurement {
        /// Blood lactate.
        Lactate => "lactate",
        /// Power meter.
        Power => "power",
        /// Pace.
        Pace => "pace",
        /// Heart rate.
        Hr => "hr",
        /// Rating of perceived exertion.
        Rpe => "rpe",
    }
}

vocab_enum! {
    /// A reason a flavour or template does not fit an athlete (python `VOCAB["contraindication"]`).
    Contraindication {
        /// First season of structured training.
        NoviceFirstSeason => "novice_first_season",
        /// A current injury.
        AcuteInjury => "acute_injury",
        /// No lifting history.
        NoLiftingHistory => "no_lifting_history",
        /// Tendinopathy.
        Tendinopathy => "tendinopathy",
        /// Shoulder pain.
        ShoulderPain => "shoulder_pain",
        /// No interval experience.
        NoIntervalExperience => "no_interval_experience",
        /// Recovery-limited athlete.
        RecoveryLimited => "recovery_limited",
    }
}

vocab_enum! {
    /// A lever a template's progression pulls, in order (python `VOCAB["lever"]`).
    ProgressionLever {
        /// One more rep.
        AddRep => "add_rep",
        /// Longer reps.
        LengthenRep => "lengthen_rep",
        /// Shorter rest.
        ShortenRest => "shorten_rest",
        /// Higher intensity.
        RaiseIntensity => "raise_intensity",
        /// One more set.
        AddSet => "add_set",
        /// Longer session.
        LengthenDuration => "lengthen_duration",
        /// More ground contacts.
        AddContacts => "add_contacts",
        /// More load on the bar.
        AddLoad => "add_load",
    }
}

vocab_enum! {
    /// The goal event's class (python `VOCAB["event_class"]`).
    EventClass {
        /// 5 km run.
        Run5k => "run_5k",
        /// 10 km run.
        Run10k => "run_10k",
        /// Half marathon.
        HalfMarathon => "half_marathon",
        /// Marathon.
        Marathon => "marathon",
        /// Ultra.
        Ultra => "ultra",
        /// Criterium.
        Crit => "crit",
        /// Road race.
        RoadRace => "road_race",
        /// Time trial.
        TimeTrial => "time_trial",
        /// Gran fondo.
        GranFondo => "gran_fondo",
        /// Sprint triathlon.
        SprintTri => "sprint_tri",
        /// Olympic triathlon.
        OlympicTri => "olympic_tri",
        /// Half iron (70.3).
        HalfIron => "half_iron",
        /// Ironman.
        Ironman => "ironman",
        /// Open-water swim.
        OpenWaterSwim => "open_water_swim",
        /// No race on the horizon.
        NoRace => "no_race",
    }
}

vocab_enum! {
    /// Weekly hours available, on the knowledge base's decision-table bands
    /// (python `VOCAB["hours_tier"]`).
    HoursTier {
        /// Under four hours.
        Under4 => "under_4",
        /// Four to six hours.
        From4To6 => "from_4_to_6",
        /// Six to ten hours.
        From6To10 => "from_6_to_10",
        /// Over ten hours.
        Over10 => "over_10",
    }
}

vocab_enum! {
    /// Strength of the evidence behind a file or a row (python `VOCAB["tier"]`).
    EvidenceTier {
        /// Meta-analysis or systematic review with pooled effect.
        MetaAnalysis => "meta_analysis",
        /// Randomised controlled trial.
        Rct => "rct",
        /// Cohort or observational study.
        Cohort => "cohort",
        /// Case series.
        CaseSeries => "case_series",
        /// Narrative review or methods paper.
        Review => "review",
        /// Grey literature: practice, books, product docs.
        Grey => "grey",
        /// Coach judgement with no source behind it.
        CoachJudgement => "coach_judgement",
    }
}

impl EvidenceTier {
    /// python `UNCITED_TIERS`: grey and coach judgement may go uncited, and
    /// must carry a caveat instead; every other tier needs a proposition.
    #[must_use]
    pub const fn requires_citation(self) -> bool {
        !matches!(self, Self::Grey | Self::CoachJudgement)
    }
}

vocab_enum! {
    /// What a phase's strength work is for (python `VOCAB["strength_goal"]`).
    StrengthGoal {
        /// Anatomical adaptation.
        AnatomicalAdaptation => "anatomical_adaptation",
        /// Maximal strength.
        Max => "max",
        /// Maintenance.
        Maintain => "maintain",
        /// Explosive strength.
        Explosive => "explosive",
    }
}

vocab_enum! {
    /// A dimension of the athlete profile the selection table keys on
    /// (python `INPUT_DIMENSIONS`).
    InputDimension {
        /// Weekly hours band — [`HoursTier`].
        HoursTier => "hours_tier",
        /// Years of structured training.
        TrainingAge => "training_age",
        /// The goal's [`EventClass`].
        EventClass => "event_class",
        /// Weeks to the goal.
        WeeksToGoal => "weeks_to_goal",
        /// What the athlete measures with — [`Measurement`].
        Measurement => "measurement",
        /// How fast the athlete recovers.
        RecoverySpeed => "recovery_speed",
        /// Injury history.
        InjuryLoad => "injury_load",
        /// Interval experience.
        IntervalExperience => "interval_experience",
        /// The sport mix.
        SportMix => "sport_mix",
        /// Where in the season the athlete is.
        SeasonPhase => "season_phase",
    }
}

vocab_enum! {
    /// Years of structured training behind the athlete (python `VOCAB["training_age"]`).
    TrainingAge {
        /// A first season of structured training.
        Novice => "novice",
        /// Training consistently, without a competitive history.
        Recreational => "recreational",
        /// Several seasons of structured work.
        Trained => "trained",
        /// Competitive at a national level or above.
        Elite => "elite",
    }
}

vocab_enum! {
    /// How much runway is left to the goal (python `VOCAB["weeks_to_goal"]`).
    WeeksToGoal {
        /// Under eight weeks.
        Under8 => "under_8",
        /// Eight to sixteen weeks.
        From8To16 => "from_8_to_16",
        /// More than sixteen weeks.
        Over16 => "over_16",
    }
}

vocab_enum! {
    /// How fast the athlete recovers — the trigger for masters-style loading,
    /// which is recovery-driven and never age-driven (python `VOCAB["recovery_speed"]`).
    RecoverySpeed {
        /// Recovers quickly between hard days.
        Fast => "fast",
        /// The usual forty-eight hours.
        Typical => "typical",
        /// Needs longer; the flavour's recovery-limited caps apply.
        Limited => "limited",
    }
}

vocab_enum! {
    /// Injury history (python `VOCAB["injury_load"]`).
    InjuryLoad {
        /// Nothing in the last year.
        None => "none",
        /// An injury within the last twelve months.
        Last12Months => "last_12_months",
    }
}

vocab_enum! {
    /// How much structured interval work the athlete has behind them
    /// (python `VOCAB["interval_experience"]`).
    IntervalExperience {
        /// No structured intervals.
        None => "none",
        /// One season of intervals.
        Some => "some",
        /// Two or more seasons.
        TwoSeasons => "two_seasons",
    }
}

vocab_enum! {
    /// The sports the athlete trains (python `VOCAB["sport_mix"]`).
    SportMix {
        /// Running only.
        Running => "running",
        /// Cycling only.
        Cycling => "cycling",
        /// Swim, bike and run.
        Triathlon => "triathlon",
        /// Swimming only.
        Swimming => "swimming",
        /// Several sports without a triathlon goal.
        Mixed => "mixed",
    }
}

vocab_enum! {
    /// Where in the season the athlete stands (python `VOCAB["season_phase"]`).
    SeasonPhase {
        /// Between seasons.
        OffSeason => "off_season",
        /// Building the aerobic base.
        Base => "base",
        /// Sharpening toward the first goal.
        PreCompetition => "pre_competition",
        /// Racing.
        Competition => "competition",
    }
}

vocab_names!(HOURS_TIER_VALUES, HoursTier);
vocab_names!(TRAINING_AGE_VALUES, TrainingAge);
vocab_names!(WEEKS_TO_GOAL_VALUES, WeeksToGoal);
vocab_names!(RECOVERY_SPEED_VALUES, RecoverySpeed);
vocab_names!(INJURY_LOAD_VALUES, InjuryLoad);
vocab_names!(INTERVAL_EXPERIENCE_VALUES, IntervalExperience);
vocab_names!(SPORT_MIX_VALUES, SportMix);
vocab_names!(SEASON_PHASE_VALUES, SeasonPhase);
vocab_names!(EVENT_CLASS_VALUES, EventClass);
vocab_names!(MEASUREMENT_VALUES, Measurement);

impl InputDimension {
    /// The values a selection row may carry for this dimension.
    #[must_use]
    pub const fn allowed_values(self) -> &'static [&'static str] {
        match self {
            Self::HoursTier => &HOURS_TIER_VALUES,
            Self::TrainingAge => &TRAINING_AGE_VALUES,
            Self::EventClass => &EVENT_CLASS_VALUES,
            Self::WeeksToGoal => &WEEKS_TO_GOAL_VALUES,
            Self::Measurement => &MEASUREMENT_VALUES,
            Self::RecoverySpeed => &RECOVERY_SPEED_VALUES,
            Self::InjuryLoad => &INJURY_LOAD_VALUES,
            Self::IntervalExperience => &INTERVAL_EXPERIENCE_VALUES,
            Self::SportMix => &SPORT_MIX_VALUES,
            Self::SeasonPhase => &SEASON_PHASE_VALUES,
        }
    }
}

// ============================================================================
// Shared value shapes
// ============================================================================

/// A share band in `0..=1` with `min <= max`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Share {
    /// Lower bound.
    pub min: f32,
    /// Upper bound.
    pub max: f32,
}

impl Share {
    /// python `check_share`: both bounds in `0..=1`, `min <= max`.
    pub(crate) fn check(&self, key: &str) -> Check {
        for (name, value) in [("min", self.min), ("max", self.max)] {
            if !(0.0..=1.0).contains(&value) {
                return Err(Violation::new(
                    format!("{key}.{name}"),
                    format!("share {value} outside 0..=1"),
                ));
            }
        }
        if self.min > self.max {
            return Err(Violation::new(
                key,
                format!("min {} > max {}", self.min, self.max),
            ));
        }
        Ok(())
    }
}

/// An integer parameter range with the default the template ships.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParamRange {
    /// Lower bound.
    pub min: u32,
    /// Upper bound.
    pub max: u32,
    /// The value the default instance uses.
    pub default: u32,
}

impl ParamRange {
    /// python `check_param_range`: `min <= default <= max`.
    pub(crate) fn check(&self, key: &str) -> Check {
        if self.min > self.default {
            return Err(Violation::new(
                key,
                format!("min {} > default {}", self.min, self.default),
            ));
        }
        if self.default > self.max {
            return Err(Violation::new(
                key,
                format!("default {} > max {}", self.default, self.max),
            ));
        }
        Ok(())
    }
}

/// A perceived-exertion band on the 1–10 scale.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RpeRange {
    /// Lower bound.
    pub min: u8,
    /// Upper bound.
    pub max: u8,
}

impl RpeRange {
    /// python `RPE_BOUNDS`.
    const BOUNDS: (u8, u8) = (1, 10);

    /// python `check_rpe`: both in `1..=10`, `min <= max`.
    pub(crate) fn check(&self, key: &str) -> Check {
        for (name, value) in [("min", self.min), ("max", self.max)] {
            if !(Self::BOUNDS.0..=Self::BOUNDS.1).contains(&value) {
                return Err(Violation::new(
                    format!("{key}.{name}"),
                    format!("RPE {value} outside 1..=10"),
                ));
            }
        }
        if self.min > self.max {
            return Err(Violation::new(
                key,
                format!("min {} > max {}", self.min, self.max),
            ));
        }
        Ok(())
    }
}

/// A day band with `min <= max`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaysRange {
    /// Lower bound, days.
    pub min: u8,
    /// Upper bound, days.
    pub max: u8,
}

impl DaysRange {
    /// python `check_days_range`: `min <= max`.
    pub(crate) fn check(&self, key: &str) -> Check {
        if self.min > self.max {
            return Err(Violation::new(
                key,
                format!("min {} > max {}", self.min, self.max),
            ));
        }
        Ok(())
    }
}

/// The message python prints for an unquoted loading pattern
/// (`LOADING_PATTERN_MSG`).
pub const LOADING_PATTERN_MSG: &str =
    "quote the loading pattern (\"3:1\"): unquoted 3:1 is the number 181 in YAML 1.1";

/// Why a loading-pattern string did not parse.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{0:?} does not match ^[1-9]\\d*:[1-9]\\d*$")]
pub struct LoadingPatternParseError(pub String);

/// Load weeks to recovery weeks, written `"3:1"` (python `LOADING_PATTERN_RE`).
///
/// Serde reads and writes the string form; a number where the string
/// belongs is the YAML 1.1 sexagesimal trap and is rejected with
/// [`LOADING_PATTERN_MSG`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LoadingPattern {
    /// Loading weeks before the recovery week.
    pub load_weeks: u8,
    /// Recovery weeks after them.
    pub recovery_weeks: u8,
}

impl fmt::Display for LoadingPattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.load_weeks, self.recovery_weeks)
    }
}

impl FromStr for LoadingPattern {
    type Err = LoadingPatternParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let reject = || LoadingPatternParseError(text.to_owned());
        let (load, recovery) = text.split_once(':').ok_or_else(reject)?;
        let count = |digits: &str| -> Result<u8, LoadingPatternParseError> {
            let leading_nonzero = digits
                .bytes()
                .next()
                .is_some_and(|b| b.is_ascii_digit() && b != b'0');
            let all_digits = digits.bytes().all(|b| b.is_ascii_digit());
            if leading_nonzero && all_digits {
                digits.parse().map_err(|_| reject())
            } else {
                Err(reject())
            }
        };
        Ok(Self {
            load_weeks: count(load)?,
            recovery_weeks: count(recovery)?,
        })
    }
}

impl Serialize for LoadingPattern {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

struct LoadingPatternVisitor;

impl Visitor<'_> for LoadingPatternVisitor {
    type Value = LoadingPattern;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a loading pattern string such as \"3:1\"")
    }

    fn visit_str<E: DeError>(self, value: &str) -> Result<Self::Value, E> {
        value.parse().map_err(E::custom)
    }

    fn visit_i64<E: DeError>(self, _: i64) -> Result<Self::Value, E> {
        Err(E::custom(LOADING_PATTERN_MSG))
    }

    fn visit_u64<E: DeError>(self, _: u64) -> Result<Self::Value, E> {
        Err(E::custom(LOADING_PATTERN_MSG))
    }

    fn visit_f64<E: DeError>(self, _: f64) -> Result<Self::Value, E> {
        Err(E::custom(LOADING_PATTERN_MSG))
    }
}

impl<'de> Deserialize<'de> for LoadingPattern {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(LoadingPatternVisitor)
    }
}

/// The loading pattern for the typical athlete and for a recovery-limited one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoadingPatterns {
    /// The pattern most athletes follow.
    pub default: LoadingPattern,
    /// The pattern a recovery-limited athlete follows.
    pub recovery_limited: LoadingPattern,
}

/// Training-intensity-distribution target: shares of time below LT1,
/// between the thresholds, and above LT2.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TidTarget {
    /// Share of time below LT1.
    pub z1: Share,
    /// Share of time between LT1 and LT2.
    pub z2: Share,
    /// Share of time above LT2.
    pub z3: Share,
}

impl TidTarget {
    /// Slack for the share sums, python's `1e-9` scaled to `f32` input.
    const SUM_TOLERANCE: f64 = 1e-6;

    /// python `check_tid_target`: each zone a valid share, the three `min`s
    /// summing to at most 1.0 and the three `max`es to at least 1.0.
    pub(crate) fn check(&self, key: &str) -> Check {
        for (zone, share) in [("z1", &self.z1), ("z2", &self.z2), ("z3", &self.z3)] {
            share.check(&format!("{key}.{zone}"))?;
        }
        let min_sum = f64::from(self.z1.min) + f64::from(self.z2.min) + f64::from(self.z3.min);
        let max_sum = f64::from(self.z1.max) + f64::from(self.z2.max) + f64::from(self.z3.max);
        if min_sum > 1.0 + Self::SUM_TOLERANCE {
            return Err(Violation::new(
                key,
                format!("z1+z2+z3 min shares sum to {min_sum:.2}, above 1.0"),
            ));
        }
        if max_sum < 1.0 - Self::SUM_TOLERANCE {
            return Err(Violation::new(
                key,
                format!("z1+z2+z3 max shares sum to {max_sum:.2}, below 1.0"),
            ));
        }
        Ok(())
    }
}
