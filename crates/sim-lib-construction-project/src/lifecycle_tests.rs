// conformance: construction lifecycle baselines, gates, actions, and decisions

use crate::{
    AcceptedBaseline, ActionResolution, ActionState, BaselineKind, ConstructionProjectError,
    ControlId, DecisionResolution, DecisionState, GateDecision, GateDecisionKind, GateRequirement,
    LifecyclePolicy, PhaseGate, PhaseOverlap, PhaseTransition, ProjectAction, ProjectBook,
    ProjectDecision, ProjectId, ProjectPhase, RoleId,
};
use sim_kernel::{Expr, Symbol};
use sim_lib_doc_core::ExternalRef;
use time::{Date, Month};

#[test]
fn lifecycle_keeps_order_and_requires_explicit_regression_decision() {
    let policy = LifecyclePolicy::standard().with_overlap(PhaseOverlap::new(
        ProjectPhase::Design,
        ProjectPhase::Procurement,
        "procurement may prepare long-lead packages before design is complete",
    ));

    assert!(policy.allows_overlap(ProjectPhase::Procurement, ProjectPhase::Design));
    assert!(ProjectPhase::Closeout > ProjectPhase::Opportunity);

    let regression = PhaseTransition::new(
        project(),
        ControlId::new("phase.closeout-to-design").unwrap(),
        ProjectPhase::Closeout,
        ProjectPhase::Design,
        RoleId::new("project-chief").unwrap(),
    );
    assert!(matches!(
        regression.validate(),
        Err(ConstructionProjectError::PhaseRegressionRequiresDecision { .. })
    ));

    regression
        .with_regression_decision(ControlId::new("decision.reopen-design").unwrap())
        .validate()
        .unwrap();
}

#[test]
fn accepted_baseline_rejects_stale_comparison_sequence() {
    let book = project_book_with_gate_facts();
    let baseline = AcceptedBaseline::new(
        crate::BaselineId::new("baseline.scope").unwrap(),
        project(),
        ControlId::new("baseline.scope").unwrap(),
        BaselineKind::Scope,
        RoleId::new("project-chief").unwrap(),
        2,
        accepted_on(),
    )
    .with_evidence(evidence_ref("baseline"));

    assert!(matches!(
        baseline.comparison_snapshot(&book, 1),
        Err(ConstructionProjectError::StaleBaselineComparison { .. })
    ));
    assert_eq!(
        baseline.comparison_snapshot(&book, 2).unwrap().through_seq,
        2
    );
}

#[test]
fn actions_and_decisions_reject_orphans_and_closed_items_without_resolution() {
    let snapshot = project_book_with_gate_facts().snapshot_at(2).unwrap();
    let action = ProjectAction::new(
        project(),
        ControlId::new("action.site-access").unwrap(),
        RoleId::new("supplier-lead").unwrap(),
        accepted_on(),
        RoleId::new("project-chief").unwrap(),
        "mobilization cannot start",
    )
    .with_reference(ControlId::new("requirement.missing").unwrap())
    .with_evidence(evidence_ref("action"));

    assert!(matches!(
        action.validate_against(&snapshot),
        Err(ConstructionProjectError::OrphanControlRef { .. })
    ));

    let closed = ProjectAction::new(
        project(),
        ControlId::new("action.site-access").unwrap(),
        RoleId::new("supplier-lead").unwrap(),
        accepted_on(),
        RoleId::new("project-chief").unwrap(),
        "mobilization cannot start",
    )
    .with_state(ActionState::Closed)
    .with_reference(ControlId::new("requirement.site-access").unwrap())
    .with_evidence(evidence_ref("action"));
    assert!(matches!(
        closed.validate_against(&snapshot),
        Err(ConstructionProjectError::MissingResolutionFact { kind: "action", .. })
    ));

    let decision = ProjectDecision::new(
        project(),
        ControlId::new("decision.site-access").unwrap(),
        RoleId::new("supplier-lead").unwrap(),
        accepted_on(),
        RoleId::new("project-chief").unwrap(),
        RoleId::new("project-chief").unwrap(),
        "late decision delays mobilization",
    )
    .with_state(DecisionState::Closed)
    .with_reference(ControlId::new("requirement.site-access").unwrap())
    .with_evidence(evidence_ref("decision"))
    .with_resolution(DecisionResolution {
        fact_seq: 3,
        decided_by: RoleId::new("supplier-lead").unwrap(),
        outcome: "approved by the wrong role".to_owned(),
    });
    assert!(matches!(
        decision.validate_against(&snapshot),
        Err(ConstructionProjectError::DecisionAuthorityMismatch { .. })
    ));

    ProjectAction::new(
        project(),
        ControlId::new("action.site-access").unwrap(),
        RoleId::new("supplier-lead").unwrap(),
        accepted_on(),
        RoleId::new("project-chief").unwrap(),
        "mobilization cannot start",
    )
    .with_state(ActionState::Closed)
    .with_reference(ControlId::new("requirement.site-access").unwrap())
    .with_evidence(evidence_ref("action"))
    .with_resolution(ActionResolution {
        fact_seq: 3,
        resolved_by: RoleId::new("supplier-lead").unwrap(),
        summary: "access confirmed".to_owned(),
    })
    .validate_against(&snapshot)
    .unwrap();
}

