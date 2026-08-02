//! Work-package identity and commercial target records.

use std::collections::BTreeSet;

use serde::ser::SerializeStruct;
use sim_ledger::Amount;
use sim_lib_doc_core::ExternalRef;
use time::Date;

use crate::{ConstructionProjectError, ControlId, CurrencyCode, ProjectId, Result, RoleId};

/// Exact commercial amount with the project currency kept beside ledger arithmetic.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CommercialAmount {
    /// Exact fixed-decimal ledger amount.
    pub amount: Amount,
    /// Exact project currency.
    pub currency: CurrencyCode,
}

impl CommercialAmount {
    /// Builds a checked positive commercial amount.
    pub fn new(amount: Amount, currency: CurrencyCode) -> Result<Self> {
        if amount.0 <= 0 {
            return Err(ConstructionProjectError::NonPositiveAmount {
                field: "commercial.amount",
            });
        }
        Ok(Self { amount, currency })
    }

    /// Parses a fixed two-decimal amount string with a project currency.
    pub fn parse(text: &str, currency: CurrencyCode) -> Result<Self> {
        let amount = Amount::parse(text)
            .map_err(|_| ConstructionProjectError::EmptyField("commercial.amount"))?;
        Self::new(amount, currency)
    }

    /// Validates this amount against the project charter currency.
    pub fn validate_currency(
        &self,
        field: &'static str,
        charter_currency: &CurrencyCode,
    ) -> Result<()> {
        if &self.currency != charter_currency {
            return Err(ConstructionProjectError::CurrencyMismatch {
                field,
                expected: charter_currency.as_str().to_owned(),
                actual: self.currency.as_str().to_owned(),
            });
        }
        if self.amount.0 <= 0 {
            return Err(ConstructionProjectError::NonPositiveAmount { field });
        }
        Ok(())
    }

    /// Checked signed difference from another amount in the same currency.
    pub fn checked_difference(
        &self,
        other: &Self,
        field: &'static str,
        charter_currency: &CurrencyCode,
    ) -> Result<Amount> {
        self.validate_currency(field, charter_currency)?;
        other.validate_currency(field, charter_currency)?;
        self.amount
            .0
            .checked_sub(other.amount.0)
            .map(Amount)
            .ok_or(ConstructionProjectError::AmountOverflow { field })
    }
}

impl serde::Serialize for CommercialAmount {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("CommercialAmount", 2)?;
        state.serialize_field("amount", &self.amount.to_decimal_string())?;
        state.serialize_field("currency", &self.currency)?;
        state.end()
    }
}

impl<'de> serde::Deserialize<'de> for CommercialAmount {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Wire {
            amount: String,
            currency: CurrencyCode,
        }

        let wire = Wire::deserialize(deserializer)?;
        CommercialAmount::parse(&wire.amount, wire.currency).map_err(serde::de::Error::custom)
    }
}

/// Package supplier candidate.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SupplierCandidate {
    /// Stable supplier organization id.
    pub supplier: String,
    /// True when the supplier is still eligible for award.
    pub awardable: bool,
    /// Qualification or rejection note.
    pub note: String,
    /// Candidate evidence references.
    pub evidence: Vec<ExternalRef>,
}

impl SupplierCandidate {
    /// Builds an awardable supplier candidate.
    #[must_use]
    pub fn new(supplier: impl Into<String>, note: impl Into<String>) -> Self {
        Self {
            supplier: supplier.into(),
            awardable: true,
            note: note.into(),
            evidence: Vec::new(),
        }
    }

    /// Marks the candidate rejected for award.
    #[must_use]
    pub fn rejected(mut self, note: impl Into<String>) -> Self {
        self.awardable = false;
        self.note = note.into();
        self
    }

    /// Adds candidate evidence.
    #[must_use]
    pub fn with_evidence(mut self, evidence: ExternalRef) -> Self {
        self.evidence.push(evidence);
        self
    }

    fn validate(&self) -> Result<()> {
        if self.supplier.trim().is_empty() {
            return Err(ConstructionProjectError::EmptyField("supplier.supplier"));
        }
        if self.note.trim().is_empty() {
            return Err(ConstructionProjectError::EmptyField("supplier.note"));
        }
        if self.evidence.is_empty() {
            return Err(ConstructionProjectError::EmptyCollection(
                "supplier.evidence",
            ));
        }
        Ok(())
    }
}

