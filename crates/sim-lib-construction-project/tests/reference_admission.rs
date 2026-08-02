// conformance: evidence-backed reference claims require current facts, clearance, and authority

use sim_kernel::{Expr, Symbol};
use sim_ledger::Amount;
use sim_lib_construction_project::{
    AccountableCloseout, CloseoutControlSet, CloseoutDecision, CloseoutObligation,
    CloseoutObligationKind, CommercialEvidenceSource, ControlId, CurrencyCode, DisclosureClearance,
    DisclosureCondition, EvidenceState, FinalEconomyAmountFact, FinalEconomyBasis,
    FinalEconomyControl, FinalEconomyFactKind, FinalEconomyReconciliation, OutcomeControlReport,
    OutcomeTargetReport, OutcomeVariance, ProjectBook, ProjectFact, ProjectId, ProjectObligation,
    ReferenceAdmissionBlocker, ReferenceApproval, ReferenceClaim, ReferenceClaimKind,
    ReferenceDecisionKind, ReferencePackAdmission, Requirement, RequirementLane, RoleId,
    Visibility,
};
use sim_lib_doc_core::ExternalRef;
use time::{Date, Month};

#[test]
fn rejected_lesson_evidence_is_not_admitted() {
    let mut fixture = fixture();
    fixture
        .book
        .append(source_fact(7, "charter.lesson"))
        .unwrap();
    fixture
        .book
        .append(source_fact(8, "lesson.evidence").with_evidence_state(EvidenceState::Rejected))
        .unwrap();
    let claim = claim(
        "claim.lesson",
        ReferenceClaimKind::Lesson,
        7,
        8,
        Visibility::ReferenceCandidate,
    );

    let report = admission(claim, 8)
        .evaluate(&fixture.book, &fixture.closeout, &[])
        .unwrap();

    assert!(report.manifest.is_none());
    assert!(report.claims[0].blockers.iter().any(|blocker| matches!(
        blocker,
        ReferenceAdmissionBlocker::SourceFactNotAccepted {
            sequence: 8,
            state: EvidenceState::Rejected
        }
    )));
}

#[test]
fn outcome_shortfall_blocks_people_or_place_achievement_claim() {
    let fixture = fixture_with_sources(Visibility::ReferenceCandidate);
    let claim = claim(
        "claim.people",
        ReferenceClaimKind::PeopleDevelopment,
        7,
        8,
        Visibility::ReferenceCandidate,
    )
    .asserts_outcome(id("outcome.people"));
    let shortfall = outcome_report(
        id("outcome.people"),
        OutcomeVariance::ReportedDifferent {
            target: quantity("10"),
            current: quantity("8"),
        },
    );

    let report = admission(claim, 8)
        .evaluate(&fixture.book, &fixture.closeout, &[shortfall])
        .unwrap();

    assert!(report.manifest.is_none());
    assert!(
        report.claims[0]
            .blockers
            .contains(&ReferenceAdmissionBlocker::OutcomeShortfall(id(
                "outcome.people"
            )))
    );
}

#[test]
fn confidential_source_requires_named_clearance() {
    let fixture = fixture_with_sources(Visibility::Restricted(Symbol::qualified(
        "construction",
        "confidential",
    )));
    let claim = claim(
        "claim.property",
        ReferenceClaimKind::PropertyOutcome,
        7,
        8,
        Visibility::ReferenceCandidate,
    );

    let report = admission(claim, 8)
        .evaluate(&fixture.book, &fixture.closeout, &[])
        .unwrap();

    assert!(report.manifest.is_none());
    assert!(
        report.claims[0]
            .blockers
            .contains(&ReferenceAdmissionBlocker::ConfidentialityUnsatisfied)
    );
}

