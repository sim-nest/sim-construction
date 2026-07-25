// conformance: six-week construction production readiness derives from shared facts

use crate::{
    AcceptedTaskWindow, BaselineId, CONSTRUCTION_EXCEPTION_CAPABILITY, ConstructionProjectError,
    ControlId, EvidenceState, EvidenceValidity, ExceptionDecision, ExceptionScope, LookaheadWindow,
    ObligationPolicy, ProductionActivity, ProductionCommitment, ProductionPlan,
    ProductionReadinessState, ProjectBook, ProjectFact, ProjectId, ProjectObligation, Requirement,
    RequirementLane, RoleId, ScheduleBaseline, ScheduleJoinKind, SchedulePlanRevision,
    ScheduleTaskJoin, ScheduleTaskJoinSet, Visibility,
};
use sim_kernel::{Expr, Symbol};
use sim_lib_doc_core::ExternalRef;
use sim_lib_gantt::{GanttPlan, Task};
use time::{Date, Month};

#[test]
fn moved_task_is_reclassified_into_three_week_commitment_window() {
    let snapshot = ready_plan()
        .with_activity(activity("activity.frame", "task.frame", 8).requires(req("release")))
        .with_obligation(obligation("release"))
        .with_accepted_task_window(AcceptedTaskWindow::new(
            "task.frame",
            offset_day(35),
            offset_day(37),
        ))
        .derive_readiness(
            &book(vec![accepted_fact(1, "release")]),
            &plan(vec![task("task.frame", 8, 10)]),
            &joins(vec![join("activity.frame", "task.frame")]),
            day(1),
        )
        .unwrap();

    assert_eq!(snapshot.six_week_demand.len(), 1);
    assert_eq!(snapshot.three_week_commitment.len(), 1);
    let report = &snapshot.three_week_commitment[0];
    assert_eq!(report.window, LookaheadWindow::ThreeWeekCommitment);
    assert_eq!(report.state, ProductionReadinessState::Ready);
    assert_eq!(
        report.movement.as_ref().unwrap().accepted_start,
        offset_day(35)
    );
    assert!(report.explanation.contains("after schedule movement"));
}

#[test]
fn missing_release_blocks_production_readiness() {
    let report = single_activity_report(vec![], vec!["release"], None);

    assert_eq!(report.state, ProductionReadinessState::NotReady);
    assert_eq!(ids(&report.constraints), vec!["release"]);
    assert_eq!(report.constraints[0].evidence_state, EvidenceState::Missing);
    assert_eq!(report.constraints[0].owner, role("design-lead"));
}

#[test]
fn expired_risk_assessment_is_not_ready_with_sequence_retained() {
    let report = ready_plan()
        .with_activity(activity("activity.frame", "task.frame", 8).requires(req("risk")))
        .with_obligation(
            obligation("risk").with_evidence_validity(EvidenceValidity::new(None, Some(day(5)))),
        )
        .derive_readiness(
            &book(vec![accepted_fact(11, "risk")]),
            &plan(vec![task("task.frame", 8, 10)]),
            &joins(vec![join("activity.frame", "task.frame")]),
            day(8),
        )
        .unwrap()
        .three_week_commitment
        .remove(0);

    assert_eq!(report.state, ProductionReadinessState::NotReady);
    assert_eq!(report.constraints[0].evidence_state, EvidenceState::Expired);
    assert_eq!(report.constraints[0].current_seq, Some(11));
}

#[test]
fn unintroduced_worker_blocks_staffing_readiness() {
    let report = single_activity_report(
        vec![accepted_fact(1, "release")],
        vec!["introduction"],
        None,
    );

    assert_eq!(report.state, ProductionReadinessState::NotReady);
    assert_eq!(ids(&report.constraints), vec!["introduction"]);
    assert!(report.constraints[0].consequence.contains("cannot start"));
}

