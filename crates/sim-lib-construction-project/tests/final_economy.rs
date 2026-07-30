// conformance: construction final economy uses exact Amount facts and explicit reconciliation

use sim_ledger::Amount;
use sim_lib_construction_project::{
    CommercialEvidenceSource, ControlId, CurrencyCode, EvidenceState, FinalEconomyAmountFact,
    FinalEconomyBasis, FinalEconomyBlocker, FinalEconomyControl, FinalEconomyFactKind,
    FinalEconomyReconciliation, ProjectId,
};
use sim_lib_doc_core::ExternalRef;
use time::{Date, Month};

#[test]
fn unsettled_exposure_stays_visible_and_blocks_closeout() {
    let control = base_control().with_fact(amount_fact(
        "economy.claim",
        FinalEconomyFactKind::Claim,
        "125000.00",
        5,
        CommercialEvidenceSource::Document,
    ));

    let report = control.derive().unwrap();

    assert!(!report.ready);
    assert_eq!(report.unsettled_exposure, vec![id("economy.claim")]);
    assert!(
        report
            .blockers
            .contains(&FinalEconomyBlocker::UnsettledExposure(id("economy.claim")))
    );
}

#[test]
fn ledger_mismatch_fails_exact_reconciliation() {
    let mut control = base_control();
    control
        .facts
        .retain(|fact| fact.kind != FinalEconomyFactKind::LedgerBalance);
    control.facts.push(amount_fact(
        "economy.ledger",
        FinalEconomyFactKind::LedgerBalance,
        "899999.99",
        4,
        CommercialEvidenceSource::LedgerBalance,
    ));

    let report = control.derive().unwrap();

    assert!(!report.ledger_reconciled);
    assert!(report.blockers.iter().any(|blocker| matches!(
        blocker,
        FinalEconomyBlocker::LedgerMismatch { final_position, ledger_balance }
            if *final_position == amount("900000.00")
                && *ledger_balance == amount("899999.99")
    )));
}

#[test]
fn accepted_settlement_and_ledger_evidence_produce_ready_exact_totals() {
    let control = base_control()
        .with_fact(amount_fact(
            "economy.change",
            FinalEconomyFactKind::OpenChange,
            "25000.00",
            5,
            CommercialEvidenceSource::Document,
        ))
        .with_fact(
            amount_fact(
                "economy.customer-settlement",
                FinalEconomyFactKind::CustomerSettlement,
                "25000.00",
                6,
                CommercialEvidenceSource::LedgerBalance,
            )
            .settles(id("economy.change")),
        )
        .with_fact(
            amount_fact(
                "economy.guarantee",
                FinalEconomyFactKind::Guarantee,
                "5000.00",
                7,
                CommercialEvidenceSource::Document,
            )
            .supersedes(id("economy.guarantee.open")),
        )
        .with_fact(amount_fact(
            "economy.guarantee.open",
            FinalEconomyFactKind::Guarantee,
            "5000.00",
            6,
            CommercialEvidenceSource::Document,
        ))
        .with_fact(
            amount_fact(
                "economy.supplier-settlement",
                FinalEconomyFactKind::SupplierSettlement,
                "5000.00",
                8,
                CommercialEvidenceSource::LedgerBalance,
            )
            .settles(id("economy.guarantee")),
        );

    let report = control.derive().unwrap();

    assert!(report.ready);
    assert!(report.ledger_reconciled);
    assert!(report.unsettled_exposure.is_empty());
    assert_eq!(
        report.total(FinalEconomyFactKind::AcceptedContract),
        Some(amount("1000000.00"))
    );
    assert_eq!(
        report.total(FinalEconomyFactKind::CustomerSettlement),
        Some(amount("25000.00"))
    );
}

fn base_control() -> FinalEconomyControl {
    FinalEconomyControl::new(
        project(),
        currency(),
        FinalEconomyBasis::new(
            20,
            date(30),
            "accepted project facts through sequence 20 at the July cutoff",
        ),
        FinalEconomyReconciliation::new(
            id("economy.final-position"),
            id("economy.ledger"),
            "final position matches the versioned ledger balance at one cutoff",
        ),
    )
    .with_fact(amount_fact(
        "economy.contract",
        FinalEconomyFactKind::AcceptedContract,
        "1000000.00",
        1,
        CommercialEvidenceSource::Document,
    ))
    .with_fact(amount_fact(
        "economy.forecast",
        FinalEconomyFactKind::CurrentForecast,
        "900000.00",
        2,
        CommercialEvidenceSource::Document,
    ))
    .with_fact(amount_fact(
        "economy.final-position",
        FinalEconomyFactKind::FinalPosition,
        "900000.00",
        3,
        CommercialEvidenceSource::Document,
    ))
    .with_fact(amount_fact(
        "economy.ledger",
        FinalEconomyFactKind::LedgerBalance,
        "900000.00",
        4,
        CommercialEvidenceSource::LedgerBalance,
    ))
}

fn amount_fact(
    id_text: &str,
    kind: FinalEconomyFactKind,
    value: &str,
    sequence: u64,
    source: CommercialEvidenceSource,
) -> FinalEconomyAmountFact {
    FinalEconomyAmountFact::new(
        project(),
        id(id_text),
        kind,
        amount(value),
        currency(),
        date(u8::try_from(sequence + 5).unwrap()),
        sequence,
        source,
        ExternalRef::new(
            match source {
                CommercialEvidenceSource::Document => "doc/synthetic",
                CommercialEvidenceSource::LedgerBalance => "ledger/synthetic",
            },
            id_text,
            Some(format!("snapshot-{sequence}")),
            None,
        ),
    )
    .with_evidence_state(EvidenceState::Accepted)
}

fn project() -> ProjectId {
    ProjectId::new("reference-center").unwrap()
}

fn id(value: &str) -> ControlId {
    ControlId::new(value).unwrap()
}

fn currency() -> CurrencyCode {
    CurrencyCode::new("SEK").unwrap()
}

fn amount(value: &str) -> Amount {
    Amount::parse(value).unwrap()
}

fn date(day: u8) -> Date {
    Date::from_calendar_date(2026, Month::July, day).unwrap()
}
