//! Hierarchy roll-up of commissioning evidence.

use std::collections::{BTreeMap, BTreeSet};

use time::Date;

use crate::{
    CommissioningControlSet, CommissioningRequirement, CommissioningRequirementKind,
    ConstructionProjectError, ControlId, EvidenceState, ExceptionDecision, GatePolicy,
    HandoverGateKind, HandoverHierarchy, ObligationPolicy, ProjectBook, Result,
};

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
    /// Derives readiness and exact burn-down for any hierarchy level.
    pub fn readiness_for(
        &self,
        hierarchy: &HandoverHierarchy,
        target: &ControlId,
        assessment: &CommissioningAssessment<'_>,
    ) -> Result<CommissioningReadinessReport> {
        self.derive_readiness(hierarchy, target, assessment, None)
    }

    pub(crate) fn readiness_for_gate(
        &self,
        hierarchy: &HandoverHierarchy,
        target: &ControlId,
        assessment: &CommissioningAssessment<'_>,
        gate: HandoverGateKind,
    ) -> Result<CommissioningReadinessReport> {
        self.derive_readiness(hierarchy, target, assessment, Some(gate))
    }

    fn derive_readiness(
        &self,
        hierarchy: &HandoverHierarchy,
        target: &ControlId,
        assessment: &CommissioningAssessment<'_>,
        gate: Option<HandoverGateKind>,
    ) -> Result<CommissioningReadinessReport> {
        self.validate(hierarchy)?;
        if assessment.book.project() != &self.project {
            return Err(ConstructionProjectError::ProjectMismatch {
                expected: self.project.clone(),
                actual: assessment.book.project().clone(),
            });
        }

        let requirements = self.requirements_for(hierarchy, target, gate)?;
        if requirements.is_empty() {
            return Err(ConstructionProjectError::EmptyCollection(
                "commissioning.scope_requirements",
            ));
        }
        let mut policy = GatePolicy::new();
        for requirement in &requirements {
            policy = policy.with_obligation(requirement.obligation.clone());
        }
        for exception in self.applicable_exceptions(&requirements, assessment, gate) {
            policy = policy.with_exception(exception);
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
        gate: Option<HandoverGateKind>,
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
            .filter(|requirement| gate.is_none_or(|gate| requirement.required_at(gate)))
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

    fn applicable_exceptions(
        &self,
        requirements: &[&CommissioningRequirement],
        assessment: &CommissioningAssessment<'_>,
        gate: Option<HandoverGateKind>,
    ) -> Vec<ExceptionDecision> {
        let allowed = requirements
            .iter()
            .filter(|requirement| gate.is_none_or(|gate| requirement.exception_allowed_at(gate)))
            .map(|requirement| requirement.id())
            .collect::<BTreeSet<_>>();
        assessment
            .exceptions
            .iter()
            .filter_map(|exception| {
                let mut exception = exception.clone();
                exception
                    .scope
                    .requirements
                    .retain(|requirement| allowed.contains(requirement));
                (!exception.scope.requirements.is_empty()).then_some(exception)
            })
            .collect()
    }
}
