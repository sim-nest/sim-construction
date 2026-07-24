//! Evidence-aware construction gate policy and explanations.

use std::collections::{BTreeMap, BTreeSet};

use time::Date;

use crate::{
    ConstructionProjectError, ControlEdgeKind, ControlExplanationPath, ControlGraph, ControlId,
    ControlNodeKind, EvidenceState, ExceptionDecision, ObligationPolicy, ProjectBook,
    ProjectObligation, Result,
};

/// Stable explanation for one requirement under a gate policy.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RequirementExplanation {
    /// Requirement id.
    pub requirement: ControlId,
    /// Current fact sequence, when one exists.
    pub current_seq: Option<u64>,
    /// Rule used to derive this explanation.
    pub rule: String,
    /// Current evidence state.
    pub evidence_state: EvidenceState,
    /// Dependency requirement ids.
    pub dependencies: Vec<ControlId>,
    /// Applied exception id, when one covers the obligation.
    pub exception: Option<ControlId>,
    /// Stable blocker-to-requirement paths from the canonical control graph.
    pub paths: Vec<ControlExplanationPath>,
    /// Human-readable deterministic reason.
    pub reason: String,
}

/// Derived gate-policy report.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GatePolicyReport {
    /// Snapshot sequence used for derivation.
    pub as_of_seq: u64,
    /// True when no mandatory obligation remains blocked.
    pub ready: bool,
    /// Requirement explanations in stable requirement-id order.
    pub explanations: Vec<RequirementExplanation>,
}

/// Evidence-aware policy for mixed construction obligations.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GatePolicy {
    /// Project obligations checked by this policy.
    pub obligations: Vec<ProjectObligation>,
    /// Exception decisions available to the policy.
    pub exceptions: Vec<ExceptionDecision>,
    /// Capability names granted for policy evaluation.
    pub granted_capabilities: Vec<String>,
}

impl GatePolicy {
    /// Builds an empty gate policy.
    #[must_use]
    pub fn new() -> Self {
        Self {
            obligations: Vec::new(),
            exceptions: Vec::new(),
            granted_capabilities: Vec::new(),
        }
    }

    /// Adds one obligation.
    #[must_use]
    pub fn with_obligation(mut self, obligation: ProjectObligation) -> Self {
        self.obligations.push(obligation);
        self
    }

    /// Adds one exception.
    #[must_use]
    pub fn with_exception(mut self, exception: ExceptionDecision) -> Self {
        self.exceptions.push(exception);
        self
    }

    /// Adds one granted capability name.
    #[must_use]
    pub fn with_capability(mut self, capability: impl Into<String>) -> Self {
        self.granted_capabilities.push(capability.into());
        self
    }

