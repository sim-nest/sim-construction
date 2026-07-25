//! Accountable work-package award decisions.

use sim_lib_doc_core::ExternalRef;
use time::Date;

use crate::{ControlId, RoleId};

/// Accountable award decision outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AwardDecisionKind {
    /// Award the work package to the supplier named by the selected tender.
    Award,
    /// Reject all tenders for now.
    RejectAll,
    /// Defer the award decision.
    Defer,
}

/// Accountable work-package award decision.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AwardDecision {
    /// Stable award control id.
    pub control: ControlId,
    /// Package being awarded.
    pub package: ControlId,
    /// Award outcome.
    pub kind: AwardDecisionKind,
    /// Selected tender when the package is awarded.
    pub selected_tender: Option<ControlId>,
    /// Role that made the award decision.
    pub decided_by: RoleId,
    /// Date the award decision was made.
    pub decided_on: Date,
    /// Decision rationale.
    pub rationale: String,
    /// Award evidence.
    pub evidence: Vec<ExternalRef>,
}

impl AwardDecision {
    /// Builds an award decision.
    #[must_use]
    pub fn new(
        control: ControlId,
        package: ControlId,
        kind: AwardDecisionKind,
        decided_by: RoleId,
        decided_on: Date,
        rationale: impl Into<String>,
    ) -> Self {
        Self {
            control,
            package,
            kind,
            selected_tender: None,
            decided_by,
            decided_on,
            rationale: rationale.into(),
            evidence: Vec::new(),
        }
    }

    /// Selects the tender for an award.
    #[must_use]
    pub fn selects(mut self, tender: ControlId) -> Self {
        self.selected_tender = Some(tender);
        self
    }

    /// Adds award evidence.
    #[must_use]
    pub fn with_evidence(mut self, evidence: ExternalRef) -> Self {
        self.evidence.push(evidence);
        self
    }
}