#[test]
fn withdrawn_consent_blocks_people_claim() {
    let fixture = fixture_with_sources(Visibility::ReferenceCandidate);
    let claim = claim(
        "claim.people",
        ReferenceClaimKind::PeopleDevelopment,
        7,
        8,
        Visibility::ReferenceCandidate,
    )
    .requires_consent();
    let clearance = DisclosureClearance::new(id("claim.people"))
        .with_consent(DisclosureCondition::Withdrawn(ext("consent/withdrawn")));

    let report = admission(claim, 8)
        .with_clearance(clearance)
        .evaluate(&fixture.book, &fixture.closeout, &[])
        .unwrap();

    assert!(report.manifest.is_none());
    assert!(
        report.claims[0]
            .blockers
            .contains(&ReferenceAdmissionBlocker::ConsentWithdrawn)
    );
}

#[test]
fn misleading_superseded_claim_cannot_use_stale_fact_sequence() {
    let mut fixture = fixture_with_sources(Visibility::ReferenceCandidate);
    fixture
        .book
        .append(source_fact(9, "outcome.synthetic").supersedes(8))
        .unwrap();
    let claim = claim(
        "claim.lesson",
        ReferenceClaimKind::Lesson,
        7,
        8,
        Visibility::ReferenceCandidate,
    );

    let report = admission(claim, 9)
        .evaluate(&fixture.book, &fixture.closeout, &[])
        .unwrap();

    assert!(report.manifest.is_none());
    assert!(
        report.claims[0]
            .blockers
            .contains(&ReferenceAdmissionBlocker::SourceFactNotCurrent {
                sequence: 8,
                current: Some(9),
            })
    );
}

#[test]
fn admitted_synthetic_people_property_and_city_claims_create_only_an_immutable_manifest() {
    let mut fixture = fixture_with_sources(Visibility::ReferenceCandidate);
    fixture
        .book
        .append(source_fact(9, "charter.property"))
        .unwrap();
    fixture
        .book
        .append(source_fact(10, "outcome.property"))
        .unwrap();
    fixture
        .book
        .append(
            source_fact(11, "charter.city").with_visibility(Visibility::Restricted(
                Symbol::qualified("construction", "customer-confidential"),
            )),
        )
        .unwrap();
    fixture
        .book
        .append(source_fact(12, "outcome.city"))
        .unwrap();

    let people = claim(
        "claim.people",
        ReferenceClaimKind::PeopleDevelopment,
        7,
        8,
        Visibility::ReferenceCandidate,
    )
    .requires_consent()
    .asserts_outcome(id("outcome.people"));
    let property = claim(
        "claim.property",
        ReferenceClaimKind::PropertyOutcome,
        9,
        10,
        Visibility::ReferenceCandidate,
    )
    .asserts_outcome(id("outcome.property"));
    let city = claim(
        "claim.city",
        ReferenceClaimKind::CityDistrictOutcome,
        11,
        12,
        Visibility::ReferenceCandidate,
    )
    .requires_confidentiality_clearance()
    .asserts_outcome(id("outcome.city"));
    let pack = ReferencePackAdmission::new(project(), 12, date(30), role("reference-authority"))
        .with_claim(people)
        .with_claim(property)
        .with_claim(city)
        .with_clearance(
            DisclosureClearance::new(id("claim.people"))
                .with_consent(DisclosureCondition::Satisfied(ext("consent/current"))),
        )
        .with_clearance(
            DisclosureClearance::new(id("claim.city")).with_confidentiality(
                DisclosureCondition::Satisfied(ext("confidentiality/approved")),
            ),
        )
        .with_approval(approval("claim.people", 12, 13))
        .with_approval(approval("claim.property", 12, 14))
        .with_approval(approval("claim.city", 12, 15));
    let outcomes = [
        outcome_report(id("outcome.people"), OutcomeVariance::OnTarget),
        outcome_report(id("outcome.property"), OutcomeVariance::OnTarget),
        outcome_report(id("outcome.city"), OutcomeVariance::OnTarget),
    ];

    let report = pack
        .evaluate(&fixture.book, &fixture.closeout, &outcomes)
        .unwrap();
    let manifest = report.manifest.unwrap();

    assert!(report.claims.iter().all(|claim| claim.admitted));
    assert_eq!(manifest.project(), &project());
    assert_eq!(manifest.closeout_decision().as_str(), "closeout.final");
    assert_eq!(manifest.claims().len(), 3);
    assert_eq!(manifest.claims()[0].claim_id().as_str(), "claim.city");
    assert_eq!(manifest.claims()[0].source_fact_sequences(), &[11, 12]);
    assert_eq!(
        manifest.claims()[0].approving_decision().as_str(),
        "decision.claim.city"
    );
    assert_eq!(manifest.claims()[0].external_refs().len(), 1);
}

