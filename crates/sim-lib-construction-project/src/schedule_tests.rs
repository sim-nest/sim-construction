// conformance: construction control graph joins canonical sim-lib-gantt facts

use crate::{
    BaselineId, ConstructionProjectError, ControlEdgeKind, ControlGraph, ControlId,
    ControlNodeKind, EvidenceState, ScheduleBaseline, ScheduleControlState,
    ScheduleExplanationKind, ScheduleJoinKind, SchedulePlanRevision, ScheduleTaskJoin,
    ScheduleTaskJoinSet, explain_schedule_impact,
};
use sim_kernel::Cx;
use sim_lib_gantt::{GanttPlan, LinkKind, Task, TaskLink};
use std::sync::Arc;
use time::{Date, Month};

#[test]
fn dangling_join_fails_closed() {
    let result = ScheduleTaskJoinSet::new(
        baseline(),
        revision("rev-a", 7),
        vec![join("package.frame", "missing", ScheduleJoinKind::Package)],
    )
    .unwrap()
    .validate_against_plan(&plan());

    assert!(matches!(
        result,
        Err(ConstructionProjectError::MissingScheduleTask { .. })
    ));
}

#[test]
fn duplicate_joined_task_ids_fail_closed() {
    let result = ScheduleTaskJoinSet::new(
        baseline(),
        revision("rev-a", 7),
        vec![
            join(
                "package.frame",
                "frame-fabrication",
                ScheduleJoinKind::Package,
            ),
            join(
                "decision.frame-release",
                "frame-fabrication",
                ScheduleJoinKind::Decision,
            ),
        ],
    )
    .unwrap()
    .validate_against_plan(&plan());

    assert!(matches!(
        result,
        Err(ConstructionProjectError::DuplicateScheduleTaskJoin { .. })
    ));
}

#[test]
fn plan_revision_mismatch_does_not_silently_become_baseline() {
    let result = ScheduleTaskJoinSet::new(
        baseline(),
        revision("rev-b", 8),
        vec![join(
            "package.frame",
            "frame-fabrication",
            ScheduleJoinKind::Package,
        )],
    );

    assert!(matches!(
        result,
        Err(ConstructionProjectError::ScheduleRevisionMismatch { .. })
    ));
}

#[test]
fn critical_blocker_and_downstream_controls_are_explained() {
    let report = report(
        date(2026, Month::July, 9),
        vec![missing("decision.frame-release")],
    );

    assert_eq!(
        report.critical_tasks,
        vec![
            "design-release".to_owned(),
            "frame-fabrication".to_owned(),
            "handover".to_owned(),
        ]
    );
    let critical = report
        .explanations
        .iter()
        .find(|item| item.kind == ScheduleExplanationKind::CriticalBlocker)
        .unwrap();
    assert_eq!(critical.control.as_str(), "package.frame");
    assert_eq!(ids(&critical.blockers), vec!["decision.frame-release"]);
    assert_eq!(
        ids(&critical.downstream_controls),
        vec!["change.frame", "handover.frame"]
    );
}

#[test]
fn non_critical_delayed_task_is_reported_without_critical_blocker() {
    let report = report(
        date(2026, Month::July, 25),
        vec![accepted("decision.frame-release")],
    );

    let delayed = report
        .explanations
        .iter()
        .find(|item| item.control.as_str() == "decision.lighting")
        .unwrap();
    assert_eq!(delayed.kind, ScheduleExplanationKind::NonCriticalDelay);
    assert!(!delayed.critical);
}

#[test]
fn procurement_lead_time_consequence_moves_need_date_before_task_start() {
    let report = report(
        date(2026, Month::July, 9),
        vec![accepted("decision.frame-release")],
    );

    let procurement = report
        .explanations
        .iter()
        .find(|item| item.kind == ScheduleExplanationKind::ProcurementLeadTime)
        .unwrap();
    assert_eq!(procurement.control.as_str(), "procurement.frame-award");
    assert_eq!(procurement.need_on, date(2026, Month::June, 28));
}

#[test]
fn change_impact_names_downstream_controls() {
    let report = report(
        date(2026, Month::July, 9),
        vec![accepted("decision.frame-release")],
    );

    let change = report
        .explanations
        .iter()
        .find(|item| item.kind == ScheduleExplanationKind::ChangeImpact)
        .unwrap();
    assert_eq!(change.control.as_str(), "change.frame");
    assert_eq!(ids(&change.downstream_controls), vec!["handover.frame"]);
}

#[test]
fn stable_order_is_by_need_date_control_task_and_explanation_kind() {
    let report = report(
        date(2026, Month::July, 25),
        vec![missing("decision.frame-release")],
    );

    assert_eq!(
        report
            .explanations
            .iter()
            .map(|item| (item.need_on, item.control.as_str(), item.kind))
            .collect::<Vec<_>>(),
        vec![
            (
                date(2026, Month::June, 28),
                "procurement.frame-award",
                ScheduleExplanationKind::ProcurementLeadTime,
            ),
            (
                date(2026, Month::July, 5),
                "decision.frame-release",
                ScheduleExplanationKind::LateDecision,
            ),
            (
                date(2026, Month::July, 10),
                "package.frame",
                ScheduleExplanationKind::CriticalBlocker,
            ),
            (
                date(2026, Month::July, 12),
                "change.frame",
                ScheduleExplanationKind::ChangeImpact,
            ),
            (
                date(2026, Month::July, 15),
                "decision.lighting",
                ScheduleExplanationKind::NonCriticalDelay,
            ),
            (
                date(2026, Month::July, 20),
                "handover.frame",
                ScheduleExplanationKind::CriticalBlocker,
            ),
        ]
    );
}

