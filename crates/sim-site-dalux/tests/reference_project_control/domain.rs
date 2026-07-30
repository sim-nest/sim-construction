use sim_kernel::{Cx, Symbol};
use sim_lib_construction_project::{
    AwardDecision, AwardDecisionKind, BidDecision, BidDecisionKind, CollaborationCharter,
    CommercialAmount, CustomerIntentAcceptance, DesignControlSet, DesignRelease,
    DesignReleasePurpose, DesignRevision, DisclosureState, DomainQuantity, EvidenceState,
    EvidenceValidity, GateDecision, GateDecisionKind, GateRequirement, OrganizationId,
    OutcomeBoundary, OutcomeMethod, OutcomeRecord, OutcomeRecordKind, OutcomeRecordSpec,
    OutcomeTargetKind, PackageHandoff, PackageHandoffControlSet, PermitRecord, PermitState,
    PhaseGate, ProcurementControlSet, ProductionActivity, ProductionPlan, ProductionReadinessState,
    ProjectBook, ProjectPhase, QualificationEvidence, QualificationRequirement,
    QualificationStatus, RegisteredOutcomeShape, ScheduleBaseline, ScheduleJoinKind,
    SchedulePlanRevision, ScheduleTaskJoin, ScheduleTaskJoinSet, ScopeCompliance,
    SupplierCandidate, SupplierQualificationArea, SupplierQualificationSet, SupplierReference,
    SustainabilityTarget, SustainabilityTargetSpec, TenderComparison, TenderQualification,
    WorkPackage, evaluate_outcomes,
};
use sim_lib_gantt_control::{GanttPlan, Task};

use super::support::{
    accepted_book, august, baseline_id, control_fact, currency, day, id, mandatory, project,
    reference, role,
};

#[derive(Debug, PartialEq, Eq)]
pub(super) struct DomainProof {
    pub opportunity_bid_collaboration: bool,
    pub late_customer_decision: bool,
    pub missing_collaboration_evidence: bool,
    pub mobilization: bool,
    pub design_permit: bool,
    pub procurement_supplier_handoff: bool,
    pub supplier_expired_then_renewed: bool,
    pub sustainability: bool,
    pub lookahead: bool,
    pub field_safety: bool,
    pub non_waivable_safety_blocked: bool,
    pub bounded_exception_expired: bool,
    pub risk: bool,
    pub change_economy: bool,
    pub partial_change_approval: bool,
    pub double_count_prevented: bool,
    pub critical_schedule_effect_days: i64,
}

pub(super) fn run(cx: &mut Cx) -> DomainProof {
    let (opportunity_bid_collaboration, late_customer_decision, missing_collaboration_evidence) =
        prove_opportunity_bid_collaboration();
    let (procurement_supplier_handoff, supplier_expired_then_renewed) =
        prove_procurement_supplier_handoff();
    let controls = super::control::run(cx);
    DomainProof {
        opportunity_bid_collaboration,
        late_customer_decision,
        missing_collaboration_evidence,
        mobilization: prove_mobilization(),
        design_permit: prove_design_and_permit(),
        procurement_supplier_handoff,
        supplier_expired_then_renewed,
        sustainability: prove_sustainability(),
        lookahead: prove_lookahead(),
        field_safety: controls.field_safety,
        non_waivable_safety_blocked: controls.non_waivable_safety_blocked,
        bounded_exception_expired: controls.bounded_exception_expired,
        risk: controls.risk,
        change_economy: controls.change_economy,
        partial_change_approval: controls.partial_change_approval,
        double_count_prevented: controls.double_count_prevented,
        critical_schedule_effect_days: controls.critical_schedule_effect_days,
    }
}

