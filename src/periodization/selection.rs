// ABOUTME: The profile-to-flavour selection table — one row per (input dimension, value) naming preferred and excluded flavours
// ABOUTME: Parses training_catalogue/selection.yaml and validates it against the python selection rules
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

use std::collections::BTreeSet;

use serde::{Deserialize, Deserializer, Serialize};

use super::vocab::{EvidenceTier, InputDimension};
use super::{
    check_citation, check_ref_shapes, parse_error, unresolved_in, CatalogueError,
    CatalogueValidationError, Check, UnresolvedReference, Violation,
};

/// The selection table: how an athlete profile maps to flavours.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectionTable {
    /// The rows, one per (input, value).
    pub rows: Vec<SelectionRow>,
}

/// One cell of the decision table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectionRow {
    /// The profile dimension.
    pub input: InputDimension,
    /// The dimension's value, one of [`InputDimension::allowed_values`].
    pub value: String,
    /// Flavours this cell favours, with a weight.
    #[serde(default, deserialize_with = "null_as_empty")]
    pub prefer: Vec<FlavourWeight>,
    /// Flavours this cell rules out, with the reason.
    #[serde(default, deserialize_with = "null_as_empty")]
    pub exclude: Vec<FlavourExclusion>,
    /// Strength of the evidence for the mapping (not for the flavour).
    pub tier: EvidenceTier,
    /// The propositions behind it, as `evidence/sports_science/<category>/<slug>.md`.
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    /// A coach-voice note on the cell.
    #[serde(default)]
    pub note: Option<String>,
}

/// A favoured flavour and how strongly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlavourWeight {
    /// The flavour id.
    pub id: String,
    /// The weight, `1..=5`.
    pub weight: u8,
}

/// A ruled-out flavour and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlavourExclusion {
    /// The flavour id.
    pub id: String,
    /// Why it is out.
    pub reason: String,
}

/// python reads `prefer: null` as the empty list (`row.get("prefer", []) or []`).
fn null_as_empty<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<Vec<T>>::deserialize(deserializer).map(Option::unwrap_or_default)
}

impl SelectionRow {
    /// python `SELECTION_WEIGHT`.
    const WEIGHT_BOUNDS: (u8, u8) = (1, 5);

    /// python `check_selection` on one row: the value allowed for the
    /// input, a preference or an exclusion present, weights in bounds, and
    /// the citation rule at the row's tier.
    fn check(&self, key: &str) -> Check {
        let allowed = self.input.allowed_values();
        if !allowed.contains(&self.value.as_str()) {
            return Err(Violation::new(
                format!("{key}.value"),
                format!(
                    "{:?} is not allowed for {}; allowed: {}",
                    self.value,
                    self.input,
                    allowed.join(", ")
                ),
            ));
        }
        if self.prefer.is_empty() && self.exclude.is_empty() {
            return Err(Violation::new(
                key,
                "neither prefer nor exclude; a row must say something",
            ));
        }
        for (j, item) in self.prefer.iter().enumerate() {
            if !(Self::WEIGHT_BOUNDS.0..=Self::WEIGHT_BOUNDS.1).contains(&item.weight) {
                return Err(Violation::new(
                    format!("{key}.prefer[{j}].weight"),
                    format!("{} outside 1..=5", item.weight),
                ));
            }
        }
        let prefix = format!("{key}.");
        check_ref_shapes(&prefix, &self.evidence_refs)?;
        check_citation(&prefix, &self.evidence_refs, self.tier, "tier")
    }
}

impl SelectionTable {
    /// The shape name a parse error carries.
    const KIND: &'static str = "selection table";

    /// Parse the selection YAML document and validate it.
    ///
    /// # Errors
    ///
    /// [`CatalogueError::Parse`] when the YAML does not deserialize;
    /// [`CatalogueError::Validation`] when a selection rule fails.
    pub fn from_yaml(text: &str) -> Result<Self, CatalogueError> {
        let table: Self = serde_yaml::from_str(text).map_err(|e| parse_error(Self::KIND, e))?;
        table.validate()?;
        Ok(table)
    }

    /// The selection rules — python `check_selection`, one for one.
    ///
    /// # Errors
    ///
    /// The first broken rule, naming the row and the key.
    pub fn validate(&self) -> Result<(), CatalogueValidationError> {
        self.checks()
            .map_err(|violation| CatalogueValidationError::Selection {
                key: violation.key,
                message: violation.message,
            })
    }

    fn checks(&self) -> Check {
        let mut seen: BTreeSet<(InputDimension, &str)> = BTreeSet::new();
        for (i, row) in self.rows.iter().enumerate() {
            let key = format!("rows[{i}]");
            row.check(&key)?;
            // python check_selection: the same (input, value) twice.
            if !seen.insert((row.input, row.value.as_str())) {
                return Err(Violation::new(
                    key,
                    format!("({}, {}) appears twice", row.input, row.value),
                ));
            }
        }
        Ok(())
    }

    /// Every flavour id the table names with the key it sits under, row by
    /// row, prefer before exclude.
    fn flavour_references(&self) -> impl Iterator<Item = (String, &str)> {
        self.rows.iter().enumerate().flat_map(|(i, row)| {
            let prefer = row
                .prefer
                .iter()
                .enumerate()
                .map(move |(j, item)| (format!("rows[{i}].prefer[{j}].id"), item.id.as_str()));
            let exclude = row
                .exclude
                .iter()
                .enumerate()
                .map(move |(j, item)| (format!("rows[{i}].exclude[{j}].id"), item.id.as_str()));
            prefer.chain(exclude)
        })
    }

    /// The cross-file half of python `check_selection`: every `prefer` or
    /// `exclude` id `exists(id)` denies, keyed `rows[i].prefer[j].id` or
    /// `rows[i].exclude[j].id`. The registry answers with whether a
    /// `flavours/<id>.yaml` is loaded.
    pub fn unresolved_flavours(&self, exists: &dyn Fn(&str) -> bool) -> Vec<UnresolvedReference> {
        self.flavour_references()
            .filter(|(_, id)| !exists(id))
            .map(|(key, id)| UnresolvedReference {
                owner: "selection table".to_owned(),
                key,
                reference: id.to_owned(),
            })
            .collect()
    }

    /// Every flavour id the table names — the flat view over
    /// [`Self::unresolved_flavours`]'s walk.
    #[must_use]
    pub fn flavour_ids(&self) -> BTreeSet<&str> {
        self.flavour_references().map(|(_, id)| id).collect()
    }

    /// Every `evidence_refs` entry `exists(category, slug)` denies, row by row.
    pub fn unresolved_references(
        &self,
        exists: &dyn Fn(&str, &str) -> bool,
    ) -> Vec<UnresolvedReference> {
        self.rows
            .iter()
            .enumerate()
            .flat_map(|(i, row)| {
                unresolved_in(
                    "selection table",
                    &format!("rows[{i}]."),
                    &row.evidence_refs,
                    exists,
                )
            })
            .collect()
    }
}
