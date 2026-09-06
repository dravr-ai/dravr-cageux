// ABOUTME: The profile-to-flavour rule — eligibility first, then ranking, with every exclusion carrying its reason
// ABOUTME: Pure over the catalogue: no clock, no I/O, no athlete storage; the caller supplies the profile
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

//! Choosing a flavour for an athlete.
//!
//! The rule is deliberately small and legible, because most of its rows are
//! coach judgement rather than settled literature: the catalogue holds the
//! knowledge and this module only applies it. Two properties matter more than
//! the ranking itself.
//!
//! **Eligibility comes before ranking.** A flavour the athlete cannot run is
//! removed and says why, rather than being quietly out-weighed. Three sources
//! can remove one: an `exclude` entry on a selection row, a prerequisite the
//! athlete does not meet, and a contraindication the profile raises. Only the
//! first carries prose in the catalogue, so the other two are stated here.
//!
//! **A missing device is not a missing dimension.** The catalogue has no
//! `measurement: none` value, so an athlete with nothing to measure with is
//! read as steering by effort. Treating them as unmeasured instead would fire
//! no row at all and drop the lactate-guided flavours silently, which is the
//! one outcome the selection must never produce.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::flavour::Flavour;
use super::selection::SelectionTable;
use super::vocab::{
    EventClass, EvidenceTier, HoursTier, InjuryLoad, InputDimension, IntervalExperience,
    Measurement, RecoverySpeed, SeasonPhase, SportMix, TrainingAge, WeeksToGoal,
};

/// The hours-per-week band edges, as inclusive floors.
///
/// The catalogue states hours prerequisites as `min_hours_per_week` floors of
/// 4.0, 6.0 and 10.0, so the bands they imply are `[4,6)`, `[6,10)` and
/// `[10,∞)`. Nothing in the data states this, and an athlete at exactly six
/// hours lands in a different band under the other reading, so it is pinned
/// here and tested at the edges.
const HOURS_BAND_FLOORS: [(f32, HoursTier); 3] = [
    (10.0, HoursTier::Over10),
    (6.0, HoursTier::From6To10),
    (4.0, HoursTier::From4To6),
];

/// Weeks-to-goal band edges, matching `WeeksToGoal`'s own vocabulary.
const WEEKS_BAND_FLOORS: [(u32, WeeksToGoal); 2] =
    [(16, WeeksToGoal::Over16), (8, WeeksToGoal::From8To16)];

/// The athlete's side of the choice.
///
/// Every field is a value the questionnaire or the athlete's history can
/// answer. The optional ones are genuinely optional: an athlete without a
/// goal race has no event class and no runway, and the rule says so in
/// [`FlavourVerdict::missing_inputs`] rather than guessing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlavourInputs {
    /// Weekly training hours.
    pub hours_per_week: f32,
    /// Weekly sessions.
    pub sessions_per_week: u8,
    /// Years of structured training, as a number.
    pub training_age_years: f32,
    /// Years of structured training, as the catalogue's band.
    pub training_age: TrainingAge,
    /// The goal event, when there is one.
    pub event_class: Option<EventClass>,
    /// Weeks until the goal, when there is one.
    pub weeks_to_goal: Option<u32>,
    /// What the athlete can steer intensity by. Empty means effort only.
    pub measurements: BTreeSet<Measurement>,
    /// How fast the athlete recovers.
    pub recovery_speed: RecoverySpeed,
    /// Injury history.
    pub injury_load: InjuryLoad,
    /// Structured interval history.
    pub interval_experience: IntervalExperience,
    /// The sports trained.
    pub sport_mix: SportMix,
    /// Where in the season the athlete stands.
    pub season_phase: Option<SeasonPhase>,
    /// A flavour the coach's package pins.
    pub coach_preference: Option<String>,
}

impl FlavourInputs {
    /// The hours band this athlete falls in.
    #[must_use]
    pub fn hours_tier(&self) -> HoursTier {
        HOURS_BAND_FLOORS
            .iter()
            .find(|(floor, _)| self.hours_per_week >= *floor)
            .map_or(HoursTier::Under4, |(_, tier)| *tier)
    }

    /// The runway band, when there is a goal.
    #[must_use]
    pub fn weeks_band(&self) -> Option<WeeksToGoal> {
        self.weeks_to_goal.map(|weeks| {
            WEEKS_BAND_FLOORS
                .iter()
                .find(|(floor, _)| weeks >= *floor)
                .map_or(WeeksToGoal::Under8, |(_, band)| *band)
        })
    }