fn prove_opportunity_bid_collaboration() -> (bool, bool, bool) {
    let opportunity = sim_lib_construction_project::OpportunityRecord::new(
        project(),
        id("opportunity.renovation"),
        sim_lib_construction_project::OpportunitySource::Customer,
        role("project-chief"),
        "Invented Nordhamn market renovation",
    )
    .with_evidence(reference("opportunity/invitation", "A"));
    opportunity.validate().unwrap();

    let bid = BidDecision::new(
        project(),
        id("bid.conditional"),
        id("intent.scope"),
        role("project-chief"),
        role("project-chief"),
        BidDecisionKind::ConditionalBid,
    )
    .with_capacity_view("renovation team reserved")
    .with_risk("occupied market access awaits decision")
    .with_opportunity("reuse existing luminaires")
    .with_assumption("night delivery is bounded")
    .valid_until(day(15))
    .with_price_basis(reference("bid/price", "B"))
    .with_schedule_basis(reference("bid/schedule", "B"))
    .with_evidence(reference("bid/decision", "B"));
    bid.validate().unwrap();

    let late = CustomerIntentAcceptance::new(
        id("decision.customer-access"),
        role("project-chief"),
        role("project-chief"),
        day(8),
        day(11),
    )
    .with_evidence(reference("intent/access-decision", "A"));
    late.validate().unwrap();
    assert!(late.is_late());

    let mut book = ProjectBook::new(project(), role("project-chief"));
    book.append(control_fact(
        1,
        "requirement.intent-scope",
        EvidenceState::Accepted,
    ))
    .unwrap();
    let charter = CollaborationCharter::new(project(), id("collaboration.charter"), "weekly")
        .with_objective("convert accepted intent into contract evidence")
        .with_working_principle("unstated assumptions remain missing")
        .with_organization("invented customer and delivery core team")
        .with_decision_right("project chief accepts contract basis")
        .with_investigation("occupied-hall access survey")
        .with_target_design_buildability_work("phased fire design")
        .with_open_book_rule("referenced cost facts only")
        .with_escalation_role(role("project-chief"))
        .with_main_contract_evidence(id("contract.main"))
        .with_obligation(mandatory("requirement.intent-scope", "intent"))
        .with_evidence(reference("collaboration/charter", "A"));
    let missing = charter.readiness_report(&book, 1, day(11)).unwrap();
    assert!(!missing.ready);
    assert_eq!(
        missing.missing_main_contract_evidence,
        [id("contract.main")]
    );
    book.append(control_fact(2, "contract.main", EvidenceState::Accepted))
        .unwrap();
    let ready = charter.readiness_report(&book, 2, day(11)).unwrap();
    (
        ready.ready,
        late.is_late(),
        !missing.ready && missing.missing_main_contract_evidence == [id("contract.main")],
    )
}

fn prove_mobilization() -> bool {
    let mut book = ProjectBook::new(project(), role("project-chief"));
    book.append(control_fact(1, "contract.main", EvidenceState::Accepted))
        .unwrap();
    let gate = PhaseGate::new(
        project(),
        id("gate.mobilization"),
        ProjectPhase::Mobilization,
        role("project-chief"),
    )
    .with_requirement(GateRequirement::new(id("contract.main")))
    .with_evidence(reference("gate/mobilization", "A"));
    let report = gate.report_at(&book, 1, day(12)).unwrap();
    let decision = GateDecision::new(
        id("gate.mobilization"),
        project(),
        GateDecisionKind::Approve,
        role("project-chief"),
        1,
        2,
    )
    .with_evidence(reference("gate/mobilization-decision", "A"));
    decision.validate_against(&gate, &report).unwrap();
    report.ready
}

fn design_set() -> DesignControlSet {
    DesignControlSet::new()
        .with_revision(
            DesignRevision::new(project(), id("design.fire"), "C", role("designer"), day(12))
                .with_evidence_state(EvidenceState::Accepted)
                .affects(id("package.electrical"))
                .with_external_ref(reference("design/fire", "C")),
        )
        .with_release(
            DesignRelease::new(
                project(),
                id("release.fire-production"),
                id("design.fire"),
                "C",
                DesignReleasePurpose::Production,
                role("designer"),
                role("project-chief"),
                role("project-chief"),
                day(12),
            )
            .with_evidence_state(EvidenceState::Accepted)
            .affects(id("package.electrical"))
            .with_external_ref(reference("release/fire", "C")),
        )
        .with_permit(
            PermitRecord::new(
                project(),
                id("permit.fire"),
                role("authority-lead"),
                day(13),
            )
            .with_state(PermitState::Granted)
            .with_evidence_state(EvidenceState::Accepted)
            .with_validity(EvidenceValidity::new(None, Some(day(30))))
            .affects(id("package.electrical"))
            .with_external_ref(reference("permit/fire", "A")),
        )
}

fn prove_design_and_permit() -> bool {
    design_set()
        .readiness_for(
            id("package.electrical"),
            DesignReleasePurpose::Production,
            day(14),
        )
        .unwrap()
        .ready
}