struct Fixture {
    book: ProjectBook,
    closeout: AccountableCloseout,
}

fn fixture_with_sources(visibility: Visibility) -> Fixture {
    let mut fixture = fixture();
    fixture
        .book
        .append(source_fact(7, "charter.synthetic"))
        .unwrap();
    fixture
        .book
        .append(source_fact(8, "outcome.synthetic").with_visibility(visibility))
        .unwrap();
    fixture
}

fn fixture() -> Fixture {
    let controls = closeout_controls();
    let mut book = ProjectBook::new(project(), role("project-chief"));
    for (index, subject) in [
        "closeout.warranty",
        "closeout.retention",
        "closeout.unresolved",
        "closeout.evidence",
        "closeout.lesson",
    ]
    .into_iter()
    .enumerate()
    {
        book.append(source_fact(u64::try_from(index + 1).unwrap(), subject))
            .unwrap();
    }
    let economy = ready_economy().derive().unwrap();
    let report = controls.report(&book, &economy, 5, date(30)).unwrap();
    let closeout = CloseoutDecision::new(id("closeout.final"), 5, 6, role("project-director"))
        .with_evidence(ext("closeout/decision"))
        .close(&controls, &report)
        .unwrap();
    Fixture { book, closeout }
}

fn closeout_controls() -> CloseoutControlSet {
    [
        CloseoutObligationKind::WarrantyContactHandoff,
        CloseoutObligationKind::RetentionPolicy,
        CloseoutObligationKind::UnresolvedWork,
        CloseoutObligationKind::EvidenceDisposition,
        CloseoutObligationKind::Lesson,
    ]
    .into_iter()
    .enumerate()
    .fold(
        CloseoutControlSet::new(project(), id("closeout.final"), role("project-director")),
        |controls, (index, kind)| {
            let requirement = [
                "closeout.warranty",
                "closeout.retention",
                "closeout.unresolved",
                "closeout.evidence",
                "closeout.lesson",
            ][index];
            controls.with_obligation(CloseoutObligation::new(
                kind,
                ProjectObligation::mandatory(
                    project(),
                    Requirement::new(
                        id(requirement),
                        RequirementLane::new(Symbol::qualified("construction", "closeout")),
                        format!("{kind:?}"),
                        role("project-chief"),
                        role("project-director"),
                    )
                    .with_evidence_kind(Symbol::qualified("construction", "closeout-evidence"))
                    .with_source_ref(ext(&format!("policy/{requirement}"))),
                ),
            ))
        },
    )
}

