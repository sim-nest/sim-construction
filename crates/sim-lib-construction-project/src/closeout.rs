//! Evidence-backed construction closeout and accountable closure.

use std::collections::{BTreeMap, BTreeSet};

use sim_lib_doc_core::ExternalRef;
use time::Date;

use crate::{
    ConstructionProjectError, ControlId, EvidenceState, FinalEconomyReport, GatePolicy,
    ObligationPolicy, ProjectBook, ProjectId, ProjectObligation, Result, RoleId,
};

/// Required closeout lane kept distinct in the final report.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum CloseoutObligationKind {
    /// Warranty scope and named contact handoff.
    WarrantyContactHandoff,
    /// Retention period, owner, access, and disposal policy.
    RetentionPolicy,
    /// Explicit disposition of every remaining-work item.
    UnresolvedWork,
    /// Evidence retention, transfer, restriction, or disposal.
    EvidenceDisposition,
    /// Accepted project lesson with source-minimal evidence.
    Lesson,
}

/// One typed closeout lane over the shared construction obligation shape.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CloseoutObligation {
    /// Closeout meaning.
    pub kind: CloseoutObligationKind,
    /// Shared project obligation and evidence policy.
    pub obligation: ProjectObligation,
}

impl CloseoutObligation {
    /// Builds one mandatory closeout obligation.
    #[must_use]
    pub fn new(kind: CloseoutObligationKind, mut obligation: ProjectObligation) -> Self {
        obligation.policy = ObligationPolicy::Mandatory;
        Self { kind, obligation }
    }

    /// Returns the stable shared requirement id.
    #[must_use]
    pub fn id(&self) -> &ControlId {
        &self.obligation.requirement.id
    }
}

/// Evidence state and explanation for one closeout lane.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CloseoutItemReport {
    /// Stable requirement id.
    pub requirement: ControlId,
    /// Closeout meaning.
    pub kind: CloseoutObligationKind,
    /// Current evidence state.
    pub evidence_state: EvidenceState,
    /// Current supporting fact sequence.
    pub current_seq: Option<u64>,
    /// Deterministic evidence or dependency rule.
    pub rule: String,
    /// Deterministic explanation.
    pub reason: String,
}

/// Reproducible final economy and obligation report.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CloseoutReport {
    /// Stable closeout control.
    pub control: ControlId,
    /// Stable project identity.
    pub project: ProjectId,
    /// Inclusive fact sequence.
    pub as_of_seq: u64,
    /// Evaluation date.
    pub as_of_date: Date,
    /// Whether the exact final-economy report is ready.
    pub final_economy_ready: bool,
    /// Typed obligation reports in stable requirement-id order.
    pub items: Vec<CloseoutItemReport>,
    /// True only when economy and every closeout obligation are ready.
    pub ready: bool,
}

/// Typed final closeout control with one named closure authority.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CloseoutControlSet {
    /// Stable project identity.
    pub project: ProjectId,
    /// Stable closeout control.
    pub control: ControlId,
    /// Role authorized to close the project control record.
    pub closure_authority: RoleId,
    /// Typed closeout obligations.
    pub obligations: Vec<CloseoutObligation>,
}

impl CloseoutControlSet {
    /// Builds an empty closeout control.
    #[must_use]
    pub fn new(project: ProjectId, control: ControlId, closure_authority: RoleId) -> Self {
        Self {
            project,
            control,
            closure_authority,
            obligations: Vec::new(),
        }
    }

    /// Adds one typed obligation.
    #[must_use]
    pub fn with_obligation(mut self, obligation: CloseoutObligation) -> Self {
        self.obligations.push(obligation);
        self
    }

    /// Derives closeout readiness from shared facts and exact final economy.
    pub fn report(
        &self,
        book: &ProjectBook,
        economy: &FinalEconomyReport,
        as_of_seq: u64,
        as_of_date: Date,
    ) -> Result<CloseoutReport> {
        self.validate(book, economy)?;
        let mut policy = GatePolicy::new();
        for obligation in &self.obligations {
            policy = policy.with_obligation(obligation.obligation.clone());
        }
        let policy_report = policy.evaluate(book, as_of_seq, as_of_date)?;
        let kinds = self
            .obligations
            .iter()
            .map(|obligation| (obligation.id().clone(), obligation.kind))
            .collect::<BTreeMap<_, _>>();
        let items = policy_report
            .explanations
            .into_iter()
            .map(|explanation| CloseoutItemReport {
                kind: kinds[&explanation.requirement],
                requirement: explanation.requirement,
                evidence_state: explanation.evidence_state,
                current_seq: explanation.current_seq,
                rule: explanation.rule,
                reason: explanation.reason,
            })
            .collect();
        Ok(CloseoutReport {
            control: self.control.clone(),
            project: self.project.clone(),
            as_of_seq,
            as_of_date,
            final_economy_ready: economy.ready,
            items,
            ready: economy.ready && policy_report.ready,
        })
    }

