//! Design review and decision records.

use sim_lib_doc_core::ExternalRef;
use time::Date;

use crate::{ControlId, EvidenceState, ProjectId, RoleId};

/// Outcome of a design review or decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DesignReviewState {
    /// Review is open.
    Open,
    /// Review is accepted.
    Accepted,
    /// Review accepted with comments that still need control.
    AcceptedWithComments,
    /// Review is rejected.
    Rejected,
}

/// Review or decision record over a design revision.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DesignReview {
    /// Stable construction project identity.
    pub project: ProjectId,
    /// Review control id.
    pub control: ControlId,
    /// Reviewed design revision control id.
    pub design_revision: ControlId,
    /// Role responsible for review closure.
    pub responsible_role: RoleId,
    /// Date the review is needed.
    pub need_date: Date,
    /// Package, task, or control ids affected by the review.
    pub affected_control_ids: Vec<ControlId>,
    /// Current evidence state.
    pub evidence_state: EvidenceState,
    /// Review state.
    pub state: DesignReviewState,
    /// Reference-only review artifacts.
    pub external_refs: Vec<ExternalRef>,
}

impl DesignReview {
    /// Builds an open design review.
    #[must_use]
    pub fn new(
        project: ProjectId,
        control: ControlId,
        design_revision: ControlId,
        responsible_role: RoleId,
        need_date: Date,
    ) -> Self {
        Self {
            project,
            control,
            design_revision,
            responsible_role,
            need_date,
            affected_control_ids: Vec::new(),
            evidence_state: EvidenceState::Reported,
            state: DesignReviewState::Open,
            external_refs: Vec::new(),
        }
    }

    /// Sets the review state.
    #[must_use]
    pub fn with_state(mut self, state: DesignReviewState) -> Self {
        self.state = state;
        self
    }

    /// Sets the evidence state.
    #[must_use]
    pub fn with_evidence_state(mut self, evidence_state: EvidenceState) -> Self {
        self.evidence_state = evidence_state;
        self
    }

    /// Adds an affected package, task, or control id.
    #[must_use]
    pub fn affects(mut self, control: ControlId) -> Self {
        self.affected_control_ids.push(control);
        self
    }

    /// Adds a reference-only review artifact.
    #[must_use]
    pub fn with_external_ref(mut self, external_ref: ExternalRef) -> Self {
        self.external_refs.push(external_ref);
        self
    }
}
