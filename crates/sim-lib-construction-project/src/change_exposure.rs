//! Deterministic commercial exposure views over one-chain change facts.

use std::collections::{BTreeMap, BTreeSet};

use sim_ledger::Amount;
use time::Date;

use crate::{
    ChangeFact, ChangeId, ChangeRecord, ChangeScheduleImpact, ChangeStage, ChangeStatus,
    CommercialSide, ConstructionProjectError, ControlId, CurrencyCode, ReferencedAmountEvidence,
    Result, commercial::checked_total,
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

/// Current derived view of one stable construction change chain.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ChangeExposureView {
    /// Stable change identity.
    pub change: ChangeId,
    /// Status of the furthest current stage.
    pub status: ChangeStatus,
    /// Current stage facts, ordered by lifecycle stage.
    pub current_facts: Vec<ControlId>,
    /// Union of affected construction controls.
    pub affected_controls: Vec<ControlId>,
    /// Union of affected canonical Gantt task ids.
    pub affected_tasks: Vec<String>,
    /// Union of affected work-package controls.
    pub affected_packages: Vec<ControlId>,
    /// Current supplier-side exposure.
    #[serde(with = "crate::work_package::amount_serde")]
    pub supplier_exposure: Amount,
    /// Current quoted or settled customer recovery.
    #[serde(with = "crate::work_package::amount_serde")]
    pub customer_recovery: Amount,
    /// Explicitly approved customer value.
    #[serde(with = "crate::work_package::amount_serde")]
    pub approved_customer: Amount,
    /// Supplier exposure less customer recovery.
    #[serde(with = "crate::work_package::amount_serde")]
    pub net_exposure: Amount,
    /// True while a current non-zero schedule effect remains open.
    pub time_risk: bool,
    /// Current baseline-aware schedule impact, when assessed.
    pub schedule_impact: Option<ChangeScheduleImpact>,
    /// True when required contractual notice is overdue and unrecorded.
    pub overdue_notice: bool,
    /// Final settlement values, when a settlement fact exists.
    pub settlement: Option<ChangeSettlementView>,
}

/// Portfolio views and exact aggregate exposure at one deterministic date.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ChangeExposureReport {
    /// Evaluation date for notice and open-risk views.
    pub as_of_date: Date,
    /// Project charter currency used by every checked component.
    pub currency: CurrencyCode,
    /// Current change chains in stable identity order.
    pub changes: Vec<ChangeExposureView>,
    /// Changes with full or partial accountable approval.
    pub approved: Vec<ChangeId>,
    /// Instructed or noticed changes without a customer price.
    pub instructed_unpriced: Vec<ChangeId>,
    /// Quoted changes still awaiting authority decision.
    pub submitted: Vec<ChangeId>,
    /// Changes with a current disputed decision or settlement.
    pub disputed: Vec<ChangeId>,
    /// Changes with supplier exposure and no customer recovery.
    pub supplier_only: Vec<ChangeId>,
    /// Changes with customer recovery and no supplier exposure.
    pub customer_only: Vec<ChangeId>,
    /// Open changes with non-zero schedule effect.
    pub time_risk: Vec<ChangeId>,
    /// Changes whose contractual notice date is overdue.
    pub overdue_notice: Vec<ChangeId>,
    /// Final settlement views.
    pub settlements: Vec<ChangeSettlementView>,
    /// Checked sum of per-change supplier exposure less customer recovery.
    #[serde(with = "crate::work_package::amount_serde")]
    pub net_exposure: Amount,
}

/// Initiating records and immutable stage facts for construction changes.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ChangeControlSet {
    /// One initiating record per stable change identity.
    pub changes: Vec<ChangeRecord>,
    /// Immutable stage and correction facts.
    pub facts: Vec<ChangeFact>,
}

impl ChangeControlSet {
    /// Builds an empty change-control set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an initiating change record.
    #[must_use]
    pub fn with_change(mut self, change: ChangeRecord) -> Self {
        self.changes.push(change);
        self
    }

    /// Adds an immutable stage fact.
    #[must_use]
    pub fn with_fact(mut self, fact: ChangeFact) -> Self {
        self.facts.push(fact);
        self
    }