fn ready_economy() -> FinalEconomyControl {
    let basis = FinalEconomyBasis::new(5, date(30), "accepted closeout cutoff");
    let reconciliation = FinalEconomyReconciliation::new(
        id("economy.final"),
        id("economy.ledger"),
        "final position matches ledger evidence",
    );
    [
        (
            "economy.contract",
            FinalEconomyFactKind::AcceptedContract,
            CommercialEvidenceSource::Document,
        ),
        (
            "economy.forecast",
            FinalEconomyFactKind::CurrentForecast,
            CommercialEvidenceSource::Document,
        ),
        (
            "economy.final",
            FinalEconomyFactKind::FinalPosition,
            CommercialEvidenceSource::Document,
        ),
        (
            "economy.ledger",
            FinalEconomyFactKind::LedgerBalance,
            CommercialEvidenceSource::LedgerBalance,
        ),
    ]
    .into_iter()
    .enumerate()
    .fold(
        FinalEconomyControl::new(project(), currency(), basis, reconciliation),
        |control, (index, (fact_id, kind, source))| {
            control.with_fact(
                FinalEconomyAmountFact::new(
                    project(),
                    id(fact_id),
                    kind,
                    Amount::parse("1000000.00").unwrap(),
                    currency(),
                    date(25),
                    u64::try_from(index + 1).unwrap(),
                    source,
                    ext(fact_id),
                )
                .with_evidence_state(EvidenceState::Accepted),
            )
        },
    )
}

fn source_fact(sequence: u64, subject: &str) -> ProjectFact {
    ProjectFact::new(
        sequence,
        project(),
        id(subject),
        Symbol::qualified("construction", "reference-evidence"),
        date(25),
        role("project-chief"),
        Expr::String(format!("accepted aggregate fact {subject}")),
    )
    .with_evidence(ext(&format!("evidence/{subject}")))
}

fn claim(
    claim_id: &str,
    kind: ReferenceClaimKind,
    charter_seq: u64,
    outcome_seq: u64,
    visibility: Visibility,
) -> ReferenceClaim {
    ReferenceClaim::new(
        project(),
        id(claim_id),
        kind,
        format!("synthetic aggregate {kind:?} claim"),
        charter_seq,
        visibility,
    )
    .with_source_fact(outcome_seq)
    .with_external_ref(ext(&format!("reference/{claim_id}")))
}

fn admission(claim: ReferenceClaim, as_of_seq: u64) -> ReferencePackAdmission {
    let claim_id = claim.id.clone();
    ReferencePackAdmission::new(project(), as_of_seq, date(30), role("reference-authority"))
        .with_claim(claim)
        .with_approval(
            ReferenceApproval::new(
                claim_id.clone(),
                id(&format!("decision.{claim_id}")),
                as_of_seq,
                as_of_seq + 1,
                ReferenceDecisionKind::Approve,
                role("reference-authority"),
            )
            .with_evidence(ext("approval/reference")),
        )
}

fn approval(claim_id: &str, report_seq: u64, decision_seq: u64) -> ReferenceApproval {
    ReferenceApproval::new(
        id(claim_id),
        id(&format!("decision.{claim_id}")),
        report_seq,
        decision_seq,
        ReferenceDecisionKind::Approve,
        role("reference-authority"),
    )
    .with_evidence(ext(&format!("approval/{claim_id}")))
}

fn outcome_report(target: ControlId, variance: OutcomeVariance) -> OutcomeControlReport {
    OutcomeControlReport {
        project: project(),
        as_of: date(30),
        targets: vec![OutcomeTargetReport {
            target,
            current_record: Some(id("outcome.record")),
            forecasts: Vec::new(),
            covered: true,
            variance,
            blockers: Vec::new(),
            reference_claim_admissible: true,
        }],
        gates_clear: true,
    }
}

fn quantity(value: &str) -> sim_lib_construction_project::DomainQuantity {
    sim_lib_construction_project::DomainQuantity::new(value, Symbol::qualified("unit", "aggregate"))
}

fn project() -> ProjectId {
    ProjectId::new("reference-center").unwrap()
}

fn id(value: &str) -> ControlId {
    ControlId::new(value).unwrap()
}

fn role(value: &str) -> RoleId {
    RoleId::new(value).unwrap()
}

fn currency() -> CurrencyCode {
    CurrencyCode::new("SEK").unwrap()
}

fn ext(value: &str) -> ExternalRef {
    ExternalRef::new("doc/synthetic", value, Some("rev-a".to_owned()), None)
}

fn date(day: u8) -> Date {
    Date::from_calendar_date(2026, Month::July, day).unwrap()
}