#[test]
fn material_delay_names_constraint_owner_need_date_and_consequence() {
    let report = single_activity_report(vec![reported_fact(4, "material")], vec!["material"], None);

    assert_eq!(report.state, ProductionReadinessState::NotReady);
    let constraint = &report.constraints[0];
    assert_eq!(constraint.requirement.as_str(), "material");
    assert_eq!(constraint.owner, role("design-lead"));
    assert_eq!(constraint.need_on, day(8));
    assert_eq!(constraint.current_seq, Some(4));
    assert!(constraint.consequence.contains("activity.frame"));
}

#[test]
fn accepted_exception_can_make_waivable_constraint_ready() {
    let exception = ExceptionDecision::new(
        control("exception.material"),
        ExceptionScope::new(project()).covers(req("material")),
        role("project-chief"),
        role("project-chief"),
        "Supplier delay accepted for resequenced work",
        day(6),
        day(12),
    )
    .with_evidence(reference("exception/material"));
    let snapshot = ready_plan()
        .with_activity(activity("activity.frame", "task.frame", 8).requires(req("material")))
        .with_obligation(obligation("material"))
        .with_exception(exception)
        .with_capability(CONSTRUCTION_EXCEPTION_CAPABILITY)
        .derive_readiness(
            &book(vec![]),
            &plan(vec![task("task.frame", 8, 10)]),
            &joins(vec![join("activity.frame", "task.frame")]),
            day(8),
        )
        .unwrap();

    let report = &snapshot.three_week_commitment[0];
    assert_eq!(report.state, ProductionReadinessState::Ready);
    assert_eq!(
        report.constraints[0].exception.as_ref().unwrap().as_str(),
        "exception.material"
    );
}

#[test]
fn non_waivable_safety_exception_fails_closed() {
    let exception = ExceptionDecision::new(
        control("exception.safety"),
        ExceptionScope::new(project()).covers(req("safety")),
        role("project-chief"),
        role("project-chief"),
        "Never acceptable",
        day(6),
        day(12),
    )
    .with_evidence(reference("exception/safety"));
    let result = ready_plan()
        .with_activity(activity("activity.frame", "task.frame", 8).requires(req("safety")))
        .with_obligation(obligation("safety").requirement_non_waivable())
        .with_exception(exception)
        .with_capability(CONSTRUCTION_EXCEPTION_CAPABILITY)
        .derive_readiness(
            &book(vec![]),
            &plan(vec![task("task.frame", 8, 10)]),
            &joins(vec![join("activity.frame", "task.frame")]),
            day(8),
        );

    assert!(matches!(
        result,
        Err(ConstructionProjectError::NonWaivableRequirement { .. })
            | Err(ConstructionProjectError::NonWaivableProductionBlocker { .. })
    ));
}

#[test]
fn committed_but_not_ready_keeps_commitment_separate_from_computation() {
    let report = single_activity_report(
        vec![],
        vec!["release"],
        Some(
            ProductionCommitment::new(
                control("activity.frame"),
                role("site-manager"),
                day(2),
                19,
                "Foreman commits to start frame work",
            )
            .with_evidence(reference("meeting/week-31")),
        ),
    );

    assert_eq!(report.state, ProductionReadinessState::NotReady);
    assert_eq!(report.commitment.as_ref().unwrap().seq, 19);
    assert!(report.explanation.contains("with human commitment"));
}

fn single_activity_report(
    facts: Vec<ProjectFact>,
    requirements: Vec<&str>,
    commitment: Option<ProductionCommitment>,
) -> crate::ProductionActivityReadiness {
    let mut activity = activity("activity.frame", "task.frame", 8);
    let mut readiness = ready_plan();
    for requirement in requirements {
        activity = activity.requires(req(requirement));
        readiness = readiness.with_obligation(obligation(requirement));
    }
    readiness = readiness.with_activity(activity);
    if let Some(commitment) = commitment {
        readiness = readiness.with_commitment(commitment);
    }
    readiness
        .derive_readiness(
            &book(facts),
            &plan(vec![task("task.frame", 8, 10)]),
            &joins(vec![join("activity.frame", "task.frame")]),
            day(1),
        )
        .unwrap()
        .three_week_commitment
        .remove(0)
}

