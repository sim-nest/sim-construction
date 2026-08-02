//! Requests for information that affect construction design readiness.

use sim_lib_doc_core::ExternalRef;
use time::Date;

use crate::{ControlId, EvidenceState, ProjectId, RoleId};

/// Current RFI acceptance state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RfiState {
    /// RFI is open.
    Open,
    /// RFI has an answer but it is not accepted for control use.
    Answered,
    /// RFI answer is accepted.
    Accepted,
    /// RFI was rejected.
    Rejected,
}

/// Request for information record.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RfiRecord {
    /// Stable construction project identity.
    pub project: ProjectId,
    /// RFI control id.
    pub control: ControlId,
    /// Role responsible for closure.
    pub responsible_role: RoleId,
    /// Date the answer is needed.
    pub need_date: Date,
    /// Package, task, or control ids affected by the RFI.
    pub affected_control_ids: Vec<ControlId>,
    /// Current evidence state.
    pub evidence_state: EvidenceState,
    /// RFI state.
    pub state: RfiState,
    /// Reference-only source and answer records.
    pub external_refs: Vec<ExternalRef>,
}

impl RfiRecord {
    /// Builds an open RFI record.
    #[must_use]
    pub fn new(
        project: ProjectId,
        control: ControlId,
        responsible_role: RoleId,
        need_date: Date,
    ) -> Self {
        Self {
            project,
            control,
            responsible_role,
            need_date,
            affected_control_ids: Vec::new(),
            evidence_state: EvidenceState::Reported,
            state: RfiState::Open,
            external_refs: Vec::new(),
        }
    }

    /// Sets the RFI state.
    #[must_use]
    pub fn with_state(mut self, state: RfiState) -> Self {
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

    /// Adds a reference-only RFI artifact.
    #[must_use]
    pub fn with_external_ref(mut self, external_ref: ExternalRef) -> Self {
        self.external_refs.push(external_ref);
        self
    }
}
