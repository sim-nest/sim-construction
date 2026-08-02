// conformance: each construction handover completion meaning is an accountable sequenced gate

use crate::{
    CONSTRUCTION_EXCEPTION_CAPABILITY, CommissioningAssessment, CommissioningControlSet,
    CommissioningRequirement, CommissioningRequirementKind, ConstructionProjectError, ControlId,
    EvidenceState, EvidenceValidity, ExceptionDecision, ExceptionScope, HandoverControlKind,
    HandoverGate, HandoverGateDecision, HandoverGateDecisionKind, HandoverGateKind,
    HandoverHierarchy, ProjectBook, ProjectFact, ProjectId, ProjectObligation, Requirement,
    RequirementLane, RoleId,
};
use sim_kernel::{Expr, Symbol};
use sim_lib_doc_core::ExternalRef;
use time::{Date, Month};

#[test]
fn six_completion_meanings_keep_separate_reports_and_accountable_sequences() {
    let mut hierarchy = HandoverHierarchy::new(project());
    hierarchy
        .add_control(control("system.heating"), HandoverControlKind::System)
        .unwrap();
    let controls = CommissioningControlSet::new(project())
        .with_requirement(requirement(
            CommissioningRequirementKind::Activity,
            "requirement.activity",
        ))
        .with_requirement(requirement(
            CommissioningRequirementKind::OperationsMaintenanceDeliverable,
            "requirement.om",
        ))
        .with_requirement(requirement(
            CommissioningRequirementKind::Training,
            "requirement.training",
        ))
        .with_requirement(requirement(
            CommissioningRequirementKind::AuthorityClosure,
            "requirement.authority",
        ))
        .with_requirement(requirement(
            CommissioningRequirementKind::CustomerAcceptance,
            "requirement.customer",
        ))
        .with_requirement(requirement(
            CommissioningRequirementKind::RemainingWork,
            "requirement.remaining",
        ));
    let mut book = ProjectBook::new(project(), writer());
    book.append(accepted_fact(1, "requirement.activity"))
        .unwrap();
    book.append(accepted_fact(2, "requirement.authority"))
        .unwrap();
    book.append(accepted_fact(3, "requirement.customer"))
        .unwrap();
    let assessment = CommissioningAssessment::new(&book, 3, today())
        .with_exception(remaining_work_exception())
        .with_capability(CONSTRUCTION_EXCEPTION_CAPABILITY);

    let reports = all_gate_kinds()
        .into_iter()
        .map(|kind| {
            let gate = handover_gate(kind);
            let report = gate.report(&controls, &hierarchy, &assessment).unwrap();
            (kind, gate, report)
        })
        .collect::<Vec<_>>();

    assert_eq!(
        reports
            .iter()
            .map(|(kind, _, report)| (*kind, report.readiness.ready))
            .collect::<Vec<_>>(),
        vec![
            (HandoverGateKind::TechnicalCompletion, true),
            (HandoverGateKind::EvidenceCompletion, false),
            (HandoverGateKind::AuthorityCompletion, true),
            (HandoverGateKind::ContractualAcceptance, true),
            (HandoverGateKind::OccupancyUseReadiness, false),
            (HandoverGateKind::FinalCompletion, false),
        ]
    );
    assert!(reports.iter().all(|(_, _, report)| report.as_of_seq == 3));
    let (_, contractual_gate, contractual_report) = &reports[3];
    assert_eq!(contractual_report.readiness.burn_down.excepted, 1);
    HandoverGateDecision::new(
        contractual_gate,
        contractual_report.as_of_seq,
        4,
        HandoverGateDecisionKind::Accept,
        role("role.customer"),
    )
    .with_evidence(external_ref("contractual-acceptance"))
    .validate_against(contractual_gate, contractual_report)
    .unwrap();

    let (_, final_gate, final_report) = &reports[5];
    assert_eq!(final_report.readiness.burn_down.excepted, 0);
    assert_eq!(final_report.readiness.burn_down.missing, 3);
    let result = HandoverGateDecision::new(
        final_gate,
        final_report.as_of_seq,
        4,
        HandoverGateDecisionKind::Accept,
        role("role.customer"),
    )
    .with_evidence(external_ref("premature-final-acceptance"))
    .validate_against(final_gate, final_report);
    assert!(matches!(
        result,
        Err(ConstructionProjectError::GateReportNotReady { .. })
    ));
}

