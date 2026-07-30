// conformance: one-chain construction change commercial exposure and settlement

use sim_ledger::Amount;
use sim_lib_doc_core::ExternalRef;
use time::{Date, Month};

use crate::{
    BaselineId, ChangeAmountComponent, ChangeControlSet, ChangeDirection, ChangeFact, ChangeId,
    ChangeRecord, ChangeScheduleImpact, ChangeStage, ChangeStatus, CommercialAmount,
    CommercialEvidenceSource, CommercialSide, ConstructionProjectError, ContractualBasis,
    ControlId, CurrencyCode, EvidenceState, ProjectId, ReferencedAmount, ReferencedAmountEvidence,
    RoleId,
};

#[test]
fn instruction_without_price_and_supplier_before_customer_notice_remain_exposed() {
    let set = ChangeControlSet::new()
        .with_change(change_record())
        .with_fact(
            fact(
                "fact.supplier",
                1,
                ChangeStage::SupplierExposure,
                ChangeStatus::Assessing,
            )
            .with_amount(amount(
                "amount.supplier",
                CommercialSide::Supplier,
                "45000.00",
            )),
        );

    let report = set.derive(&currency(), date(10)).unwrap();
    assert_eq!(ids(&report.instructed_unpriced), vec!["change.cooling"]);
    assert_eq!(ids(&report.supplier_only), vec!["change.cooling"]);
    assert_eq!(ids(&report.overdue_notice), vec!["change.cooling"]);
    assert_eq!(report.changes[0].supplier_exposure.0, 4_500_000);
    assert_eq!(report.changes[0].customer_recovery.0, 0);
}

#[test]
fn partial_approval_keeps_quoted_recovery_distinct_from_approved_value() {
    let set = ChangeControlSet::new()
        .with_change(change_record().notice_given_on(date(4)))
        .with_fact(
            fact(
                "fact.supplier",
                1,
                ChangeStage::SupplierExposure,
                ChangeStatus::Assessing,
            )
            .with_amount(amount(
                "amount.supplier",
                CommercialSide::Supplier,
                "70000.00",
            )),
        )
        .with_fact(
            fact(
                "fact.quotation",
                2,
                ChangeStage::Quotation,
                ChangeStatus::Submitted,
            )
            .with_amount(amount(
                "amount.quoted",
                CommercialSide::Customer,
                "100000.00",
            )),
        )
        .with_fact(
            fact(
                "fact.authority",
                3,
                ChangeStage::AuthorityDecision,
                ChangeStatus::PartiallyApproved,
            )
            .with_amount(amount(
                "amount.approved",
                CommercialSide::Customer,
                "60000.00",
            ))
            .with_reference(document_reference("approval", Some("60000.00"))),
        );

    let report = set.derive(&currency(), date(12)).unwrap();
    let view = &report.changes[0];
    assert_eq!(ids(&report.approved), vec!["change.cooling"]);
    assert_eq!(view.customer_recovery.0, 10_000_000);
    assert_eq!(view.approved_customer.0, 6_000_000);
    assert_eq!(view.net_exposure.0, -3_000_000);
}

#[test]
fn rejected_quotation_is_not_promoted_by_accepted_document_evidence() {
    let set = ChangeControlSet::new()
        .with_change(change_record())
        .with_fact(
            fact(
                "fact.quotation",
                1,
                ChangeStage::Quotation,
                ChangeStatus::Submitted,
            )
            .with_amount(amount(
                "amount.quoted",
                CommercialSide::Customer,
                "100000.00",
            )),
        )
        .with_fact(
            fact(
                "fact.rejection",
                2,
                ChangeStage::AuthorityDecision,
                ChangeStatus::Rejected,
            )
            .with_reference(document_reference("rejection", Some("100000.00"))),
        );

    let report = set.derive(&currency(), date(12)).unwrap();
    assert!(report.approved.is_empty());
    assert!(report.submitted.is_empty());
    assert_eq!(report.changes[0].approved_customer.0, 0);
    assert_eq!(report.changes[0].status, ChangeStatus::Rejected);
}