fn report(as_of_date: Date, states: Vec<ScheduleControlState>) -> crate::ScheduleStatusReport {
    let mut cx = Cx::new(
        Arc::new(sim_kernel::NoopEvalPolicy),
        Arc::new(sim_kernel::DefaultFactory),
    );
    explain_schedule_impact(&mut cx, &plan(), &joins(), &graph(), &states, as_of_date).unwrap()
}

fn graph() -> ControlGraph {
    let mut graph = ControlGraph::new();
    for (id, kind) in [
        ("decision.frame-release", ControlNodeKind::Decision),
        ("package.frame", ControlNodeKind::Package),
        ("procurement.frame-award", ControlNodeKind::Package),
        ("change.frame", ControlNodeKind::Change),
        ("handover.frame", ControlNodeKind::HandoverItem),
        ("decision.lighting", ControlNodeKind::Decision),
    ] {
        graph.add_node(control_id(id), kind).unwrap();
    }
    graph
        .add_edge(
            control_id("decision.frame-release"),
            control_id("package.frame"),
            ControlEdgeKind::Prerequisite,
        )
        .unwrap();
    graph
        .add_edge(
            control_id("package.frame"),
            control_id("change.frame"),
            ControlEdgeKind::Changes,
        )
        .unwrap();
    graph
        .add_edge(
            control_id("change.frame"),
            control_id("handover.frame"),
            ControlEdgeKind::HandsOver,
        )
        .unwrap();
    graph
}

fn joins() -> ScheduleTaskJoinSet {
    ScheduleTaskJoinSet::new(
        baseline(),
        revision("rev-a", 7),
        vec![
            join(
                "decision.frame-release",
                "design-release",
                ScheduleJoinKind::Decision,
            )
            .needs_on(date(2026, Month::July, 5)),
            join(
                "package.frame",
                "frame-fabrication",
                ScheduleJoinKind::Package,
            ),
            join(
                "procurement.frame-award",
                "procurement-float",
                ScheduleJoinKind::ProcurementDate,
            )
            .with_lead_time(12),
            join("change.frame", "change-review", ScheduleJoinKind::Change),
            join("handover.frame", "handover", ScheduleJoinKind::HandoverItem),
            join(
                "decision.lighting",
                "lighting-selection",
                ScheduleJoinKind::Control,
            ),
        ],
    )
    .unwrap()
}

fn plan() -> GanttPlan {
    GanttPlan::new(
        "baseline-plan",
        vec![
            task("design-release", "Design release", 1, 5),
            task("frame-fabrication", "Frame fabrication", 10, 20),
            task("procurement-float", "Procurement float", 10, 11),
            task("change-review", "Change review", 12, 12),
            task("handover", "Handover", 20, 22),
            task("lighting-selection", "Lighting selection", 15, 16),
        ],
        vec![
            TaskLink::new(
                "design-release",
                "frame-fabrication",
                LinkKind::FinishStart,
                0,
            ),
            TaskLink::new("frame-fabrication", "handover", LinkKind::FinishStart, 0),
        ],
    )
}

fn baseline() -> ScheduleBaseline {
    ScheduleBaseline::new(
        baseline_id("baseline.schedule"),
        "baseline-plan",
        "rev-a",
        6,
    )
    .unwrap()
}

fn revision(revision: &str, as_of_seq: u64) -> SchedulePlanRevision {
    SchedulePlanRevision::new("baseline-plan", revision, as_of_seq).unwrap()
}

fn join(control: &str, task: &str, kind: ScheduleJoinKind) -> ScheduleTaskJoin {
    ScheduleTaskJoin::new(control_id(control), task, kind)
}

fn accepted(control: &str) -> ScheduleControlState {
    ScheduleControlState::new(control_id(control), EvidenceState::Accepted).at_sequence(8)
}

fn missing(control: &str) -> ScheduleControlState {
    ScheduleControlState::new(control_id(control), EvidenceState::Missing)
}

fn task(id: &str, name: &str, start_day: u8, finish_day: u8) -> Task {
    Task::new(
        id,
        name,
        date(2026, Month::July, start_day),
        date(2026, Month::July, finish_day),
        0,
    )
}

fn date(year: i32, month: Month, day: u8) -> Date {
    Date::from_calendar_date(year, month, day).unwrap()
}

fn control_id(id: &str) -> ControlId {
    ControlId::new(id).unwrap()
}

fn baseline_id(id: &str) -> BaselineId {
    BaselineId::new(id).unwrap()
}

fn ids(values: &[ControlId]) -> Vec<&str> {
    values.iter().map(ControlId::as_str).collect()
}