#[test]
fn partial_system_acceptance_does_not_accept_its_sibling_or_parent_area() {
    let mut hierarchy = HandoverHierarchy::new(project());
    add(&mut hierarchy, "area.building", HandoverControlKind::Area);
    add(
        &mut hierarchy,
        "system.heating",
        HandoverControlKind::System,
    );
    add(
        &mut hierarchy,
        "system.cooling",
        HandoverControlKind::System,
    );
    hierarchy
        .add_member(control("system.heating"), control("area.building"))
        .unwrap();
    hierarchy
        .add_member(control("system.cooling"), control("area.building"))
        .unwrap();
    let controls = CommissioningControlSet::new(project())
        .with_requirement(requirement_for(
            CommissioningRequirementKind::CustomerAcceptance,
            "requirement.accept-heating",
            "system.heating",
        ))
        .with_requirement(requirement_for(
            CommissioningRequirementKind::CustomerAcceptance,
            "requirement.accept-cooling",
            "system.cooling",
        ));
    let mut book = ProjectBook::new(project(), writer());
    book.append(accepted_fact(1, "requirement.accept-heating"))
        .unwrap();
    let assessment = CommissioningAssessment::new(&book, 1, today());

    let heating = gate_for(HandoverGateKind::ContractualAcceptance, "system.heating")
        .report(&controls, &hierarchy, &assessment)
        .unwrap();
    let cooling = gate_for(HandoverGateKind::ContractualAcceptance, "system.cooling")
        .report(&controls, &hierarchy, &assessment)
        .unwrap();
    let area = gate_for(HandoverGateKind::ContractualAcceptance, "area.building")
        .report(&controls, &hierarchy, &assessment)
        .unwrap();

    assert!(heating.readiness.ready);
    assert!(!cooling.readiness.ready);
    assert!(!area.readiness.ready);
    assert_eq!(area.readiness.burn_down.accepted, 1);
    assert_eq!(area.readiness.burn_down.missing, 1);
}

#[test]
fn one_cross_area_system_rolls_the_same_leaf_evidence_into_each_area() {
    let mut hierarchy = HandoverHierarchy::new(project());
    add(&mut hierarchy, "area.east", HandoverControlKind::Area);
    add(&mut hierarchy, "area.west", HandoverControlKind::Area);
    add(
        &mut hierarchy,
        "system.fire-alarm",
        HandoverControlKind::System,
    );
    hierarchy
        .add_member(control("system.fire-alarm"), control("area.east"))
        .unwrap();
    hierarchy
        .add_member(control("system.fire-alarm"), control("area.west"))
        .unwrap();
    let controls = CommissioningControlSet::new(project()).with_requirement(requirement_for(
        CommissioningRequirementKind::Test,
        "requirement.fire-alarm-test",
        "system.fire-alarm",
    ));
    let mut book = ProjectBook::new(project(), writer());
    book.append(accepted_fact(1, "requirement.fire-alarm-test"))
        .unwrap();
    let assessment = CommissioningAssessment::new(&book, 1, today());

    for area in ["area.east", "area.west"] {
        let report = gate_for(HandoverGateKind::TechnicalCompletion, area)
            .report(&controls, &hierarchy, &assessment)
            .unwrap();
        assert!(report.readiness.ready);
        assert_eq!(report.readiness.burn_down.total, 1);
        assert_eq!(report.readiness.burn_down.accepted, 1);
    }
}

