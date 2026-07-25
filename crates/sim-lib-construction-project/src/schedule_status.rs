//! Schedule status derived by composing construction controls with Gantt facts.

use std::collections::{BTreeMap, BTreeSet};

use sim_kernel::Cx;
use sim_lib_gantt::{GanttPlan, critical_tasks, validate_gantt_plan};
use time::{Date, Duration};

use crate::{
    ConstructionProjectError, ControlGraph, ControlId, EvidenceState, Result, ScheduleJoinKind,
    ScheduleTaskJoin, ScheduleTaskJoinSet,
};

/// Evidence state for one construction control at a project sequence.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ScheduleControlState {
    /// Stable control id.
    pub control: ControlId,
    /// Current fact sequence for this control.
    pub current_seq: Option<u64>,
    /// Evidence state at the evaluated sequence.
    pub evidence_state: EvidenceState,
    /// Applied exception, when one covers this control.
    pub exception: Option<ControlId>,
}

impl ScheduleControlState {
    /// Builds a control state.
    #[must_use]
    pub fn new(control: ControlId, evidence_state: EvidenceState) -> Self {
        Self {
            control,
            current_seq: None,
            evidence_state,
            exception: None,
        }
    }

    /// Adds the current fact sequence.
    #[must_use]
    pub fn at_sequence(mut self, sequence: u64) -> Self {
        self.current_seq = Some(sequence);
        self
    }

    /// Adds an applied exception id.
    #[must_use]
    pub fn with_exception(mut self, exception: ControlId) -> Self {
        self.exception = Some(exception);
        self
    }
}

/// Explanation category for a schedule consequence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ScheduleExplanationKind {
    /// Joined control sits on the Gantt critical path.
    CriticalBlocker,
    /// Joined control is not critical but its date is already exposed.
    NonCriticalDelay,
    /// Procurement or supplier lead time moves the need date earlier than task start.
    ProcurementLeadTime,
    /// Change control affects downstream schedule controls.
    ChangeImpact,
    /// Accountable decision is late for the task need date.
    LateDecision,
}

/// Derived schedule consequence for one joined control.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ScheduleImpactExplanation {
    /// Explanation category.
    pub kind: ScheduleExplanationKind,
    /// Joined construction control.
    pub control: ControlId,
    /// Joined Gantt task id.
    pub task_id: String,
    /// Scheduled task name.
    pub task_name: String,
    /// Date the construction control needs the task ready.
    pub need_on: Date,
    /// Task start date in the accepted baseline plan.
    pub task_start: Date,
    /// Task finish date in the accepted baseline plan.
    pub task_finish: Date,
    /// True when Gantt critical-path analysis marks the task critical.
    pub critical: bool,
    /// Downstream construction controls reached through the control graph.
    pub downstream_controls: Vec<ControlId>,
    /// Blocking construction controls that can reach this control.
    pub blockers: Vec<ControlId>,
    /// Explanation text suitable for user-facing schedule reports.
    pub explanation: String,
}

/// Baseline-aware schedule status report for construction controls.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ScheduleStatusReport {
    /// Accepted schedule baseline id.
    pub baseline: crate::BaselineId,
    /// Accepted baseline revision.
    pub accepted_revision: String,
    /// Evaluation sequence.
    pub as_of_seq: u64,
    /// Critical Gantt task ids from canonical schedule analysis.
    pub critical_tasks: Vec<String>,
    /// Per-join impact explanations in stable order.
    pub explanations: Vec<ScheduleImpactExplanation>,
}