    /// What the athlete steers by, with an empty set read as effort.
    ///
    /// The catalogue cannot express "no device": `InputDimension::Measurement`
    /// allows only the five real ones. An athlete with none of them is steering
    /// by perceived effort, which is exactly `Measurement::Rpe`, so that is what
    /// they are read as. The alternative — firing no measurement row — would
    /// leave the lactate-guided flavours neither preferred nor refused, and an
    /// athlete would never be told why they cannot have one.
    #[must_use]
    pub fn effective_measurements(&self) -> BTreeSet<Measurement> {
        if self.measurements.is_empty() {
            return BTreeSet::from([Measurement::Rpe]);
        }
        self.measurements.clone()
    }

    /// The contraindications this profile raises.
    ///
    /// A flavour naming one of these is refused. The selection table also
    /// excludes most of them by name, and it should: the table says what a
    /// coach would say, while this says what the flavour file itself declares.
    /// Reading both means a flavour is still protected when a row is silent.
    #[must_use]
    pub fn contraindications(&self) -> BTreeSet<super::vocab::Contraindication> {
        use super::vocab::Contraindication;
        let mut out = BTreeSet::new();
        if self.training_age == TrainingAge::Novice {
            out.insert(Contraindication::NoviceFirstSeason);
        }
        if self.interval_experience == IntervalExperience::None {
            out.insert(Contraindication::NoIntervalExperience);
        }
        if self.recovery_speed == RecoverySpeed::Limited {
            out.insert(Contraindication::RecoveryLimited);
        }
        out
    }

    /// The `(dimension, value)` keys this profile fires, in dimension order.
    ///
    /// Measurement is the one dimension that fires more than once: an athlete
    /// with a power meter and a strap fires both rows, because each says
    /// something the other does not.
    fn fired_keys(&self) -> Vec<(InputDimension, String)> {
        let mut keys: Vec<(InputDimension, String)> = vec![
            (
                InputDimension::HoursTier,
                self.hours_tier().as_str().to_owned(),
            ),
            (
                InputDimension::TrainingAge,
                self.training_age.as_str().to_owned(),
            ),
            (
                InputDimension::RecoverySpeed,
                self.recovery_speed.as_str().to_owned(),
            ),
            (
                InputDimension::InjuryLoad,
                self.injury_load.as_str().to_owned(),
            ),
            (
                InputDimension::IntervalExperience,
                self.interval_experience.as_str().to_owned(),
            ),
            (InputDimension::SportMix, self.sport_mix.as_str().to_owned()),
        ];
        if let Some(event) = self.event_class {
            keys.push((InputDimension::EventClass, event.as_str().to_owned()));
        }
        if let Some(weeks) = self.weeks_band() {
            keys.push((InputDimension::WeeksToGoal, weeks.as_str().to_owned()));
        }
        if let Some(phase) = self.season_phase {
            keys.push((InputDimension::SeasonPhase, phase.as_str().to_owned()));
        }
        for m in self.effective_measurements() {
            keys.push((InputDimension::Measurement, m.as_str().to_owned()));
        }
        keys
    }

    /// Dimensions the profile cannot answer.
    fn missing(&self) -> Vec<InputDimension> {
        let mut out = Vec::new();
        if self.event_class.is_none() {
            out.push(InputDimension::EventClass);
        }
        if self.weeks_to_goal.is_none() {
            out.push(InputDimension::WeeksToGoal);
        }
        if self.season_phase.is_none() {
            out.push(InputDimension::SeasonPhase);
        }
        if self.measurements.is_empty() {
            out.push(InputDimension::Measurement);
        }
        out
    }
}

/// Why a flavour scored where it did — one entry per row that spoke for it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reason {
    /// The dimension that spoke.
    pub dimension: InputDimension,
    /// The value it spoke at.
    pub value: String,
    /// What it contributed.
    pub weight: u8,
    /// How strong the evidence for that mapping is.
    pub tier: EvidenceTier,
    /// The propositions behind it.
    pub evidence_refs: Vec<String>,
    /// The row's coach-voice note, when it has one.
    pub note: Option<String>,
}

/// A flavour the athlete can run, with what argued for it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoredFlavour {
    /// The flavour id.
    pub id: String,
    /// Summed `prefer` weights.
    pub score: u32,
    /// The rows that contributed, heaviest first.
    pub reasons: Vec<Reason>,
}

