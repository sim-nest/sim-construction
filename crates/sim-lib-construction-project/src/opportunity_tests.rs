// conformance: customer intent, bid decisions, and collaboration readiness

use crate::{
    BidDecision, BidDecisionKind, CollaborationCharter, ConstructionProjectError,
    ConstructionVariant, ControlId, CustomerIntent, CustomerIntentAcceptance, EvidenceState,
    IntentField, OpportunityRecord, OpportunitySource, ProjectBook, ProjectFact, ProjectId,
    ProjectObligation, Requirement, RequirementLane, RoleId,
};
use sim_kernel::{Expr, Symbol};
use sim_lib_doc_core::ExternalRef;
use time::{Date, Month};

#[test]
fn bid_no_bid_and_conditional_bid_validate_accountable_basis() {
    let no_bid = bid(BidDecisionKind::NoBid)
        .with_capacity_view("no design manager available")
        .with_risk("customer target budget is below market")
        .with_evidence(evidence_ref("bid/no-bid"));
    no_bid.validate().unwrap();

    let conditional = bid(BidDecisionKind::ConditionalBid)
        .with_capacity_view("team available after permit package")
        .with_risk("tenant phasing remains open")
        .with_opportunity("repeat customer reference value")
        .with_assumption("night work is excluded")
        .valid_until(Date::from_calendar_date(2026, Month::August, 15).unwrap())
        .with_price_basis(evidence_ref("basis/price"))
        .with_schedule_basis(evidence_ref("basis/schedule"))
        .with_evidence(evidence_ref("bid/conditional"));
    conditional.validate().unwrap();

    let missing_basis = bid(BidDecisionKind::Bid)
        .with_capacity_view("team assigned")
        .with_risk("long-lead switchgear")
        .valid_until(Date::from_calendar_date(2026, Month::August, 15).unwrap())
        .with_evidence(evidence_ref("bid/bid"));
    assert_eq!(
        missing_basis.validate(),
        Err(ConstructionProjectError::EmptyCollection("bid.price_basis"))
    );
}

#[test]
fn expired_offer_basis_is_reported_without_rewriting_decision() {
    let decision = bid(BidDecisionKind::Bid)
        .with_capacity_view("team assigned")
        .with_risk("long-lead switchgear")
        .valid_until(Date::from_calendar_date(2026, Month::July, 20).unwrap())
        .with_price_basis(evidence_ref("basis/price"))
        .with_schedule_basis(evidence_ref("basis/schedule"))
        .with_evidence(evidence_ref("bid/bid"));

    let report = decision
        .offer_basis_report(Date::from_calendar_date(2026, Month::July, 23).unwrap())
        .unwrap();

    assert!(report.expired);
    assert!(report.has_price_basis);
    assert!(report.has_schedule_basis);
}

#[test]
fn conflicting_intent_blocks_coverage_through_requirement_graph() {
    let mut book = ProjectBook::new(project(), writer());
    book.append(fact(1, "requirement.intent.scope").with_evidence(evidence_ref("scope-a")))
        .unwrap();
    book.append(fact(2, "requirement.intent.scope").with_evidence(evidence_ref("scope-b")))
        .unwrap();

    let report = complete_intent()
        .coverage_report(
            [mandatory("requirement.intent.scope", "customer")],
            &book,
            2,
            today(),
        )
        .unwrap();

    assert!(!report.ready);
    assert_eq!(
        report.requirement_report.explanations[0].evidence_state,
        EvidenceState::Conflicted
    );
    assert!(report.unknown_fields.is_empty());
}

#[test]
fn unknown_customer_assumption_does_not_become_accepted_requirement() {
    let book = ProjectBook::new(project(), writer());
    let mut intent = complete_intent().with_assumption("customer may keep lobby occupied");
    intent.tenant_constraints = IntentField::unknown("customer has not stated tenant phasing");

    let report = intent
        .coverage_report(
            [mandatory("requirement.intent.tenant-phasing", "customer")],
            &book,
            0,
            today(),
        )
        .unwrap();

    assert!(!report.ready);
    assert_eq!(report.unknown_fields, vec!["tenant_constraints"]);
    assert_eq!(
        report.requirement_report.explanations[0].evidence_state,
        EvidenceState::Missing
    );
}

#[test]
fn late_customer_decision_is_explicit_and_still_authority_checked() {
    let decision = CustomerIntentAcceptance::new(
        ControlId::new("intent.reference-center").unwrap(),
        RoleId::new("project-chief").unwrap(),
        RoleId::new("project-chief").unwrap(),
        Date::from_calendar_date(2026, Month::July, 20).unwrap(),
        Date::from_calendar_date(2026, Month::July, 23).unwrap(),
    )
    .with_evidence(evidence_ref("intent/acceptance"));

    decision.validate().unwrap();
    assert!(decision.is_late());
}

#[test]
fn missing_main_contract_evidence_blocks_collaboration_readiness() {
    let mut book = ProjectBook::new(project(), writer());
    book.append(fact(1, "requirement.intent.scope").with_evidence(evidence_ref("scope")))
        .unwrap();

    let charter = collaboration_charter()
        .with_main_contract_evidence(ControlId::new("evidence.main-contract.form").unwrap())
        .with_obligation(mandatory("requirement.intent.scope", "customer"));

    let report = charter.readiness_report(&book, 1, today()).unwrap();

    assert!(!report.ready);
    assert_eq!(
        report.missing_main_contract_evidence,
        vec![ControlId::new("evidence.main-contract.form").unwrap()]
    );
    assert!(report.requirement_report.ready);
}

