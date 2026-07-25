//! Production lookahead readiness derived from shared project facts.

use std::collections::{BTreeMap, BTreeSet};

use sim_lib_gantt::GanttPlan;
use time::{Date, Duration};

use crate::{
    AcceptedTaskWindow, ConstructionProjectError, ControlId, EvidenceState, ExceptionDecision,
    GatePolicy, LookaheadWindow, ObligationPolicy, ProductionActivity, ProductionActivityReadiness,
    ProductionCommitment, ProductionConstraint, ProductionReadinessSnapshot,
    ProductionReadinessState, ProductionTaskMovement, ProjectBook, ProjectObligation,
    RequirementExplanation, Result, RoleId, ScheduleTaskJoinSet,
};

/// Production lookahead model composed from shared facts, requirements, and Gantt joins.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProductionPlan {
    /// Activities under production control.
    pub activities: Vec<ProductionActivity>,
    /// Shared obligations available to readiness.
    pub obligations: Vec<ProjectObligation>,
    /// Bounded exceptions available to readiness.
    pub exceptions: Vec<ExceptionDecision>,
    /// Human commitments, distinct from computed readiness.
    pub commitments: Vec<ProductionCommitment>,
    /// Retained accepted baseline task windows.
    pub accepted_task_windows: Vec<AcceptedTaskWindow>,
    /// Granted policy capability names.
    pub granted_capabilities: Vec<String>,
}

impl ProductionPlan {
    /// Builds an empty production plan.
    #[must_use]
    pub fn new() -> Self {
        Self {
            activities: Vec::new(),
            obligations: Vec::new(),
            exceptions: Vec::new(),
            commitments: Vec::new(),
            accepted_task_windows: Vec::new(),
            granted_capabilities: Vec::new(),
        }
    }

    /// Adds one production activity.
    #[must_use]
    pub fn with_activity(mut self, activity: ProductionActivity) -> Self {
        self.activities.push(activity);
        self
    }

    /// Adds one shared obligation.
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

    /// Adds one human commitment.
    #[must_use]
    pub fn with_commitment(mut self, commitment: ProductionCommitment) -> Self {
        self.commitments.push(commitment);
        self
    }

    /// Adds a retained accepted task window.
    #[must_use]
    pub fn with_accepted_task_window(mut self, window: AcceptedTaskWindow) -> Self {
        self.accepted_task_windows.push(window);
        self
    }

    /// Adds one granted capability name.
    #[must_use]
    pub fn with_capability(mut self, capability: impl Into<String>) -> Self {
        self.granted_capabilities.push(capability.into());
        self
    }

    /// Derives production readiness for the six-week demand and three-week commitment windows.
    pub fn derive_readiness(
        &self,
        book: &ProjectBook,
        plan: &GanttPlan,
        joins: &ScheduleTaskJoinSet,
        as_of_date: Date,
    ) -> Result<ProductionReadinessSnapshot> {
        joins.validate_against_plan(plan)?;
        let policy = self.policy();
        let policy_report = policy.evaluate(book, joins.revision.as_of_seq, as_of_date)?;
        let explanations = policy_report
            .explanations
            .iter()
            .map(|explanation| (explanation.requirement.clone(), explanation))
            .collect::<BTreeMap<_, _>>();
        let obligations = self
            .obligations
            .iter()
            .map(|obligation| (obligation.requirement.id.clone(), obligation))
            .collect::<BTreeMap<_, _>>();
        let commitments = self.commitment_map()?;
        let accepted_windows = self.accepted_window_map()?;
        let joined_tasks = joins
            .joins
            .iter()
            .map(|join| (join.control.clone(), join.task_id.as_str()))
            .collect::<BTreeMap<_, _>>();

        let mut six_week_demand = Vec::new();
        let mut three_week_commitment = Vec::new();
        for activity in sorted_activities(&self.activities) {
            activity.validate()?;
            if activity.accepted_baseline != joins.baseline.baseline {
                return Err(ConstructionProjectError::StaleBaselineComparison {
                    baseline: ControlId::new(activity.accepted_baseline.as_str())?,
                    accepted_seq: joins.baseline.accepted_seq,
                    as_of_seq: joins.revision.as_of_seq,
                });
            }
            let joined_task = joined_tasks.get(&activity.control).ok_or_else(|| {
                ConstructionProjectError::MissingScheduleTask {
                    control: activity.control.clone(),
                    task_id: activity.task_id.clone(),
                }
            })?;
            if *joined_task != activity.task_id {
                return Err(ConstructionProjectError::MissingScheduleTask {
                    control: activity.control.clone(),
                    task_id: activity.task_id.clone(),
                });
            }
            let task = plan.task(&activity.task_id).ok_or_else(|| {
                ConstructionProjectError::MissingScheduleTask {
                    control: activity.control.clone(),
                    task_id: activity.task_id.clone(),
                }
            })?;
            let Some(window) = window_for(as_of_date, task.start) else {
                continue;
            };
            let readiness = self.activity_readiness(
                activity,
                task.start,
                task.finish,
                window,
                &obligations,
                &explanations,
                commitments.get(&activity.control).cloned(),
                accepted_windows.get(&activity.task_id),
                joins.revision.as_of_seq,
            )?;
            six_week_demand.push(readiness.clone());
            if window == LookaheadWindow::ThreeWeekCommitment {
                three_week_commitment.push(readiness);
            }
        }

        Ok(ProductionReadinessSnapshot {
            baseline: joins.baseline.baseline.clone(),
            accepted_revision: joins.baseline.accepted_revision.clone(),
            imported_revision: joins.revision.revision.clone(),
            as_of_seq: joins.revision.as_of_seq,
            as_of_date,
            six_week_demand,
            three_week_commitment,
        })
    }

