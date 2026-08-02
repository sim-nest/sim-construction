//! Collaboration charter records and main-contract readiness.

use sim_lib_doc_core::ExternalRef;
use time::Date;

use crate::{
    ConstructionProjectError, ControlId, GatePolicy, GatePolicyReport, ProjectBook, ProjectId,
    ProjectObligation, Result, RoleId, intent::validate_texts,
};

/// Collaboration charter for a construction opportunity or early project.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CollaborationCharter {
    /// Stable construction project identity.
    pub project: ProjectId,
    /// Collaboration charter control id.
    pub control: ControlId,
    /// Collaboration objectives.
    pub objectives: Vec<String>,
    /// Working principles.
    pub working_principles: Vec<String>,
    /// Organization description.
    pub organization: Vec<String>,
    /// Decision rights.
    pub decision_rights: Vec<String>,
    /// Meeting cadence.
    pub meeting_cadence: String,
    /// Investigations to complete.
    pub investigations: Vec<String>,
    /// Target design/buildability work.
    pub target_design_buildability_work: Vec<String>,
    /// Open-book commercial rules, when applicable.
    pub open_book_rules: Vec<String>,
    /// Escalation path.
    pub escalation: Vec<RoleId>,
    /// Evidence controls required before a main-contract gate.
    pub main_contract_evidence: Vec<ControlId>,
    /// Shared obligations checked for collaboration readiness.
    pub obligations: Vec<ProjectObligation>,
    /// Reference-only evidence for the charter.
    pub evidence: Vec<ExternalRef>,
}

impl CollaborationCharter {
    /// Starts a collaboration charter.
    #[must_use]
    pub fn new(project: ProjectId, control: ControlId, meeting_cadence: impl Into<String>) -> Self {
        Self {
            project,
            control,
            objectives: Vec::new(),
            working_principles: Vec::new(),
            organization: Vec::new(),
            decision_rights: Vec::new(),
            meeting_cadence: meeting_cadence.into(),
            investigations: Vec::new(),
            target_design_buildability_work: Vec::new(),
            open_book_rules: Vec::new(),
            escalation: Vec::new(),
            main_contract_evidence: Vec::new(),
            obligations: Vec::new(),
            evidence: Vec::new(),
        }
    }

    /// Adds an objective.
    #[must_use]
    pub fn with_objective(mut self, value: impl Into<String>) -> Self {
        self.objectives.push(value.into());
        self
    }

    /// Adds a working principle.
    #[must_use]
    pub fn with_working_principle(mut self, value: impl Into<String>) -> Self {
        self.working_principles.push(value.into());
        self
    }

    /// Adds an organization note.
    #[must_use]
    pub fn with_organization(mut self, value: impl Into<String>) -> Self {
        self.organization.push(value.into());
        self
    }

    /// Adds a decision right.
    #[must_use]
    pub fn with_decision_right(mut self, value: impl Into<String>) -> Self {
        self.decision_rights.push(value.into());
        self
    }

    /// Adds an investigation.
    #[must_use]
    pub fn with_investigation(mut self, value: impl Into<String>) -> Self {
        self.investigations.push(value.into());
        self
    }

    /// Adds target design/buildability work.
    #[must_use]
    pub fn with_target_design_buildability_work(mut self, value: impl Into<String>) -> Self {
        self.target_design_buildability_work.push(value.into());
        self
    }

    /// Adds an open-book rule.
    #[must_use]
    pub fn with_open_book_rule(mut self, value: impl Into<String>) -> Self {
        self.open_book_rules.push(value.into());
        self
    }

    /// Adds one escalation role.
    #[must_use]
    pub fn with_escalation_role(mut self, role: RoleId) -> Self {
        self.escalation.push(role);
        self
    }

    /// Adds one required main-contract evidence control.
    #[must_use]
    pub fn with_main_contract_evidence(mut self, control: ControlId) -> Self {
        self.main_contract_evidence.push(control);
        self
    }

    /// Adds one shared obligation.
    #[must_use]
    pub fn with_obligation(mut self, obligation: ProjectObligation) -> Self {
        self.obligations.push(obligation);
        self
    }

    /// Adds reference-only evidence.
    #[must_use]
    pub fn with_evidence(mut self, evidence: ExternalRef) -> Self {
        self.evidence.push(evidence);
        self
    }

    /// Validates the charter's accountable collaboration fields.
    pub fn validate(&self) -> Result<()> {
        validate_required_texts(&self.objectives, "collaboration.objectives")?;
        validate_required_texts(&self.working_principles, "collaboration.working_principles")?;
        validate_required_texts(&self.organization, "collaboration.organization")?;
        validate_required_texts(&self.decision_rights, "collaboration.decision_rights")?;
        if self.meeting_cadence.trim().is_empty() {
            return Err(ConstructionProjectError::EmptyField(
                "collaboration.meeting_cadence",
            ));
        }
        validate_required_texts(&self.investigations, "collaboration.investigations")?;
        validate_required_texts(
            &self.target_design_buildability_work,
            "collaboration.target_design_buildability_work",
        )?;
        validate_texts(&self.open_book_rules, "collaboration.open_book_rules")?;
        if self.escalation.is_empty() {
            return Err(ConstructionProjectError::EmptyCollection(
                "collaboration.escalation",
            ));
        }
        if self.main_contract_evidence.is_empty() {
            return Err(ConstructionProjectError::EmptyCollection(
                "collaboration.main_contract_evidence",
            ));
        }
        if self.obligations.is_empty() {
            return Err(ConstructionProjectError::EmptyCollection(
                "collaboration.obligations",
            ));
        }
        if self.evidence.is_empty() {
            return Err(ConstructionProjectError::EmptyCollection(
                "collaboration.evidence",
            ));
        }
        Ok(())
    }

    /// Derives collaboration readiness through the shared requirement graph.
    pub fn readiness_report(
        &self,
        book: &ProjectBook,
        as_of_seq: u64,
        as_of_date: Date,
    ) -> Result<CollaborationReadinessReport> {
        self.validate()?;
        let snapshot = book.snapshot_at(as_of_seq)?;
        let missing_main_contract_evidence = self
            .main_contract_evidence
            .iter()
            .filter(|control| !snapshot.current.contains_key(*control))
            .cloned()
            .collect::<Vec<_>>();
        let mut policy = GatePolicy::new();
        for obligation in &self.obligations {
            policy = policy.with_obligation(obligation.clone());
        }
        let requirement_report = policy.evaluate(book, as_of_seq, as_of_date)?;
        Ok(CollaborationReadinessReport {
            charter: self.control.clone(),
            as_of_seq,
            ready: missing_main_contract_evidence.is_empty() && requirement_report.ready,
            missing_main_contract_evidence,
            requirement_report,
        })
    }
}

/// Derived collaboration readiness for a main-contract gate.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CollaborationReadinessReport {
    /// Collaboration charter control id.
    pub charter: ControlId,
    /// Snapshot sequence used for derivation.
    pub as_of_seq: u64,
    /// True when requirements and main-contract evidence are ready.
    pub ready: bool,
    /// Required main-contract evidence controls missing from the snapshot.
    pub missing_main_contract_evidence: Vec<ControlId>,
    /// Requirement graph report.
    pub requirement_report: GatePolicyReport,
}

fn validate_required_texts(values: &[String], field: &'static str) -> Result<()> {
    if values.is_empty() {
        return Err(ConstructionProjectError::EmptyCollection(field));
    }
    validate_texts(values, field)
}
