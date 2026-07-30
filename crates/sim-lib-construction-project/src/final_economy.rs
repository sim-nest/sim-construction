//! Exact, as-of construction final-economy control.

use std::collections::{BTreeMap, BTreeSet};

use sim_ledger::Amount;
use sim_lib_doc_core::ExternalRef;
use time::Date;

use crate::{
    CommercialEvidenceSource, ConstructionProjectError, ControlId, CurrencyCode, EvidenceState,
    ProjectId, Result,
};

/// Commercial meaning carried by one final-economy amount fact.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum FinalEconomyFactKind {
    /// Accepted main-contract value.
    AcceptedContract,
    /// Current project forecast.
    CurrentForecast,
    /// Accountably stated final position.
    FinalPosition,
    /// Current open-change exposure.
    OpenChange,
    /// Customer-side settlement.
    CustomerSettlement,
    /// Supplier-side settlement.
    SupplierSettlement,
    /// Outstanding guarantee exposure.
    Guarantee,
    /// Outstanding retention exposure.
    Retention,
    /// Outstanding customer or supplier claim.
    Claim,
    /// Read-only balance observed from the ledger owner.
    LedgerBalance,
}

impl FinalEconomyFactKind {
    fn is_exposure(self) -> bool {
        matches!(
            self,
            Self::OpenChange | Self::Guarantee | Self::Retention | Self::Claim
        )
    }

    fn is_settlement(self) -> bool {
        matches!(self, Self::CustomerSettlement | Self::SupplierSettlement)
    }
}

/// One checked fixed-decimal amount retained as an as-of project fact.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FinalEconomyAmountFact {
    /// Stable project identity.
    pub project: ProjectId,
    /// Stable amount-fact identity.
    pub id: ControlId,
    /// Commercial meaning of the amount.
    pub kind: FinalEconomyFactKind,
    /// Exact signed fixed-decimal amount.
    #[serde(with = "crate::work_package::amount_serde")]
    pub amount: Amount,
    /// Project-charter currency.
    pub currency: CurrencyCode,
    /// Date on which the amount was stated.
    pub as_of: Date,
    /// Project fact sequence supporting this amount.
    pub fact_seq: u64,
    /// Accountable evidence review state.
    pub evidence_state: EvidenceState,
    /// Whether the source is a document or a read-only ledger balance.
    pub source: CommercialEvidenceSource,
    /// Versioned reference to the source-owned evidence.
    pub reference: ExternalRef,
    /// Prior amount fact corrected by this fact.
    pub supersedes: Option<ControlId>,
    /// Exposure facts discharged by this settlement fact.
    pub settles: Vec<ControlId>,
}

impl FinalEconomyAmountFact {
    /// Builds an exact amount fact without inferring approval, journal, or payment state.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        project: ProjectId,
        id: ControlId,
        kind: FinalEconomyFactKind,
        amount: Amount,
        currency: CurrencyCode,
        as_of: Date,
        fact_seq: u64,
        source: CommercialEvidenceSource,
        reference: ExternalRef,
    ) -> Self {
        Self {
            project,
            id,
            kind,
            amount,
            currency,
            as_of,
            fact_seq,
            evidence_state: EvidenceState::Reported,
            source,
            reference,
            supersedes: None,
            settles: Vec::new(),
        }
    }

    /// Sets the accountable evidence state.
    #[must_use]
    pub fn with_evidence_state(mut self, state: EvidenceState) -> Self {
        self.evidence_state = state;
        self
    }

    /// Corrects one prior amount fact.
    #[must_use]
    pub fn supersedes(mut self, prior: ControlId) -> Self {
        self.supersedes = Some(prior);
        self
    }

    /// Records an exposure discharged by this settlement.
    #[must_use]
    pub fn settles(mut self, exposure: ControlId) -> Self {
        self.settles.push(exposure);
        self
    }
}

/// Reproducible date, sequence, and explanation behind a final-economy view.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FinalEconomyBasis {
    /// Inclusive project fact sequence.
    pub through_seq: u64,
    /// Commercial reporting date.
    pub as_of: Date,
    /// Named source and cutoff explanation.
    pub explanation: String,
}

impl FinalEconomyBasis {
    /// Builds an explicit as-of basis.
    #[must_use]
    pub fn new(through_seq: u64, as_of: Date, explanation: impl Into<String>) -> Self {
        Self {
            through_seq,
            as_of,
            explanation: explanation.into(),
        }
    }
}

