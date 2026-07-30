//! Quality deviations, defects, and corrective actions.

use time::Date;

use crate::{
    ConstructionProjectError, ControlId, FieldItem, FieldItemKind, FieldItemState, Result, RoleId,
};

/// Quality deviation from an accepted requirement or method.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct QualityDeviation {
    /// Shared accountable field-control record.
    pub field_item: FieldItem,
    /// Requirement or method from which the work deviates.
    pub requirement: ControlId,
    /// Concise deviation statement.
    pub description: String,
}

impl QualityDeviation {
    /// Validates the deviation and common field-control record.
    pub fn validate(&self) -> Result<()> {
        if self.field_item.kind != FieldItemKind::Deviation {
            return Err(ConstructionProjectError::EmptyField(
                "quality_deviation.deviation_kind",
            ));
        }
        if self.description.trim().is_empty() {
            return Err(ConstructionProjectError::EmptyField(
                "quality_deviation.description",
            ));
        }
        self.field_item.validate()
    }
}

/// Confirmed production defect.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Defect {
    /// Shared accountable field-control record.
    pub field_item: FieldItem,
    /// Date the defect was detected.
    pub detected_on: Date,
}

impl Defect {
    /// Returns whether this defect is overdue as of `date`.
    #[must_use]
    pub fn is_overdue(&self, date: Date) -> bool {
        self.field_item.is_overdue(date)
    }

    /// Validates defect kind and common field-control record.
    pub fn validate(&self) -> Result<()> {
        if self.field_item.kind != FieldItemKind::Defect {
            return Err(ConstructionProjectError::EmptyField("defect.defect_kind"));
        }
        self.field_item.validate()
    }
}

/// Evidence-backed action correcting one or more field controls.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CorrectiveAction {
    /// Shared accountable field-control record.
    pub field_item: FieldItem,
    /// Defects, incidents, observations, or deviations this action corrects.
    pub corrects: Vec<ControlId>,
    /// Role accepting the corrective result.
    pub accepted_by: Option<RoleId>,
}

impl CorrectiveAction {
    /// Returns whether the corrective result is closed with accepted evidence.
    #[must_use]
    pub fn has_accepted_evidence(&self) -> bool {
        self.field_item.state == FieldItemState::Closed
            && self.field_item.evidence_state.satisfies_required_evidence()
            && !self.field_item.evidence.is_empty()
            && self.accepted_by.is_some()
    }

    /// Validates corrective scope, closure authority, and evidence.
    pub fn validate(&self) -> Result<()> {
        if self.field_item.kind != FieldItemKind::CorrectiveAction {
            return Err(ConstructionProjectError::EmptyField(
                "corrective_action.corrective_action_kind",
            ));
        }
        if self.corrects.is_empty() {
            return Err(ConstructionProjectError::EmptyCollection(
                "corrective_action.corrects",
            ));
        }
        self.field_item.validate()?;
        if self.field_item.state == FieldItemState::Closed && !self.has_accepted_evidence() {
            return Err(ConstructionProjectError::EmptyCollection(
                "corrective_action.accepted_evidence",
            ));
        }
        Ok(())
    }
}
