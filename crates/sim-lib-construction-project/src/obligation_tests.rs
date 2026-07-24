// conformance: evidence-aware construction requirements, obligations, and exceptions

use crate::{
    CONSTRUCTION_EXCEPTION_CAPABILITY, ConstructionProjectError, ControlId, EvidenceState,
    EvidenceValidity, ExceptionDecision, ExceptionScope, GatePolicy, ProjectBook, ProjectFact,
    ProjectId, ProjectObligation, Requirement, RequirementLane, RoleId,
};
use sim_kernel::{Expr, Symbol};
use sim_lib_doc_core::ExternalRef;
use time::{Date, Month};

#[test]
fn missing_reported_rejected_expired_and_conflicted_evidence_block_mandatory_obligations() {
    let mut book = ProjectBook::new(project(), writer());
    book.append(fact(1, "requirement.reported").with_evidence_state(EvidenceState::Reported))
        .unwrap();
    book.append(
        fact(2, "requirement.rejected")
            .with_evidence(evidence_ref("rejected"))
            .with_evidence_state(EvidenceState::Rejected),
    )
    .unwrap();
    book.append(fact(3, "requirement.expired").with_evidence(evidence_ref("expired")))
        .unwrap();
    book.append(fact(4, "requirement.conflicted").with_evidence(evidence_ref("conflict-a")))
        .unwrap();
    book.append(fact(5, "requirement.conflicted").with_evidence(evidence_ref("conflict-b")))
        .unwrap();

    let report = GatePolicy::new()
        .with_obligation(mandatory("requirement.missing", lane("customer")))
        .with_obligation(mandatory("requirement.reported", lane("quality")))
        .with_obligation(mandatory("requirement.rejected", lane("supplier")))
        .with_obligation(
            mandatory("requirement.expired", lane("environment")).with_evidence_validity(
                EvidenceValidity::new(
                    None,
                    Some(Date::from_calendar_date(2026, Month::July, 22).unwrap()),
                ),
            ),
        )
        .with_obligation(mandatory("requirement.conflicted", lane("design")))
        .evaluate(&book, 5, today())
        .unwrap();

    assert!(!report.ready);
    assert_eq!(
        report
            .explanations
            .iter()
            .map(|explanation| (
                explanation.requirement.as_str().to_owned(),
                explanation.evidence_state,
                explanation.reason.as_str()
            ))
            .collect::<Vec<_>>(),
        vec![
            (
                "requirement.conflicted".to_owned(),
                EvidenceState::Conflicted,
                "evidence is conflicted"
            ),
            (
                "requirement.expired".to_owned(),
                EvidenceState::Expired,
                "evidence is expired"
            ),
            (
                "requirement.missing".to_owned(),
                EvidenceState::Missing,
                "evidence is missing"
            ),
            (
                "requirement.rejected".to_owned(),
                EvidenceState::Rejected,
                "evidence was rejected"
            ),
            (
                "requirement.reported".to_owned(),
                EvidenceState::Reported,
                "reported without accepted evidence"
            ),
        ]
    );
}

#[test]
fn accepted_dependencies_are_required_before_dependent_obligation_is_green() {
    let mut book = ProjectBook::new(project(), writer());
    book.append(fact(1, "requirement.authority").with_evidence(evidence_ref("authority")))
        .unwrap();
    book.append(fact(2, "requirement.procurement").with_evidence(evidence_ref("procurement")))
        .unwrap();

    let report = GatePolicy::new()
        .with_obligation(mandatory("requirement.authority", lane("authority")))
        .with_obligation(
            mandatory("requirement.procurement", lane("procurement"))
                .requirement_depends_on("requirement.supplier"),
        )
        .evaluate(&book, 2, today())
        .unwrap();

    assert!(!report.ready);
    let procurement = report
        .explanations
        .iter()
        .find(|explanation| explanation.requirement.as_str() == "requirement.procurement")
        .unwrap();
    assert_eq!(procurement.rule, "dependency");
    assert_eq!(
        procurement.dependencies,
        vec![ControlId::new("requirement.supplier").unwrap()]
    );
}

#[test]
fn optional_obligations_are_explained_but_do_not_block() {
    let report = GatePolicy::new()
        .with_obligation(ProjectObligation::optional(
            project(),
            requirement("requirement.sustainability", lane("sustainability")).evidence_optional(),
        ))
        .evaluate(&ProjectBook::new(project(), writer()), 0, today())
        .unwrap();

    assert!(report.ready);
    assert_eq!(report.explanations[0].rule, "optional");
    assert_eq!(
        report.explanations[0].evidence_state,
        EvidenceState::Missing
    );
}

