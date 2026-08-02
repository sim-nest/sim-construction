//! Production lookahead records shared by readiness derivation and reports.

use sim_lib_doc_core::ExternalRef;
use time::Date;

use crate::{BaselineId, ConstructionProjectError, ControlId, EvidenceState, Result, RoleId};

/// Lookahead window used for near-term production planning.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum LookaheadWindow {
    /// Task starts inside the six-week demand window.
    SixWeekDemand,
    /// Task starts inside the three-week commitment window.
    ThreeWeekCommitment,
}

/// Computed readiness state for a production activity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ProductionReadinessState {
    /// All shared requirements are accepted or covered by a valid waivable exception.
    Ready,
    /// One or more shared requirements currently block production.
    NotReady,
    /// The activity lacks enough shared requirement or schedule evidence to decide.
    Unknown,
}

/// Stable production activity joined to a Gantt task and work package.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProductionActivity {
    /// Stable production activity control id.
    pub control: ControlId,
    /// Joined canonical Gantt task id.
    pub task_id: String,
    /// Work package this activity executes.
    pub work_package: ControlId,
    /// Building system or discipline.
    pub system: String,
    /// Area.
    pub area: String,
    /// Physical location.
    pub location: String,
    /// Responsible production role.
    pub responsible_role: RoleId,
    /// Planned production start.
    pub planned_start: Date,
    /// Planned production finish.
    pub planned_finish: Date,
    /// Accepted schedule baseline this activity belongs to.
    pub accepted_baseline: BaselineId,
    /// Shared requirement ids that prove production readiness.
    pub requirements: Vec<ControlId>,
}

impl ProductionActivity {
    /// Builds a production activity joined to an accepted schedule task.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        control: ControlId,
        task_id: impl Into<String>,
        work_package: ControlId,
        system: impl Into<String>,
        area: impl Into<String>,
        location: impl Into<String>,
        responsible_role: RoleId,
        planned_start: Date,
        planned_finish: Date,
        accepted_baseline: BaselineId,
    ) -> Self {
        Self {
            control,
            task_id: task_id.into(),
            work_package,
            system: system.into(),
            area: area.into(),
            location: location.into(),
            responsible_role,
            planned_start,
            planned_finish,
            accepted_baseline,
            requirements: Vec::new(),
        }
    }

    /// Adds a shared production-readiness requirement.
    #[must_use]
    pub fn requires(mut self, requirement: ControlId) -> Self {
        self.requirements.push(requirement);
        self
    }

    /// Validates the activity shape before derivation.
    pub fn validate(&self) -> Result<()> {
        if self.task_id.trim().is_empty() {
            return Err(ConstructionProjectError::EmptyField(
                "production_activity.task_id",
            ));
        }
        require_text(&self.system, "production_activity.system")?;
        require_text(&self.area, "production_activity.area")?;
        require_text(&self.location, "production_activity.location")?;
        if self.planned_finish < self.planned_start {
            return Err(ConstructionProjectError::InvalidSnapshotRange {
                from_seq: self.planned_finish.to_julian_day() as u64,
                through_seq: self.planned_start.to_julian_day() as u64,
            });
        }
        if self.requirements.is_empty() {
            return Err(ConstructionProjectError::EmptyCollection(
                "production_activity.requirements",
            ));
        }
        Ok(())
    }
}

/// Accepted baseline position retained for movement detection.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AcceptedTaskWindow {
    /// Stable task id.
    pub task_id: String,
    /// Accepted baseline task start.
    pub start: Date,
    /// Accepted baseline task finish.
    pub finish: Date,
}

impl AcceptedTaskWindow {
    /// Builds a retained accepted task window.
    #[must_use]
    pub fn new(task_id: impl Into<String>, start: Date, finish: Date) -> Self {
        Self {
            task_id: task_id.into(),
            start,
            finish,
        }
    }
}

/// Accountable human commitment fact, separate from computed readiness.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProductionCommitment {
    /// Activity being committed.
    pub activity: ControlId,
    /// Role making the commitment.
    pub committed_by: RoleId,
    /// Commitment date.
    pub committed_on: Date,
    /// Fact sequence carrying the commitment.
    pub seq: u64,
    /// Commitment text.
    pub note: String,
    /// Reference evidence for the commitment.
    pub evidence: Vec<ExternalRef>,
}

