//! Exact commercial components and reference-only external evidence.

use std::collections::BTreeSet;

use sim_ledger::Amount;
use sim_lib_doc_core::ExternalRef;
use time::Date;

use crate::{
    CommercialAmount, ConstructionProjectError, ControlId, CurrencyCode, EvidenceState, Result,
};

/// Commercial side of a construction change.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum CommercialSide {
    /// Cost or settlement exposure toward a supplier.
    Supplier,
    /// Recovery, approval, or settlement value toward the customer.
    Customer,
}

impl CommercialSide {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Supplier => "supplier",
            Self::Customer => "customer",
        }
    }
}

/// One exact amount component inside a stage-specific commercial fact.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ChangeAmountComponent {
    /// Stable component identity within the stage fact.
    pub control: ControlId,
    /// Supplier cost or customer recovery side.
    pub side: CommercialSide,
    /// Open component category such as labor, material, time, or markup.
    pub category: String,
    /// Checked fixed-decimal amount in the project charter currency.
    pub amount: CommercialAmount,
    /// Optional grouping component; a fact must not total both parent and child.
    pub parent: Option<ControlId>,
}

impl ChangeAmountComponent {
    /// Builds an exact commercial component.
    #[must_use]
    pub fn new(
        control: ControlId,
        side: CommercialSide,
        category: impl Into<String>,
        amount: CommercialAmount,
    ) -> Self {
        Self {
            control,
            side,
            category: category.into(),
            amount,
            parent: None,
        }
    }

    /// Records the grouping component that already summarizes this component.
    #[must_use]
    pub fn with_parent(mut self, parent: ControlId) -> Self {
        self.parent = Some(parent);
        self
    }

    pub(crate) fn validate(&self, charter_currency: &CurrencyCode) -> Result<()> {
        if self.category.trim().is_empty() {
            return Err(ConstructionProjectError::EmptyField(
                "change.amount.category",
            ));
        }
        self.amount
            .validate_currency("change.amount", charter_currency)
    }
}

/// External source class for a referenced commercial value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CommercialEvidenceSource {
    /// A value stated by a document; existence is not approval.
    Document,
    /// A balance read from the ledger owner; the construction layer never posts it.
    LedgerBalance,
}

/// Exact signed value observed in an external source.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ReferencedAmount {
    /// Signed fixed-decimal value reported by the source.
    #[serde(with = "crate::work_package::amount_serde")]
    pub amount: Amount,
    /// Currency reported by the source.
    pub currency: CurrencyCode,
}

impl ReferencedAmount {
    /// Builds a signed reference value without turning it into a project decision.
    #[must_use]
    pub const fn new(amount: Amount, currency: CurrencyCode) -> Self {
        Self { amount, currency }
    }
}

/// Versioned, dated evidence about an external document value or ledger balance.
///
/// Acceptance applies to this evidence record only. It never means that a
/// quotation was approved, a journal was posted, or a settlement was paid.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ReferencedAmountEvidence {
    /// Whether the source is a document or a ledger-owned balance.
    pub source: CommercialEvidenceSource,
    /// Reference to the source-owned record.
    pub reference: ExternalRef,
    /// Date at which the external value was observed.
    pub as_of: Date,
    /// Optional exact value reported by that source.
    pub stated_value: Option<ReferencedAmount>,
    /// Evidence review state, separate from commercial authority.
    pub evidence_state: EvidenceState,
}

impl ReferencedAmountEvidence {
    /// Builds reference-only external evidence.
    #[must_use]
    pub fn new(
        source: CommercialEvidenceSource,
        reference: ExternalRef,
        as_of: Date,
        evidence_state: EvidenceState,
    ) -> Self {
        Self {
            source,
            reference,
            as_of,
            stated_value: None,
            evidence_state,
        }
    }

    /// Records the exact value reported by the external source.
    #[must_use]
    pub fn with_stated_value(mut self, value: ReferencedAmount) -> Self {
        self.stated_value = Some(value);
        self
    }

    pub(crate) fn validate(&self, charter_currency: &CurrencyCode) -> Result<()> {
        if self.reference.backend.trim().is_empty() {
            return Err(ConstructionProjectError::EmptyField(
                "change.reference.backend",
            ));
        }
        if self.reference.external_id.trim().is_empty() {
            return Err(ConstructionProjectError::EmptyField(
                "change.reference.external_id",
            ));
        }
        if self
            .reference
            .version
            .as_deref()
            .is_none_or(|version| version.trim().is_empty())
        {
            return Err(ConstructionProjectError::MissingChangeAsOfMarker {
                reference: self.reference.external_id.clone(),
            });
        }
        if let Some(value) = &self.stated_value
            && &value.currency != charter_currency
        {
            return Err(ConstructionProjectError::CurrencyMismatch {
                field: "change.reference.stated_value",
                expected: charter_currency.as_str().to_owned(),
                actual: value.currency.as_str().to_owned(),
            });
        }
        Ok(())
    }
}

pub(crate) fn validate_components(
    components: &[ChangeAmountComponent],
    charter_currency: &CurrencyCode,
) -> Result<()> {
    let mut controls = BTreeSet::new();
    for component in components {
        component.validate(charter_currency)?;
        if !controls.insert(component.control.clone()) {
            return Err(ConstructionProjectError::DuplicateId {
                kind: "change_amount_component",
                id: component.control.as_str().to_owned(),
            });
        }
    }
    for component in components {
        if let Some(parent) = &component.parent
            && controls.contains(parent)
        {
            return Err(ConstructionProjectError::ChangeAmountDoubleCount {
                parent: parent.clone(),
                child: component.control.clone(),
            });
        }
    }
    Ok(())
}

pub(crate) fn checked_total<'a>(
    components: impl IntoIterator<Item = &'a ChangeAmountComponent>,
    side: CommercialSide,
    field: &'static str,
) -> Result<Amount> {
    components
        .into_iter()
        .filter(|component| component.side == side)
        .try_fold(Amount(0), |total, component| {
            total
                .0
                .checked_add(component.amount.amount.0)
                .map(Amount)
                .ok_or(ConstructionProjectError::AmountOverflow { field })
        })
}