/// A flavour the athlete cannot run, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExcludedFlavour {
    /// The flavour id.
    pub id: String,
    /// Every reason it is out, in the order they were found.
    pub reasons: Vec<String>,
}

/// How much the verdict should be leaned on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    /// The profile is thin, or the top two are level.
    Low,
    /// A clear leader on a partly answered profile.
    Moderate,
    /// A clear leader on a fully answered profile.
    High,
}

/// The outcome: what the athlete can run, what they cannot, and how sure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlavourVerdict {
    /// Eligible flavours, highest score first, ties broken by id for stability.
    pub ranked: Vec<ScoredFlavour>,
    /// Everything removed, with its reasons.
    pub excluded: Vec<ExcludedFlavour>,
    /// How much to lean on it.
    pub confidence: Confidence,
    /// Dimensions the profile could not answer.
    pub missing_inputs: Vec<InputDimension>,
    /// Set when a coach package pinned a flavour and it survived eligibility.
    pub coach_pinned: Option<String>,
}

impl FlavourVerdict {
    /// The flavour to propose, if any survived.
    #[must_use]
    pub fn top(&self) -> Option<&ScoredFlavour> {
        self.ranked.first()
    }

    /// Why `id` is not on offer, if it was removed.
    #[must_use]
    pub fn exclusion(&self, id: &str) -> Option<&ExcludedFlavour> {
        self.excluded.iter().find(|e| e.id == id)
    }
}

/// Choose a flavour for the athlete.
///
/// Eligibility runs first and is absolute: a flavour excluded by any of the
/// three sources cannot be recovered by weight. What remains is ranked by the
/// summed `prefer` weights of the rows the profile fired.
///
/// `flavours` is the catalogue's own set; a `prefer` or `exclude` naming an id
/// absent from it is ignored, because the catalogue's cross-reference check
/// already fails on that and the rule should not fail twice.
#[must_use]
pub fn select_flavour(
    inputs: &FlavourInputs,
    table: &SelectionTable,
    flavours: &[Flavour],
) -> FlavourVerdict {
    let by_id: BTreeMap<&str, &Flavour> = flavours.iter().map(|f| (f.id.as_str(), f)).collect();
    let (scores, mut reasons, mut excluded) = score_table(inputs, table, &by_id);
    add_ineligible(inputs, flavours, &mut excluded);

    let mut ranked: Vec<ScoredFlavour> = scores
        .into_iter()
        .filter(|(id, _)| !excluded.contains_key(id))
        .map(|(id, score)| {
            let mut rs = reasons.remove(id).unwrap_or_default();
            rs.sort_by(|a, b| {
                b.weight
                    .cmp(&a.weight)
                    .then_with(|| a.dimension.cmp(&b.dimension))
            });
            ScoredFlavour {
                id: id.to_owned(),
                score,
                reasons: rs,
            }
        })
        .collect();
    ranked.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.id.cmp(&b.id)));

    // A coach's pinned flavour outranks the table when the athlete can run it;
    // the plan calls the override the product, not the exception.
    let coach_pinned = inputs
        .coach_preference
        .as_deref()
        .filter(|id| ranked.iter().any(|s| s.id == *id))
        .map(str::to_owned);
    if let Some(pinned) = coach_pinned.as_deref() {
        if let Some(at) = ranked.iter().position(|s| s.id == pinned) {
            let chosen = ranked.remove(at);
            ranked.insert(0, chosen);
        }
    }

    let missing_inputs = inputs.missing();
    let confidence = confidence_of(&ranked, &missing_inputs);
    let mut excluded: Vec<ExcludedFlavour> = excluded
        .into_iter()
        .map(|(id, reasons)| ExcludedFlavour {
            id: id.to_owned(),
            reasons,
        })
        .collect();
    excluded.sort_by(|a, b| a.id.cmp(&b.id));

    FlavourVerdict {
        ranked,
        excluded,
        confidence,
        missing_inputs,
        coach_pinned,
    }
}

/// Walk the rows the profile fires, summing `prefer` weights and collecting
/// `exclude` reasons in the catalogue's own words.
type TableScores<'a> = (
    BTreeMap<&'a str, u32>,
    BTreeMap<&'a str, Vec<Reason>>,
    BTreeMap<&'a str, Vec<String>>,
);

