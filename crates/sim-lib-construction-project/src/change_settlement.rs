//! Final settlement and closure reconciliation for construction changes.

use std::collections::BTreeMap;

use sim_ledger::Amount;
use time::Date;

use crate::{
    ChangeFact, ChangeId, ChangeRecord, ChangeStage, CommercialSide, ConstructionProjectError,
    ReferencedAmountEvidence, Result, commercial::checked_total,
};

const CLOSEOUT_STAGES: [ChangeStage; 10] = [
    ChangeStage::ScopeAssessment,
    ChangeStage::TimeEffect,
    ChangeStage::SupplierExposure,
    ChangeStage::CustomerRecovery,
    ChangeStage::Quotation,
    ChangeStage::AuthorityDecision,
    ChangeStage::Forecast,
    ChangeStage::Execution,
    ChangeStage::Settlement,
    ChangeStage::Closure,
];

/// Final supplier/customer settlement values with reference-only evidence.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ChangeSettlementView {
    /// Stable change identity.
    pub change: ChangeId,
    /// Effective date of the settlement fact.
    pub settled_on: Date,
    /// Exact supplier settlement, without any inferred payment state.
    #[serde(with = "crate::work_package::amount_serde")]
    pub supplier: Amount,
    /// Exact customer settlement, without any inferred receipt state.
    #[serde(with = "crate::work_package::amount_serde")]
    pub customer: Amount,
    /// Supplier settlement less customer settlement.
    #[serde(with = "crate::work_package::amount_serde")]
    pub net: Amount,
    /// Whether an accountable closure fact reconciles to these exact values.
    pub closed: bool,
    /// Versioned document or ledger references used by the settlement fact.
    pub references: Vec<ReferencedAmountEvidence>,
}

pub(crate) fn settlement_view(
    change: &ChangeRecord,
    facts: &BTreeMap<ChangeStage, &ChangeFact>,
) -> Result<Option<ChangeSettlementView>> {
    let Some(settlement) = facts.get(&ChangeStage::Settlement) else {
        if facts.contains_key(&ChangeStage::Closure) {
            return Err(ConstructionProjectError::ChangeDerivation {
                change: change.id.clone(),
                reason: "closure has no current settlement fact",
            });
        }
        return Ok(None);
    };
    let supplier = checked_total(
        &settlement.amount_components,
        CommercialSide::Supplier,
        "change.settlement.supplier",
    )?;
    let customer = checked_total(
        &settlement.amount_components,
        CommercialSide::Customer,
        "change.settlement.customer",
    )?;
    let net = supplier.0.checked_sub(customer.0).map(Amount).ok_or(
        ConstructionProjectError::AmountOverflow {
            field: "change.settlement.net",
        },
    )?;
    let closed = if let Some(closure) = facts.get(&ChangeStage::Closure) {
        validate_closeout_stages(change, facts)?;
        validate_closure_total(
            change,
            closure,
            CommercialSide::Supplier,
            supplier,
            "change.closure.supplier",
        )?;
        validate_closure_total(
            change,
            closure,
            CommercialSide::Customer,
            customer,
            "change.closure.customer",
        )?;
        true
    } else {
        false
    };
    Ok(Some(ChangeSettlementView {
        change: change.id.clone(),
        settled_on: settlement.effective_on,
        supplier,
        customer,
        net,
        closed,
        references: settlement.references.clone(),
    }))
}

fn validate_closeout_stages(
    change: &ChangeRecord,
    facts: &BTreeMap<ChangeStage, &ChangeFact>,
) -> Result<()> {
    if CLOSEOUT_STAGES
        .iter()
        .any(|required| !facts.contains_key(required))
    {
        return Err(ConstructionProjectError::ChangeDerivation {
            change: change.id.clone(),
            reason: "closure requires one current fact for every change-chain stage",
        });
    }
    Ok(())
}

fn validate_closure_total(
    change: &ChangeRecord,
    closure: &ChangeFact,
    side: CommercialSide,
    settlement: Amount,
    field: &'static str,
) -> Result<()> {
    let closure_total = checked_total(&closure.amount_components, side, field)?;
    if closure_total != settlement {
        return Err(ConstructionProjectError::ChangeSettlementMismatch {
            change: change.id.clone(),
            side: side.label(),
            settlement: settlement.to_decimal_string(),
            closure: closure_total.to_decimal_string(),
        });
    }
    Ok(())
}
