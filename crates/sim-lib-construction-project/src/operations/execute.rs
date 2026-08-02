//! Construction constructor and operation implementations.

use std::sync::{Arc, Mutex};

use sim_kernel::{Cx, Error, PreparedArgs, Result, Value};
use sim_lib_gantt::{GanttPlan, LinkKind, Task, TaskLink};
use sim_shape::Bindings;
use time::Date;

use super::{
    ConstructionExplanationReport, ConstructionStatusReport, ProjectBookHandle, arity, book_handle,
    boxed, lock_book, plain_value, string,
};
use crate::{
    AcceptedBaseline, AccountableCloseout, ChangeControlSet, ChangeRecord, CommissioningAssessment,
    CommissioningControlSet, CommissioningRequirement, ControlGraph, ControlId, CurrencyCode,
    ExceptionDecision, OutcomeControlReport, OutcomeRecord, PhaseGate, ProductionPlan, ProjectBook,
    ProjectFact, ProjectId, ProjectSnapshot, ReferencePackAdmission, Requirement,
    ScheduleControlState, ScheduleTaskJoinSet, WorkPackage,
    citizen::{ConstructionCitizenSpec, decode_fact_value, decode_value},
    construction_project_read_capability, construction_project_write_capability,
    construction_reference_publish_capability, explain_schedule_impact,
};

pub(super) fn project_id_impl(cx: &mut Cx, args: &PreparedArgs, _: Bindings) -> Result<Value> {
    let [id] = args.values() else {
        return arity("project-id", 1, args.len());
    };
    let id = string(cx, id, "project-id")?;
    boxed(cx, ProjectId::new(id)?)
}

pub(super) fn book_impl(cx: &mut Cx, args: &PreparedArgs, _: Bindings) -> Result<Value> {
    let [project, writer] = args.values() else {
        return arity("book", 2, args.len());
    };
    cx.require(&construction_project_write_capability())?;
    let project = decode_value::<ProjectId>(cx, project, "book project")?;
    let writer = crate::RoleId::new(string(cx, writer, "book writer")?)?;
    cx.factory().opaque(Arc::new(ProjectBookHandle {
        book: Arc::new(Mutex::new(ProjectBook::new(project, writer))),
    }))
}

pub(super) fn fact_impl(cx: &mut Cx, args: &PreparedArgs, _: Bindings) -> Result<Value> {
    let [value] = args.values() else {
        return arity("fact", 1, args.len());
    };
    let fact = decode_fact_value(cx, value)?;
    boxed(cx, fact)
}

pub(super) fn baseline_impl(cx: &mut Cx, args: &PreparedArgs, _: Bindings) -> Result<Value> {
    construct::<AcceptedBaseline>(cx, args, "baseline")
}

pub(super) fn requirement_impl(cx: &mut Cx, args: &PreparedArgs, _: Bindings) -> Result<Value> {
    construct::<Requirement>(cx, args, "requirement")
}

pub(super) fn package_impl(cx: &mut Cx, args: &PreparedArgs, _: Bindings) -> Result<Value> {
    construct::<WorkPackage>(cx, args, "package")
}

pub(super) fn join_impl(cx: &mut Cx, args: &PreparedArgs, _: Bindings) -> Result<Value> {
    construct::<ScheduleTaskJoinSet>(cx, args, "join")
}

pub(super) fn change_impl(cx: &mut Cx, args: &PreparedArgs, _: Bindings) -> Result<Value> {
    construct::<ChangeRecord>(cx, args, "change")
}

pub(super) fn handover_item_impl(cx: &mut Cx, args: &PreparedArgs, _: Bindings) -> Result<Value> {
    construct::<CommissioningRequirement>(cx, args, "handover-item")
}

pub(super) fn outcome_impl(cx: &mut Cx, args: &PreparedArgs, _: Bindings) -> Result<Value> {
    construct::<OutcomeRecord>(cx, args, "outcome")
}