#[test]
fn accepted_retest_supersedes_a_rejected_test_at_a_later_sequence() {
    let mut hierarchy = HandoverHierarchy::new(project());
    add(
        &mut hierarchy,
        "system.heating",
        HandoverControlKind::System,
    );
    let controls = CommissioningControlSet::new(project()).with_requirement(requirement(
        CommissioningRequirementKind::Test,
        "requirement.functional-test",
    ));
    let mut book = ProjectBook::new(project(), writer());
    book.append(
        accepted_fact(1, "requirement.functional-test")
            .with_evidence_state(EvidenceState::Rejected),
    )
    .unwrap();
    let gate = handover_gate(HandoverGateKind::TechnicalCompletion);
    let rejected = gate
        .report(
            &controls,
            &hierarchy,
            &CommissioningAssessment::new(&book, 1, today()),
        )
        .unwrap();
    assert!(!rejected.readiness.ready);
    assert_eq!(rejected.readiness.burn_down.rejected, 1);

    book.append(accepted_fact(2, "requirement.functional-test").supersedes(1))
        .unwrap();
    let retested = gate
        .report(
            &controls,
            &hierarchy,
            &CommissioningAssessment::new(&book, 2, today()),
        )
        .unwrap();
    assert!(retested.readiness.ready);
    assert_eq!(retested.readiness.burn_down.accepted, 1);
    assert_eq!(retested.readiness.items[0].current_seq, Some(2));
}

#[test]
fn expired_certificate_critical_defect_and_absent_training_block_exact_gates() {
    let mut hierarchy = HandoverHierarchy::new(project());
    add(
        &mut hierarchy,
        "system.heating",
        HandoverControlKind::System,
    );
    let mut certificate = requirement(
        CommissioningRequirementKind::Certification,
        "requirement.certificate",
    );
    certificate.obligation.evidence_validity =
        EvidenceValidity::new(None, Some(date(2026, Month::July, 29)));
    let controls = CommissioningControlSet::new(project())
        .with_requirement(certificate)
        .with_requirement(
            requirement(
                CommissioningRequirementKind::Defect,
                "requirement.critical-defect",
            )
            .critical(),
        )
        .with_requirement(requirement(
            CommissioningRequirementKind::Training,
            "requirement.training",
        ));
    let mut book = ProjectBook::new(project(), writer());
    book.append(accepted_fact(1, "requirement.certificate"))
        .unwrap();
    let assessment = CommissioningAssessment::new(&book, 1, today());

    let technical = handover_gate(HandoverGateKind::TechnicalCompletion)
        .report(&controls, &hierarchy, &assessment)
        .unwrap();
    let evidence = handover_gate(HandoverGateKind::EvidenceCompletion)
        .report(&controls, &hierarchy, &assessment)
        .unwrap();
    let authority = handover_gate(HandoverGateKind::AuthorityCompletion)
        .report(&controls, &hierarchy, &assessment)
        .unwrap();
    let occupancy = handover_gate(HandoverGateKind::OccupancyUseReadiness)
        .report(&controls, &hierarchy, &assessment)
        .unwrap();

    assert!(technical.readiness.blockers().any(|item| item.critical));
    assert_eq!(evidence.readiness.burn_down.expired, 1);
    assert_eq!(evidence.readiness.burn_down.missing, 1);
    assert_eq!(authority.readiness.burn_down.expired, 1);
    assert!(!occupancy.readiness.ready);
    assert_eq!(occupancy.readiness.blockers().count(), 3);
}