#[test]
fn gate_report_is_derived_and_gate_approval_is_separate_authority() {
    let book = project_book_with_gate_facts();
    let gate = mobilization_gate();

    let early = gate.report_at(&book, 1, accepted_on()).unwrap();
    assert!(!early.ready);
    assert_eq!(
        early.unmet,
        vec![ControlId::new("requirement.risk").unwrap()]
    );

    let ready = gate.report_at(&book, 2, accepted_on()).unwrap();
    assert!(ready.ready);

    let wrong_authority = GateDecision::new(
        gate.gate.clone(),
        project(),
        GateDecisionKind::Approve,
        RoleId::new("supplier-lead").unwrap(),
        2,
        3,
    )
    .with_evidence(evidence_ref("gate-decision"));
    assert!(matches!(
        wrong_authority.validate_against(&gate, &ready),
        Err(ConstructionProjectError::ApprovalAuthorityMismatch { .. })
    ));

    let wrong_sequence = GateDecision::new(
        gate.gate.clone(),
        project(),
        GateDecisionKind::Approve,
        RoleId::new("project-chief").unwrap(),
        1,
        3,
    )
    .with_evidence(evidence_ref("gate-decision"));
    assert!(matches!(
        wrong_sequence.validate_against(&gate, &ready),
        Err(ConstructionProjectError::GateSequenceMismatch { .. })
    ));

    let approved = GateDecision::new(
        gate.gate.clone(),
        project(),
        GateDecisionKind::Approve,
        RoleId::new("project-chief").unwrap(),
        2,
        3,
    )
    .with_evidence(evidence_ref("gate-decision"));
    approved.validate_against(&gate, &ready).unwrap();
}

#[test]
fn gate_report_tracks_conflicts_expiry_and_exceptions() {
    let mut book = ProjectBook::new(project(), writer());
    book.append(project_fact(1, "requirement.risk", "risk reviewed"))
        .unwrap();
    book.append(project_fact(2, "requirement.risk", "competing risk"))
        .unwrap();
    book.append(project_fact(3, "exception.site-access", "customer waiver"))
        .unwrap();
    book.append(project_fact(4, "requirement.reporting", "weekly pack"))
        .unwrap();

    let gate = PhaseGate::new(
        project(),
        ControlId::new("gate.mobilization").unwrap(),
        ProjectPhase::Mobilization,
        RoleId::new("project-chief").unwrap(),
    )
    .with_requirement(GateRequirement::new(
        ControlId::new("requirement.risk").unwrap(),
    ))
    .with_requirement(
        GateRequirement::new(ControlId::new("requirement.site-access").unwrap())
            .with_exception(ControlId::new("exception.site-access").unwrap()),
    )
    .with_requirement(
        GateRequirement::new(ControlId::new("requirement.reporting").unwrap())
            .expires_on(Date::from_calendar_date(2026, Month::July, 22).unwrap()),
    )
    .with_evidence(evidence_ref("gate"));

    let report = gate.report_at(&book, 4, accepted_on()).unwrap();

    assert_eq!(
        report.conflicted,
        vec![ControlId::new("requirement.risk").unwrap()]
    );
    assert_eq!(
        report.expired,
        vec![ControlId::new("requirement.reporting").unwrap()]
    );
    assert_eq!(
        report.applied_exceptions,
        vec![ControlId::new("exception.site-access").unwrap()]
    );
    assert!(!report.ready);
    assert!(matches!(
        GateDecision::new(
            gate.gate.clone(),
            project(),
            GateDecisionKind::Approve,
            RoleId::new("project-chief").unwrap(),
            4,
            5
        )
        .with_evidence(evidence_ref("gate-decision"))
        .validate_against(&gate, &report),
        Err(ConstructionProjectError::GateReportNotReady { .. })
    ));
}

fn project_book_with_gate_facts() -> ProjectBook {
    let mut book = ProjectBook::new(project(), writer());
    book.append(project_fact(
        1,
        "requirement.site-access",
        "site access accepted",
    ))
    .unwrap();
    book.append(project_fact(2, "requirement.risk", "risk review accepted"))
        .unwrap();
    book
}

fn mobilization_gate() -> PhaseGate {
    PhaseGate::new(
        project(),
        ControlId::new("gate.mobilization").unwrap(),
        ProjectPhase::Mobilization,
        RoleId::new("project-chief").unwrap(),
    )
    .with_requirement(GateRequirement::new(
        ControlId::new("requirement.site-access").unwrap(),
    ))
    .with_requirement(GateRequirement::new(
        ControlId::new("requirement.risk").unwrap(),
    ))
    .with_evidence(evidence_ref("gate"))
}

fn project_fact(seq: u64, subject: &str, body: &str) -> crate::ProjectFact {
    crate::ProjectFact::new(
        seq,
        project(),
        ControlId::new(subject).unwrap(),
        Symbol::qualified("construction", "control"),
        accepted_on(),
        writer(),
        Expr::String(body.to_owned()),
    )
    .with_evidence(evidence_ref(subject))
}

fn project() -> ProjectId {
    ProjectId::new("reference-center").unwrap()
}

fn writer() -> RoleId {
    RoleId::new("project-chief").unwrap()
}

fn evidence_ref(id: &str) -> ExternalRef {
    ExternalRef::new(
        "doc/synthetic",
        format!("control/reference-center/{id}"),
        Some("rev-a".to_owned()),
        None,
    )
}

fn accepted_on() -> Date {
    Date::from_calendar_date(2026, Month::July, 23).unwrap()
}