/// Explicit comparison between the final position and ledger-owned evidence.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FinalEconomyReconciliation {
    /// Final-position fact being reconciled.
    pub final_position: ControlId,
    /// Ledger-balance fact used as read-only evidence.
    pub ledger_balance: ControlId,
    /// Human-readable explanation of scope, cutoff, and reconciling items.
    pub explanation: String,
}

impl FinalEconomyReconciliation {
    /// Builds an explicit reconciliation pair.
    #[must_use]
    pub fn new(
        final_position: ControlId,
        ledger_balance: ControlId,
        explanation: impl Into<String>,
    ) -> Self {
        Self {
            final_position,
            ledger_balance,
            explanation: explanation.into(),
        }
    }
}

/// Reason the final-economy report is not ready for closeout.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FinalEconomyBlocker {
    /// A required current fact kind is absent.
    MissingKind(FinalEconomyFactKind),
    /// More than one non-superseded fact claims a singular kind.
    ConflictingKind(FinalEconomyFactKind),
    /// A current amount fact lacks accepted evidence.
    EvidenceNotAccepted {
        /// Blocked fact.
        fact: ControlId,
        /// Current evidence state.
        state: EvidenceState,
    },
    /// A current exposure is not discharged by a current accepted settlement.
    UnsettledExposure(ControlId),
    /// A settlement names a missing or non-exposure fact.
    UnknownSettlementTarget(ControlId),
    /// The explicit reconciliation references the wrong fact kind.
    InvalidReconciliationFact(ControlId),
    /// Final position and ledger balance differ exactly.
    LedgerMismatch {
        /// Final-position value.
        #[serde(with = "crate::work_package::amount_serde")]
        final_position: Amount,
        /// Ledger-observed value.
        #[serde(with = "crate::work_package::amount_serde")]
        ledger_balance: Amount,
    },
}

/// Exact totals and blockers derived from current final-economy facts.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FinalEconomyTotal {
    /// Commercial meaning totaled.
    pub kind: FinalEconomyFactKind,
    /// Checked exact total.
    #[serde(with = "crate::work_package::amount_serde")]
    pub amount: Amount,
}

/// Exact totals and blockers derived from current final-economy facts.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FinalEconomyReport {
    /// Stable project identity.
    pub project: ProjectId,
    /// Project currency used by every amount.
    pub currency: CurrencyCode,
    /// Reproducible cutoff.
    pub basis: FinalEconomyBasis,
    /// Non-superseded facts in stable id order.
    pub current_facts: Vec<FinalEconomyAmountFact>,
    /// Checked totals by commercial meaning.
    pub totals: Vec<FinalEconomyTotal>,
    /// Current exposures not discharged by accepted settlements.
    pub unsettled_exposure: Vec<ControlId>,
    /// Whether the named final position matches ledger evidence exactly.
    pub ledger_reconciled: bool,
    /// Deterministic closeout blockers.
    pub blockers: Vec<FinalEconomyBlocker>,
    /// True only when core facts, settlement, and reconciliation are complete.
    pub ready: bool,
}

impl FinalEconomyReport {
    /// Returns the checked total for one commercial meaning.
    #[must_use]
    pub fn total(&self, kind: FinalEconomyFactKind) -> Option<Amount> {
        self.totals
            .iter()
            .find(|total| total.kind == kind)
            .map(|total| total.amount)
    }
}

/// Final-economy facts plus their explicit cutoff and reconciliation.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FinalEconomyControl {
    /// Stable project identity.
    pub project: ProjectId,
    /// Project-charter currency.
    pub currency: CurrencyCode,
    /// Reproducible reporting cutoff.
    pub basis: FinalEconomyBasis,
    /// Explicit final-position to ledger comparison.
    pub reconciliation: FinalEconomyReconciliation,
    /// Append-only corrected amount facts.
    pub facts: Vec<FinalEconomyAmountFact>,
}

impl FinalEconomyControl {
    /// Builds an empty final-economy control.
    #[must_use]
    pub fn new(
        project: ProjectId,
        currency: CurrencyCode,
        basis: FinalEconomyBasis,
        reconciliation: FinalEconomyReconciliation,
    ) -> Self {
        Self {
            project,
            currency,
            basis,
            reconciliation,
            facts: Vec::new(),
        }
    }

    /// Adds one amount fact.
    #[must_use]
    pub fn with_fact(mut self, fact: FinalEconomyAmountFact) -> Self {
        self.facts.push(fact);
        self
    }

