//! Stable construction change identity and stage facts.

use std::collections::BTreeSet;

use sim_lib_doc_core::ExternalRef;
use time::Date;

use crate::{
    BaselineId, ChangeAmountComponent, ChangeId, ConstructionProjectError, ControlId, CurrencyCode,
    ProjectId, ReferencedAmountEvidence, Result, RoleId,
};

/// Direction and initiating contractual act for a construction change.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ChangeDirection {
    /// Customer instruction received by the project.
    CustomerInstruction,
    /// Notice issued by the project to the customer.
    ContractorNotice,
    /// Notice received from a supplier.
    SupplierNotice,
    /// Internal direction whose customer or supplier basis remains to be established.
    InternalDirection,
}

/// Contract and clause retained as the initiating basis for a change.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ContractualBasis {
    /// Open basis kind such as instructed variation or compensation event.
    pub kind: String,
    /// Contract clause or other controlling term.
    pub clause: String,
    /// Versioned reference to the controlling contract record.
    pub contract: ExternalRef,
}

impl ContractualBasis {
    /// Builds an explicit contractual basis.
    #[must_use]
    pub fn new(kind: impl Into<String>, clause: impl Into<String>, contract: ExternalRef) -> Self {
        Self {
            kind: kind.into(),
            clause: clause.into(),
            contract,
        }
    }

    pub(crate) fn validate(&self) -> Result<()> {
        if self.kind.trim().is_empty() {
            return Err(ConstructionProjectError::EmptyField(
                "change.contractual_basis.kind",
            ));
        }
        if self.clause.trim().is_empty() {
            return Err(ConstructionProjectError::EmptyField(
                "change.contractual_basis.clause",
            ));
        }
        if self.contract.backend.trim().is_empty()
            || self.contract.external_id.trim().is_empty()
            || self
                .contract
                .version
                .as_deref()
                .is_none_or(|version| version.trim().is_empty())
        {
            return Err(ConstructionProjectError::EmptyField(
                "change.contractual_basis.contract",
            ));
        }
        Ok(())
    }
}

/// Initiating record shared by every fact in one change chain.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ChangeRecord {
    /// Stable construction project identity.
    pub project: ProjectId,
    /// Stable identity carried through the complete change chain.
    pub id: ChangeId,
    /// Initiating direction and contractual act.
    pub direction: ChangeDirection,
    /// Explicit contractual basis.
    pub contractual_basis: ContractualBasis,
    /// Initially affected construction controls.
    pub affected_controls: Vec<ControlId>,
    /// Initially affected canonical Gantt task ids.
    pub affected_tasks: Vec<String>,
    /// Initially affected work-package controls.
    pub affected_packages: Vec<ControlId>,
    /// Role responsible for the change chain.
    pub responsible_role: RoleId,
    /// Instruction or notice date.
    pub initiated_on: Date,
    /// Contractual notice due date, when notice is required.
    pub notice_due_on: Option<Date>,
    /// Date the accountable notice was actually given.
    pub notice_given_on: Option<Date>,
    /// Reference-only instruction or notice evidence.
    pub evidence: Vec<ExternalRef>,
}

impl ChangeRecord {
    /// Builds the initiating change record.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        project: ProjectId,
        id: ChangeId,
        direction: ChangeDirection,
        contractual_basis: ContractualBasis,
        responsible_role: RoleId,
        initiated_on: Date,
        notice_due_on: Option<Date>,
    ) -> Self {
        Self {
            project,
            id,
            direction,
            contractual_basis,
            affected_controls: Vec::new(),
            affected_tasks: Vec::new(),
            affected_packages: Vec::new(),
            responsible_role,
            initiated_on,
            notice_due_on,
            notice_given_on: None,
            evidence: Vec::new(),
        }
    }

    /// Adds an affected construction control.
    #[must_use]
    pub fn affects_control(mut self, control: ControlId) -> Self {
        self.affected_controls.push(control);
        self
    }

    /// Adds an affected canonical Gantt task id.
    #[must_use]
    pub fn affects_task(mut self, task: impl Into<String>) -> Self {
        self.affected_tasks.push(task.into());
        self
    }

    /// Adds an affected work-package control.
    #[must_use]
    pub fn affects_package(mut self, package: ControlId) -> Self {
        self.affected_packages.push(package);
        self
    }

    /// Records the date accountable contractual notice was given.
    #[must_use]
    pub const fn notice_given_on(mut self, date: Date) -> Self {
        self.notice_given_on = Some(date);
        self
    }

    /// Adds instruction or notice evidence.
    #[must_use]
    pub fn with_evidence(mut self, evidence: ExternalRef) -> Self {
        self.evidence.push(evidence);
        self
    }

    pub(crate) fn validate(&self) -> Result<()> {
        self.contractual_basis.validate()?;
        validate_unique_controls("change.affected_controls", &self.affected_controls)?;
        validate_unique_controls("change.affected_packages", &self.affected_packages)?;
        validate_tasks(&self.affected_tasks)?;
        if let (Some(_), Some(given)) = (self.notice_due_on, self.notice_given_on)
            && given < self.initiated_on
        {
            return Err(ConstructionProjectError::ChangeDerivation {
                change: self.id.clone(),
                reason: "notice date predates the initiating instruction or notice",
            });
        }
        if let Some(due) = self.notice_due_on
            && due < self.initiated_on
        {
            return Err(ConstructionProjectError::ChangeDerivation {
                change: self.id.clone(),
                reason: "notice due date predates the initiating instruction or notice",
            });
        }
        if self.evidence.is_empty() {
            return Err(ConstructionProjectError::EmptyCollection("change.evidence"));
        }
        Ok(())
    }
}

