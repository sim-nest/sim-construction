//! Accountable completion and acceptance gates for construction handover.

use sim_lib_doc_core::ExternalRef;

use crate::{
    CommissioningAssessment, CommissioningControlSet, CommissioningReadinessReport,
    ConstructionProjectError, ControlId, HandoverHierarchy, ProjectId, Result, RoleId,
};

/// Distinct completion meaning evaluated by a handover gate.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum HandoverGateKind {
    /// Installed work and commissioning activity are technically complete.
    TechnicalCompletion,
    /// Required records and deliverables carry accepted evidence.
    EvidenceCompletion,
    /// Mandatory authority controls are closed.
    AuthorityCompletion,
    /// The customer has contractually accepted the defined scope.
    ContractualAcceptance,
    /// The defined scope is ready for occupancy or intended use.
    OccupancyUseReadiness,
    /// All technical, evidence, authority, contractual, and remaining work is complete.
    FinalCompletion,
}

/// One accountable gate over one hierarchy control.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HandoverGate {
    /// Stable project identity.
    pub project: ProjectId,
    /// Stable gate control id.
    pub control: ControlId,
    /// Hierarchy control evaluated by the gate.
    pub target: ControlId,
    /// Completion meaning kept distinct by this gate.
    pub kind: HandoverGateKind,
    /// Role accountable for accepting or rejecting the gate report.
    pub acceptance_authority: RoleId,
}

impl HandoverGate {
    /// Builds an accountable handover gate.
    #[must_use]
    pub fn new(
        project: ProjectId,
        control: ControlId,
        target: ControlId,
        kind: HandoverGateKind,
        acceptance_authority: RoleId,
    ) -> Self {
        Self {
            project,
            control,
            target,
            kind,
            acceptance_authority,
        }
    }

    /// Derives a reproducible gate report from the selected fact sequence.
    pub fn report(
        &self,
        controls: &CommissioningControlSet,
        hierarchy: &HandoverHierarchy,
        assessment: &CommissioningAssessment<'_>,
    ) -> Result<HandoverGateReport> {
        if controls.project != self.project {
            return Err(ConstructionProjectError::ProjectMismatch {
                expected: self.project.clone(),
                actual: controls.project.clone(),
            });
        }
        let readiness =
            controls.readiness_for_gate(hierarchy, &self.target, assessment, self.kind)?;
        Ok(HandoverGateReport {
            gate: self.control.clone(),
            project: self.project.clone(),
            target: self.target.clone(),
            kind: self.kind,
            as_of_seq: assessment.as_of_seq,
            readiness,
        })
    }
}

/// Reproducible evidence report for one completion meaning.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HandoverGateReport {
    /// Gate control id.
    pub gate: ControlId,
    /// Stable project identity.
    pub project: ProjectId,
    /// Evaluated hierarchy control.
    pub target: ControlId,
    /// Completion meaning evaluated.
    pub kind: HandoverGateKind,
    /// Exact project fact sequence evaluated.
    pub as_of_seq: u64,
    /// Leaf evidence and burn-down behind the report.
    pub readiness: CommissioningReadinessReport,
}

/// Accountable decision over a handover gate report.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum HandoverGateDecisionKind {
    /// Accept a ready report.
    Accept,
    /// Reject a report or decline acceptance.
    Reject,
}

/// Accountable acceptance or rejection against an exact report sequence.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HandoverGateDecision {
    /// Gate control id.
    pub gate: ControlId,
    /// Stable project identity.
    pub project: ProjectId,
    /// Completion meaning being decided.
    pub kind: HandoverGateKind,
    /// Accepted or rejected report sequence.
    pub report_seq: u64,
    /// Later fact sequence carrying this decision.
    pub decision_seq: u64,
    /// Accountable outcome.
    pub decision: HandoverGateDecisionKind,
    /// Role that made the decision.
    pub decided_by: RoleId,
    /// Reference-only acceptance evidence.
    pub evidence: Vec<ExternalRef>,
}

impl HandoverGateDecision {
    /// Builds a decision over one report sequence.
    #[must_use]
    pub fn new(
        gate: &HandoverGate,
        report_seq: u64,
        decision_seq: u64,
        decision: HandoverGateDecisionKind,
        decided_by: RoleId,
    ) -> Self {
        Self {
            gate: gate.control.clone(),
            project: gate.project.clone(),
            kind: gate.kind,
            report_seq,
            decision_seq,
            decision,
            decided_by,
            evidence: Vec::new(),
        }
    }

    /// Adds reference-only acceptance evidence.
    #[must_use]
    pub fn with_evidence(mut self, evidence: ExternalRef) -> Self {
        self.evidence.push(evidence);
        self
    }

    /// Validates identity, authority, evidence, readiness, and exact sequence.
    pub fn validate_against(&self, gate: &HandoverGate, report: &HandoverGateReport) -> Result<()> {
        if self.project != gate.project || self.project != report.project {
            return Err(ConstructionProjectError::ProjectMismatch {
                expected: gate.project.clone(),
                actual: self.project.clone(),
            });
        }
        if self.gate != gate.control
            || self.gate != report.gate
            || self.kind != gate.kind
            || self.kind != report.kind
        {
            return Err(ConstructionProjectError::GateMismatch {
                expected: gate.control.clone(),
                actual: self.gate.clone(),
            });
        }
        if self.report_seq != report.as_of_seq {
            return Err(ConstructionProjectError::GateSequenceMismatch {
                gate: self.gate.clone(),
                report_seq: report.as_of_seq,
                decision_seq: self.report_seq,
            });
        }
        if self.decision_seq <= self.report_seq {
            return Err(ConstructionProjectError::InvalidSequence {
                field: "handover_gate.decision_seq",
                sequence: self.decision_seq,
            });
        }
        if self.decided_by != gate.acceptance_authority {
            return Err(ConstructionProjectError::ApprovalAuthorityMismatch {
                gate: self.gate.clone(),
                expected: gate.acceptance_authority.clone(),
                actual: self.decided_by.clone(),
            });
        }
        if self.evidence.is_empty() {
            return Err(ConstructionProjectError::EmptyCollection(
                "handover_gate_decision.evidence",
            ));
        }
        if self.decision == HandoverGateDecisionKind::Accept && !report.readiness.ready {
            return Err(ConstructionProjectError::GateReportNotReady {
                gate: self.gate.clone(),
            });
        }
        Ok(())
    }
}
