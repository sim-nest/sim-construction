//! Evidence requirements for commissioning and handover.

use std::collections::{BTreeMap, BTreeSet};

use time::Date;

use crate::{
    ConstructionProjectError, ControlEdgeKind, ControlGraph, ControlId, ControlNodeKind,
    EvidenceState, ExceptionDecision, GatePolicy, HandoverHierarchy, ObligationPolicy, ProjectBook,
    ProjectId, ProjectObligation, Result,
};

/// Kind of leaf requirement controlled through commissioning and handover.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum CommissioningRequirementKind {
    /// Planned commissioning activity or functional sequence.
    Activity,
    /// Inspection point.
    Inspection,
    /// Test point, including a superseding retest.
    Test,
    /// Defect that must be closed or accountably excepted.
    Defect,
    /// Operations and maintenance deliverable retained by an external store.
    OperationsMaintenanceDeliverable,
    /// As-built deliverable retained by an external store.
    AsBuiltDeliverable,
    /// Operator, maintainer, or customer training.
    Training,
    /// Certificate with an evidence validity window.
    Certification,
    /// Closure controlled by a public or delegated authority.
    AuthorityClosure,
    /// Customer acceptance evidence.
    CustomerAcceptance,
    /// Explicit remaining-work item.
    RemainingWork,
}

/// One typed commissioning requirement over the shared obligation shape.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CommissioningRequirement {
    /// Commissioning or handover kind.
    pub kind: CommissioningRequirementKind,
    /// Shared project obligation, including requirement, evidence, and policy.
    pub obligation: ProjectObligation,
    /// Leaf or aggregate handover controls to which this requirement applies.
    pub targets: Vec<ControlId>,
    /// True when failure has a critical consequence.
    pub critical: bool,
}

impl CommissioningRequirement {
    /// Builds a typed requirement for one handover control.
    ///
    /// Authority closure is always made non-waivable. All other policy remains
    /// explicit on the supplied common obligation.
    #[must_use]
    pub fn new(
        kind: CommissioningRequirementKind,
        mut obligation: ProjectObligation,
        target: ControlId,
    ) -> Self {
        if kind == CommissioningRequirementKind::AuthorityClosure {
            obligation.requirement.non_waivable = true;
            obligation.policy = ObligationPolicy::Mandatory;
        }
        Self {
            kind,
            obligation,
            targets: vec![target],
            critical: false,
        }
    }

    /// Adds another handover control affected by the same requirement.
    #[must_use]
    pub fn with_target(mut self, target: ControlId) -> Self {
        self.targets.push(target);
        self
    }

    /// Marks the requirement as critical.
    #[must_use]
    pub fn critical(mut self) -> Self {
        self.critical = true;
        self
    }

    /// Returns the stable common requirement id.
    #[must_use]
    pub fn id(&self) -> &ControlId {
        &self.obligation.requirement.id
    }

    /// Validates commissioning-specific evidence and target rules.
    pub fn validate(&self, project: &ProjectId, hierarchy: &HandoverHierarchy) -> Result<()> {
        if &self.obligation.project != project {
            return Err(ConstructionProjectError::ProjectMismatch {
                expected: project.clone(),
                actual: self.obligation.project.clone(),
            });
        }
        self.obligation.requirement.validate()?;
        if !self.obligation.requirement.evidence_required {
            return Err(ConstructionProjectError::EmptyCollection(
                "commissioning_requirement.required_evidence",
            ));
        }
        if self.obligation.requirement.evidence_kinds.is_empty() {
            return Err(ConstructionProjectError::EmptyCollection(
                "commissioning_requirement.evidence_kinds",
            ));
        }
        if self.targets.is_empty() {
            return Err(ConstructionProjectError::EmptyCollection(
                "commissioning_requirement.targets",
            ));
        }
        let mut seen = BTreeSet::new();
        for target in &self.targets {
            if hierarchy.kind(target).is_none() {
                return Err(ConstructionProjectError::ControlGraphMissingEndpoint {
                    edge: "commissioning-target",
                    endpoint_role: "target",
                    endpoint: target.clone(),
                });
            }
            if !seen.insert(target) {
                return Err(ConstructionProjectError::DuplicateId {
                    kind: "commissioning_requirement.target",
                    id: target.to_string(),
                });
            }
        }
        if self.kind == CommissioningRequirementKind::AuthorityClosure
            && (!self.obligation.requirement.non_waivable
                || self.obligation.policy != ObligationPolicy::Mandatory)
        {
            return Err(ConstructionProjectError::EmptyField(
                "commissioning.authority_closure_non_waivable",
            ));
        }
        Ok(())
    }
}
/// Typed commissioning requirements projected onto one handover hierarchy.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CommissioningControlSet {
    /// Stable project identity.
    pub project: ProjectId,
    /// Typed commissioning requirements.
    pub requirements: Vec<CommissioningRequirement>,
}