fn ready_plan() -> ProductionPlan {
    ProductionPlan::new()
}

fn obligation(id: &str) -> ProjectObligation {
    ProjectObligation {
        project: project(),
        requirement: requirement(id),
        policy: ObligationPolicy::Mandatory,
        evidence_validity: EvidenceValidity::unbounded(),
    }
}

trait RequirementTestExt {
    fn requirement_non_waivable(self) -> Self;
}

impl RequirementTestExt for ProjectObligation {
    fn requirement_non_waivable(mut self) -> Self {
        self.requirement = self.requirement.non_waivable();
        self
    }
}

fn requirement(id: &str) -> Requirement {
    Requirement::new(
        req(id),
        RequirementLane::new(Symbol::qualified("construction-lookahead", id)),
        format!("{id} requirement"),
        role("design-lead"),
        role("project-chief"),
    )
    .with_source_ref(reference(id))
}

fn activity(id: &str, task_id: &str, start_day: u8) -> ProductionActivity {
    ProductionActivity::new(
        control(id),
        task_id,
        control("package.frame"),
        "structure",
        "area-a",
        "level-02",
        role("site-manager"),
        day(start_day),
        day(start_day + 2),
        baseline_id("baseline.schedule"),
    )
}

fn accepted_fact(seq: u64, subject: &str) -> ProjectFact {
    fact(seq, subject, EvidenceState::Accepted).with_evidence(reference(subject))
}

fn reported_fact(seq: u64, subject: &str) -> ProjectFact {
    fact(seq, subject, EvidenceState::Reported).with_evidence(reference(subject))
}

fn fact(seq: u64, subject: &str, state: EvidenceState) -> ProjectFact {
    ProjectFact::new(
        seq,
        project(),
        req(subject),
        Symbol::qualified("construction-readiness", subject),
        day(1),
        role("writer"),
        Expr::String(subject.to_owned()),
    )
    .with_evidence_state(state)
    .with_visibility(Visibility::Project)
}

fn book(facts: Vec<ProjectFact>) -> ProjectBook {
    ProjectBook::from_facts(project(), role("writer"), facts).unwrap()
}

fn joins(items: Vec<ScheduleTaskJoin>) -> ScheduleTaskJoinSet {
    ScheduleTaskJoinSet::new(
        ScheduleBaseline::new(baseline_id("baseline.schedule"), "plan", "rev-a", 1).unwrap(),
        SchedulePlanRevision::new("plan", "rev-a", 20).unwrap(),
        items,
    )
    .unwrap()
}

fn join(control_id: &str, task_id: &str) -> ScheduleTaskJoin {
    ScheduleTaskJoin::new(control(control_id), task_id, ScheduleJoinKind::Package)
}

fn plan(tasks: Vec<Task>) -> GanttPlan {
    GanttPlan::new("plan", tasks, vec![])
}

fn task(id: &str, start_day: u8, finish_day: u8) -> Task {
    Task::new(id, id, day(start_day), day(finish_day), 0)
}

fn reference(id: &str) -> ExternalRef {
    ExternalRef::new("doc/synthetic", id, Some("rev-a".to_owned()), None)
}

fn project() -> ProjectId {
    ProjectId::new("reference-center").unwrap()
}

fn control(id: &str) -> ControlId {
    ControlId::new(id).unwrap()
}

fn req(id: &str) -> ControlId {
    control(id)
}

fn baseline_id(id: &str) -> BaselineId {
    BaselineId::new(id).unwrap()
}

fn role(id: &str) -> RoleId {
    RoleId::new(id).unwrap()
}

fn day(day: u8) -> Date {
    Date::from_calendar_date(2026, Month::July, day).unwrap()
}

fn offset_day(offset: i64) -> Date {
    day(1) + time::Duration::days(offset - 1)
}

fn ids(values: &[crate::ProductionConstraint]) -> Vec<&str> {
    values
        .iter()
        .map(|constraint| constraint.requirement.as_str())
        .collect()
}