    fn validate(&self, book: &ProjectBook, economy: &FinalEconomyReport) -> Result<()> {
        if book.project() != &self.project {
            return Err(ConstructionProjectError::ProjectMismatch {
                expected: self.project.clone(),
                actual: book.project().clone(),
            });
        }
        if economy.project != self.project {
            return Err(ConstructionProjectError::ProjectMismatch {
                expected: self.project.clone(),
                actual: economy.project.clone(),
            });
        }
        let mut ids = BTreeSet::new();
        let mut kinds = BTreeSet::new();
        for obligation in &self.obligations {
            if obligation.obligation.project != self.project {
                return Err(ConstructionProjectError::ProjectMismatch {
                    expected: self.project.clone(),
                    actual: obligation.obligation.project.clone(),
                });
            }
            obligation.obligation.requirement.validate()?;
            if !ids.insert(obligation.id()) {
                return Err(ConstructionProjectError::DuplicateId {
                    kind: "closeout_obligation",
                    id: obligation.id().to_string(),
                });
            }
            kinds.insert(obligation.kind);
        }
        for required in [
            CloseoutObligationKind::WarrantyContactHandoff,
            CloseoutObligationKind::RetentionPolicy,
            CloseoutObligationKind::UnresolvedWork,
            CloseoutObligationKind::EvidenceDisposition,
            CloseoutObligationKind::Lesson,
        ] {
            if !kinds.contains(&required) {
                return Err(ConstructionProjectError::EmptyCollection(
                    "closeout.required_lanes",
                ));
            }
        }
        Ok(())
    }
}

/// Proposed accountable decision over an exact closeout report.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CloseoutDecision {
    /// Closeout control being decided.
    pub control: ControlId,
    /// Report sequence being accepted.
    pub report_seq: u64,
    /// Later sequence carrying the decision.
    pub decision_seq: u64,
    /// Named authority that made the decision.
    pub decided_by: RoleId,
    /// Reference-only closure evidence.
    pub evidence: Vec<ExternalRef>,
}

impl CloseoutDecision {
    /// Builds a decision over one exact report sequence.
    #[must_use]
    pub fn new(control: ControlId, report_seq: u64, decision_seq: u64, decided_by: RoleId) -> Self {
        Self {
            control,
            report_seq,
            decision_seq,
            decided_by,
            evidence: Vec::new(),
        }
    }

    /// Adds reference-only closure evidence.
    #[must_use]
    pub fn with_evidence(mut self, evidence: ExternalRef) -> Self {
        self.evidence.push(evidence);
        self
    }

    /// Validates the decision and returns an immutable accountable closure.
    pub fn close(
        self,
        controls: &CloseoutControlSet,
        report: &CloseoutReport,
    ) -> Result<AccountableCloseout> {
        if self.control != controls.control || self.control != report.control {
            return Err(ConstructionProjectError::GateMismatch {
                expected: controls.control.clone(),
                actual: self.control,
            });
        }
        if self.report_seq != report.as_of_seq {
            return Err(ConstructionProjectError::GateSequenceMismatch {
                gate: report.control.clone(),
                report_seq: report.as_of_seq,
                decision_seq: self.report_seq,
            });
        }
        if self.decision_seq <= self.report_seq {
            return Err(ConstructionProjectError::InvalidSequence {
                field: "closeout.decision_seq",
                sequence: self.decision_seq,
            });
        }
        if self.decided_by != controls.closure_authority {
            return Err(ConstructionProjectError::ApprovalAuthorityMismatch {
                gate: report.control.clone(),
                expected: controls.closure_authority.clone(),
                actual: self.decided_by,
            });
        }
        if self.evidence.is_empty() {
            return Err(ConstructionProjectError::EmptyCollection(
                "closeout.decision.evidence",
            ));
        }
        if !report.ready {
            return Err(ConstructionProjectError::GateReportNotReady {
                gate: report.control.clone(),
            });
        }
        Ok(AccountableCloseout {
            project: controls.project.clone(),
            control: report.control.clone(),
            report_seq: self.report_seq,
            decision_seq: self.decision_seq,
            decided_by: controls.closure_authority.clone(),
            evidence: self.evidence,
        })
    }
}

/// Immutable proof that a named authority closed a ready report.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AccountableCloseout {
    project: ProjectId,
    control: ControlId,
    report_seq: u64,
    decision_seq: u64,
    decided_by: RoleId,
    evidence: Vec<ExternalRef>,
}

impl AccountableCloseout {
    /// Returns the closed project.
    #[must_use]
    pub fn project(&self) -> &ProjectId {
        &self.project
    }

    /// Returns the approving closeout decision control.
    #[must_use]
    pub fn control(&self) -> &ControlId {
        &self.control
    }

    /// Returns the exact report sequence.
    #[must_use]
    pub fn report_seq(&self) -> u64 {
        self.report_seq
    }

    /// Returns the later closure-decision sequence.
    #[must_use]
    pub fn decision_seq(&self) -> u64 {
        self.decision_seq
    }

    /// Returns the named closure authority.
    #[must_use]
    pub fn decided_by(&self) -> &RoleId {
        &self.decided_by
    }

    /// Returns the reference-only closure evidence.
    #[must_use]
    pub fn evidence(&self) -> &[ExternalRef] {
        &self.evidence
    }
}