/// Evidence snapshot and exception policy used for one readiness derivation.
#[derive(Clone, Debug)]
pub struct CommissioningAssessment<'a> {
    /// Shared append-only project fact book.
    pub book: &'a ProjectBook,
    /// Fact sequence at which readiness is reproduced.
    pub as_of_seq: u64,
    /// Date used for evidence and exception validity.
    pub as_of_date: Date,
    /// Bounded exception decisions available to the assessment.
    pub exceptions: Vec<ExceptionDecision>,
    /// Capabilities granted for exception evaluation.
    pub granted_capabilities: Vec<String>,
}

impl<'a> CommissioningAssessment<'a> {
    /// Builds an assessment over a fact-book sequence and date.
    #[must_use]
    pub fn new(book: &'a ProjectBook, as_of_seq: u64, as_of_date: Date) -> Self {
        Self {
            book,
            as_of_seq,
            as_of_date,
            exceptions: Vec::new(),
            granted_capabilities: Vec::new(),
        }
    }

    /// Adds a bounded exception decision.
    #[must_use]
    pub fn with_exception(mut self, exception: ExceptionDecision) -> Self {
        self.exceptions.push(exception);
        self
    }

    /// Grants a capability used while evaluating the assessment.
    #[must_use]
    pub fn with_capability(mut self, capability: impl Into<String>) -> Self {
        self.granted_capabilities.push(capability.into());
        self
    }
}

/// Exact commissioning evidence counts behind a readiness percentage.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CommissioningBurnDown {
    /// Requirements in scope.
    pub total: usize,
    /// Requirements satisfied by current accepted evidence.
    pub accepted: usize,
    /// Requirements with no current fact.
    pub missing: usize,
    /// Requirements reported without accepted evidence.
    pub reported: usize,
    /// Requirements with evidence awaiting acceptance.
    pub evidenced: usize,
    /// Requirements rejected by an accountable role.
    pub rejected: usize,
    /// Requirements whose accepted evidence is no longer current.
    pub expired: usize,
    /// Requirements with competing current facts.
    pub conflicted: usize,
    /// Requirements covered by a current bounded exception.
    pub excepted: usize,
}

impl CommissioningBurnDown {
    /// Returns the accepted-or-excepted integer completion percentage.
    ///
    /// Gate readiness is deliberately separate: a high percentage cannot
    /// override even one blocked mandatory requirement.
    #[must_use]
    pub fn completion_percent(self) -> u8 {
        if self.total == 0 {
            return 0;
        }
        let completed = self.accepted.saturating_add(self.excepted);
        u8::try_from(completed.saturating_mul(100) / self.total).unwrap_or(100)
    }

    /// Returns requirements not yet accepted or excepted.
    #[must_use]
    pub fn open(self) -> usize {
        self.total
            .saturating_sub(self.accepted.saturating_add(self.excepted))
    }

