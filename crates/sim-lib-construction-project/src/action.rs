//! Accountable project actions.

use sim_lib_doc_core::ExternalRef;
use time::Date;

use crate::{ConstructionProjectError, ControlId, ProjectId, ProjectSnapshot, Result, RoleId};

/// State of an accountable project action.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ActionState {
    /// Action is open.
    Open,
    /// Action is escalated after the normal due path.
    Escalated,
    /// Action is closed and must retain a resolution fact.
    Closed,
}

/// Resolution retained by a closed action.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ActionResolution {
    /// Fact sequence carrying the resolution.
    pub fact_seq: u64,
    /// Role that accepted the resolution.
    pub resolved_by: RoleId,
    /// Resolution summary.
    pub summary: String,
}

/// Accountable action with due date, escalation, consequence, state, and evidence.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProjectAction {
    /// Stable construction project identity.
    pub project: ProjectId,
    /// Control subject for the action.
    pub control: ControlId,
    /// Role that owns the action.
    pub owner: RoleId,
    /// Calendar due date.
    pub due_on: Date,
    /// Escalation target role.
    pub escalation_target: RoleId,
    /// Consequence when the action is not resolved.
    pub consequence: String,
    /// Action state.
    pub state: ActionState,
    /// Control ids this action depends on.
    pub references: Vec<ControlId>,
    /// Reference-only evidence links.
    pub evidence: Vec<ExternalRef>,
    /// Resolution retained after closure.
    pub resolution: Option<ActionResolution>,
}

impl ProjectAction {
    /// Builds an open project action.
    #[must_use]
    pub fn new(
        project: ProjectId,
        control: ControlId,
        owner: RoleId,
        due_on: Date,
        escalation_target: RoleId,
        consequence: impl Into<String>,
    ) -> Self {
        Self {
            project,
            control,
            owner,
            due_on,
            escalation_target,
            consequence: consequence.into(),
            state: ActionState::Open,
            references: Vec::new(),
            evidence: Vec::new(),
            resolution: None,
        }
    }

    /// Sets the action state.
    #[must_use]
    pub fn with_state(mut self, state: ActionState) -> Self {
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

    /// Sets the resolution retained by a closed action.
    #[must_use]
    pub fn with_resolution(mut self, resolution: ActionResolution) -> Self {
        self.resolution = Some(resolution);
        self
    }

    /// Validates required evidence, resolution, and referenced controls.
    pub fn validate_against(&self, snapshot: &ProjectSnapshot) -> Result<()> {
        if self.consequence.trim().is_empty() {
            return Err(ConstructionProjectError::EmptyField("action.consequence"));
        }
        if self.evidence.is_empty() {
            return Err(ConstructionProjectError::EmptyCollection("action.evidence"));
        }
        if self.state == ActionState::Closed && self.resolution.is_none() {
            return Err(ConstructionProjectError::MissingResolutionFact {
                kind: "action",
                control: self.control.clone(),
            });
        }
        if let Some(resolution) = &self.resolution {
            validate_resolution("action.resolution", resolution.fact_seq)?;
            if resolution.summary.trim().is_empty() {
                return Err(ConstructionProjectError::EmptyField(
                    "action.resolution.summary",
                ));
            }
        }
        validate_refs(&self.references, snapshot)
    }
}

fn validate_resolution(field: &'static str, sequence: u64) -> Result<()> {
    if sequence == 0 {
        return Err(ConstructionProjectError::InvalidSequence { field, sequence });
    }
    Ok(())
}

pub(crate) fn validate_refs(references: &[ControlId], snapshot: &ProjectSnapshot) -> Result<()> {
    for control in references {
        if !snapshot.current.contains_key(control) {
            return Err(ConstructionProjectError::OrphanControlRef {
                control: control.clone(),
                as_of_seq: snapshot.through_seq,
            });
        }
    }
    Ok(())
}