/// Derives construction schedule status from canonical Gantt facts and the control graph.
pub fn explain_schedule_impact(
    cx: &mut Cx,
    plan: &GanttPlan,
    joins: &ScheduleTaskJoinSet,
    graph: &ControlGraph,
    states: &[ScheduleControlState],
    as_of_date: Date,
) -> Result<ScheduleStatusReport> {
    joins.validate_against_plan(plan)?;
    validate_gantt_plan(plan).map_err(|error| ConstructionProjectError::SchedulePlan {
        reason: error.to_string(),
    })?;
    graph.validate_readiness()?;

    let critical =
        critical_tasks(cx, plan).map_err(|error| ConstructionProjectError::SchedulePlan {
            reason: error.to_string(),
        })?;
    let critical_set = critical.iter().cloned().collect::<BTreeSet<_>>();
    let state_by_control = state_map(states)?;
    let mut explanations = Vec::new();

    for join in sorted_joins(&joins.joins) {
        let task = plan.task(&join.task_id).ok_or_else(|| {
            ConstructionProjectError::MissingScheduleTask {
                control: join.control.clone(),
                task_id: join.task_id.clone(),
            }
        })?;
        let analysis = graph.analyze_target(
            &join.control,
            |control| {
                state_by_control
                    .get(control)
                    .is_none_or(|state| !state.evidence_state.satisfies_required_evidence())
            },
            |control| {
                state_by_control
                    .get(control)
                    .map_or((None, EvidenceState::Missing), |state| {
                        (state.current_seq, state.evidence_state)
                    })
            },
            |control| {
                state_by_control
                    .get(control)
                    .and_then(|state| state.exception.clone())
            },
        )?;

        let need_on = need_date(join, task.start);
        let critical_task = critical_set.contains(&join.task_id);
        if critical_task && !analysis.transitive_blockers.is_empty() {
            explanations.push(explanation(
                ScheduleExplanationKind::CriticalBlocker,
                join,
                task.name.clone(),
                need_on,
                task.start,
                task.finish,
                critical_task,
                analysis.affected_dependents.clone(),
                analysis.transitive_blockers.clone(),
                "critical-path joined control is blocked",
            ));
        }
        if reports_generic_delay(join.kind) && !critical_task && as_of_date > need_on {
            explanations.push(explanation(
                ScheduleExplanationKind::NonCriticalDelay,
                join,
                task.name.clone(),
                need_on,
                task.start,
                task.finish,
                critical_task,
                analysis.affected_dependents.clone(),
                analysis.transitive_blockers.clone(),
                "non-critical joined control is past its need date",
            ));
        }
        if join.lead_time_days.is_some() {
            explanations.push(explanation(
                ScheduleExplanationKind::ProcurementLeadTime,
                join,
                task.name.clone(),
                need_on,
                task.start,
                task.finish,
                critical_task,
                analysis.affected_dependents.clone(),
                analysis.transitive_blockers.clone(),
                "procurement lead time moves the need date before task start",
            ));
        }
        if join.kind == ScheduleJoinKind::Change && !analysis.affected_dependents.is_empty() {
            explanations.push(explanation(
                ScheduleExplanationKind::ChangeImpact,
                join,
                task.name.clone(),
                need_on,
                task.start,
                task.finish,
                critical_task,
                analysis.affected_dependents.clone(),
                analysis.transitive_blockers.clone(),
                "change control affects downstream schedule controls",
            ));
        }
        if join.kind == ScheduleJoinKind::Decision && as_of_date > need_on {
            explanations.push(explanation(
                ScheduleExplanationKind::LateDecision,
                join,
                task.name.clone(),
                need_on,
                task.start,
                task.finish,
                critical_task,
                analysis.affected_dependents.clone(),
                analysis.transitive_blockers.clone(),
                "accountable decision is late for the schedule need date",
            ));
        }
    }

    explanations.sort_by(|left, right| {
        (
            left.need_on,
            left.control.as_str(),
            left.task_id.as_str(),
            explanation_rank(left.kind),
        )
            .cmp(&(
                right.need_on,
                right.control.as_str(),
                right.task_id.as_str(),
                explanation_rank(right.kind),
            ))
    });

    Ok(ScheduleStatusReport {
        baseline: joins.baseline.baseline.clone(),
        accepted_revision: joins.baseline.accepted_revision.clone(),
        as_of_seq: joins.revision.as_of_seq,
        critical_tasks: critical,
        explanations,
    })
}

fn state_map(
    states: &[ScheduleControlState],
) -> Result<BTreeMap<ControlId, &ScheduleControlState>> {
    let mut by_control = BTreeMap::new();
    for state in states {
        if by_control.insert(state.control.clone(), state).is_some() {
            return Err(ConstructionProjectError::DuplicateId {
                kind: "schedule_control_state",
                id: state.control.as_str().to_owned(),
            });
        }
    }
    Ok(by_control)
}

fn sorted_joins(joins: &[ScheduleTaskJoin]) -> Vec<&ScheduleTaskJoin> {
    let mut sorted = joins.iter().collect::<Vec<_>>();
    sorted.sort_by(|left, right| {
        (left.control.as_str(), left.task_id.as_str())
            .cmp(&(right.control.as_str(), right.task_id.as_str()))
    });
    sorted
}

fn need_date(join: &ScheduleTaskJoin, task_start: Date) -> Date {
    if let Some(need_on) = join.need_on {
        need_on
    } else if let Some(days) = join.lead_time_days {
        task_start - Duration::days(i64::from(days))
    } else {
        task_start
    }
}

#[allow(clippy::too_many_arguments)]
fn explanation(
    kind: ScheduleExplanationKind,
    join: &ScheduleTaskJoin,
    task_name: String,
    need_on: Date,
    task_start: Date,
    task_finish: Date,
    critical: bool,
    downstream_controls: Vec<ControlId>,
    blockers: Vec<ControlId>,
    reason: &'static str,
) -> ScheduleImpactExplanation {
    ScheduleImpactExplanation {
        kind,
        control: join.control.clone(),
        task_id: join.task_id.clone(),
        task_name,
        need_on,
        task_start,
        task_finish,
        critical,
        downstream_controls,
        blockers,
        explanation: reason.to_owned(),
    }
}

fn explanation_rank(kind: ScheduleExplanationKind) -> u8 {
    match kind {
        ScheduleExplanationKind::CriticalBlocker => 0,
        ScheduleExplanationKind::LateDecision => 1,
        ScheduleExplanationKind::ProcurementLeadTime => 2,
        ScheduleExplanationKind::ChangeImpact => 3,
        ScheduleExplanationKind::NonCriticalDelay => 4,
    }
}

fn reports_generic_delay(kind: ScheduleJoinKind) -> bool {
    matches!(
        kind,
        ScheduleJoinKind::Control
            | ScheduleJoinKind::Package
            | ScheduleJoinKind::DesignDeliverable
            | ScheduleJoinKind::HandoverItem
    )
}