    fn record(&mut self, state: EvidenceState, excepted: bool) {
        self.total += 1;
        if excepted {
            self.excepted += 1;
            return;
        }
        match state {
            EvidenceState::Missing => self.missing += 1,
            EvidenceState::Reported => self.reported += 1,
            EvidenceState::Evidenced => self.evidenced += 1,
            EvidenceState::Accepted => self.accepted += 1,
            EvidenceState::Rejected => self.rejected += 1,
            EvidenceState::Expired => self.expired += 1,
            EvidenceState::Conflicted => self.conflicted += 1,
        }
    }
}

/// Readiness of one leaf commissioning requirement.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CommissioningItemReadiness {
    /// Stable requirement id.
    pub requirement: ControlId,
    /// Typed commissioning kind.
    pub kind: CommissioningRequirementKind,
    /// Handover controls named by the requirement.
    pub targets: Vec<ControlId>,
    /// Current effective evidence state.
    pub evidence_state: EvidenceState,
    /// Current fact sequence, when present.
    pub current_seq: Option<u64>,
    /// Applied bounded exception, when present.
    pub exception: Option<ControlId>,
    /// True when this item blocks the derived readiness result.
    pub blocks: bool,
    /// True when failure has a critical consequence.
    pub critical: bool,
    /// Deterministic rule selected by shared gate policy.
    pub rule: String,
    /// Deterministic readiness explanation.
    pub reason: String,
}

/// Hierarchy-level readiness and burn-down derived from leaf evidence.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CommissioningReadinessReport {
    /// Hierarchy control being rolled up.
    pub target: ControlId,
    /// Fact sequence used for derivation.
    pub as_of_seq: u64,
    /// True only when no mandatory requirement in scope remains blocked.
    pub ready: bool,
    /// Exact evidence-state counts.
    pub burn_down: CommissioningBurnDown,
    /// Leaf results in stable requirement-id order.
    pub items: Vec<CommissioningItemReadiness>,
}

impl CommissioningReadinessReport {
    /// Returns the accepted-or-excepted completion percentage.
    #[must_use]
    pub fn completion_percent(&self) -> u8 {
        self.burn_down.completion_percent()
    }

    /// Returns blocked mandatory leaf requirements.
    pub fn blockers(&self) -> impl Iterator<Item = &CommissioningItemReadiness> {
        self.items.iter().filter(|item| item.blocks)
    }
}

impl CommissioningControlSet {
    /// Starts an empty commissioning control set.
    #[must_use]
    pub fn new(project: ProjectId) -> Self {
        Self {
            project,
            requirements: Vec::new(),
        }
    }

    /// Adds one typed requirement.
    #[must_use]
    pub fn with_requirement(mut self, requirement: CommissioningRequirement) -> Self {
        self.requirements.push(requirement);
        self
    }

    /// Validates ids, evidence shapes, targets, and dependency endpoints.
    pub fn validate(&self, hierarchy: &HandoverHierarchy) -> Result<()> {
        if hierarchy.project() != &self.project {
            return Err(ConstructionProjectError::ProjectMismatch {
                expected: self.project.clone(),
                actual: hierarchy.project().clone(),
            });
        }
        if self.requirements.is_empty() {
            return Err(ConstructionProjectError::EmptyCollection(
                "commissioning.requirements",
            ));
        }
        let mut seen = BTreeSet::new();
        for requirement in &self.requirements {
            requirement.validate(&self.project, hierarchy)?;
            if !seen.insert(requirement.id()) {
                return Err(ConstructionProjectError::DuplicateId {
                    kind: "commissioning_requirement",
                    id: requirement.id().to_string(),
                });
            }
        }
        self.control_graph(hierarchy)?.validate_readiness()
    }

    /// Projects requirements, dependencies, targets, and membership onto the
    /// common construction control graph.
    pub fn control_graph(&self, hierarchy: &HandoverHierarchy) -> Result<ControlGraph> {
        let mut graph = hierarchy.control_graph().clone();
        for requirement in &self.requirements {
            graph.add_node(requirement.id().clone(), ControlNodeKind::Requirement)?;
        }
        for requirement in &self.requirements {
            for dependency in &requirement.obligation.requirement.dependencies {
                graph.add_edge(
                    dependency.clone(),
                    requirement.id().clone(),
                    ControlEdgeKind::Prerequisite,
                )?;
            }
            for target in &requirement.targets {
                graph.add_edge(
                    requirement.id().clone(),
                    target.clone(),
                    ControlEdgeKind::Prerequisite,
                )?;
            }
        }
        Ok(graph)
    }

