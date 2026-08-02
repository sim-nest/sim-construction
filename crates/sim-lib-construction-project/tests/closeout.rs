// conformance: construction closeout keeps typed obligations and accountable closure

use sim_kernel::{Expr, Symbol};
use sim_ledger::Amount;
use sim_lib_construction_project::{
    CloseoutControlSet, CloseoutDecision, CloseoutObligation, CloseoutObligationKind,
    CommercialEvidenceSource, ConstructionProjectError, ControlId, CurrencyCode, EvidenceState,
    FinalEconomyAmountFact, FinalEconomyBasis, FinalEconomyControl, FinalEconomyFactKind,
    FinalEconomyReconciliation, ProjectBook, ProjectFact, ProjectId, ProjectObligation,
    Requirement, RequirementLane, RoleId,
};
use sim_lib_doc_core::ExternalRef;
use time::{Date, Month};

#[test]
fn unresolved_obligation_blocks_accountable_closure() {
    let controls = closeout_controls();
    let mut book = closeout_book();
    book.append(
        evidence_fact(3, "closeout.unresolved-work").with_evidence_state(EvidenceState::Rejected),
    )
    .unwrap();
    append_fact(&mut book, 4, "closeout.evidence-disposition");
    append_fact(&mut book, 5, "closeout.lesson");
    let economy = ready_economy().derive().unwrap();

    let report = controls.report(&book, &economy, 5, date(30)).unwrap();

    assert!(!report.ready);
    assert_eq!(
        report
            .items
            .iter()
            .find(|item| item.kind == CloseoutObligationKind::UnresolvedWork)
            .unwrap()
            .evidence_state,
        EvidenceState::Rejected
    );
    let result = CloseoutDecision::new(id("closeout.final"), 5, 6, role("project-director"))
        .with_evidence(ext("closeout/decision"))
        .close(&controls, &report);
    assert!(matches!(
        result,
        Err(ConstructionProjectError::GateReportNotReady { .. })
    ));
}

#[test]
fn ready_obligations_create_immutable_accountable_closeout() {
    let controls = closeout_controls();
    let mut book = closeout_book();
    append_fact(&mut book, 3, "closeout.unresolved-work");
    append_fact(&mut book, 4, "closeout.evidence-disposition");
    append_fact(&mut book, 5, "closeout.lesson");
    let economy = ready_economy().derive().unwrap();
    let report = controls.report(&book, &economy, 5, date(30)).unwrap();

    let closure = CloseoutDecision::new(id("closeout.final"), 5, 6, role("project-director"))
        .with_evidence(ext("closeout/decision"))
        .close(&controls, &report)
        .unwrap();

    assert!(report.ready);
    assert_eq!(closure.project(), &project());
    assert_eq!(closure.report_seq(), 5);
    assert_eq!(closure.decision_seq(), 6);
    assert_eq!(closure.decided_by(), &role("project-director"));
}

fn closeout_controls() -> CloseoutControlSet {
    [
        (
            CloseoutObligationKind::WarrantyContactHandoff,
            "closeout.warranty-contact",
        ),
        (
            CloseoutObligationKind::RetentionPolicy,
            "closeout.retention-policy",
        ),
        (
            CloseoutObligationKind::UnresolvedWork,
            "closeout.unresolved-work",
        ),
        (
            CloseoutObligationKind::EvidenceDisposition,
            "closeout.evidence-disposition",
        ),
        (CloseoutObligationKind::Lesson, "closeout.lesson"),
    ]
    .into_iter()
    .fold(
        CloseoutControlSet::new(project(), id("closeout.final"), role("project-director")),
        |controls, (kind, requirement)| {
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

fn closeout_book() -> ProjectBook {
    let mut book = ProjectBook::new(project(), role("project-chief"));
    append_fact(&mut book, 1, "closeout.warranty-contact");
    append_fact(&mut book, 2, "closeout.retention-policy");
    book
}

fn append_fact(book: &mut ProjectBook, sequence: u64, subject: &str) {
    book.append(evidence_fact(sequence, subject)).unwrap();
}

fn evidence_fact(sequence: u64, subject: &str) -> ProjectFact {
    ProjectFact::new(
        sequence,
        project(),
        id(subject),
        Symbol::qualified("construction", "closeout-evidence"),
        date(u8::try_from(sequence + 20).unwrap()),
        role("project-chief"),
        Expr::String(format!("{subject} accepted")),
    )
    .with_evidence(ext(&format!("evidence/{subject}")))
}

fn ready_economy() -> FinalEconomyControl {
    let basis = FinalEconomyBasis::new(5, date(30), "accepted closeout cutoff");
    let reconciliation = FinalEconomyReconciliation::new(
        id("economy.final"),
        id("economy.ledger"),
        "final position and ledger evidence share the cutoff",
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
