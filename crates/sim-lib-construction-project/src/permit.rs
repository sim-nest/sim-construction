//! Permits, inspections, and authority obligations.

use sim_lib_doc_core::ExternalRef;
use time::Date;

use crate::{ControlId, EvidenceState, EvidenceValidity, ProjectId, RoleId};

/// Authority permit status.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PermitState {
    /// Permit is requested.
    Requested,
    /// Permit is granted.
    Granted,
    /// Permit is rejected.
    Rejected,
    /// Permit is on hold by the authority.
    Hold,
}

/// Inspection status.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum InspectionState {
    /// Inspection is requested.
    Requested,
    /// Inspection passed.
    Passed,
    /// Inspection failed.
    Failed,
    /// Inspection is on authority hold.
    Hold,
}

/// Authority obligation status.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AuthorityObligationState {
    /// Obligation is open.
    Open,
    /// Obligation is satisfied.
    Satisfied,
    /// Authority placed the obligation on hold.
    Hold,
    /// Obligation is rejected or failed.
    Rejected,
}

/// Authority permit record.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PermitRecord {
    /// Stable construction project identity.
    pub project: ProjectId,
    /// Permit control id.
    pub control: ControlId,
    /// Role responsible for permit control.
    pub responsible_role: RoleId,
    /// Date the permit is needed.
    pub need_date: Date,
    /// Package, task, or control ids affected by the permit.
    pub affected_control_ids: Vec<ControlId>,
    /// Evidence state.
    pub evidence_state: EvidenceState,
    /// Permit state.
    pub state: PermitState,
    /// Validity window for the permit.
    pub validity: EvidenceValidity,
    /// Reference-only permit artifacts.
    pub external_refs: Vec<ExternalRef>,
}

impl PermitRecord {
    /// Builds a requested permit.
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
            state: PermitState::Requested,
            validity: EvidenceValidity::unbounded(),
            external_refs: Vec::new(),
        }
    }

    /// Sets the permit state.
    #[must_use]
    pub fn with_state(mut self, state: PermitState) -> Self {
        self.state = state;
        self
    }

    /// Sets the evidence state.
    #[must_use]
    pub fn with_evidence_state(mut self, evidence_state: EvidenceState) -> Self {
        self.evidence_state = evidence_state;
        self
    }

    /// Sets the validity window.
    #[must_use]
    pub fn with_validity(mut self, validity: EvidenceValidity) -> Self {
        self.validity = validity;
        self
    }

    /// Adds an affected package, task, or control id.
    #[must_use]
    pub fn affects(mut self, control: ControlId) -> Self {
        self.affected_control_ids.push(control);
        self
    }

    /// Adds a reference-only permit artifact.
    #[must_use]
    pub fn with_external_ref(mut self, external_ref: ExternalRef) -> Self {
        self.external_refs.push(external_ref);
        self
    }
}

/// Authority inspection record.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct InspectionRecord {
    /// Stable construction project identity.
    pub project: ProjectId,
    /// Inspection control id.
    pub control: ControlId,
    /// Role responsible for inspection closure.
    pub responsible_role: RoleId,
    /// Date the inspection is needed.
    pub need_date: Date,
    /// Package, task, or control ids affected by the inspection.
    pub affected_control_ids: Vec<ControlId>,
    /// Evidence state.
    pub evidence_state: EvidenceState,
    /// Inspection state.
    pub state: InspectionState,
    /// Reference-only inspection artifacts.
    pub external_refs: Vec<ExternalRef>,
}

impl InspectionRecord {
    /// Builds a requested inspection.
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
            state: InspectionState::Requested,
            external_refs: Vec::new(),
        }
    }

    /// Sets the inspection state.
    #[must_use]
    pub fn with_state(mut self, state: InspectionState) -> Self {
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

    /// Adds a reference-only inspection artifact.
    #[must_use]
    pub fn with_external_ref(mut self, external_ref: ExternalRef) -> Self {
        self.external_refs.push(external_ref);
        self
    }
}

/// Authority obligation record.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AuthorityObligation {
    /// Stable construction project identity.
    pub project: ProjectId,
    /// Obligation control id.
    pub control: ControlId,
    /// Role responsible for authority closure.
    pub responsible_role: RoleId,
    /// Date the obligation is needed.
    pub need_date: Date,
    /// Package, task, or control ids affected by the obligation.
    pub affected_control_ids: Vec<ControlId>,
    /// Evidence state.
    pub evidence_state: EvidenceState,
    /// Authority obligation state.
    pub state: AuthorityObligationState,
    /// True when this authority item cannot be waived for production readiness.
    pub non_waivable: bool,
    /// Reference-only authority artifacts.
    pub external_refs: Vec<ExternalRef>,
}

impl AuthorityObligation {
    /// Builds an open authority obligation.
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
            state: AuthorityObligationState::Open,
            non_waivable: false,
            external_refs: Vec::new(),
        }
    }

    /// Sets the authority state.
    #[must_use]
    pub fn with_state(mut self, state: AuthorityObligationState) -> Self {
        self.state = state;
        self
    }

    /// Sets the evidence state.
    #[must_use]
    pub fn with_evidence_state(mut self, evidence_state: EvidenceState) -> Self {
        self.evidence_state = evidence_state;
        self
    }

    /// Marks this obligation non-waivable.
    #[must_use]
    pub fn non_waivable(mut self) -> Self {
        self.non_waivable = true;
        self
    }

    /// Adds an affected package, task, or control id.
    #[must_use]
    pub fn affects(mut self, control: ControlId) -> Self {
        self.affected_control_ids.push(control);
        self
    }

    /// Adds a reference-only authority artifact.
    #[must_use]
    pub fn with_external_ref(mut self, external_ref: ExternalRef) -> Self {
        self.external_refs.push(external_ref);
        self
    }
}
