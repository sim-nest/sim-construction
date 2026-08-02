//! Stable construction joins from project controls to canonical Gantt task ids.

use std::collections::BTreeSet;

use sim_lib_gantt::{GanttPlan, validate_gantt_plan};
use time::Date;

use crate::{BaselineId, ConstructionProjectError, ControlId, Result};

/// Baseline acceptance metadata for a construction schedule.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ScheduleBaseline {
    /// Accepted schedule baseline id.
    pub baseline: BaselineId,
    /// Stable plan id accepted for this baseline.
    pub plan_id: String,
    /// Accepted baseline revision.
    pub accepted_revision: String,
    /// Fact sequence that accepted the baseline.
    pub accepted_seq: u64,
}

impl ScheduleBaseline {
    /// Builds baseline metadata for an accepted schedule plan.
    pub fn new(
        baseline: BaselineId,
        plan_id: impl Into<String>,
        accepted_revision: impl Into<String>,
        accepted_seq: u64,
    ) -> Result<Self> {
        let baseline = Self {
            baseline,
            plan_id: plan_id.into(),
            accepted_revision: accepted_revision.into(),
            accepted_seq,
        };
        baseline.validate()?;
        Ok(baseline)
    }

    /// Validates required baseline identity, revision, and acceptance sequence.
    pub fn validate(&self) -> Result<()> {
        if self.plan_id.trim().is_empty() {
            return Err(ConstructionProjectError::EmptyField(
                "schedule_baseline.plan_id",
            ));
        }
        if self.accepted_revision.trim().is_empty() {
            return Err(ConstructionProjectError::EmptyField(
                "schedule_baseline.accepted_revision",
            ));
        }
        if self.accepted_seq == 0 {
            return Err(ConstructionProjectError::InvalidSequence {
                field: "schedule_baseline.accepted_seq",
                sequence: self.accepted_seq,
            });
        }
        Ok(())
    }
}

/// Imported plan revision evaluated against an accepted baseline.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SchedulePlanRevision {
    /// Imported plan id.
    pub plan_id: String,
    /// Imported plan revision.
    pub revision: String,
    /// Project fact sequence used for this schedule evaluation.
    pub as_of_seq: u64,
}

impl SchedulePlanRevision {
    /// Builds explicit imported schedule revision metadata.
    pub fn new(
        plan_id: impl Into<String>,
        revision: impl Into<String>,
        as_of_seq: u64,
    ) -> Result<Self> {
        let revision = Self {
            plan_id: plan_id.into(),
            revision: revision.into(),
            as_of_seq,
        };
        revision.validate()?;
        Ok(revision)
    }

    /// Validates required imported revision identity.
    pub fn validate(&self) -> Result<()> {
        if self.plan_id.trim().is_empty() {
            return Err(ConstructionProjectError::EmptyField(
                "schedule_revision.plan_id",
            ));
        }
        if self.revision.trim().is_empty() {
            return Err(ConstructionProjectError::EmptyField(
                "schedule_revision.revision",
            ));
        }
        if self.as_of_seq == 0 {
            return Err(ConstructionProjectError::InvalidSequence {
                field: "schedule_revision.as_of_seq",
                sequence: self.as_of_seq,
            });
        }
        Ok(())
    }
}

/// Construction-owned kind of a control-to-task schedule join.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ScheduleJoinKind {
    /// General stable control id.
    Control,
    /// Work package or package scope.
    Package,
    /// Design deliverable or release.
    DesignDeliverable,
    /// Accountable project decision.
    Decision,
    /// Procurement inquiry, award, or material date.
    ProcurementDate,
    /// Change record affecting planned work.
    Change,
    /// Handover item or completion requirement.
    HandoverItem,
}

/// Stable join from a construction control to a canonical Gantt task id.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ScheduleTaskJoin {
    /// Stable construction control id.
    pub control: ControlId,
    /// Stable task id in the imported `GanttPlan`.
    pub task_id: String,
    /// Construction-owned join kind.
    pub kind: ScheduleJoinKind,
    /// Explicit external need date, when the construction control has one.
    pub need_on: Option<Date>,
    /// Procurement or supplier lead time in calendar days.
    pub lead_time_days: Option<u16>,
}