#[test]
fn correction_replaces_the_prior_stage_fact_without_changing_change_id() {
    let set = ChangeControlSet::new()
        .with_change(change_record())
        .with_fact(
            fact(
                "fact.quotation.original",
                1,
                ChangeStage::Quotation,
                ChangeStatus::Submitted,
            )
            .with_amount(amount(
                "amount.original",
                CommercialSide::Customer,
                "100000.00",
            )),
        )
        .with_fact(
            fact(
                "fact.quotation.corrected",
                2,
                ChangeStage::Quotation,
                ChangeStatus::Submitted,
            )
            .supersedes(control("fact.quotation.original"))
            .with_amount(amount(
                "amount.corrected",
                CommercialSide::Customer,
                "80000.00",
            )),
        );

    let report = set.derive(&currency(), date(12)).unwrap();
    assert_eq!(report.changes[0].customer_recovery.0, 8_000_000);
    assert_eq!(
        report.changes[0]
            .current_facts
            .iter()
            .map(ControlId::as_str)
            .collect::<Vec<_>>(),
        vec!["fact.quotation.corrected"]
    );
}

#[test]
fn non_zero_baseline_impact_stays_in_the_open_time_risk_view() {
    let set = ChangeControlSet::new()
        .with_change(change_record())
        .with_fact(
            fact(
                "fact.time",
                1,
                ChangeStage::TimeEffect,
                ChangeStatus::Assessing,
            )
            .affects_task("task.commission")
            .with_schedule_impact(ChangeScheduleImpact::new(
                BaselineId::new("baseline.schedule").unwrap(),
                7,
                date(8),
                12,
                true,
                vec!["task.commission".to_owned()],
            )),
        );

    let report = set.derive(&currency(), date(12)).unwrap();
    assert_eq!(ids(&report.time_risk), vec!["change.cooling"]);
    assert_eq!(
        report.changes[0]
            .schedule_impact
            .as_ref()
            .unwrap()
            .completion_delta_days,
        12
    );
}

#[test]
fn settlement_mismatch_fails_closed() {
    let result = complete_chain("110000.00").derive(&currency(), date(30));
    assert!(matches!(
        result,
        Err(ConstructionProjectError::ChangeSettlementMismatch {
            side: "customer",
            ..
        })
    ));
}

#[test]
fn checked_component_and_portfolio_arithmetic_reject_overflow() {
    let set = ChangeControlSet::new()
        .with_change(change_record())
        .with_fact(
            fact(
                "fact.supplier",
                1,
                ChangeStage::SupplierExposure,
                ChangeStatus::Assessing,
            )
            .with_amount(raw_amount(
                "amount.maximum",
                CommercialSide::Supplier,
                Amount(i64::MAX),
            ))
            .with_amount(raw_amount(
                "amount.plus-one",
                CommercialSide::Supplier,
                Amount(1),
            )),
        );

    assert!(matches!(
        set.derive(&currency(), date(12)),
        Err(ConstructionProjectError::AmountOverflow {
            field: "change.supplier_exposure"
        })
    ));
}

#[test]
fn final_closure_reconciles_every_stage_without_inferred_payment() {
    let report = complete_chain("100000.00")
        .derive(&currency(), date(30))
        .unwrap();
    let view = &report.changes[0];
    let settlement = view.settlement.as_ref().unwrap();

    assert_eq!(view.status, ChangeStatus::Closed);
    assert!(!view.time_risk);
    assert!(settlement.closed);
    assert_eq!(settlement.supplier.0, 7_000_000);
    assert_eq!(settlement.customer.0, 10_000_000);
    assert_eq!(settlement.net.0, -3_000_000);
    assert!(settlement.references.iter().any(|reference| {
        reference.source == CommercialEvidenceSource::LedgerBalance
            && reference.as_of == date(27)
            && reference.reference.version.as_deref() == Some("ledger-snapshot-27")
    }));
}

