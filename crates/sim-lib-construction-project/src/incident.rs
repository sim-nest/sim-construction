//! Field incidents and accountable escalation.

use sim_lib_doc_core::ExternalRef;
use time::Date;

use crate::{
    ConstructionProjectError, FieldItem, FieldItemKind, FieldLane, FieldSeverity, Result, RoleId,
};

/// Accountable escalation retained for a field incident.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct IncidentEscalation {
    /// Role receiving the escalation.
    pub escalated_to: RoleId,
    /// Date of escalation.
    pub escalated_on: Date,
    /// Concise reason for escalation.
    pub reason: String,
    /// Reference-only evidence for the escalation decision or response.
    pub evidence: Vec<ExternalRef>,
}

/// Production, safety, work-environment, quality, or environmental incident.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProjectIncident {
    /// Shared accountable field-control record.
    pub field_item: FieldItem,
    /// Date the incident occurred.
    pub occurred_on: Date,
    /// Accountable escalation, when raised.
    pub escalation: Option<IncidentEscalation>,
}

impl ProjectIncident {
    /// Builds an incident.
    #[must_use]
    pub fn new(field_item: FieldItem, occurred_on: Date) -> Self {
        Self {
            field_item,
            occurred_on,
            escalation: None,
        }
    }

    /// Adds accountable escalation evidence.
    #[must_use]
    pub fn with_escalation(mut self, escalation: IncidentEscalation) -> Self {
        self.escalation = Some(escalation);
        self
    }

    /// Returns whether policy requires immediate incident escalation.
    #[must_use]
    pub fn requires_escalation(&self) -> bool {
        self.field_item.severity <= FieldSeverity::Critical
            || matches!(
                self.field_item.lane,
                FieldLane::Safety | FieldLane::WorkEnvironment
            )
            || self.field_item.non_waivable
    }

    /// Validates incident kind, common controls, and mandatory escalation.
    pub fn validate(&self) -> Result<()> {
        if self.field_item.kind != FieldItemKind::Incident {
            return Err(ConstructionProjectError::EmptyField(
                "incident.incident_kind",
            ));
        }
        self.field_item.validate()?;
        if self.requires_escalation() && self.escalation.is_none() {
            return Err(ConstructionProjectError::EmptyCollection(
                "incident.escalation",
            ));
        }
        if let Some(escalation) = &self.escalation {
            if escalation.reason.trim().is_empty() {
                return Err(ConstructionProjectError::EmptyField(
                    "incident.escalation.reason",
                ));
            }
            if escalation.evidence.is_empty() {
                return Err(ConstructionProjectError::EmptyCollection(
                    "incident.escalation.evidence",
                ));
            }
        }
        Ok(())
    }
}