#[test]
fn non_waivable_authority_closure_rejects_an_exception_attempt() {
    let mut hierarchy = HandoverHierarchy::new(project());
    add(
        &mut hierarchy,
        "system.heating",
        HandoverControlKind::System,
    );
    let controls = CommissioningControlSet::new(project()).with_requirement(requirement(
        CommissioningRequirementKind::AuthorityClosure,
        "requirement.authority",
    ));
    let book = ProjectBook::new(project(), writer());
    let exception = ExceptionDecision::new(
        control("exception.authority"),
        ExceptionScope::new(project()).covers(control("requirement.authority")),
        role("role.customer"),
        role("role.customer"),
        "attempted authority override",
        today(),
        date(2026, Month::August, 30),
    )
    .with_evidence(external_ref("authority-exception"));
    let assessment = CommissioningAssessment::new(&book, 0, today())
        .with_exception(exception)
        .with_capability(CONSTRUCTION_EXCEPTION_CAPABILITY);

    let result = handover_gate(HandoverGateKind::AuthorityCompletion).report(
        &controls,
        &hierarchy,
        &assessment,
    );

    assert!(matches!(
        result,
        Err(ConstructionProjectError::NonWaivableRequirement { .. })
    ));
}

fn all_gate_kinds() -> [HandoverGateKind; 6] {
    [
        HandoverGateKind::TechnicalCompletion,
        HandoverGateKind::EvidenceCompletion,
        HandoverGateKind::AuthorityCompletion,
        HandoverGateKind::ContractualAcceptance,
        HandoverGateKind::OccupancyUseReadiness,
        HandoverGateKind::FinalCompletion,
    ]
}

fn handover_gate(kind: HandoverGateKind) -> HandoverGate {
    gate_for(kind, "system.heating")
}

fn gate_for(kind: HandoverGateKind, target: &str) -> HandoverGate {
    HandoverGate::new(
        project(),
        control(&format!("gate.{kind:?}.{target}").to_ascii_lowercase()),
        control(target),
        kind,
        role("role.customer"),
    )
}

fn requirement(kind: CommissioningRequirementKind, id: &str) -> CommissioningRequirement {
    requirement_for(kind, id, "system.heating")
}

fn requirement_for(
    kind: CommissioningRequirementKind,
    id: &str,
    target: &str,
) -> CommissioningRequirement {
    let requirement = Requirement::new(
        control(id),
        RequirementLane::new(Symbol::qualified("construction", "handover")),
        format!("{kind:?} evidence"),
        role("role.commissioning-lead"),
        role("role.customer"),
    )
    .with_evidence_kind(Symbol::qualified("construction", "commissioning-record"))
    .with_source_ref(external_ref(id));
    CommissioningRequirement::new(
        kind,
        ProjectObligation::mandatory(project(), requirement),
        control(target),
    )
}

fn accepted_fact(seq: u64, requirement: &str) -> ProjectFact {
    ProjectFact::new(
        seq,
        project(),
        control(requirement),
        Symbol::qualified("construction", "commissioning-evidence"),
        today(),
        writer(),
        Expr::Nil,
    )
    .with_evidence(external_ref(requirement))
}

fn remaining_work_exception() -> ExceptionDecision {
    ExceptionDecision::new(
        control("exception.remaining-work"),
        ExceptionScope::new(project()).covers(control("requirement.remaining")),
        role("role.customer"),
        role("role.customer"),
        "bounded remaining work accepted for partial handover",
        today(),
        Date::from_calendar_date(2026, Month::August, 30).unwrap(),
    )
    .with_evidence(external_ref("remaining-work-acceptance"))
}

fn project() -> ProjectId {
    ProjectId::new("project.handover").unwrap()
}

fn writer() -> RoleId {
    role("role.project-writer")
}

fn role(id: &str) -> RoleId {
    RoleId::new(id).unwrap()
}

fn control(id: &str) -> ControlId {
    ControlId::new(id).unwrap()
}

fn external_ref(id: &str) -> ExternalRef {
    ExternalRef::new("doc/synthetic", id, Some("v1".to_owned()), None)
}

fn today() -> Date {
    date(2026, Month::July, 30)
}

fn date(year: i32, month: Month, day: u8) -> Date {
    Date::from_calendar_date(year, month, day).unwrap()
}

fn add(hierarchy: &mut HandoverHierarchy, id: &str, kind: HandoverControlKind) {
    hierarchy.add_control(control(id), kind).unwrap();
}
