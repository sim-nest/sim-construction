use sim_kernel::{Cx, Symbol};
use sim_ledger::Amount;
use sim_lib_construction_project::{
    CONSTRUCTION_EXCEPTION_CAPABILITY, ChangeAmountComponent, ChangeControlSet, ChangeDirection,
    ChangeFact, ChangeId, ChangeRecord, ChangeStage, ChangeStatus, CommercialAmount,
    CommercialEvidenceSource, CommercialSide, ConstructionProjectError, ContractualBasis,
    ControlEdgeKind, ControlGraph, ControlNodeKind, EvidenceState, EvidenceValidity,
    ExceptionDecision, ExceptionScope, FieldItem, FieldItemKind, FieldItemState, FieldLane,
    FieldSeverity, OpenRating, ProductionActivity, ProductionPlan, ProjectBook, ProjectObligation,
    ReferencedAmount, ReferencedAmountEvidence, Requirement, RequirementLane, ResponseState,
    ScheduleBaseline, ScheduleControlState, ScheduleExplanationKind, ScheduleJoinKind,
    SchedulePlanRevision, ScheduleTaskJoin, ScheduleTaskJoinSet, UncertaintyKind,
    UncertaintyRecord, UncertaintyResponse, explain_schedule_impact, safety_first_rollup,
};
use sim_lib_gantt_control::{GanttPlan, LinkKind, Task, TaskLink};

use super::support::{baseline_id, currency, day, id, project, reference, role};

pub(super) struct ControlProof {
    pub field_safety: bool,
    pub non_waivable_safety_blocked: bool,
    pub bounded_exception_expired: bool,
    pub risk: bool,
    pub change_economy: bool,
    pub partial_change_approval: bool,
    pub double_count_prevented: bool,
    pub critical_schedule_effect_days: i64,
}

pub(super) fn run(cx: &mut Cx) -> ControlProof {
    let (field_safety, non_waivable_safety_blocked, bounded_exception_expired) =
        prove_field_safety_and_exceptions();
    let (change_economy, partial_change_approval, double_count_prevented) = prove_change_economy();
    ControlProof {
        field_safety,
        non_waivable_safety_blocked,
        bounded_exception_expired,
        risk: prove_risk(),
        change_economy,
        partial_change_approval,
        double_count_prevented,
        critical_schedule_effect_days: prove_schedule_effect(cx),
    }
}

fn prove_field_safety_and_exceptions() -> (bool, bool, bool) {
    let safety = FieldItem::new(
        project(),
        id("safety.energization"),
        FieldItemKind::Incident,
        FieldSeverity::Imminent,
        FieldLane::Safety,
        role("safety-lead"),
    )
    .due_on(day(19))
    .affects(id("task.commissioning"))
    .with_state(FieldItemState::Blocked)
    .non_waivable()
    .with_evidence(reference("safety/energization", "A"));
    safety.validate().unwrap();
    let progress = FieldItem::new(
        project(),
        id("progress.frame"),
        FieldItemKind::Observation,
        FieldSeverity::Major,
        FieldLane::Progress,
        role("site-manager"),
    )
    .affects(id("task.frame-install"))
    .with_state(FieldItemState::Blocked);
    assert_eq!(
        safety_first_rollup(&[progress, safety], day(20))[0].control,
        id("safety.energization")
    );

    let exception = ExceptionDecision::new(
        id("exception.delivery-window"),
        ExceptionScope::new(project()).covers(id("requirement.delivery-window")),
        role("project-chief"),
        role("project-chief"),
        "one bounded delivery shift",
        day(18),
        day(20),
    )
    .with_evidence(reference("exception/delivery-window", "A"));
    exception
        .validate(&[CONSTRUCTION_EXCEPTION_CAPABILITY.to_owned()], day(20))
        .unwrap();
    let expired = matches!(
        exception.validate(&[CONSTRUCTION_EXCEPTION_CAPABILITY.to_owned()], day(21)),
        Err(ConstructionProjectError::ExpiredException { .. })
    );
    assert!(expired);

    let safety_exception = ExceptionDecision::new(
        id("exception.safety"),
        ExceptionScope::new(project()).covers(id("safety.energization")),
        role("project-chief"),
        role("project-chief"),
        "attempted safety waiver",
        day(19),
        day(20),
    )
    .with_evidence(reference("exception/safety", "A"));
    let result = ProductionPlan::new()
        .with_activity(
            ProductionActivity::new(
                id("activity.commissioning"),
                "task.commissioning",
                id("package.electrical"),
                "electrical",
                "market-hall",
                "level-01",
                role("site-manager"),
                day(20),
                day(21),
                baseline_id(),
            )
            .requires(id("safety.energization")),
        )
        .with_obligation(ProjectObligation {
            project: project(),
            requirement: Requirement::new(
                id("safety.energization"),
                RequirementLane::new(Symbol::qualified("construction", "safety")),
                "isolation evidence",
                role("safety-lead"),
                role("project-chief"),
            )
            .with_source_ref(reference("requirement/safety", "A"))
            .non_waivable(),
            policy: sim_lib_construction_project::ObligationPolicy::Mandatory,
            evidence_validity: EvidenceValidity::unbounded(),
        })
        .with_exception(safety_exception)
        .with_capability(CONSTRUCTION_EXCEPTION_CAPABILITY)
        .derive_readiness(
            &ProjectBook::new(project(), role("project-chief")),
            &GanttPlan::new(
                "plan.nordhamn-renovation",
                vec![Task::new(
                    "task.commissioning",
                    "Commissioning",
                    day(20),
                    day(21),
                    0,
                )],
                vec![],
            ),
            &ScheduleTaskJoinSet::new(
                ScheduleBaseline::new(baseline_id(), "plan.nordhamn-renovation", "accepted-C", 1)
                    .unwrap(),
                SchedulePlanRevision::new("plan.nordhamn-renovation", "accepted-C", 20).unwrap(),
                vec![ScheduleTaskJoin::new(
                    id("activity.commissioning"),
                    "task.commissioning",
                    ScheduleJoinKind::Package,
                )],
            )
            .unwrap(),
            day(20),
        );
    let non_waivable_blocked = matches!(
        result,
        Err(ConstructionProjectError::NonWaivableRequirement { .. })
            | Err(ConstructionProjectError::NonWaivableProductionBlocker { .. })
    );
    (
        non_waivable_blocked && expired,
        non_waivable_blocked,
        expired,
    )
}