impl ScheduleTaskJoin {
    /// Builds a control-to-task join.
    #[must_use]
    pub fn new(control: ControlId, task_id: impl Into<String>, kind: ScheduleJoinKind) -> Self {
        Self {
            control,
            task_id: task_id.into(),
            kind,
            need_on: None,
            lead_time_days: None,
        }
    }

    /// Adds an explicit need date.
    #[must_use]
    pub fn needs_on(mut self, need_on: Date) -> Self {
        self.need_on = Some(need_on);
        self
    }

    /// Adds a procurement or supplier lead time.
    #[must_use]
    pub fn with_lead_time(mut self, lead_time_days: u16) -> Self {
        self.lead_time_days = Some(lead_time_days);
        self
    }
}

/// A baseline-scoped set of construction control joins into a Gantt plan.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ScheduleTaskJoinSet {
    /// Accepted baseline used for the joins.
    pub baseline: ScheduleBaseline,
    /// Imported revision being evaluated.
    pub revision: SchedulePlanRevision,
    /// Stable construction joins.
    pub joins: Vec<ScheduleTaskJoin>,
}

impl ScheduleTaskJoinSet {
    /// Builds a baseline-scoped join set.
    pub fn new(
        baseline: ScheduleBaseline,
        revision: SchedulePlanRevision,
        joins: Vec<ScheduleTaskJoin>,
    ) -> Result<Self> {
        let set = Self {
            baseline,
            revision,
            joins,
        };
        set.validate_revision()?;
        Ok(set)
    }

    /// Validates plan metadata, revision acceptance, task ids, and join uniqueness.
    pub fn validate_against_plan(&self, plan: &GanttPlan) -> Result<()> {
        self.validate_revision()?;
        validate_gantt_plan(plan).map_err(|error| ConstructionProjectError::SchedulePlan {
            reason: error.to_string(),
        })?;
        if plan.id != self.baseline.plan_id || plan.id != self.revision.plan_id {
            return Err(ConstructionProjectError::SchedulePlanMismatch {
                baseline_plan: self.baseline.plan_id.clone(),
                imported_plan: self.revision.plan_id.clone(),
                actual_plan: plan.id.clone(),
            });
        }

        let mut joined_controls = BTreeSet::new();
        let mut joined_tasks = BTreeSet::new();
        for join in &self.joins {
            if join.task_id.trim().is_empty() {
                return Err(ConstructionProjectError::EmptyField(
                    "schedule_join.task_id",
                ));
            }
            if plan.task(&join.task_id).is_none() {
                return Err(ConstructionProjectError::MissingScheduleTask {
                    control: join.control.clone(),
                    task_id: join.task_id.clone(),
                });
            }
            if !joined_controls.insert(join.control.clone()) {
                return Err(ConstructionProjectError::DuplicateId {
                    kind: "schedule_join.control",
                    id: join.control.as_str().to_owned(),
                });
            }
            if !joined_tasks.insert(join.task_id.clone()) {
                return Err(ConstructionProjectError::DuplicateScheduleTaskJoin {
                    task_id: join.task_id.clone(),
                });
            }
        }
        Ok(())
    }

    fn validate_revision(&self) -> Result<()> {
        self.baseline.validate()?;
        self.revision.validate()?;
        if self.baseline.accepted_revision != self.revision.revision {
            return Err(ConstructionProjectError::ScheduleRevisionMismatch {
                baseline: self.baseline.baseline.clone(),
                accepted_revision: self.baseline.accepted_revision.clone(),
                imported_revision: self.revision.revision.clone(),
            });
        }
        if self.revision.as_of_seq < self.baseline.accepted_seq {
            return Err(ConstructionProjectError::StaleBaselineComparison {
                baseline: ControlId::new(self.baseline.baseline.as_str())?,
                accepted_seq: self.baseline.accepted_seq,
                as_of_seq: self.revision.as_of_seq,
            });
        }
        Ok(())
    }
}