    /// Evaluates the policy against the fact book.
    pub fn evaluate(
        &self,
        book: &ProjectBook,
        as_of_seq: u64,
        as_of_date: Date,
    ) -> Result<GatePolicyReport> {
        if self.obligations.is_empty() {
            return Err(ConstructionProjectError::EmptyCollection(
                "gate_policy.obligations",
            ));
        }
        let snapshot = book.snapshot_at(as_of_seq)?;
        let mut graph = ControlGraph::new();
        for obligation in &self.obligations {
            graph.add_node(
                obligation.requirement.id.clone(),
                ControlNodeKind::Requirement,
            )?;
        }
        for obligation in &self.obligations {
            obligation.requirement.validate()?;
            let requirement = &obligation.requirement.id;
            for dependency in &obligation.requirement.dependencies {
                graph.add_edge(
                    dependency.clone(),
                    requirement.clone(),
                    ControlEdgeKind::Prerequisite,
                )?;
            }
        }
        graph.validate_readiness()?;

        let mut fact_states = BTreeMap::new();
        for obligation in &self.obligations {
            let requirement = &obligation.requirement.id;
            let (current_seq, evidence_state) = if snapshot.conflicted.contains_key(requirement) {
                (None, EvidenceState::Conflicted)
            } else if let Some(fact) = snapshot.current.get(requirement) {
                let state = if fact.evidence_state == EvidenceState::Reported
                    && !fact.evidence.is_empty()
                {
                    EvidenceState::Evidenced
                } else if fact.evidence_state == EvidenceState::Accepted && fact.evidence.is_empty()
                {
                    EvidenceState::Reported
                } else {
                    fact.evidence_state
                };
                (Some(fact.seq), state)
            } else if let Some(rejected) = snapshot.rejected.get(requirement) {
                let seq = rejected.iter().map(|fact| fact.seq).max();
                (seq, EvidenceState::Rejected)
            } else {
                (None, EvidenceState::Missing)
            };
            fact_states.insert(requirement.clone(), (current_seq, evidence_state));
        }

        let mut exceptions = BTreeMap::new();
        for obligation in self.sorted_obligations() {
            exceptions.insert(
                obligation.requirement.id.clone(),
                self.find_exception(obligation, as_of_date)?,
            );
        }

        let mut effective_states = BTreeMap::new();
        for obligation in self.sorted_obligations() {
            let (current_seq, evidence_state) = fact_states
                .get(&obligation.requirement.id)
                .cloned()
                .unwrap_or((None, EvidenceState::Missing));
            let evidence_state = if current_seq.is_some()
                && evidence_state.satisfies_required_evidence()
                && !obligation.evidence_validity.contains(as_of_date)
            {
                EvidenceState::Expired
            } else {
                evidence_state
            };
            effective_states.insert(
                obligation.requirement.id.clone(),
                (current_seq, evidence_state),
            );
        }

        let mut explanations = Vec::new();
        let mut ready = true;
        for obligation in self.sorted_obligations() {
            let requirement = &obligation.requirement;
            let (current_seq, evidence_state) = fact_states
                .get(&requirement.id)
                .cloned()
                .unwrap_or((None, EvidenceState::Missing));
            let evidence_state = if current_seq.is_some()
                && evidence_state.satisfies_required_evidence()
                && !obligation.evidence_validity.contains(as_of_date)
            {
                EvidenceState::Expired
            } else {
                evidence_state
            };
            let exception = exceptions.get(&requirement.id).cloned().flatten();
            let analysis = graph.analyze_target(
                &requirement.id,
                |control| {
                    if exceptions.get(control).is_some_and(Option::is_some) {
                        return false;
                    }
                    let Some(dependency) = self
                        .obligations
                        .iter()
                        .find(|candidate| candidate.requirement.id == *control)
                    else {
                        return true;
                    };
                    if dependency.policy == ObligationPolicy::Optional
                        || !dependency.requirement.evidence_required
                    {
                        return false;
                    }
                    effective_states
                        .get(control)
                        .is_none_or(|(_, state)| !state.satisfies_required_evidence())
                },
                |control| {
                    effective_states
                        .get(control)
                        .cloned()
                        .unwrap_or((None, EvidenceState::Missing))
                },
                |control| exceptions.get(control).cloned().flatten(),
            )?;
            let blocked_dependencies = analysis.transitive_blockers;
            let mut rule = "mandatory".to_owned();
            let mut reason = "accepted evidence satisfies the obligation".to_owned();
            let mut blocks_gate = false;

            if obligation.policy == ObligationPolicy::Optional {
                rule = "optional".to_owned();
                reason = "optional obligation is reported but does not block the gate".to_owned();
            } else if let Some(exception) = &exception {
                rule = "exception".to_owned();
                reason = format!("bounded exception {exception} covers the obligation");
            } else if !blocked_dependencies.is_empty() {
                rule = "dependency".to_owned();
                reason = "dependency is not satisfied".to_owned();
                blocks_gate = true;
            } else if requirement.evidence_required && !evidence_state.satisfies_required_evidence()
            {
                rule = "evidence-required".to_owned();
                reason = match evidence_state {
                    EvidenceState::Missing => "evidence is missing",
                    EvidenceState::Reported => "reported without accepted evidence",
                    EvidenceState::Evidenced => "evidence is present but not accepted",
                    EvidenceState::Rejected => "evidence was rejected",
                    EvidenceState::Expired => "evidence is expired",
                    EvidenceState::Conflicted => "evidence is conflicted",
                    EvidenceState::Accepted => "accepted evidence satisfies the obligation",
                }
                .to_owned();
                blocks_gate = true;
            }

            if blocks_gate {
                ready = false;
            }
            explanations.push(RequirementExplanation {
                requirement: requirement.id.clone(),
                current_seq,
                rule,
                evidence_state,
                dependencies: requirement.dependencies.clone(),
                exception,
                paths: analysis.explanation_paths,
                reason,
            });
        }

        Ok(GatePolicyReport {
            as_of_seq,
            ready,
            explanations,
        })
    }

    fn sorted_obligations(&self) -> Vec<&ProjectObligation> {
        let mut obligations = self.obligations.iter().collect::<Vec<_>>();
        obligations.sort_by(|left, right| left.requirement.id.cmp(&right.requirement.id));
        obligations
    }

    fn find_exception(
        &self,
        obligation: &ProjectObligation,
        as_of_date: Date,
    ) -> Result<Option<ControlId>> {
        if obligation.requirement.non_waivable {
            for exception in &self.exceptions {
                if exception
                    .scope
                    .covers_requirement(&obligation.requirement.id)
                {
                    return Err(ConstructionProjectError::NonWaivableRequirement {
                        requirement: obligation.requirement.id.clone(),
                        exception: exception.id.clone(),
                    });
                }
            }
            return Ok(None);
        }

        let mut seen = BTreeSet::new();
        for exception in &self.exceptions {
            if !exception
                .scope
                .covers_requirement(&obligation.requirement.id)
            {
                continue;
            }
            if !seen.insert(exception.id.clone()) {
                return Err(ConstructionProjectError::DuplicateId {
                    kind: "exception",
                    id: exception.id.to_string(),
                });
            }
            exception.validate(&self.granted_capabilities, as_of_date)?;
            return Ok(Some(exception.id.clone()));
        }
        Ok(None)
    }
}

impl Default for GatePolicy {
    fn default() -> Self {
        Self::new()
    }
}
