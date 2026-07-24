//! Phase gates and accountable gate decisions.

use sim_lib_doc_core::ExternalRef;
use time::Date;

use crate::{
    ConstructionProjectError, ControlId, ProjectBook, ProjectId, ProjectPhase, Result, RoleId,
};

/// One required control for a phase gate.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GateRequirement {
    /// Required control id.
    pub control: ControlId,
    /// Optional date after which the current fact is stale for this gate.
    pub expires_on: Option<Date>,
    /// Optional exception control that can cover this requirement.
    pub exception: Option<ControlId>,
}

impl GateRequirement {
    /// Builds a required gate control.
    #[must_use]
    pub fn new(control: ControlId) -> Self {
        Self {
            control,
            expires_on: None,
            exception: None,
        }
    }

    /// Sets the date after which the requirement is expired.
    #[must_use]
    pub fn expires_on(mut self, date: Date) -> Self {
        self.expires_on = Some(date);
        self
    }

    /// Sets the exception control that may cover this requirement.
    #[must_use]
    pub fn with_exception(mut self, exception: ControlId) -> Self {
        self.exception = Some(exception);
        self
    }
}

/// Derived gate report. Approval is carried separately by `GateDecision`.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GateReport {
    /// Gate control id.
    pub gate: ControlId,
    /// Snapshot sequence used for derivation.
    pub as_of_seq: u64,
    /// True when no unmet, conflicted, or expired requirement remains.
    pub ready: bool,
    /// Missing requirement controls.
    pub unmet: Vec<ControlId>,
    /// Conflicted requirement controls.
    pub conflicted: Vec<ControlId>,
    /// Expired requirement controls.
    pub expired: Vec<ControlId>,
    /// Exception controls applied to cover requirements.
    pub applied_exceptions: Vec<ControlId>,
}

/// Phase gate definition.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PhaseGate {
    /// Stable construction project identity.
    pub project: ProjectId,
    /// Gate control id.
    pub gate: ControlId,
    /// Lifecycle phase this gate controls.
    pub phase: ProjectPhase,
    /// Role authorized to approve the gate.
    pub approval_authority: RoleId,
    /// Required controls checked by the gate.
    pub requirements: Vec<GateRequirement>,
    /// Reference-only evidence links for the gate definition.
    pub evidence: Vec<ExternalRef>,
}

impl PhaseGate {
    /// Builds a phase gate.
    #[must_use]
    pub fn new(
        project: ProjectId,
        gate: ControlId,
        phase: ProjectPhase,
        approval_authority: RoleId,
    ) -> Self {
        Self {
            project,
            gate,
            phase,
            approval_authority,
            requirements: Vec::new(),
            evidence: Vec::new(),
        }
    }

    /// Adds one requirement.
    #[must_use]
    pub fn with_requirement(mut self, requirement: GateRequirement) -> Self {
        self.requirements.push(requirement);
        self
    }

    /// Adds a reference-only evidence link.
    #[must_use]
    pub fn with_evidence(mut self, evidence: ExternalRef) -> Self {
        self.evidence.push(evidence);
        self
    }

    /// Derives a gate report from the project fact book at a sequence.
    pub fn report_at(
        &self,
        book: &ProjectBook,
        as_of_seq: u64,
        as_of_date: Date,
    ) -> Result<GateReport> {
        if self.requirements.is_empty() {
            return Err(ConstructionProjectError::EmptyCollection(
                "gate.requirements",
            ));
        }
        if self.evidence.is_empty() {
            return Err(ConstructionProjectError::EmptyCollection("gate.evidence"));
        }

        let snapshot = book.snapshot_at(as_of_seq)?;
        let mut unmet = Vec::new();
        let mut conflicted = Vec::new();
        let mut expired = Vec::new();
        let mut applied_exceptions = Vec::new();

        for requirement in &self.requirements {
            if let Some(exception) = &requirement.exception
                && snapshot.current.contains_key(exception)
            {
                applied_exceptions.push(exception.clone());
                continue;
            }
            if snapshot.conflicted.contains_key(&requirement.control) {
                conflicted.push(requirement.control.clone());
                continue;
            }
            let Some(fact) = snapshot.current.get(&requirement.control) else {
                unmet.push(requirement.control.clone());
                continue;
            };
            if let Some(expires_on) = requirement.expires_on
                && as_of_date > expires_on
            {
                expired.push(requirement.control.clone());
                continue;
            }
            if fact.effective_on > as_of_date {
                unmet.push(requirement.control.clone());
            }
        }

        let ready = unmet.is_empty() && conflicted.is_empty() && expired.is_empty();
        Ok(GateReport {
            gate: self.gate.clone(),
            as_of_seq,
            ready,
            unmet,
            conflicted,
            expired,
            applied_exceptions,
        })
    }
}

/// Accountable gate decision outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum GateDecisionKind {
    /// Approve the gate report.
    Approve,
    /// Reject the gate report.
    Reject,
}

/// Accountable decision over a derived gate report.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GateDecision {
    /// Gate control id.
    pub gate: ControlId,
    /// Project identity.
    pub project: ProjectId,
    /// Decision kind.
    pub decision: GateDecisionKind,
    /// Role that made the decision.
    pub decided_by: RoleId,
    /// Report sequence being decided.
    pub as_of_seq: u64,
    /// Fact sequence carrying this decision.
    pub decision_seq: u64,
    /// Reference-only evidence links.
    pub evidence: Vec<ExternalRef>,
}

impl GateDecision {
    /// Builds a gate decision.
    #[must_use]
    pub fn new(
        gate: ControlId,
        project: ProjectId,
        decision: GateDecisionKind,
        decided_by: RoleId,
        as_of_seq: u64,
        decision_seq: u64,
    ) -> Self {
        Self {
            gate,
            project,
            decision,
            decided_by,
            as_of_seq,
            decision_seq,
            evidence: Vec::new(),
        }
    }

    /// Adds a reference-only evidence link.
    #[must_use]
    pub fn with_evidence(mut self, evidence: ExternalRef) -> Self {
        self.evidence.push(evidence);
        self
    }

    /// Validates this decision against the gate definition and derived report.
    pub fn validate_against(&self, gate: &PhaseGate, report: &GateReport) -> Result<()> {
        if self.gate != report.gate {
            return Err(ConstructionProjectError::GateMismatch {
                expected: report.gate.clone(),
                actual: self.gate.clone(),
            });
        }
        if self.gate != gate.gate {
            return Err(ConstructionProjectError::GateMismatch {
                expected: gate.gate.clone(),
                actual: self.gate.clone(),
            });
        }
        if self.as_of_seq != report.as_of_seq {
            return Err(ConstructionProjectError::GateSequenceMismatch {
                gate: self.gate.clone(),
                report_seq: report.as_of_seq,
                decision_seq: self.as_of_seq,
            });
        }
        if self.decision_seq == 0 {
            return Err(ConstructionProjectError::InvalidSequence {
                field: "gate.decision_seq",
                sequence: self.decision_seq,
            });
        }
        if self.evidence.is_empty() {
            return Err(ConstructionProjectError::EmptyCollection(
                "gate.decision.evidence",
            ));
        }
        if self.decision == GateDecisionKind::Approve {
            if self.decided_by != gate.approval_authority {
                return Err(ConstructionProjectError::ApprovalAuthorityMismatch {
                    gate: self.gate.clone(),
                    expected: gate.approval_authority.clone(),
                    actual: self.decided_by.clone(),
                });
            }
            if !report.ready {
                return Err(ConstructionProjectError::GateReportNotReady {
                    gate: self.gate.clone(),
                });
            }
        }
        Ok(())
    }
}