    fn policy(&self) -> GatePolicy {
        let mut policy = GatePolicy::new();
        for obligation in &self.obligations {
            policy = policy.with_obligation(obligation.clone());
        }
        for exception in &self.exceptions {
            policy = policy.with_exception(exception.clone());
        }
        for capability in &self.granted_capabilities {
            policy = policy.with_capability(capability.clone());
        }
        policy
    }

    #[allow(clippy::too_many_arguments)]
    fn activity_readiness(
        &self,
        activity: &ProductionActivity,
        task_start: Date,
        task_finish: Date,
        window: LookaheadWindow,
        obligations: &BTreeMap<ControlId, &ProjectObligation>,
        explanations: &BTreeMap<ControlId, &RequirementExplanation>,
        commitment: Option<&ProductionCommitment>,
        accepted_window: Option<&&AcceptedTaskWindow>,
        as_of_seq: u64,
    ) -> Result<ProductionActivityReadiness> {
        let mut constraints = Vec::new();
        let mut unknown = false;
        let mut blocked = false;
        let mut seen = BTreeSet::new();
        for requirement in &activity.requirements {
            if !seen.insert(requirement.clone()) {
                return Err(ConstructionProjectError::DuplicateId {
                    kind: "production_activity.requirement",
                    id: requirement.as_str().to_owned(),
                });
            }
            let Some(obligation) = obligations.get(requirement) else {
                unknown = true;
                constraints.push(unknown_constraint(
                    requirement,
                    &activity.responsible_role,
                    task_start,
                ));
                continue;
            };
            let Some(explanation) = explanations.get(requirement) else {
                unknown = true;
                constraints.push(unknown_constraint(
                    requirement,
                    &obligation.requirement.owner,
                    task_start,
                ));
                continue;
            };
            if obligation.requirement.non_waivable && explanation.exception.is_some() {
                return Err(ConstructionProjectError::NonWaivableProductionBlocker {
                    target: activity.control.clone(),
                    blocker: requirement.clone(),
                });
            }
            let blocks = obligation.policy == ObligationPolicy::Mandatory
                && obligation.requirement.evidence_required
                && explanation.exception.is_none()
                && !explanation.evidence_state.satisfies_required_evidence();
            if blocks {
                blocked = true;
            }
            if blocks
                || explanation.exception.is_some()
                || explanation.evidence_state != EvidenceState::Accepted
            {
                constraints.push(ProductionConstraint {
                    requirement: requirement.clone(),
                    owner: obligation.requirement.owner.clone(),
                    need_on: task_start,
                    evidence_state: explanation.evidence_state,
                    current_seq: explanation.current_seq,
                    exception: explanation.exception.clone(),
                    non_waivable: obligation.requirement.non_waivable,
                    escalation: obligation.requirement.acceptance_authority.clone(),
                    consequence: format!(
                        "production activity {} cannot start on {} until requirement {} is resolved",
                        activity.control, task_start, requirement
                    ),
                    explanation: explanation.reason.clone(),
                });
            }
        }

        let movement = accepted_window.and_then(|accepted| {
            (accepted.start != task_start || accepted.finish != task_finish).then_some(
                ProductionTaskMovement {
                    accepted_start: accepted.start,
                    accepted_finish: accepted.finish,
                    current_start: task_start,
                    current_finish: task_finish,
                },
            )
        });
        let state = if blocked {
            ProductionReadinessState::NotReady
        } else if unknown {
            ProductionReadinessState::Unknown
        } else {
            ProductionReadinessState::Ready
        };
        let explanation = explanation_text(state, commitment.is_some(), movement.is_some());
        Ok(ProductionActivityReadiness {
            activity: activity.control.clone(),
            task_id: activity.task_id.clone(),
            work_package: activity.work_package.clone(),
            window,
            state,
            commitment: commitment.cloned(),
            constraints,
            movement,
            as_of_seq,
            explanation,
        })
    }