/// Ordered stage represented by a change fact.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum ChangeStage {
    /// Scope assessment after instruction or notice.
    ScopeAssessment,
    /// Schedule and time-effect assessment.
    TimeEffect,
    /// Supplier-side cost exposure.
    SupplierExposure,
    /// Customer-side recovery assessment.
    CustomerRecovery,
    /// Quotation submitted to the customer.
    Quotation,
    /// Accountable authority decision.
    AuthorityDecision,
    /// Current commercial forecast.
    Forecast,
    /// Execution of changed work.
    Execution,
    /// Final supplier or customer settlement terms.
    Settlement,
    /// Accountable closure after settlement reconciliation.
    Closure,
}

/// Accountable lifecycle status carried by a change fact.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ChangeStatus {
    /// Instruction or notice is under assessment.
    Assessing,
    /// Quotation has been submitted for decision.
    Submitted,
    /// Full quoted value was approved.
    Approved,
    /// A stated subset of the quoted value was approved.
    PartiallyApproved,
    /// Quotation was rejected.
    Rejected,
    /// Commercial basis or value remains disputed.
    Disputed,
    /// Changed work is being executed.
    Executing,
    /// Supplier or customer settlement terms were recorded.
    Settled,
    /// Accountable closure was recorded after settlement reconciliation.
    Closed,
}

/// Baseline-aware time effect retained as a change fact, not recalculated here.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ChangeScheduleImpact {
    /// Accepted schedule baseline used by the assessment.
    pub baseline: BaselineId,
    /// Project fact sequence at which the assessment was made.
    pub as_of_seq: u64,
    /// Calendar date of the assessment.
    pub as_of_date: Date,
    /// Signed completion movement in calendar days.
    pub completion_delta_days: i32,
    /// Whether the referenced schedule analysis put the effect on the critical path.
    pub critical_path: bool,
    /// Canonical Gantt task ids affected by the assessment.
    pub affected_tasks: Vec<String>,
}

impl ChangeScheduleImpact {
    /// Builds a referenced schedule impact assessment.
    #[must_use]
    pub fn new(
        baseline: BaselineId,
        as_of_seq: u64,
        as_of_date: Date,
        completion_delta_days: i32,
        critical_path: bool,
        affected_tasks: Vec<String>,
    ) -> Self {
        Self {
            baseline,
            as_of_seq,
            as_of_date,
            completion_delta_days,
            critical_path,
            affected_tasks,
        }
    }

    pub(crate) fn validate(&self) -> Result<()> {
        if self.as_of_seq == 0 {
            return Err(ConstructionProjectError::InvalidSequence {
                field: "change.schedule_impact.as_of_seq",
                sequence: self.as_of_seq,
            });
        }
        if self.affected_tasks.is_empty() {
            return Err(ConstructionProjectError::EmptyCollection(
                "change.schedule_impact.affected_tasks",
            ));
        }
        validate_tasks(&self.affected_tasks)
    }
}