/// Stable work-package procurement identity and basis.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WorkPackage {
    /// Stable construction project identity.
    pub project: ProjectId,
    /// Stable package control id.
    pub control: ControlId,
    /// Human-facing package name.
    pub name: String,
    /// Scope inclusions.
    pub scope_inclusions: Vec<String>,
    /// Scope exclusions.
    pub scope_exclusions: Vec<String>,
    /// Exposed package interfaces.
    pub interfaces: Vec<ControlId>,
    /// Design input controls required before inquiry.
    pub design_inputs: Vec<ControlId>,
    /// Date when inquiry must be sent.
    pub inquiry_due_on: Date,
    /// Date when award must be decided.
    pub award_due_on: Date,
    /// Date when production or delivery needs the package.
    pub need_on: Date,
    /// Procurement owner role.
    pub procurement_owner: RoleId,
    /// Award authority role.
    pub award_authority: RoleId,
    /// Target commercial amount.
    pub target_amount: CommercialAmount,
    /// Supplier candidates.
    pub supplier_candidates: Vec<SupplierCandidate>,
    /// Reference-only package and inquiry basis evidence.
    pub evidence: Vec<ExternalRef>,
}

impl WorkPackage {
    /// Builds a work package with required dates and commercial target.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        project: ProjectId,
        control: ControlId,
        name: impl Into<String>,
        procurement_owner: RoleId,
        award_authority: RoleId,
        inquiry_due_on: Date,
        award_due_on: Date,
        need_on: Date,
        target_amount: CommercialAmount,
    ) -> Self {
        Self {
            project,
            control,
            name: name.into(),
            scope_inclusions: Vec::new(),
            scope_exclusions: Vec::new(),
            interfaces: Vec::new(),
            design_inputs: Vec::new(),
            inquiry_due_on,
            award_due_on,
            need_on,
            procurement_owner,
            award_authority,
            target_amount,
            supplier_candidates: Vec::new(),
            evidence: Vec::new(),
        }
    }

    /// Adds scope included in the package.
    #[must_use]
    pub fn includes(mut self, scope: impl Into<String>) -> Self {
        self.scope_inclusions.push(scope.into());
        self
    }

    /// Adds scope excluded from the package.
    #[must_use]
    pub fn excludes(mut self, scope: impl Into<String>) -> Self {
        self.scope_exclusions.push(scope.into());
        self
    }

    /// Adds an exposed interface control.
    #[must_use]
    pub fn exposes_interface(mut self, interface: ControlId) -> Self {
        self.interfaces.push(interface);
        self
    }

    /// Adds a design input control required for inquiry.
    #[must_use]
    pub fn requires_design_input(mut self, design_input: ControlId) -> Self {
        self.design_inputs.push(design_input);
        self
    }

    /// Adds a supplier candidate.
    #[must_use]
    pub fn with_supplier(mut self, candidate: SupplierCandidate) -> Self {
        self.supplier_candidates.push(candidate);
        self
    }

    /// Adds reference-only package evidence.
    #[must_use]
    pub fn with_evidence(mut self, evidence: ExternalRef) -> Self {
        self.evidence.push(evidence);
        self
    }

    /// Validates package identity, basis, candidates, dates, and currency.
    pub fn validate(&self, charter_currency: &CurrencyCode) -> Result<()> {
        if self.name.trim().is_empty() {
            return Err(ConstructionProjectError::EmptyField("work_package.name"));
        }
        if self.scope_inclusions.is_empty() {
            return Err(ConstructionProjectError::EmptyCollection(
                "work_package.scope_inclusions",
            ));
        }
        if self.design_inputs.is_empty() {
            return Err(ConstructionProjectError::EmptyCollection(
                "work_package.design_inputs",
            ));
        }
        if self.supplier_candidates.is_empty() {
            return Err(ConstructionProjectError::EmptyCollection(
                "work_package.supplier_candidates",
            ));
        }
        if self.evidence.is_empty() {
            return Err(ConstructionProjectError::EmptyCollection(
                "work_package.evidence",
            ));
        }
        if self.inquiry_due_on > self.award_due_on {
            return Err(ConstructionProjectError::EmptyField(
                "work_package.inquiry_due_on",
            ));
        }
        if self.award_due_on > self.need_on {
            return Err(ConstructionProjectError::EmptyField(
                "work_package.award_due_on",
            ));
        }
        self.target_amount
            .validate_currency("work_package.target_amount", charter_currency)?;

        let mut seen = BTreeSet::new();
        for candidate in &self.supplier_candidates {
            candidate.validate()?;
            if !seen.insert(candidate.supplier.clone()) {
                return Err(ConstructionProjectError::DuplicateId {
                    kind: "supplier",
                    id: candidate.supplier.clone(),
                });
            }
        }
        Ok(())
    }

    pub(crate) fn candidate(&self, supplier: &str) -> Option<&SupplierCandidate> {
        self.supplier_candidates
            .iter()
            .find(|candidate| candidate.supplier == supplier)
    }
}

pub(crate) mod amount_serde {
    use serde::Deserialize;
    use sim_ledger::Amount;

    pub(crate) fn serialize<S>(amount: &Amount, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&amount.to_decimal_string())
    }

    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<Amount, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let text = String::deserialize(deserializer)?;
        Amount::parse(&text).map_err(serde::de::Error::custom)
    }
}