fn prove_procurement_supplier_handoff() -> (bool, bool) {
    let procurement = awarded_procurement()
        .readiness_for(&work_package(), &currency(), day(20))
        .unwrap();

    let supplier = OrganizationId::new("vendor.blue-arc-installations").unwrap();
    let requirement = supplier_requirement();
    let expired_set = SupplierQualificationSet::new()
        .with_supplier(supplier_reference())
        .with_requirement(requirement.clone())
        .with_evidence(
            QualificationEvidence::new(
                supplier.clone(),
                id("qual.insurance"),
                EvidenceState::Accepted,
                role("commercial-lead"),
                day(10),
                "invented policy expired before evaluation",
            )
            .with_validity(EvidenceValidity::new(None, Some(day(18))))
            .with_evidence(reference("supplier/insurance", "A")),
        );
    let expired = expired_set
        .qualification_for(&supplier, day(20), day(25))
        .unwrap();
    assert_eq!(expired.status, QualificationStatus::NotQualified);

    let qualified = SupplierQualificationSet::new()
        .with_supplier(supplier_reference())
        .with_requirement(requirement)
        .with_evidence(
            QualificationEvidence::new(
                supplier.clone(),
                id("qual.insurance"),
                EvidenceState::Accepted,
                role("commercial-lead"),
                day(19),
                "invented policy renewed",
            )
            .with_validity(EvidenceValidity::new(None, Some(day(30))))
            .with_evidence(reference("supplier/insurance", "B")),
        )
        .qualification_for(&supplier, day(20), day(25))
        .unwrap();
    assert_eq!(qualified.status, QualificationStatus::Qualified);

    let design = design_set()
        .readiness_for(
            id("package.electrical"),
            DesignReleasePurpose::Production,
            day(20),
        )
        .unwrap();
    let handoff = PackageHandoffControlSet::new()
        .with_handoff(
            PackageHandoff::new(
                id("handoff.electrical"),
                id("package.electrical"),
                id("award.electrical"),
                supplier,
                id("release.fire-production"),
                id("material.electrical"),
                24,
                august(20),
                role("site-manager"),
            )
            .accepts_responsibility(day(20), "supplier accepts package responsibility")
            .with_evidence(reference("handoff/electrical", "A")),
        )
        .readiness_for(
            &id("handoff.electrical"),
            &procurement,
            &qualified,
            &design,
            day(20),
        )
        .unwrap();
    assert!(handoff.ready, "{:?}", handoff.blockers);
    (
        handoff.ready,
        expired.status == QualificationStatus::NotQualified
            && qualified.status == QualificationStatus::Qualified,
    )
}

fn work_package() -> WorkPackage {
    WorkPackage::new(
        project(),
        id("package.electrical"),
        "Electrical renovation package",
        role("procurement-lead"),
        role("project-chief"),
        day(5),
        day(18),
        day(30),
        CommercialAmount::parse("4200000.00", currency()).unwrap(),
    )
    .includes("electrical installation and controls")
    .requires_design_input(id("design.fire"))
    .with_supplier(
        SupplierCandidate::new("vendor.blue-arc-installations", "qualified")
            .with_evidence(reference("supplier/candidate", "A")),
    )
    .with_evidence(reference("package/electrical", "A"))
}

fn awarded_procurement() -> ProcurementControlSet {
    ProcurementControlSet::new()
        .with_tender(
            TenderComparison::new(
                id("tender.electrical"),
                id("package.electrical"),
                "vendor.blue-arc-installations",
                CommercialAmount::parse("3940000.00", currency()).unwrap(),
            )
            .with_lead_time_days(24)
            .with_capacity("installation team reserved")
            .with_scope_compliance(ScopeCompliance::Compliant)
            .with_qualification(TenderQualification::Qualified)
            .with_evidence(reference("tender/electrical", "B")),
        )
        .with_award(
            AwardDecision::new(
                id("award.electrical"),
                id("package.electrical"),
                AwardDecisionKind::Award,
                role("project-chief"),
                day(18),
                "qualified synthetic tender accepted",
            )
            .selects(id("tender.electrical"))
            .with_evidence(reference("award/electrical", "A")),
        )
}

fn supplier_reference() -> SupplierReference {
    SupplierReference::new(
        project(),
        OrganizationId::new("vendor.blue-arc-installations").unwrap(),
        role("installer"),
        role("commercial-lead"),
    )
    .with_validity(EvidenceValidity::new(None, Some(day(30))))
    .with_evidence(reference("supplier/project-reference", "A"))
}

fn supplier_requirement() -> QualificationRequirement {
    QualificationRequirement::new(
        SupplierQualificationArea::Insurance,
        mandatory("qual.insurance", "supplier"),
    )
}