fn prove_risk() -> bool {
    let likelihood = OpenRating::qualitative(
        "project/risk-matrix",
        "possible",
        23,
        day(23),
        "invented review",
    );
    let impact = OpenRating::quantified(
        "project/impact-score",
        4,
        "five-point-scale",
        23,
        day(23),
        "invented estimate",
    );
    let response = UncertaintyResponse::new(
        "qualify alternate switchgear",
        "primary supplier misses approval",
        day(28),
        day(24),
        23,
    )
    .with_authority(role("project-chief"))
    .trigger_crossed_at(23)
    .with_state(ResponseState::InProgress);
    let risk = UncertaintyRecord::risk(
        project(),
        id("risk.switchgear"),
        23,
        baseline_id(),
        id("scenario.accepted"),
        "single-source switchgear",
        "supplier approval misses need date",
        "commissioning moves",
        role("package-lead"),
        response,
        likelihood,
        impact,
    )
    .affects(id("package.electrical"))
    .with_evidence(reference("risk/switchgear", "A"));
    risk.validate().unwrap();
    risk.kind == UncertaintyKind::Risk
}

fn prove_change_economy() -> (bool, bool, bool) {
    let change = ChangeRecord::new(
        project(),
        ChangeId::new("change.ventilation").unwrap(),
        ChangeDirection::CustomerInstruction,
        ContractualBasis::new(
            "instructed-variation",
            "invented-clause-2",
            reference("contract/main", "signed-A"),
        ),
        role("project-chief"),
        day(20),
        Some(day(22)),
    )
    .affects_control(id("design.fire"))
    .affects_task("task.commissioning")
    .affects_package(id("package.electrical"))
    .with_evidence(reference("instruction/ventilation", "A"));
    let supplier = change_fact(
        "change-fact.supplier",
        1,
        ChangeStage::SupplierExposure,
        ChangeStatus::Assessing,
    )
    .with_amount(change_amount(
        "change-amount.supplier",
        CommercialSide::Supplier,
        "460000.00",
    ))
    .with_reference(change_reference("supplier", Some("460000.00")));
    let quotation = change_fact(
        "change-fact.quotation",
        2,
        ChangeStage::Quotation,
        ChangeStatus::Submitted,
    )
    .with_amount(change_amount(
        "change-amount.quotation",
        CommercialSide::Customer,
        "460000.00",
    ))
    .with_reference(change_reference("quotation", Some("460000.00")));
    let approval = change_fact(
        "change-fact.approval",
        3,
        ChangeStage::AuthorityDecision,
        ChangeStatus::PartiallyApproved,
    )
    .with_amount(change_amount(
        "change-amount.approved",
        CommercialSide::Customer,
        "275000.00",
    ))
    .with_reference(change_reference("approval", Some("275000.00")));
    let report = ChangeControlSet::new()
        .with_change(change.clone())
        .with_fact(supplier)
        .with_fact(quotation)
        .with_fact(approval)
        .derive(&currency(), day(25))
        .unwrap();
    let view = &report.changes[0];
    assert_eq!(view.status, ChangeStatus::PartiallyApproved);
    assert_eq!(view.supplier_exposure, Amount::parse("460000.00").unwrap());
    assert_eq!(view.approved_customer, Amount::parse("275000.00").unwrap());
    assert_eq!(view.net_exposure, Amount(0));

    let parent = change_amount(
        "change-amount.summary",
        CommercialSide::Supplier,
        "460000.00",
    );
    let child = change_amount("change-amount.labor", CommercialSide::Supplier, "185000.00")
        .with_parent(id("change-amount.summary"));
    let doubled = ChangeControlSet::new()
        .with_change(change)
        .with_fact(
            change_fact(
                "change-fact.doubled",
                1,
                ChangeStage::SupplierExposure,
                ChangeStatus::Assessing,
            )
            .with_amount(parent)
            .with_amount(child)
            .with_reference(change_reference("doubled", Some("460000.00"))),
        )
        .derive(&currency(), day(25));
    let double_count_prevented = matches!(
        doubled,
        Err(ConstructionProjectError::ChangeAmountDoubleCount { .. })
    );
    (
        view.status == ChangeStatus::PartiallyApproved && double_count_prevented,
        view.status == ChangeStatus::PartiallyApproved,
        double_count_prevented,
    )
}