    /// Derives exact current totals, unsettled exposure, and ledger reconciliation.
    pub fn derive(&self) -> Result<FinalEconomyReport> {
        self.validate_shape()?;
        let current = self.current_facts()?;
        let mut blockers = Vec::new();
        for fact in current.values() {
            if !fact.evidence_state.satisfies_required_evidence() {
                blockers.push(FinalEconomyBlocker::EvidenceNotAccepted {
                    fact: fact.id.clone(),
                    state: fact.evidence_state,
                });
            }
        }
        for kind in [
            FinalEconomyFactKind::AcceptedContract,
            FinalEconomyFactKind::CurrentForecast,
            FinalEconomyFactKind::FinalPosition,
            FinalEconomyFactKind::LedgerBalance,
        ] {
            let count = current.values().filter(|fact| fact.kind == kind).count();
            match count {
                0 => blockers.push(FinalEconomyBlocker::MissingKind(kind)),
                1 => {}
                _ => blockers.push(FinalEconomyBlocker::ConflictingKind(kind)),
            }
        }

        let settled = current
            .values()
            .filter(|fact| {
                fact.kind.is_settlement() && fact.evidence_state.satisfies_required_evidence()
            })
            .flat_map(|fact| fact.settles.iter().cloned())
            .collect::<BTreeSet<_>>();
        let exposure_ids = current
            .values()
            .filter(|fact| fact.kind.is_exposure())
            .map(|fact| fact.id.clone())
            .collect::<BTreeSet<_>>();
        for target in current
            .values()
            .filter(|fact| fact.kind.is_settlement())
            .flat_map(|fact| &fact.settles)
        {
            if !exposure_ids.contains(target) {
                blockers.push(FinalEconomyBlocker::UnknownSettlementTarget(target.clone()));
            }
        }
        let unsettled_exposure = exposure_ids
            .difference(&settled)
            .cloned()
            .collect::<Vec<_>>();
        blockers.extend(
            unsettled_exposure
                .iter()
                .cloned()
                .map(FinalEconomyBlocker::UnsettledExposure),
        );

        let totals = checked_totals(current.values().copied())?;
        let ledger_reconciled = self.reconcile(&current, &mut blockers);
        blockers.sort_by_key(blocker_sort_key);
        let current_facts = current.into_values().cloned().collect();
        Ok(FinalEconomyReport {
            project: self.project.clone(),
            currency: self.currency.clone(),
            basis: self.basis.clone(),
            current_facts,
            totals,
            unsettled_exposure,
            ledger_reconciled,
            ready: blockers.is_empty(),
            blockers,
        })
    }

    fn validate_shape(&self) -> Result<()> {
        if self.basis.through_seq == 0 {
            return Err(ConstructionProjectError::InvalidSequence {
                field: "final_economy.basis",
                sequence: 0,
            });
        }
        if self.basis.explanation.trim().is_empty() {
            return Err(ConstructionProjectError::EmptyField(
                "final_economy.basis.explanation",
            ));
        }
        if self.reconciliation.explanation.trim().is_empty() {
            return Err(ConstructionProjectError::EmptyField(
                "final_economy.reconciliation.explanation",
            ));
        }
        Ok(())
    }

    fn current_facts(&self) -> Result<BTreeMap<ControlId, &FinalEconomyAmountFact>> {
        let mut by_id = BTreeMap::new();
        for fact in &self.facts {
            validate_fact(fact, self)?;
            if by_id.insert(fact.id.clone(), fact).is_some() {
                return Err(ConstructionProjectError::DuplicateId {
                    kind: "final_economy_fact",
                    id: fact.id.to_string(),
                });
            }
        }
        let mut superseded = BTreeSet::new();
        for fact in &self.facts {
            if let Some(prior) = &fact.supersedes {
                let Some(previous) = by_id.get(prior) else {
                    return Err(ConstructionProjectError::EmptyField(
                        "final_economy.supersedes",
                    ));
                };
                if previous.kind != fact.kind || previous.fact_seq >= fact.fact_seq {
                    return Err(ConstructionProjectError::EmptyField(
                        "final_economy.supersession",
                    ));
                }
                if !superseded.insert(prior.clone()) {
                    return Err(ConstructionProjectError::DuplicateId {
                        kind: "final_economy_supersession",
                        id: prior.to_string(),
                    });
                }
            }
        }
        Ok(by_id
            .into_iter()
            .filter(|(id, _)| !superseded.contains(id))
            .collect())
    }