    /// Derives deterministic status, exposure, notice, time-risk, and settlement views.
    pub fn derive(
        &self,
        charter_currency: &CurrencyCode,
        as_of_date: Date,
    ) -> Result<ChangeExposureReport> {
        let change_by_id = self.change_map()?;
        let current_by_change = self.current_facts(&change_by_id, charter_currency)?;

        let mut changes = Vec::new();
        let mut approved = Vec::new();
        let mut instructed_unpriced = Vec::new();
        let mut submitted = Vec::new();
        let mut disputed = Vec::new();
        let mut supplier_only = Vec::new();
        let mut customer_only = Vec::new();
        let mut time_risk = Vec::new();
        let mut overdue_notice = Vec::new();
        let mut settlements = Vec::new();
        let mut net_exposure = Amount(0);

        for change in self.sorted_changes() {
            let facts = current_by_change
                .get(&change.id)
                .cloned()
                .unwrap_or_default();
            let view = derive_change(change, &facts, as_of_date)?;
            let has_authority = facts.contains_key(&ChangeStage::AuthorityDecision);
            let has_quotation = facts.contains_key(&ChangeStage::Quotation);

            if view.approved_customer.0 > 0 {
                approved.push(change.id.clone());
            }
            if view.customer_recovery.0 == 0 && !has_quotation && !has_authority {
                instructed_unpriced.push(change.id.clone());
            }
            if has_quotation && !has_authority {
                submitted.push(change.id.clone());
            }
            if facts
                .values()
                .any(|fact| fact.status == ChangeStatus::Disputed)
            {
                disputed.push(change.id.clone());
            }
            if view.supplier_exposure.0 > 0 && view.customer_recovery.0 == 0 {
                supplier_only.push(change.id.clone());
            }
            if view.customer_recovery.0 > 0 && view.supplier_exposure.0 == 0 {
                customer_only.push(change.id.clone());
            }
            if view.time_risk {
                time_risk.push(change.id.clone());
            }
            if view.overdue_notice {
                overdue_notice.push(change.id.clone());
            }
            if let Some(settlement) = &view.settlement {
                settlements.push(settlement.clone());
            }
            net_exposure = net_exposure
                .0
                .checked_add(view.net_exposure.0)
                .map(Amount)
                .ok_or(ConstructionProjectError::AmountOverflow {
                    field: "change.portfolio.net_exposure",
                })?;
            changes.push(view);
        }

        Ok(ChangeExposureReport {
            as_of_date,
            currency: charter_currency.clone(),
            changes,
            approved,
            instructed_unpriced,
            submitted,
            disputed,
            supplier_only,
            customer_only,
            time_risk,
            overdue_notice,
            settlements,
            net_exposure,
        })
    }

    fn change_map(&self) -> Result<BTreeMap<ChangeId, &ChangeRecord>> {
        let mut by_id = BTreeMap::new();
        let mut project: Option<&crate::ProjectId> = None;
        for change in &self.changes {
            change.validate()?;
            if let Some(expected) = project
                && expected != &change.project
            {
                return Err(ConstructionProjectError::ProjectMismatch {
                    expected: expected.clone(),
                    actual: change.project.clone(),
                });
            }
            project = Some(&change.project);
            if by_id.insert(change.id.clone(), change).is_some() {
                return Err(ConstructionProjectError::DuplicateId {
                    kind: "change",
                    id: change.id.as_str().to_owned(),
                });
            }
        }
        Ok(by_id)
    }

    fn current_facts<'a>(
        &'a self,
        changes: &BTreeMap<ChangeId, &ChangeRecord>,
        charter_currency: &CurrencyCode,
    ) -> Result<BTreeMap<ChangeId, BTreeMap<ChangeStage, &'a ChangeFact>>> {
        let mut fact_by_id = BTreeMap::new();
        let mut fact_by_seq = BTreeMap::new();
        for fact in &self.facts {
            fact.validate(charter_currency)?;
            if !changes.contains_key(&fact.change) {
                return Err(ConstructionProjectError::ChangeFactDerivation {
                    fact: fact.control.clone(),
                    reason: "fact references a missing change",
                });
            }
            if fact_by_id.insert(fact.control.clone(), fact).is_some() {
                return Err(ConstructionProjectError::DuplicateId {
                    kind: "change_fact",
                    id: fact.control.as_str().to_owned(),
                });
            }
            if fact_by_seq.insert(fact.fact_seq, fact).is_some() {
                return Err(ConstructionProjectError::DuplicateSequence {
                    sequence: fact.fact_seq,
                });
            }
        }

        let mut corrected = BTreeSet::new();
        let mut corrected_by = BTreeMap::new();
        for fact in &self.facts {
            let Some(prior_id) = &fact.supersedes else {
                continue;
            };
            let prior = fact_by_id.get(prior_id).ok_or_else(|| {
                ConstructionProjectError::ChangeFactDerivation {
                    fact: fact.control.clone(),
                    reason: "correction references a missing prior fact",
                }
            })?;
            if prior.change != fact.change
                || prior.stage != fact.stage
                || prior.fact_seq >= fact.fact_seq
            {
                return Err(ConstructionProjectError::ChangeFactDerivation {
                    fact: fact.control.clone(),
                    reason: "correction must advance the same change and stage",
                });
            }
            if corrected_by.insert(prior_id, &fact.control).is_some() {
                return Err(ConstructionProjectError::ChangeFactDerivation {
                    fact: fact.control.clone(),
                    reason: "more than one correction supersedes the same fact",
                });
            }
            corrected.insert(prior_id);
        }

        let mut current = BTreeMap::<ChangeId, BTreeMap<ChangeStage, &ChangeFact>>::new();
        for fact in &self.facts {
            if corrected.contains(&fact.control) {
                continue;
            }
            let stages = current.entry(fact.change.clone()).or_default();
            if let Some(existing) = stages.insert(fact.stage, fact) {
                return Err(ConstructionProjectError::ChangeFactDerivation {
                    fact: fact.control.clone(),
                    reason: if existing.supersedes.is_some() || fact.supersedes.is_some() {
                        "correction chain leaves more than one current stage fact"
                    } else {
                        "duplicate current stage fact has no correction edge"
                    },
                });
            }
        }
        Ok(current)
    }

    fn sorted_changes(&self) -> Vec<&ChangeRecord> {
        let mut changes = self.changes.iter().collect::<Vec<_>>();
        changes.sort_by(|left, right| left.id.cmp(&right.id));
        changes
    }
}