fn score_table<'a>(
    inputs: &FlavourInputs,
    table: &SelectionTable,
    by_id: &BTreeMap<&'a str, &'a Flavour>,
) -> TableScores<'a> {
    let mut scores: BTreeMap<&str, u32> = BTreeMap::new();
    let mut reasons: BTreeMap<&str, Vec<Reason>> = BTreeMap::new();
    let mut excluded: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for (dimension, value) in inputs.fired_keys() {
        let Some(row) = table
            .rows
            .iter()
            .find(|r| r.input == dimension && r.value == value)
        else {
            continue;
        };
        for item in &row.prefer {
            let Some(flavour) = by_id.get(item.id.as_str()) else {
                continue;
            };
            *scores.entry(flavour.id.as_str()).or_default() += u32::from(item.weight);
            reasons
                .entry(flavour.id.as_str())
                .or_default()
                .push(Reason {
                    dimension,
                    value: value.clone(),
                    weight: item.weight,
                    tier: row.tier,
                    evidence_refs: row.evidence_refs.clone(),
                    note: row.note.clone(),
                });
        }
        for item in &row.exclude {
            if let Some(flavour) = by_id.get(item.id.as_str()) {
                excluded
                    .entry(flavour.id.as_str())
                    .or_default()
                    .push(item.reason.clone());
            }
        }
    }
    (scores, reasons, excluded)
}

/// Prerequisites and contraindications carry no prose in the catalogue, so the
/// sentence an athlete reads is written here.
fn add_ineligible<'a>(
    inputs: &FlavourInputs,
    flavours: &'a [Flavour],
    excluded: &mut BTreeMap<&'a str, Vec<String>>,
) {
    let devices = inputs.effective_measurements();
    let raised = inputs.contraindications();
    for flavour in flavours {
        let mut against = Vec::new();
        let pre = &flavour.prerequisites;
        if inputs.hours_per_week < pre.min_hours_per_week {
            against.push(format!(
                "needs {:.0} hours a week and this athlete trains {:.1}",
                pre.min_hours_per_week, inputs.hours_per_week
            ));
        }
        if inputs.sessions_per_week < pre.min_sessions_per_week {
            against.push(format!(
                "needs {} sessions a week and this athlete trains {}",
                pre.min_sessions_per_week, inputs.sessions_per_week
            ));
        }
        if inputs.training_age_years < pre.min_training_age_years {
            against.push(format!(
                "needs {:.0} year(s) of structured training and this athlete has {:.1}",
                pre.min_training_age_years, inputs.training_age_years
            ));
        }
        if !meets_measurement(&devices, &pre.measurement) {
            against.push(format!(
                "is steered by {}, and this athlete measures with {}",
                describe_measurement(&pre.measurement),
                devices
                    .iter()
                    .map(|m| m.as_str())
                    .collect::<Vec<_>>()
                    .join(" and ")
            ));
        }
        for c in &flavour.contraindications {
            if raised.contains(c) {
                against.push(format!("is contraindicated by {}", c.as_str()));
            }
        }
        if !against.is_empty() {
            excluded
                .entry(flavour.id.as_str())
                .or_default()
                .extend(against);
        }
    }
}

/// A margin of one weight between the top two is a tie in everything but
/// arithmetic, and the mapping rows are not precise enough to spend it.
fn confidence_of(ranked: &[ScoredFlavour], missing: &[InputDimension]) -> Confidence {
    let margin = match ranked {
        [] => return Confidence::Low,
        [_only] => u32::MAX,
        [first, second, ..] => first.score.saturating_sub(second.score),
    };
    if margin < 2 {
        return Confidence::Low;
    }
    if missing.is_empty() {
        Confidence::High
    } else {
        Confidence::Moderate
    }
}

/// `measurement` is an outer any-of over inner all-of sets: the athlete needs
/// every device in at least one of the branches.
fn meets_measurement(devices: &BTreeSet<Measurement>, required: &[Vec<Measurement>]) -> bool {
    required.is_empty()
        || required
            .iter()
            .any(|branch| branch.iter().all(|m| devices.contains(m)))
}

/// The branches in words, for an exclusion an athlete will read.
fn describe_measurement(required: &[Vec<Measurement>]) -> String {
    required
        .iter()
        .map(|branch| {
            branch
                .iter()
                .map(|m| m.as_str())
                .collect::<Vec<_>>()
                .join(" with ")
        })
        .collect::<Vec<_>>()
        .join(", or ")
}
