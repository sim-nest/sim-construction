//! Production inspection and test points.

use crate::{ConstructionProjectError, FieldItem, FieldItemKind, FieldItemState, Result, RoleId};

/// Accountable result for an inspection or test point.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum InspectionResult {
    /// Point awaits inspection or testing.
    Pending,
    /// Point passed and carries accepted evidence.
    Passed,
    /// Point was rejected and remains a production blocker.
    Rejected,
}

/// Production inspection or test point.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct InspectionPoint {
    /// Shared accountable field-control record.
    pub field_item: FieldItem,
    /// Inspection or test result.
    pub result: InspectionResult,
    /// Role that accepted or rejected the result.
    pub decided_by: Option<RoleId>,
}

impl InspectionPoint {
    /// Builds a pending inspection or test point.
    #[must_use]
    pub fn new(field_item: FieldItem) -> Self {
        Self {
            field_item,
            result: InspectionResult::Pending,
            decided_by: None,
        }
    }

    /// Sets an accountable inspection or test result.
    #[must_use]
    pub fn with_result(mut self, result: InspectionResult, decided_by: RoleId) -> Self {
        self.result = result;
        self.decided_by = Some(decided_by);
        self
    }

    /// Returns whether the point blocks affected production controls.
    #[must_use]
    pub fn blocks_production(&self) -> bool {
        self.result != InspectionResult::Passed
            || self.field_item.state != FieldItemState::Closed
            || !self.field_item.evidence_state.satisfies_required_evidence()
    }

    /// Validates point kind and accepted-result evidence.
    pub fn validate(&self) -> Result<()> {
        if !matches!(
            self.field_item.kind,
            FieldItemKind::InspectionPoint | FieldItemKind::TestPoint
        ) {
            return Err(ConstructionProjectError::EmptyField(
                "inspection.inspection_or_test_kind",
            ));
        }
        self.field_item.validate()?;
        if self.result != InspectionResult::Pending && self.decided_by.is_none() {
            return Err(ConstructionProjectError::EmptyField(
                "inspection.decided_by",
            ));
        }
        if self.result == InspectionResult::Passed
            && !self.field_item.evidence_state.satisfies_required_evidence()
        {
            return Err(ConstructionProjectError::EmptyCollection(
                "inspection.accepted_evidence",
            ));
        }
        Ok(())
    }
}