    fn commitment_map(&self) -> Result<BTreeMap<ControlId, &ProductionCommitment>> {
        let mut by_activity = BTreeMap::new();
        for commitment in &self.commitments {
            commitment.validate()?;
            if by_activity
                .insert(commitment.activity.clone(), commitment)
                .is_some()
            {
                return Err(ConstructionProjectError::DuplicateId {
                    kind: "production_commitment.activity",
                    id: commitment.activity.as_str().to_owned(),
                });
            }
        }
        Ok(by_activity)
    }

    fn accepted_window_map(&self) -> Result<BTreeMap<String, &AcceptedTaskWindow>> {
        let mut by_task = BTreeMap::new();
        for window in &self.accepted_task_windows {
            if window.task_id.trim().is_empty() {
                return Err(ConstructionProjectError::EmptyField(
                    "accepted_task_window.task_id",
                ));
            }
            if window.finish < window.start {
                return Err(ConstructionProjectError::InvalidSnapshotRange {
                    from_seq: window.finish.to_julian_day() as u64,
                    through_seq: window.start.to_julian_day() as u64,
                });
            }
            if by_task.insert(window.task_id.clone(), window).is_some() {
                return Err(ConstructionProjectError::DuplicateId {
                    kind: "accepted_task_window.task_id",
                    id: window.task_id.clone(),
                });
            }
        }
        Ok(by_task)
    }
}

impl Default for ProductionPlan {
    fn default() -> Self {
        Self::new()
    }
}

fn window_for(as_of_date: Date, start: Date) -> Option<LookaheadWindow> {
    if start < as_of_date || start > as_of_date + Duration::days(42) {
        None
    } else if start <= as_of_date + Duration::days(21) {
        Some(LookaheadWindow::ThreeWeekCommitment)
    } else {
        Some(LookaheadWindow::SixWeekDemand)
    }
}

fn sorted_activities(activities: &[ProductionActivity]) -> Vec<&ProductionActivity> {
    let mut sorted = activities.iter().collect::<Vec<_>>();
    sorted.sort_by(|left, right| {
        (
            left.planned_start,
            left.control.as_str(),
            left.task_id.as_str(),
        )
            .cmp(&(
                right.planned_start,
                right.control.as_str(),
                right.task_id.as_str(),
            ))
    });
    sorted
}

fn unknown_constraint(
    requirement: &ControlId,
    owner: &RoleId,
    need_on: Date,
) -> ProductionConstraint {
    ProductionConstraint {
        requirement: requirement.clone(),
        owner: owner.clone(),
        need_on,
        evidence_state: EvidenceState::Missing,
        current_seq: None,
        exception: None,
        non_waivable: false,
        escalation: owner.clone(),
        consequence: format!("production readiness requirement {requirement} is not modeled"),
        explanation: "shared requirement is unknown to the production readiness policy".to_owned(),
    }
}

fn explanation_text(state: ProductionReadinessState, committed: bool, moved: bool) -> String {
    let decision = match state {
        ProductionReadinessState::Ready => "ready",
        ProductionReadinessState::NotReady => "not ready",
        ProductionReadinessState::Unknown => "unknown",
    };
    let commitment = if committed {
        "with human commitment"
    } else {
        "without human commitment"
    };
    let movement = if moved {
        "after schedule movement"
    } else {
        "on accepted schedule position"
    };
    format!("{decision} {commitment} {movement}")
}