fn derive_change(
    change: &ChangeRecord,
    facts: &BTreeMap<ChangeStage, &ChangeFact>,
    as_of_date: Date,
) -> Result<ChangeExposureView> {
    let status = facts
        .iter()
        .next_back()
        .map_or(ChangeStatus::Assessing, |(_, fact)| fact.status);
    let supplier_exposure = current_total(
        facts,
        &[
            ChangeStage::Settlement,
            ChangeStage::Forecast,
            ChangeStage::SupplierExposure,
        ],
        CommercialSide::Supplier,
        "change.supplier_exposure",
    )?;
    let customer_recovery = current_total(
        facts,
        &[
            ChangeStage::Settlement,
            ChangeStage::Quotation,
            ChangeStage::CustomerRecovery,
        ],
        CommercialSide::Customer,
        "change.customer_recovery",
    )?;
    let approved_customer = authority_total(facts)?;
    let net_exposure = supplier_exposure
        .0
        .checked_sub(customer_recovery.0)
        .map(Amount)
        .ok_or(ConstructionProjectError::AmountOverflow {
            field: "change.net_exposure",
        })?;

    let schedule_impact = facts
        .get(&ChangeStage::TimeEffect)
        .and_then(|fact| fact.schedule_impact.clone());
    let time_risk = status != ChangeStatus::Closed
        && schedule_impact
            .as_ref()
            .is_some_and(|impact| impact.completion_delta_days != 0);
    let overdue_notice = change
        .notice_due_on
        .is_some_and(|due| as_of_date > due && change.notice_given_on.is_none());

    let settlement = settlement_view(change, facts)?;
    let current_facts = facts.values().map(|fact| fact.control.clone()).collect();
    let (affected_controls, affected_tasks, affected_packages) =
        affected_scope(change, facts.values().copied());

    Ok(ChangeExposureView {
        change: change.id.clone(),
        status,
        current_facts,
        affected_controls,
        affected_tasks,
        affected_packages,
        supplier_exposure,
        customer_recovery,
        approved_customer,
        net_exposure,
        time_risk,
        schedule_impact,
        overdue_notice,
        settlement,
    })
}

fn current_total(
    facts: &BTreeMap<ChangeStage, &ChangeFact>,
    stages: &[ChangeStage],
    side: CommercialSide,
    field: &'static str,
) -> Result<Amount> {
    for stage in stages {
        if let Some(fact) = facts.get(stage) {
            let total = checked_total(&fact.amount_components, side, field)?;
            if total.0 != 0 || *stage == ChangeStage::Settlement {
                return Ok(total);
            }
        }
    }
    Ok(Amount(0))
}

fn authority_total(facts: &BTreeMap<ChangeStage, &ChangeFact>) -> Result<Amount> {
    let Some(authority) = facts.get(&ChangeStage::AuthorityDecision) else {
        return Ok(Amount(0));
    };
    if !matches!(
        authority.status,
        ChangeStatus::Approved | ChangeStatus::PartiallyApproved
    ) {
        return Ok(Amount(0));
    }
    checked_total(
        &authority.amount_components,
        CommercialSide::Customer,
        "change.approved_customer",
    )
}

fn settlement_view(
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

fn affected_scope<'a>(
    change: &ChangeRecord,
    facts: impl IntoIterator<Item = &'a ChangeFact>,
) -> (Vec<ControlId>, Vec<String>, Vec<ControlId>) {
    let mut controls = change
        .affected_controls
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut tasks = change
        .affected_tasks
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut packages = change
        .affected_packages
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    for fact in facts {
        controls.extend(fact.affected_controls.iter().cloned());
        tasks.extend(fact.affected_tasks.iter().cloned());
        packages.extend(fact.affected_packages.iter().cloned());
    }
    (
        controls.into_iter().collect(),
        tasks.into_iter().collect(),
        packages.into_iter().collect(),
    )
}
