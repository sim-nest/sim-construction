// conformance: each construction handover completion meaning is an accountable sequenced gate

use crate::{
    CONSTRUCTION_EXCEPTION_CAPABILITY, CommissioningAssessment, CommissioningControlSet,
    CommissioningRequirement, CommissioningRequirementKind, ConstructionProjectError, ControlId,
    ExceptionDecision, ExceptionScope, HandoverControlKind, HandoverGate, HandoverGateDecision,
    HandoverGateDecisionKind, HandoverGateKind, HandoverHierarchy, ProjectBook, ProjectFact,
    ProjectId, ProjectObligation, Requirement, RequirementLane, RoleId,
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
    HandoverGate::new(
        project(),
        control(&format!("gate.{kind:?}").to_ascii_lowercase()),
        control("system.heating"),
        kind,
        role("role.customer"),
    )
}

fn requirement(kind: CommissioningRequirementKind, id: &str) -> CommissioningRequirement {
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
        control("system.heating"),
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
    Date::from_calendar_date(2026, Month::July, 30).unwrap()
}