impl ProductionCommitment {
    /// Builds an accountable production commitment.
    #[must_use]
    pub fn new(
        activity: ControlId,
        committed_by: RoleId,
        committed_on: Date,
        seq: u64,
        note: impl Into<String>,
    ) -> Self {
        Self {
            activity,
            committed_by,
            committed_on,
            seq,
            note: note.into(),
            evidence: Vec::new(),
        }
    }

    /// Adds reference-only commitment evidence.
    #[must_use]
    pub fn with_evidence(mut self, evidence: ExternalRef) -> Self {
        self.evidence.push(evidence);
        self
    }

    pub(crate) fn validate(&self) -> Result<()> {
        if self.seq == 0 {
            return Err(ConstructionProjectError::InvalidSequence {
                field: "production_commitment.seq",
                sequence: self.seq,
            });
        }
        require_text(&self.note, "production_commitment.note")?;
        if self.evidence.is_empty() {
            return Err(ConstructionProjectError::EmptyCollection(
                "production_commitment.evidence",
            ));
        }
        Ok(())
    }
}

/// Movement from the retained accepted task window to the evaluated plan.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProductionTaskMovement {
    /// Accepted baseline start.
    pub accepted_start: Date,
    /// Accepted baseline finish.
    pub accepted_finish: Date,
    /// Evaluated plan start.
    pub current_start: Date,
    /// Evaluated plan finish.
    pub current_finish: Date,
}

/// One explicit production-readiness constraint.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProductionConstraint {
    /// Shared requirement causing the constraint.
    pub requirement: ControlId,
    /// Accountable owner role from the shared requirement.
    pub owner: RoleId,
    /// Date the constraint must clear.
    pub need_on: Date,
    /// Current evidence state.
    pub evidence_state: EvidenceState,
    /// Current fact sequence, when known.
    pub current_seq: Option<u64>,
    /// Accepted exception, when a waivable requirement is covered.
    pub exception: Option<ControlId>,
    /// True when exception policy cannot waive this requirement.
    pub non_waivable: bool,
    /// Escalation target role.
    pub escalation: RoleId,
    /// Consequence of missing the need date.
    pub consequence: String,
    /// Deterministic explanation retained with the snapshot.
    pub explanation: String,
}

/// Derived readiness for one activity.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProductionActivityReadiness {
    /// Activity control id.
    pub activity: ControlId,
    /// Joined Gantt task id.
    pub task_id: String,
    /// Work package.
    pub work_package: ControlId,
    /// Lookahead window.
    pub window: LookaheadWindow,
    /// Computed readiness state.
    pub state: ProductionReadinessState,
    /// Human commitment fact, when present.
    pub commitment: Option<ProductionCommitment>,
    /// Readiness constraints.
    pub constraints: Vec<ProductionConstraint>,
    /// Baseline movement, when the task dates changed.
    pub movement: Option<ProductionTaskMovement>,
    /// Sequence used for the snapshot.
    pub as_of_seq: u64,
    /// Deterministic explanation for weekly reliability review.
    pub explanation: String,
}

/// Production readiness snapshot for a six-week and three-week lookahead.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProductionReadinessSnapshot {
    /// Accepted schedule baseline.
    pub baseline: BaselineId,
    /// Accepted baseline revision.
    pub accepted_revision: String,
    /// Imported revision used for this evaluation.
    pub imported_revision: String,
    /// Project fact sequence used for this snapshot.
    pub as_of_seq: u64,
    /// Evaluation date.
    pub as_of_date: Date,
    /// Activities in the six-week demand window.
    pub six_week_demand: Vec<ProductionActivityReadiness>,
    /// Activities in the three-week commitment window.
    pub three_week_commitment: Vec<ProductionActivityReadiness>,
}

fn require_text(value: &str, field: &'static str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(ConstructionProjectError::EmptyField(field));
    }
    Ok(())
}