#[test]
fn synthetic_renovation_and_new_build_intents_share_the_same_control_surface() {
    let renovation = complete_intent();
    let mut new_build = complete_intent();
    new_build.control = ControlId::new("intent.new-build").unwrap();
    new_build.variant = IntentField::known(ConstructionVariant::NewBuild, evidence_ref("variant"));
    new_build.property_constraints = IntentField::known(
        vec!["greenfield plot boundary".to_owned()],
        evidence_ref("plot"),
    );

    renovation.validate().unwrap();
    new_build.validate().unwrap();
    assert_eq!(
        renovation.variant.value,
        Some(ConstructionVariant::Renovation)
    );
    assert_eq!(new_build.variant.value, Some(ConstructionVariant::NewBuild));
}

#[test]
fn opportunity_record_is_reference_only_project_control() {
    let opportunity = OpportunityRecord::new(
        project(),
        ControlId::new("opportunity.reference-center").unwrap(),
        OpportunitySource::Customer,
        RoleId::new("project-chief").unwrap(),
        "Reference center tenant improvement",
    )
    .with_evidence(evidence_ref("opportunity"));

    opportunity.validate().unwrap();
}

fn complete_intent() -> CustomerIntent {
    let mut intent = CustomerIntent::new(
        project(),
        ControlId::new("intent.reference-center").unwrap(),
    )
    .with_assumption("work hours are customer-confirmed")
    .with_exclusion("loose furniture")
    .with_evidence(evidence_ref("intent"));
    intent.intended_use =
        IntentField::known("tenant reference center".to_owned(), evidence_ref("use"));
    intent.scope_boundary = IntentField::known(
        "existing shell interior fit-out".to_owned(),
        evidence_ref("scope"),
    );
    intent.property_constraints = IntentField::known(
        vec!["existing facade is fixed".to_owned()],
        evidence_ref("property"),
    );
    intent.tenant_constraints = IntentField::known(
        vec!["occupied upper floors".to_owned()],
        evidence_ref("tenant"),
    );
    intent.success_measures = IntentField::known(
        vec!["handover accepted before opening".to_owned()],
        evidence_ref("success"),
    );
    intent.target_outcomes = IntentField::known(
        vec!["public reference candidate".to_owned()],
        evidence_ref("outcomes"),
    );
    intent.delivery_form = IntentField::known("collaboration".to_owned(), evidence_ref("delivery"));
    intent.procurement_form = IntentField::known(
        "two-stage negotiated".to_owned(),
        evidence_ref("procurement"),
    );
    intent.time_frame = IntentField::known("Q3 2026".to_owned(), evidence_ref("time"));
    intent.commercial_frame = IntentField::known(
        "target price with open book".to_owned(),
        evidence_ref("commercial"),
    );
    intent.variant = IntentField::known(ConstructionVariant::Renovation, evidence_ref("variant"));
    intent
}

fn collaboration_charter() -> CollaborationCharter {
    CollaborationCharter::new(
        project(),
        ControlId::new("collaboration.reference-center").unwrap(),
        "weekly",
    )
    .with_objective("turn stated customer intent into gate evidence")
    .with_working_principle("no unstated customer assumption is accepted")
    .with_organization("customer and supplier core team")
    .with_decision_right("project-chief accepts main-contract evidence")
    .with_investigation("tenant access survey")
    .with_target_design_buildability_work("phased logistics plan")
    .with_open_book_rule("shared cost log before target price")
    .with_escalation_role(RoleId::new("project-chief").unwrap())
    .with_evidence(evidence_ref("collaboration"))
}

fn bid(decision: BidDecisionKind) -> BidDecision {
    BidDecision::new(
        project(),
        ControlId::new("bid.reference-center").unwrap(),
        ControlId::new("intent.reference-center").unwrap(),
        RoleId::new("project-chief").unwrap(),
        RoleId::new("project-chief").unwrap(),
        decision,
    )
}

fn mandatory(requirement_id: &str, lane: &str) -> ProjectObligation {
    ProjectObligation::mandatory(
        project(),
        Requirement::new(
            ControlId::new(requirement_id).unwrap(),
            RequirementLane::new(Symbol::qualified("construction-lane", lane)),
            requirement_id,
            RoleId::new("supplier-lead").unwrap(),
            RoleId::new("project-chief").unwrap(),
        )
        .with_evidence_kind(Symbol::qualified("construction-evidence", "external-ref"))
        .with_source_ref(evidence_ref("requirement-source")),
    )
}

fn fact(seq: u64, subject: &str) -> ProjectFact {
    ProjectFact::new(
        seq,
        project(),
        ControlId::new(subject).unwrap(),
        Symbol::qualified("construction", "customer-intent"),
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

fn evidence_ref(id: &str) -> ExternalRef {
    ExternalRef::new(
        "doc/synthetic",
        format!("opportunity/reference-center/{id}"),
        Some("rev-a".to_owned()),
        None,
    )
}

fn today() -> Date {
    Date::from_calendar_date(2026, Month::July, 23).unwrap()
}
