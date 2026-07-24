//! Accountable project decisions.

use sim_lib_doc_core::ExternalRef;
use time::Date;

use crate::{
    ConstructionProjectError, ControlId, ProjectId, ProjectSnapshot, Result, RoleId,
    action::validate_refs,
};

/// State of an accountable project decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DecisionState {
    /// Decision is open.
    Open,
    /// Decision is escalated.
    Escalated,
    /// Decision is closed and must retain a resolution fact.
    Closed,
}

/// Resolution retained by a closed decision.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DecisionResolution {
    /// Fact sequence carrying the resolution.
    pub fact_seq: u64,
    /// Role that made the decision.
    pub decided_by: RoleId,
    /// Decision outcome.
    pub outcome: String,
}

/// Accountable decision with authority, due date, escalation, consequence, and evidence.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProjectDecision {
    /// Stable construction project identity.
    pub project: ProjectId,
    /// Control subject for the decision.
    pub control: ControlId,
    /// Role that owns preparing the decision.
    pub owner: RoleId,
    /// Calendar due date.
    pub due_on: Date,
    /// Role authorized to decide.
    pub decision_authority: RoleId,
    /// Escalation target role.
    pub escalation_target: RoleId,
    /// Consequence when the decision is late.
    pub consequence: String,
    /// Decision state.
    pub state: DecisionState,
    /// Control ids this decision depends on.
    pub references: Vec<ControlId>,
    /// Reference-only evidence links.
    pub evidence: Vec<ExternalRef>,
    /// Resolution retained after closure.
    pub resolution: Option<DecisionResolution>,
}

impl ProjectDecision {
    /// Builds an open project decision.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        project: ProjectId,
        control: ControlId,
        owner: RoleId,
        due_on: Date,
        decision_authority: RoleId,
        escalation_target: RoleId,
        consequence: impl Into<String>,
    ) -> Self {
        Self {
            project,
            control,
            owner,
            due_on,
            decision_authority,
            escalation_target,
            consequence: consequence.into(),
            state: DecisionState::Open,
            references: Vec::new(),
            evidence: Vec::new(),
            resolution: None,
        }
    }

    /// Sets the decision state.
    #[must_use]
    pub fn with_state(mut self, state: DecisionState) -> Self {
        self.state = state;
        self
    }

    /// Adds a control reference.
    #[must_use]
    pub fn with_reference(mut self, control: ControlId) -> Self {
        self.references.push(control);
        self
    }

    /// Adds a reference-only evidence link.
    #[must_use]
    pub fn with_evidence(mut self, evidence: ExternalRef) -> Self {
        self.evidence.push(evidence);
        self
    }

    /// Sets the resolution retained by a closed decision.
    #[must_use]
    pub fn with_resolution(mut self, resolution: DecisionResolution) -> Self {
        self.resolution = Some(resolution);
        self
    }

    /// Validates required evidence, authority resolution, and referenced controls.
    pub fn validate_against(&self, snapshot: &ProjectSnapshot) -> Result<()> {
        if self.consequence.trim().is_empty() {
            return Err(ConstructionProjectError::EmptyField("decision.consequence"));
        }
        if self.evidence.is_empty() {
            return Err(ConstructionProjectError::EmptyCollection(
                "decision.evidence",
            ));
        }
        if self.state == DecisionState::Closed && self.resolution.is_none() {
            return Err(ConstructionProjectError::MissingResolutionFact {
                kind: "decision",
                control: self.control.clone(),
            });
        }
        if let Some(resolution) = &self.resolution {
            if resolution.fact_seq == 0 {
                return Err(ConstructionProjectError::InvalidSequence {
                    field: "decision.resolution",
                    sequence: resolution.fact_seq,
                });
            }
            if resolution.decided_by != self.decision_authority {
                return Err(ConstructionProjectError::DecisionAuthorityMismatch {
                    decision: self.control.clone(),
                    expected: self.decision_authority.clone(),
                    actual: resolution.decided_by.clone(),
                });
            }
            if resolution.outcome.trim().is_empty() {
                return Err(ConstructionProjectError::EmptyField(
                    "decision.resolution.outcome",
                ));
            }
        }
        validate_refs(&self.references, snapshot)
    }
}
