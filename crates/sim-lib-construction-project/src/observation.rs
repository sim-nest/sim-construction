//! Production observations and deviations.

use time::Date;

use crate::{ConstructionProjectError, FieldItem, FieldItemKind, Result};

/// Production observation or deviation retained as a field-control item.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProjectObservation {
    /// Shared accountable field-control record.
    pub field_item: FieldItem,
    /// Date on which the condition was observed.
    pub observed_on: Date,
    /// Concise observed condition, without copied external payloads.
    pub summary: String,
}

impl ProjectObservation {
    /// Builds an observation or deviation.
    #[must_use]
    pub fn new(field_item: FieldItem, observed_on: Date, summary: impl Into<String>) -> Self {
        Self {
            field_item,
            observed_on,
            summary: summary.into(),
        }
    }

    /// Validates observation kind, summary, and common field-control invariants.
    pub fn validate(&self) -> Result<()> {
        if !matches!(
            self.field_item.kind,
            FieldItemKind::Observation | FieldItemKind::Deviation
        ) {
            return Err(ConstructionProjectError::EmptyField(
                "observation.observation_or_deviation_kind",
            ));
        }
        if self.summary.trim().is_empty() {
            return Err(ConstructionProjectError::EmptyField("observation.summary"));
        }
        self.field_item.validate()
    }
}