/// Immutable stage fact in a stable change chain.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ChangeFact {
    /// Unique fact identity.
    pub control: ControlId,
    /// Stable change identity shared across the chain.
    pub change: ChangeId,
    /// Monotone project fact sequence for deterministic corrections.
    pub fact_seq: u64,
    /// Lifecycle stage represented by the fact.
    pub stage: ChangeStage,
    /// Accountable status at this stage.
    pub status: ChangeStatus,
    /// Date on which the fact became effective.
    pub effective_on: Date,
    /// Role responsible for this stage.
    pub responsible_role: RoleId,
    /// Due date for this stage, when one applies.
    pub due_on: Option<Date>,
    /// Prior stage fact corrected by this fact.
    pub supersedes: Option<ControlId>,
    /// Additional affected construction controls.
    pub affected_controls: Vec<ControlId>,
    /// Additional affected canonical Gantt task ids.
    pub affected_tasks: Vec<String>,
    /// Additional affected work-package controls.
    pub affected_packages: Vec<ControlId>,
    /// Referenced schedule impact for a time-effect fact.
    pub schedule_impact: Option<ChangeScheduleImpact>,
    /// Exact components for this stage's commercial snapshot.
    pub amount_components: Vec<ChangeAmountComponent>,
    /// Versioned, dated external document or ledger evidence.
    pub references: Vec<ReferencedAmountEvidence>,
    /// Human-readable assessment, rationale, or execution note.
    pub note: String,
}

impl ChangeFact {
    /// Builds a stage fact with no commercial components or external references.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        control: ControlId,
        change: ChangeId,
        fact_seq: u64,
        stage: ChangeStage,
        status: ChangeStatus,
        effective_on: Date,
        responsible_role: RoleId,
        note: impl Into<String>,
    ) -> Self {
        Self {
            control,
            change,
            fact_seq,
            stage,
            status,
            effective_on,
            responsible_role,
            due_on: None,
            supersedes: None,
            affected_controls: Vec::new(),
            affected_tasks: Vec::new(),
            affected_packages: Vec::new(),
            schedule_impact: None,
            amount_components: Vec::new(),
            references: Vec::new(),
            note: note.into(),
        }
    }

    /// Sets the stage due date.
    #[must_use]
    pub const fn due_on(mut self, due_on: Date) -> Self {
        self.due_on = Some(due_on);
        self
    }

    /// Corrects a prior fact while preserving its provenance.
    #[must_use]
    pub fn supersedes(mut self, prior: ControlId) -> Self {
        self.supersedes = Some(prior);
        self
    }

    /// Adds an affected construction control.
    #[must_use]
    pub fn affects_control(mut self, control: ControlId) -> Self {
        self.affected_controls.push(control);
        self
    }

    /// Adds an affected canonical Gantt task id.
    #[must_use]
    pub fn affects_task(mut self, task: impl Into<String>) -> Self {
        self.affected_tasks.push(task.into());
        self
    }

    /// Adds an affected work-package control.
    #[must_use]
    pub fn affects_package(mut self, package: ControlId) -> Self {
        self.affected_packages.push(package);
        self
    }

    /// Adds a baseline-aware schedule impact.
    #[must_use]
    pub fn with_schedule_impact(mut self, impact: ChangeScheduleImpact) -> Self {
        self.schedule_impact = Some(impact);
        self
    }

    /// Adds an exact commercial component.
    #[must_use]
    pub fn with_amount(mut self, component: ChangeAmountComponent) -> Self {
        self.amount_components.push(component);
        self
    }

    /// Adds versioned, dated document or ledger evidence.
    #[must_use]
    pub fn with_reference(mut self, reference: ReferencedAmountEvidence) -> Self {
        self.references.push(reference);
        self
    }

    pub(crate) fn validate(&self, charter_currency: &CurrencyCode) -> Result<()> {
        crate::change_validation::validate_change_fact(self, charter_currency)
    }
}

pub(crate) fn validate_unique_controls(field: &'static str, controls: &[ControlId]) -> Result<()> {
    let mut unique = BTreeSet::new();
    for control in controls {
        if !unique.insert(control) {
            return Err(ConstructionProjectError::DuplicateId {
                kind: field,
                id: control.as_str().to_owned(),
            });
        }
    }
    Ok(())
}

pub(crate) fn validate_tasks(tasks: &[String]) -> Result<()> {
    let mut unique = BTreeSet::new();
    for task in tasks {
        if task.trim().is_empty() {
            return Err(ConstructionProjectError::EmptyField(
                "change.affected_tasks",
            ));
        }
        if !unique.insert(task) {
            return Err(ConstructionProjectError::DuplicateId {
                kind: "change.affected_task",
                id: task.clone(),
            });
        }
    }
    Ok(())
}