#[test]
fn duplicate_change_identity_and_parent_child_double_counting_fail_closed() {
    let duplicate = ChangeControlSet::new()
        .with_change(change_record())
        .with_change(change_record());
    assert!(matches!(
        duplicate.derive(&currency(), date(12)),
        Err(ConstructionProjectError::DuplicateId { kind: "change", .. })
    ));

    let parent = amount("amount.summary", CommercialSide::Supplier, "100000.00");
    let child = amount("amount.labor", CommercialSide::Supplier, "40000.00")
        .with_parent(control("amount.summary"));
    let double_counted = ChangeControlSet::new()
        .with_change(change_record())
        .with_fact(
            fact(
                "fact.supplier",
                1,
                ChangeStage::SupplierExposure,
                ChangeStatus::Assessing,
            )
            .with_amount(parent)
            .with_amount(child),
        );
    assert!(matches!(
        double_counted.derive(&currency(), date(12)),
        Err(ConstructionProjectError::ChangeAmountDoubleCount { .. })
    ));
}

#[test]
fn external_values_require_an_as_of_version_and_do_not_grant_authority() {
    let unversioned = ReferencedAmountEvidence::new(
        CommercialEvidenceSource::Document,
        ExternalRef::new("doc/synthetic", "quotation/unversioned", None, None),
        date(9),
        EvidenceState::Accepted,
    );
    let set = ChangeControlSet::new()
        .with_change(change_record())
        .with_fact(
            fact(
                "fact.customer",
                1,
                ChangeStage::CustomerRecovery,
                ChangeStatus::Assessing,
            )
            .with_amount(amount(
                "amount.customer",
                CommercialSide::Customer,
                "50000.00",
            ))
            .with_reference(unversioned),
        );

    assert!(matches!(
        set.derive(&currency(), date(12)),
        Err(ConstructionProjectError::MissingChangeAsOfMarker { .. })
    ));
}

fn complete_chain(closure_customer: &str) -> ChangeControlSet {
    ChangeControlSet::new()
        .with_change(change_record().notice_given_on(date(4)))
        .with_fact(
            fact(
                "fact.scope",
                1,
                ChangeStage::ScopeAssessment,
                ChangeStatus::Assessing,
            )
            .affects_control(control("control.cooling"))
            .affects_task("task.commission")
            .affects_package(control("package.mechanical")),
        )
        .with_fact(
            fact(
                "fact.time",
                2,
                ChangeStage::TimeEffect,
                ChangeStatus::Assessing,
            )
            .with_schedule_impact(ChangeScheduleImpact::new(
                BaselineId::new("baseline.schedule").unwrap(),
                12,
                date(8),
                12,
                true,
                vec!["task.commission".to_owned()],
            )),
        )
        .with_fact(commercial_fact(
            "fact.supplier",
            3,
            ChangeStage::SupplierExposure,
            ChangeStatus::Assessing,
            CommercialSide::Supplier,
            "70000.00",
        ))
        .with_fact(commercial_fact(
            "fact.customer",
            4,
            ChangeStage::CustomerRecovery,
            ChangeStatus::Assessing,
            CommercialSide::Customer,
            "100000.00",
        ))
        .with_fact(commercial_fact(
            "fact.quotation",
            5,
            ChangeStage::Quotation,
            ChangeStatus::Submitted,
            CommercialSide::Customer,
            "100000.00",
        ))
        .with_fact(
            commercial_fact(
                "fact.authority",
                6,
                ChangeStage::AuthorityDecision,
                ChangeStatus::Approved,
                CommercialSide::Customer,
                "100000.00",
            )
            .with_reference(document_reference("authority", Some("100000.00"))),
        )
        .with_fact(
            fact(
                "fact.forecast",
                7,
                ChangeStage::Forecast,
                ChangeStatus::Assessing,
            )
            .with_amount(amount(
                "amount.forecast.supplier",
                CommercialSide::Supplier,
                "70000.00",
            ))
            .with_amount(amount(
                "amount.forecast.customer",
                CommercialSide::Customer,
                "100000.00",
            )),
        )
        .with_fact(fact(
            "fact.execution",
            8,
            ChangeStage::Execution,
            ChangeStatus::Executing,
        ))
        .with_fact(
            fact(
                "fact.settlement",
                9,
                ChangeStage::Settlement,
                ChangeStatus::Settled,
            )
            .with_amount(amount(
                "amount.settlement.supplier",
                CommercialSide::Supplier,
                "70000.00",
            ))
            .with_amount(amount(
                "amount.settlement.customer",
                CommercialSide::Customer,
                "100000.00",
            ))
            .with_reference(ledger_reference("settlement", Some("70000.00"))),
        )
        .with_fact(
            fact(
                "fact.closure",
                10,
                ChangeStage::Closure,
                ChangeStatus::Closed,
            )
            .with_amount(amount(
                "amount.closure.supplier",
                CommercialSide::Supplier,
                "70000.00",
            ))
            .with_amount(amount(
                "amount.closure.customer",
                CommercialSide::Customer,
                closure_customer,
            ))
            .with_reference(document_reference("closure", None)),
        )
}