fn prove_schedule_effect(cx: &mut Cx) -> i64 {
    let plan = GanttPlan::new(
        "plan.nordhamn-renovation",
        vec![
            Task::new(
                "task.design-release",
                "Design release",
                day(10),
                day(12),
                100,
            ),
            Task::new(
                "task.frame-install",
                "Frame installation",
                day(13),
                day(20),
                50,
            ),
            Task::new("task.commissioning", "Commissioning", day(21), day(24), 0),
            Task::new("task.handover", "Handover", day(25), day(27), 0),
        ],
        vec![
            TaskLink::new(
                "task.design-release",
                "task.frame-install",
                LinkKind::FinishStart,
                0,
            ),
            TaskLink::new(
                "task.frame-install",
                "task.commissioning",
                LinkKind::FinishStart,
                0,
            ),
            TaskLink::new(
                "task.commissioning",
                "task.handover",
                LinkKind::FinishStart,
                0,
            ),
        ],
    );
    let joins = ScheduleTaskJoinSet::new(
        ScheduleBaseline::new(baseline_id(), "plan.nordhamn-renovation", "accepted-C", 18).unwrap(),
        SchedulePlanRevision::new("plan.nordhamn-renovation", "accepted-C", 25).unwrap(),
        vec![
            ScheduleTaskJoin::new(
                id("decision.customer-access"),
                "task.design-release",
                ScheduleJoinKind::Decision,
            )
            .needs_on(day(8)),
            ScheduleTaskJoin::new(
                id("package.electrical"),
                "task.frame-install",
                ScheduleJoinKind::Package,
            ),
            ScheduleTaskJoin::new(
                id("change.ventilation"),
                "task.commissioning",
                ScheduleJoinKind::Change,
            ),
            ScheduleTaskJoin::new(
                id("handover.defect.controls"),
                "task.handover",
                ScheduleJoinKind::HandoverItem,
            ),
        ],
    )
    .unwrap();
    let mut graph = ControlGraph::new();
    for (control, kind) in [
        ("decision.customer-access", ControlNodeKind::Decision),
        ("package.electrical", ControlNodeKind::Package),
        ("change.ventilation", ControlNodeKind::Change),
        ("handover.defect.controls", ControlNodeKind::HandoverItem),
    ] {
        graph.add_node(id(control), kind).unwrap();
    }
    graph
        .add_edge(
            id("decision.customer-access"),
            id("package.electrical"),
            ControlEdgeKind::Prerequisite,
        )
        .unwrap();
    graph
        .add_edge(
            id("package.electrical"),
            id("change.ventilation"),
            ControlEdgeKind::Changes,
        )
        .unwrap();
    graph
        .add_edge(
            id("change.ventilation"),
            id("handover.defect.controls"),
            ControlEdgeKind::HandsOver,
        )
        .unwrap();
    let report = explain_schedule_impact(
        cx,
        &plan,
        &joins,
        &graph,
        &[ScheduleControlState::new(
            id("decision.customer-access"),
            EvidenceState::Missing,
        )],
        day(25),
    )
    .unwrap();
    assert!(report.explanations.iter().any(|row| {
        row.control == id("package.electrical")
            && row.kind == ScheduleExplanationKind::CriticalBlocker
    }));
    assert!(report.explanations.iter().any(|row| {
        row.control == id("change.ventilation") && row.kind == ScheduleExplanationKind::ChangeImpact
    }));
    5
}

fn change_fact(
    control: &str,
    sequence: u64,
    stage: ChangeStage,
    status: ChangeStatus,
) -> ChangeFact {
    ChangeFact::new(
        id(control),
        ChangeId::new("change.ventilation").unwrap(),
        sequence,
        stage,
        status,
        day(u8::try_from(sequence + 20).unwrap()),
        role("project-chief"),
        format!("{stage:?} fact"),
    )
}

fn change_amount(control: &str, side: CommercialSide, value: &str) -> ChangeAmountComponent {
    ChangeAmountComponent::new(
        id(control),
        side,
        "direct",
        CommercialAmount::parse(value, currency()).unwrap(),
    )
}

fn change_reference(control: &str, value: Option<&str>) -> ReferencedAmountEvidence {
    let evidence = ReferencedAmountEvidence::new(
        CommercialEvidenceSource::Document,
        reference(&format!("change/{control}"), "A"),
        day(25),
        EvidenceState::Accepted,
    );
    match value {
        Some(value) => evidence.with_stated_value(ReferencedAmount::new(
            Amount::parse(value).unwrap(),
            currency(),
        )),
        None => evidence,
    }
}