fn construct<T>(cx: &mut Cx, args: &PreparedArgs, name: &'static str) -> Result<Value>
where
    T: ConstructionCitizenSpec + sim_kernel::Object + sim_kernel::ObjectCompat,
{
    let [value] = args.values() else {
        return arity(name, 1, args.len());
    };
    let value = decode_value::<T>(cx, value, name)?;
    boxed(cx, value)
}

pub(super) fn append_impl(cx: &mut Cx, args: &PreparedArgs, _: Bindings) -> Result<Value> {
    let [book, fact] = args.values() else {
        return arity("append", 2, args.len());
    };
    cx.require(&construction_project_write_capability())?;
    let handle = book_handle(book)?;
    let fact = decode_value::<ProjectFact>(cx, fact, "append fact")?;
    lock_book(handle)?.append(fact.clone())?;
    boxed(cx, fact)
}

pub(super) fn snapshot_impl(cx: &mut Cx, args: &PreparedArgs, _: Bindings) -> Result<Value> {
    let [book, sequence] = args.values() else {
        return arity("snapshot-as-of", 2, args.len());
    };
    cx.require(&construction_project_read_capability())?;
    let sequence = plain_value::<u64>(cx, sequence, "snapshot-as-of sequence")?;
    let snapshot = lock_book(book_handle(book)?)?.snapshot_at(sequence)?;
    boxed(cx, snapshot)
}

pub(super) fn validate_impl(cx: &mut Cx, args: &PreparedArgs, _: Bindings) -> Result<Value> {
    let [value] = args.values() else {
        return arity("validate", 1, args.len());
    };
    validate_domain_value(value)?;
    cx.factory().bool(true)
}

pub(super) fn status_impl(cx: &mut Cx, args: &PreparedArgs, _: Bindings) -> Result<Value> {
    let [snapshot] = args.values() else {
        return arity("status", 1, args.len());
    };
    cx.require(&construction_project_read_capability())?;
    let snapshot = decode_value::<ProjectSnapshot>(cx, snapshot, "status snapshot")?;
    let accepted = snapshot
        .current
        .values()
        .filter(|fact| fact.evidence_state == crate::EvidenceState::Accepted)
        .count();
    let unresolved_current = snapshot.current.len().saturating_sub(accepted);
    boxed(
        cx,
        ConstructionStatusReport {
            project: snapshot.project,
            as_of_seq: snapshot.through_seq,
            current: snapshot.current.len(),
            superseded: snapshot.superseded.values().map(Vec::len).sum(),
            conflicted: snapshot.conflicted.values().map(Vec::len).sum(),
            rejected: snapshot.rejected.values().map(Vec::len).sum(),
            accepted,
            blockers: unresolved_current + snapshot.conflicted.len(),
        },
    )
}

pub(super) fn explain_impl(cx: &mut Cx, args: &PreparedArgs, _: Bindings) -> Result<Value> {
    let [snapshot, subject] = args.values() else {
        return arity("explain", 2, args.len());
    };
    cx.require(&construction_project_read_capability())?;
    let snapshot = decode_value::<ProjectSnapshot>(cx, snapshot, "explain snapshot")?;
    let subject = ControlId::new(string(cx, subject, "explain subject")?)?;
    let current_sequence = snapshot.current_fact(&subject).map(|fact| fact.seq);
    let evidence_state = snapshot
        .current_fact(&subject)
        .map(|fact| fact.evidence_state);
    let rows = snapshot
        .explanations
        .iter()
        .filter(|row| row.subject == subject)
        .cloned()
        .collect::<Vec<_>>();
    let actionable = if snapshot.is_conflicted(&subject) {
        "resolve competing current facts with an accountable superseding fact"
    } else {
        match evidence_state {
            None => "append the missing accountable project fact",
            Some(crate::EvidenceState::Accepted) => "current accepted fact; no blocker",
            Some(crate::EvidenceState::Expired) => "replace expired evidence and accept it",
            Some(crate::EvidenceState::Rejected) => "repair the rejected evidence and resubmit",
            Some(_) => "supply current evidence and obtain accountable acceptance",
        }
    };
    boxed(
        cx,
        ConstructionExplanationReport {
            project: snapshot.project,
            as_of_seq: snapshot.through_seq,
            subject,
            current_sequence,
            evidence_state,
            rows,
            actionable: actionable.to_owned(),
        },
    )
}

pub(super) fn diff_since_impl(cx: &mut Cx, args: &PreparedArgs, _: Bindings) -> Result<Value> {
    let [book, from] = args.values() else {
        return arity("diff-since", 2, args.len());
    };
    cx.require(&construction_project_read_capability())?;
    let from = plain_value::<u64>(cx, from, "diff-since sequence")?;
    let book = lock_book(book_handle(book)?)?;
    let through = book.last_sequence().unwrap_or(0);
    boxed(cx, book.delta(from, through)?)
}

#[derive(serde::Deserialize)]
struct GateReportRequest {
    gate: PhaseGate,
    as_of_seq: u64,
    as_of_date: Date,
}

pub(super) fn gate_report_impl(cx: &mut Cx, args: &PreparedArgs, _: Bindings) -> Result<Value> {
    let [book, request] = args.values() else {
        return arity("gate-report", 2, args.len());
    };
    cx.require(&construction_project_read_capability())?;
    let request: GateReportRequest = plain_value(cx, request, "gate-report request")?;
    let book = lock_book(book_handle(book)?)?;
    boxed(
        cx,
        request
            .gate
            .report_at(&book, request.as_of_seq, request.as_of_date)?,
    )
}

#[derive(serde::Deserialize)]
struct ScheduleImpactRequest {
    plan: GanttPlanWire,
    joins: ScheduleTaskJoinSet,
    graph: ControlGraph,
    states: Vec<ScheduleControlState>,
    as_of_date: Date,
}

#[derive(serde::Deserialize)]
struct GanttPlanWire {
    id: String,
    tasks: Vec<GanttTaskWire>,
    links: Vec<GanttLinkWire>,
}

#[derive(serde::Deserialize)]
struct GanttTaskWire {
    id: String,
    name: String,
    start: Date,
    finish: Date,
    percent_complete: u8,
}

#[derive(serde::Deserialize)]
struct GanttLinkWire {
    predecessor: String,
    successor: String,
    kind: String,
    lag_days: i32,
}

impl GanttPlanWire {
    fn into_plan(self) -> Result<GanttPlan> {
        let tasks = self
            .tasks
            .into_iter()
            .map(|task| {
                Task::new(
                    task.id,
                    task.name,
                    task.start,
                    task.finish,
                    task.percent_complete,
                )
            })
            .collect();
        let links = self
            .links
            .into_iter()
            .map(|link| {
                let kind = LinkKind::from_token(&link.kind).ok_or_else(|| {
                    Error::Eval(format!(
                        "construction schedule link kind {:?} is invalid; expected finish-start, start-start, finish-finish, or start-finish",
                        link.kind
                    ))
                })?;
                Ok(TaskLink::new(
                    link.predecessor,
                    link.successor,
                    kind,
                    link.lag_days,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(GanttPlan::new(self.id, tasks, links))
    }
}

pub(super) fn schedule_impact_impl(cx: &mut Cx, args: &PreparedArgs, _: Bindings) -> Result<Value> {
    let [request] = args.values() else {
        return arity("schedule-impact", 1, args.len());
    };
    cx.require(&construction_project_read_capability())?;
    let request: ScheduleImpactRequest = plain_value(cx, request, "schedule-impact request")?;
    let report = explain_schedule_impact(
        cx,
        &request.plan.into_plan()?,
        &request.joins,
        &request.graph,
        &request.states,
        request.as_of_date,
    )?;
    boxed(cx, report)
}

#[derive(serde::Deserialize)]
struct ReadinessRequest {
    plan: ProductionPlan,
    schedule: GanttPlanWire,
    joins: ScheduleTaskJoinSet,
    as_of_date: Date,
}

pub(super) fn readiness_impl(cx: &mut Cx, args: &PreparedArgs, _: Bindings) -> Result<Value> {
    let [book, request] = args.values() else {
        return arity("readiness", 2, args.len());
    };
    cx.require(&construction_project_read_capability())?;
    let request: ReadinessRequest = plain_value(cx, request, "readiness request")?;
    let book = lock_book(book_handle(book)?)?;
    boxed(
        cx,
        request.plan.derive_readiness(
            &book,
            &request.schedule.into_plan()?,
            &request.joins,
            request.as_of_date,
        )?,
    )
}

#[derive(serde::Deserialize)]
struct ExposureRequest {
    changes: ChangeControlSet,
    currency: CurrencyCode,
    as_of_date: Date,
}

pub(super) fn exposure_impl(cx: &mut Cx, args: &PreparedArgs, _: Bindings) -> Result<Value> {
    let [request] = args.values() else {
        return arity("exposure", 1, args.len());
    };
    cx.require(&construction_project_read_capability())?;
    let request: ExposureRequest = plain_value(cx, request, "exposure request")?;
    boxed(
        cx,
        request
            .changes
            .derive(&request.currency, request.as_of_date)?,
    )
}

#[derive(serde::Deserialize)]
struct HandoverBurnDownRequest {
    controls: CommissioningControlSet,
    hierarchy: crate::HandoverHierarchy,
    target: ControlId,
    as_of_seq: u64,
    as_of_date: Date,
    #[serde(default)]
    exceptions: Vec<ExceptionDecision>,
    #[serde(default)]
    capabilities: Vec<String>,
}

pub(super) fn handover_burn_down_impl(
    cx: &mut Cx,
    args: &PreparedArgs,
    _: Bindings,
) -> Result<Value> {
    let [book, request] = args.values() else {
        return arity("handover-burn-down", 2, args.len());
    };
    cx.require(&construction_project_read_capability())?;
    let request: HandoverBurnDownRequest = plain_value(cx, request, "handover-burn-down request")?;
    let book = lock_book(book_handle(book)?)?;
    let mut assessment = CommissioningAssessment::new(&book, request.as_of_seq, request.as_of_date);
    for exception in request.exceptions {
        assessment = assessment.with_exception(exception);
    }
    for capability in request.capabilities {
        assessment = assessment.with_capability(capability);
    }
    boxed(
        cx,
        request
            .controls
            .readiness_for(&request.hierarchy, &request.target, &assessment)?,
    )
}

#[derive(serde::Deserialize)]
struct ReferenceAdmissionRequest {
    admission: ReferencePackAdmission,
    closeout: AccountableCloseout,
    outcomes: Vec<OutcomeControlReport>,
}

pub(super) fn reference_admission_impl(
    cx: &mut Cx,
    args: &PreparedArgs,
    _: Bindings,
) -> Result<Value> {
    let [book, request] = args.values() else {
        return arity("reference-admission", 2, args.len());
    };
    cx.require(&construction_project_read_capability())?;
    cx.require(&construction_reference_publish_capability())?;
    let request: ReferenceAdmissionRequest =
        plain_value(cx, request, "reference-admission request")?;
    let book = lock_book(book_handle(book)?)?;
    boxed(
        cx,
        request
            .admission
            .evaluate(&book, &request.closeout, &request.outcomes)?,
    )
}

fn validate_domain_value(value: &Value) -> Result<()> {
    if let Some(value) = value.object().downcast_ref::<ProjectId>() {
        ProjectId::new(value.as_str())?;
    } else if let Some(value) = value.object().downcast_ref::<ProjectFact>() {
        value.validate_bounds()?;
    } else if let Some(value) = value.object().downcast_ref::<AcceptedBaseline>() {
        value.validate()?;
    } else if let Some(value) = value.object().downcast_ref::<Requirement>() {
        value.validate()?;
    } else if let Some(value) = value.object().downcast_ref::<WorkPackage>() {
        value.validate(&value.target_amount.currency)?;
    } else if let Some(value) = value.object().downcast_ref::<ScheduleTaskJoinSet>() {
        value.baseline.validate()?;
        value.revision.validate()?;
    } else if let Some(value) = value.object().downcast_ref::<ChangeRecord>() {
        value.validate()?;
    } else if let Some(value) = value.object().downcast_ref::<CommissioningRequirement>() {
        value.obligation.requirement.validate()?;
    } else if let Some(value) = value.object().downcast_ref::<OutcomeRecord>() {
        value.validate()?;
    } else {
        return Err(Error::Eval(
            "construction/validate expects a registered durable construction Citizen".to_owned(),
        ));
    }
    Ok(())
}