#[test]
fn bounded_exceptions_require_capability_authority_reason_evidence_and_current_expiry() {
    let book = ProjectBook::new(project(), writer());
    let exception = exception("exception.people", "requirement.people");

    assert!(matches!(
        GatePolicy::new()
            .with_obligation(mandatory("requirement.people", lane("people")))
            .with_exception(exception.clone())
            .evaluate(&book, 0, today()),
        Err(ConstructionProjectError::MissingCapability { .. })
    ));

    let wrong_authority = ExceptionDecision::new(
        ControlId::new("exception.people").unwrap(),
        ExceptionScope::new(project()).covers(ControlId::new("requirement.people").unwrap()),
        RoleId::new("supplier-lead").unwrap(),
        RoleId::new("project-chief").unwrap(),
        "temporary staffing constraint accepted by customer",
        today(),
        Date::from_calendar_date(2026, Month::July, 30).unwrap(),
    )
    .with_evidence(evidence_ref("exception"));
    assert!(matches!(
        GatePolicy::new()
            .with_capability(CONSTRUCTION_EXCEPTION_CAPABILITY)
            .with_obligation(mandatory("requirement.people", lane("people")))
            .with_exception(wrong_authority)
            .evaluate(&book, 0, today()),
        Err(ConstructionProjectError::ExceptionAuthorityMismatch { .. })
    ));

    let expired = ExceptionDecision::new(
        ControlId::new("exception.people").unwrap(),
        ExceptionScope::new(project()).covers(ControlId::new("requirement.people").unwrap()),
        RoleId::new("project-chief").unwrap(),
        RoleId::new("project-chief").unwrap(),
        "temporary staffing constraint accepted by customer",
        today(),
        Date::from_calendar_date(2026, Month::July, 22).unwrap(),
    )
    .with_evidence(evidence_ref("exception"));
    assert!(matches!(
        GatePolicy::new()
            .with_capability(CONSTRUCTION_EXCEPTION_CAPABILITY)
            .with_obligation(mandatory("requirement.people", lane("people")))
            .with_exception(expired)
            .evaluate(&book, 0, today()),
        Err(ConstructionProjectError::ExpiredException { .. })
    ));

    let accepted = GatePolicy::new()
        .with_capability(CONSTRUCTION_EXCEPTION_CAPABILITY)
        .with_obligation(mandatory("requirement.people", lane("people")))
        .with_exception(exception)
        .evaluate(&book, 0, today())
        .unwrap();
    assert!(accepted.ready);
    assert_eq!(
        accepted.explanations[0].exception,
        Some(ControlId::new("exception.people").unwrap())
    );
}

#[test]
fn non_waivable_safety_and_authority_requirements_reject_exceptions() {
    for (requirement_id, lane) in [
        ("requirement.safety.permit", "safety"),
        ("requirement.authority.notice", "authority"),
    ] {
        assert!(matches!(
            GatePolicy::new()
                .with_capability(CONSTRUCTION_EXCEPTION_CAPABILITY)
                .with_obligation(ProjectObligation::mandatory(
                    project(),
                    requirement(requirement_id, self::lane(lane)).non_waivable(),
                ))
                .with_exception(exception("exception.non-waivable", requirement_id))
                .evaluate(&ProjectBook::new(project(), writer()), 0, today()),
            Err(ConstructionProjectError::NonWaivableRequirement { .. })
        ));
    }
}

#[test]
fn explanations_are_stable_by_requirement_id() {
    let report = GatePolicy::new()
        .with_obligation(mandatory("requirement.production", lane("production")))
        .with_obligation(mandatory("requirement.commercial", lane("commercial")))
        .with_obligation(mandatory("requirement.handover", lane("handover")))
        .with_obligation(mandatory("requirement.place", lane("place")))
        .with_obligation(mandatory("requirement.reference", lane("reference")))
        .evaluate(&ProjectBook::new(project(), writer()), 0, today())
        .unwrap();

    assert_eq!(
        report
            .explanations
            .iter()
            .map(|explanation| explanation.requirement.as_str())
            .collect::<Vec<_>>(),
        vec![
            "requirement.commercial",
            "requirement.handover",
            "requirement.place",
            "requirement.production",
            "requirement.reference",
        ]
    );
}

trait ObligationTestExt {
    fn requirement_depends_on(self, dependency: &str) -> Self;
}

impl ObligationTestExt for ProjectObligation {
    fn requirement_depends_on(mut self, dependency: &str) -> Self {
        self.requirement = self
            .requirement
            .depends_on(ControlId::new(dependency).unwrap());
        self
    }
}

fn mandatory(requirement_id: &str, lane: RequirementLane) -> ProjectObligation {
    ProjectObligation::mandatory(project(), requirement(requirement_id, lane))
}

fn requirement(requirement_id: &str, lane: RequirementLane) -> Requirement {
    Requirement::new(
        ControlId::new(requirement_id).unwrap(),
        lane,
        requirement_id,
        RoleId::new("supplier-lead").unwrap(),
        RoleId::new("project-chief").unwrap(),
    )
    .with_evidence_kind(Symbol::qualified("construction-evidence", "external-ref"))
    .with_source_ref(evidence_ref("source"))
}

fn exception(exception_id: &str, requirement_id: &str) -> ExceptionDecision {
    ExceptionDecision::new(
        ControlId::new(exception_id).unwrap(),
        ExceptionScope::new(project()).covers(ControlId::new(requirement_id).unwrap()),
        RoleId::new("project-chief").unwrap(),
        RoleId::new("project-chief").unwrap(),
        "bounded customer decision with reviewed evidence",
        today(),
        Date::from_calendar_date(2026, Month::July, 30).unwrap(),
    )
    .with_evidence(evidence_ref("exception"))
}

fn fact(seq: u64, subject: &str) -> ProjectFact {
    ProjectFact::new(
        seq,
        project(),
        ControlId::new(subject).unwrap(),
        Symbol::qualified("construction", "obligation"),
        today(),
        writer(),
        Expr::String(subject.to_owned()),
    )
}

fn project() -> ProjectId {
    ProjectId::new("reference-center").unwrap()
}

fn writer() -> RoleId {
    RoleId::new("project-chief").unwrap()
}

fn lane(name: &str) -> RequirementLane {
    RequirementLane::new(Symbol::qualified("construction-lane", name))
}

fn evidence_ref(id: &str) -> ExternalRef {
    ExternalRef::new(
        "doc/synthetic",
        format!("obligation/reference-center/{id}"),
        Some("rev-a".to_owned()),
        None,
    )
}

fn today() -> Date {
    Date::from_calendar_date(2026, Month::July, 23).unwrap()
}