fn prove_sustainability() -> bool {
    let method = OutcomeMethod::new(
        Symbol::qualified("method", "synthetic-climate"),
        "v1",
        Symbol::qualified("shape", "synthetic-climate"),
        reference("method/climate", "v1"),
    );
    let boundary = OutcomeBoundary::new(
        Symbol::qualified("boundary", "lifecycle"),
        "A1-A5",
        reference("boundary/a1-a5", "A"),
    );
    let target = SustainabilityTarget::new(SustainabilityTargetSpec {
        project: project(),
        id: id("outcome.climate"),
        kind: OutcomeTargetKind::Climate,
        category: RegisteredOutcomeShape::new(
            Symbol::qualified("construction-outcome", "climate"),
            Symbol::qualified("shape", "synthetic-climate"),
        ),
        title: "Embodied climate budget".to_owned(),
        target: DomainQuantity::new("190000", Symbol::qualified("unit", "kg-co2e")),
        method: method.clone(),
        boundary: boundary.clone(),
        responsible: role("sustainability-lead"),
    })
    .with_source_ref(reference("charter/climate", "A"))
    .allow_reference_claim();
    let original = OutcomeRecord::new(OutcomeRecordSpec {
        project: project(),
        id: id("calc.climate-original"),
        target: id("outcome.climate"),
        kind: OutcomeRecordKind::Measurement,
        quantity: DomainQuantity::new("194500", Symbol::qualified("unit", "kg-co2e")),
        method: method.clone(),
        boundary: boundary.clone(),
        responsible: role("sustainability-lead"),
        reported_on: day(16),
    })
    .with_evidence_state(EvidenceState::Accepted)
    .with_source_ref(reference("calculation/climate", "A"))
    .with_disclosure(DisclosureState::Accepted);
    let corrected = OutcomeRecord::new(OutcomeRecordSpec {
        project: project(),
        id: id("calc.climate-corrected"),
        target: id("outcome.climate"),
        kind: OutcomeRecordKind::Measurement,
        quantity: DomainQuantity::new("184500", Symbol::qualified("unit", "kg-co2e")),
        method,
        boundary,
        responsible: role("sustainability-lead"),
        reported_on: day(17),
    })
    .with_evidence_state(EvidenceState::Accepted)
    .with_source_ref(reference("calculation/climate", "B"))
    .with_disclosure(DisclosureState::Accepted)
    .supersedes(id("calc.climate-original"));
    let report = evaluate_outcomes(project(), &[target], &[original, corrected], day(20)).unwrap();
    report.gates_clear
        && report.targets[0].current_record == Some(id("calc.climate-corrected"))
        && report.targets[0].reference_claim_admissible
}

fn prove_lookahead() -> bool {
    let requirement = mandatory("prerequisite.workplace-introduction", "lookahead");
    let activity = ProductionActivity::new(
        id("activity.electrical"),
        "task.frame-install",
        id("package.electrical"),
        "electrical",
        "market-hall",
        "level-01",
        role("site-manager"),
        day(8),
        day(10),
        baseline_id(),
    )
    .requires(id("prerequisite.workplace-introduction"));
    let joins = ScheduleTaskJoinSet::new(
        ScheduleBaseline::new(baseline_id(), "plan.nordhamn-renovation", "accepted-C", 1).unwrap(),
        SchedulePlanRevision::new("plan.nordhamn-renovation", "accepted-C", 20).unwrap(),
        vec![ScheduleTaskJoin::new(
            id("activity.electrical"),
            "task.frame-install",
            ScheduleJoinKind::Package,
        )],
    )
    .unwrap();
    let gantt = GanttPlan::new(
        "plan.nordhamn-renovation",
        vec![Task::new(
            "task.frame-install",
            "Frame installation",
            day(8),
            day(10),
            0,
        )],
        vec![],
    );
    let missing = ProductionPlan::new()
        .with_activity(activity.clone())
        .with_obligation(requirement.clone())
        .derive_readiness(
            &ProjectBook::new(project(), role("project-chief")),
            &gantt,
            &joins,
            day(1),
        )
        .unwrap();
    assert_eq!(
        missing.three_week_commitment[0].state,
        ProductionReadinessState::NotReady
    );

    let book = accepted_book("prerequisite.workplace-introduction");
    ProductionPlan::new()
        .with_activity(activity)
        .with_obligation(requirement)
        .derive_readiness(&book, &gantt, &joins, day(1))
        .unwrap()
        .three_week_commitment[0]
        .state
        == ProductionReadinessState::Ready
}