    /// Derives readiness and exact burn-down for any hierarchy level.
    pub fn readiness_for(
        &self,
        hierarchy: &HandoverHierarchy,
        target: &ControlId,
        assessment: &CommissioningAssessment<'_>,
    ) -> Result<CommissioningReadinessReport> {
        self.validate(hierarchy)?;
        if assessment.book.project() != &self.project {
            return Err(ConstructionProjectError::ProjectMismatch {
                expected: self.project.clone(),
                actual: assessment.book.project().clone(),
            });
        }

        let requirements = self.requirements_for(hierarchy, target)?;
        if requirements.is_empty() {
            return Err(ConstructionProjectError::EmptyCollection(
                "commissioning.scope_requirements",
            ));
        }
        let mut policy = GatePolicy::new();
        for requirement in &requirements {
            policy = policy.with_obligation(requirement.obligation.clone());
        }
        for exception in &assessment.exceptions {
            policy = policy.with_exception(exception.clone());
        }
        for capability in &assessment.granted_capabilities {
            policy = policy.with_capability(capability.clone());
        }
        let policy_report =
            policy.evaluate(assessment.book, assessment.as_of_seq, assessment.as_of_date)?;
        let by_id = requirements
            .into_iter()
            .map(|requirement| (requirement.id().clone(), requirement))
            .collect::<BTreeMap<_, _>>();
        let mut burn_down = CommissioningBurnDown::default();
        let mut items = Vec::new();
        for explanation in policy_report.explanations {
            let requirement = by_id[&explanation.requirement];
            let blocks = requirement.obligation.policy == ObligationPolicy::Mandatory
                && explanation.exception.is_none()
                && matches!(
                    explanation.rule.as_str(),
                    "dependency" | "evidence-required"
                );
            burn_down.record(explanation.evidence_state, explanation.exception.is_some());
            items.push(CommissioningItemReadiness {
                requirement: explanation.requirement,
                kind: requirement.kind,
                targets: requirement.targets.clone(),
                evidence_state: explanation.evidence_state,
                current_seq: explanation.current_seq,
                exception: explanation.exception,
                blocks,
                critical: requirement.critical,
                rule: explanation.rule,
                reason: explanation.reason,
            });
        }

        Ok(CommissioningReadinessReport {
            target: target.clone(),
            as_of_seq: assessment.as_of_seq,
            ready: policy_report.ready,
            burn_down,
            items,
        })
    }

    fn requirements_for<'a>(
        &'a self,
        hierarchy: &HandoverHierarchy,
        target: &ControlId,
    ) -> Result<Vec<&'a CommissioningRequirement>> {
        let scope = hierarchy
            .scope(target)?
            .into_iter()
            .collect::<BTreeSet<_>>();
        let by_id = self
            .requirements
            .iter()
            .map(|requirement| (requirement.id().clone(), requirement))
            .collect::<BTreeMap<_, _>>();
        let mut selected = self
            .requirements
            .iter()
            .filter(|requirement| {
                requirement
                    .targets
                    .iter()
                    .any(|control| scope.contains(control))
            })
            .map(|requirement| requirement.id().clone())
            .collect::<BTreeSet<_>>();

        let mut frontier = selected.iter().cloned().collect::<Vec<_>>();
        while let Some(requirement_id) = frontier.pop() {
            let Some(requirement) = by_id.get(&requirement_id) else {
                continue;
            };
            for dependency in &requirement.obligation.requirement.dependencies {
                if selected.insert(dependency.clone()) {
                    frontier.push(dependency.clone());
                }
            }
        }
        Ok(selected
            .into_iter()
            .filter_map(|requirement| by_id.get(&requirement).copied())
            .collect())
    }
}