fn commercial_fact(
    id: &str,
    seq: u64,
    stage: ChangeStage,
    status: ChangeStatus,
    side: CommercialSide,
    value: &str,
) -> ChangeFact {
    fact(id, seq, stage, status).with_amount(amount(&format!("amount.{id}"), side, value))
}

fn change_record() -> ChangeRecord {
    ChangeRecord::new(
        project(),
        change_id(),
        ChangeDirection::CustomerInstruction,
        ContractualBasis::new(
            "instructed-variation",
            "ABT06-2:3",
            ExternalRef::new(
                "doc/synthetic",
                "contract/reference-center",
                Some("signed-rev-a".to_owned()),
                None,
            ),
        ),
        role(),
        date(2),
        Some(date(5)),
    )
    .affects_control(control("control.cooling"))
    .affects_task("task.commission")
    .affects_package(control("package.mechanical"))
    .with_evidence(ExternalRef::new(
        "doc/synthetic",
        "instruction/change-cooling",
        Some("instruction-rev-a".to_owned()),
        None,
    ))
}

fn fact(id: &str, seq: u64, stage: ChangeStage, status: ChangeStatus) -> ChangeFact {
    ChangeFact::new(
        control(id),
        change_id(),
        seq,
        stage,
        status,
        date(u8::try_from(seq + 8).unwrap()),
        role(),
        format!("{stage:?} fact"),
    )
}

fn amount(id: &str, side: CommercialSide, value: &str) -> ChangeAmountComponent {
    ChangeAmountComponent::new(
        control(id),
        side,
        "direct",
        CommercialAmount::parse(value, currency()).unwrap(),
    )
}

fn raw_amount(id: &str, side: CommercialSide, value: Amount) -> ChangeAmountComponent {
    ChangeAmountComponent::new(
        control(id),
        side,
        "direct",
        CommercialAmount::new(value, currency()).unwrap(),
    )
}

fn document_reference(id: &str, value: Option<&str>) -> ReferencedAmountEvidence {
    let reference = ReferencedAmountEvidence::new(
        CommercialEvidenceSource::Document,
        ExternalRef::new(
            "doc/synthetic",
            format!("change/{id}"),
            Some(format!("{id}-rev-a")),
            None,
        ),
        date(20),
        EvidenceState::Accepted,
    );
    value.map_or(reference.clone(), |value| {
        reference.with_stated_value(ReferencedAmount::new(
            Amount::parse(value).unwrap(),
            currency(),
        ))
    })
}

fn ledger_reference(id: &str, value: Option<&str>) -> ReferencedAmountEvidence {
    let reference = ReferencedAmountEvidence::new(
        CommercialEvidenceSource::LedgerBalance,
        ExternalRef::new(
            "ledger/synthetic",
            format!("change/{id}"),
            Some("ledger-snapshot-27".to_owned()),
            None,
        ),
        date(27),
        EvidenceState::Evidenced,
    );
    value.map_or(reference.clone(), |value| {
        reference.with_stated_value(ReferencedAmount::new(
            Amount::parse(value).unwrap(),
            currency(),
        ))
    })
}

fn ids(values: &[ChangeId]) -> Vec<&str> {
    values.iter().map(ChangeId::as_str).collect()
}

fn project() -> ProjectId {
    ProjectId::new("reference-center").unwrap()
}

fn change_id() -> ChangeId {
    ChangeId::new("change.cooling").unwrap()
}

fn role() -> RoleId {
    RoleId::new("project-chief").unwrap()
}

fn control(value: &str) -> ControlId {
    ControlId::new(value).unwrap()
}

fn currency() -> CurrencyCode {
    CurrencyCode::new("SEK").unwrap()
}

fn date(day: u8) -> Date {
    Date::from_calendar_date(2026, Month::July, day).unwrap()
}