    fn reconcile(
        &self,
        current: &BTreeMap<ControlId, &FinalEconomyAmountFact>,
        blockers: &mut Vec<FinalEconomyBlocker>,
    ) -> bool {
        let Some(position) = current.get(&self.reconciliation.final_position) else {
            blockers.push(FinalEconomyBlocker::InvalidReconciliationFact(
                self.reconciliation.final_position.clone(),
            ));
            return false;
        };
        let Some(ledger) = current.get(&self.reconciliation.ledger_balance) else {
            blockers.push(FinalEconomyBlocker::InvalidReconciliationFact(
                self.reconciliation.ledger_balance.clone(),
            ));
            return false;
        };
        if position.kind != FinalEconomyFactKind::FinalPosition {
            blockers.push(FinalEconomyBlocker::InvalidReconciliationFact(
                position.id.clone(),
            ));
        }
        if ledger.kind != FinalEconomyFactKind::LedgerBalance
            || ledger.source != CommercialEvidenceSource::LedgerBalance
        {
            blockers.push(FinalEconomyBlocker::InvalidReconciliationFact(
                ledger.id.clone(),
            ));
        }
        if position.amount != ledger.amount {
            blockers.push(FinalEconomyBlocker::LedgerMismatch {
                final_position: position.amount,
                ledger_balance: ledger.amount,
            });
            return false;
        }
        position.kind == FinalEconomyFactKind::FinalPosition
            && ledger.kind == FinalEconomyFactKind::LedgerBalance
            && ledger.source == CommercialEvidenceSource::LedgerBalance
    }
}

fn validate_fact(fact: &FinalEconomyAmountFact, control: &FinalEconomyControl) -> Result<()> {
    if fact.project != control.project {
        return Err(ConstructionProjectError::ProjectMismatch {
            expected: control.project.clone(),
            actual: fact.project.clone(),
        });
    }
    if fact.currency != control.currency {
        return Err(ConstructionProjectError::CurrencyMismatch {
            field: "final_economy.amount",
            expected: control.currency.as_str().to_owned(),
            actual: fact.currency.as_str().to_owned(),
        });
    }
    if fact.fact_seq == 0 || fact.fact_seq > control.basis.through_seq {
        return Err(ConstructionProjectError::InvalidSequence {
            field: "final_economy.fact",
            sequence: fact.fact_seq,
        });
    }
    if fact.as_of > control.basis.as_of {
        return Err(ConstructionProjectError::EmptyField(
            "final_economy.fact.as_of",
        ));
    }
    if fact.reference.backend.trim().is_empty()
        || fact.reference.external_id.trim().is_empty()
        || fact
            .reference
            .version
            .as_deref()
            .is_none_or(|version| version.trim().is_empty())
    {
        return Err(ConstructionProjectError::EmptyField(
            "final_economy.fact.reference",
        ));
    }
    if !fact.kind.is_settlement() && !fact.settles.is_empty() {
        return Err(ConstructionProjectError::EmptyCollection(
            "final_economy.non_settlement.settles",
        ));
    }
    Ok(())
}

fn checked_totals<'a>(
    facts: impl IntoIterator<Item = &'a FinalEconomyAmountFact>,
) -> Result<Vec<FinalEconomyTotal>> {
    let mut totals = BTreeMap::new();
    for fact in facts {
        let total = totals.entry(fact.kind).or_insert(Amount(0));
        *total = total.0.checked_add(fact.amount.0).map(Amount).ok_or(
            ConstructionProjectError::AmountOverflow {
                field: "final_economy.total",
            },
        )?;
    }
    Ok(totals
        .into_iter()
        .map(|(kind, amount)| FinalEconomyTotal { kind, amount })
        .collect())
}

fn blocker_sort_key(blocker: &FinalEconomyBlocker) -> String {
    match blocker {
        FinalEconomyBlocker::MissingKind(kind) => format!("1:{kind:?}"),
        FinalEconomyBlocker::ConflictingKind(kind) => format!("2:{kind:?}"),
        FinalEconomyBlocker::EvidenceNotAccepted { fact, .. } => format!("3:{fact}"),
        FinalEconomyBlocker::UnsettledExposure(fact) => format!("4:{fact}"),
        FinalEconomyBlocker::UnknownSettlementTarget(fact) => format!("5:{fact}"),
        FinalEconomyBlocker::InvalidReconciliationFact(fact) => format!("6:{fact}"),
        FinalEconomyBlocker::LedgerMismatch { .. } => "7".to_owned(),
    }
}
