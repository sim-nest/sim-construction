//! Complete one-chain settlement fixture shared by change conformance tests.

use crate::{
    BaselineId, ChangeControlSet, ChangeScheduleImpact, ChangeStage, ChangeStatus, CommercialSide,
    change_tests::{
        amount, change_record, control, date, document_reference, fact, ledger_reference,
    },
};

pub(crate) fn complete_chain(closure_customer: &str) -> ChangeControlSet {
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
) -> crate::ChangeFact {
    fact(id, seq, stage, status).with_amount(amount(&format!("amount.{id}"), side, value))
}
