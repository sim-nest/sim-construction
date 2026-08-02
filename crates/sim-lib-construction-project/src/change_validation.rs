//! Validation rules for construction change stage facts.

use crate::{
    ChangeFact, ChangeStage, ChangeStatus, CommercialSide, ConstructionProjectError, CurrencyCode,
    Result,
    change::{validate_tasks, validate_unique_controls},
    commercial::validate_components,
};

pub(crate) fn validate_change_fact(
    fact: &ChangeFact,
    charter_currency: &CurrencyCode,
) -> Result<()> {
    if fact.fact_seq == 0 {
        return Err(ConstructionProjectError::InvalidSequence {
            field: "change.fact_seq",
            sequence: fact.fact_seq,
        });
    }
    if fact.note.trim().is_empty() {
        return Err(ConstructionProjectError::EmptyField("change.fact.note"));
    }
    validate_unique_controls("change.affected_controls", &fact.affected_controls)?;
    validate_unique_controls("change.affected_packages", &fact.affected_packages)?;
    validate_tasks(&fact.affected_tasks)?;
    validate_components(&fact.amount_components, charter_currency)?;
    for reference in &fact.references {
        reference.validate(charter_currency)?;
    }
    validate_stage(fact)
}

fn validate_stage(fact: &ChangeFact) -> Result<()> {
    let status_allowed = match fact.stage {
        ChangeStage::ScopeAssessment
        | ChangeStage::TimeEffect
        | ChangeStage::SupplierExposure
        | ChangeStage::CustomerRecovery
        | ChangeStage::Forecast => matches!(fact.status, ChangeStatus::Assessing),
        ChangeStage::Quotation => matches!(fact.status, ChangeStatus::Submitted),
        ChangeStage::AuthorityDecision => matches!(
            fact.status,
            ChangeStatus::Approved
                | ChangeStatus::PartiallyApproved
                | ChangeStatus::Rejected
                | ChangeStatus::Disputed
        ),
        ChangeStage::Execution => matches!(fact.status, ChangeStatus::Executing),
        ChangeStage::Settlement => {
            matches!(fact.status, ChangeStatus::Settled | ChangeStatus::Disputed)
        }
        ChangeStage::Closure => matches!(fact.status, ChangeStatus::Closed),
    };
    if !status_allowed {
        return Err(ConstructionProjectError::ChangeDerivation {
            change: fact.change.clone(),
            reason: "status is incompatible with change stage",
        });
    }
    if fact.stage == ChangeStage::ScopeAssessment
        && fact.affected_controls.is_empty()
        && fact.affected_tasks.is_empty()
        && fact.affected_packages.is_empty()
    {
        return Err(ConstructionProjectError::EmptyCollection(
            "change.scope.affected",
        ));
    }
    if fact.stage == ChangeStage::TimeEffect && fact.schedule_impact.is_none() {
        return Err(ConstructionProjectError::EmptyField(
            "change.schedule_impact",
        ));
    }
    if let Some(impact) = &fact.schedule_impact {
        impact.validate()?;
    }
    if matches!(
        fact.stage,
        ChangeStage::SupplierExposure
            | ChangeStage::CustomerRecovery
            | ChangeStage::Quotation
            | ChangeStage::Forecast
            | ChangeStage::Settlement
            | ChangeStage::Closure
    ) && fact.amount_components.is_empty()
    {
        return Err(ConstructionProjectError::EmptyCollection(
            "change.amount_components",
        ));
    }
    if fact.stage == ChangeStage::SupplierExposure
        && !fact
            .amount_components
            .iter()
            .any(|component| component.side == CommercialSide::Supplier)
    {
        return Err(ConstructionProjectError::ChangeDerivation {
            change: fact.change.clone(),
            reason: "supplier exposure has no supplier amount",
        });
    }
    if matches!(
        fact.stage,
        ChangeStage::CustomerRecovery | ChangeStage::Quotation
    ) && !fact
        .amount_components
        .iter()
        .any(|component| component.side == CommercialSide::Customer)
    {
        return Err(ConstructionProjectError::ChangeDerivation {
            change: fact.change.clone(),
            reason: "customer commercial fact has no customer amount",
        });
    }
    if fact.stage == ChangeStage::AuthorityDecision
        && matches!(
            fact.status,
            ChangeStatus::Approved | ChangeStatus::PartiallyApproved
        )
        && !fact
            .amount_components
            .iter()
            .any(|component| component.side == CommercialSide::Customer)
    {
        return Err(ConstructionProjectError::ChangeDerivation {
            change: fact.change.clone(),
            reason: "approval has no explicit approved customer amount",
        });
    }
    if matches!(
        fact.stage,
        ChangeStage::AuthorityDecision | ChangeStage::Settlement | ChangeStage::Closure
    ) && fact.references.is_empty()
    {
        return Err(ConstructionProjectError::EmptyCollection(
            "change.fact.references",
        ));
    }
    Ok(())
}
