//! Construction opportunity intake records.

use sim_lib_doc_core::ExternalRef;

use crate::{ConstructionProjectError, ControlId, ProjectId, Result, RoleId};

/// Source of an opportunity record.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum OpportunitySource {
    /// Customer-originated request.
    Customer,
    /// Partner-originated lead.
    Partner,
    /// Public tender notice.
    PublicTender,
    /// Internal strategic prospect.
    Internal,
}

/// Minimal opportunity record before bid control starts.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OpportunityRecord {
    /// Stable construction project identity.
    pub project: ProjectId,
    /// Opportunity control id.
    pub control: ControlId,
    /// Source category.
    pub source: OpportunitySource,
    /// Accountable owner.
    pub owner: RoleId,
    /// Customer or opportunity label.
    pub label: String,
    /// Reference-only source evidence.
    pub evidence: Vec<ExternalRef>,
}

impl OpportunityRecord {
    /// Builds an opportunity record.
    #[must_use]
    pub fn new(
        project: ProjectId,
        control: ControlId,
        source: OpportunitySource,
        owner: RoleId,
        label: impl Into<String>,
    ) -> Self {
        Self {
            project,
            control,
            source,
            owner,
            label: label.into(),
            evidence: Vec::new(),
        }
    }

    /// Adds reference-only evidence.
    #[must_use]
    pub fn with_evidence(mut self, evidence: ExternalRef) -> Self {
        self.evidence.push(evidence);
        self
    }

    /// Validates required opportunity fields.
    pub fn validate(&self) -> Result<()> {
        if self.label.trim().is_empty() {
            return Err(ConstructionProjectError::EmptyField("opportunity.label"));
        }
        if self.evidence.is_empty() {
            return Err(ConstructionProjectError::EmptyCollection(
                "opportunity.evidence",
            ));
        }
        Ok(())
    }
}
