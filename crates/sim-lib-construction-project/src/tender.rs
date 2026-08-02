//! Tender comparison facts for construction work packages.

use sim_lib_doc_core::ExternalRef;

use crate::{CommercialAmount, ControlId};

/// Tender scope compliance statement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ScopeCompliance {
    /// Tender complies with the inquiry scope.
    Compliant,
    /// Tender has reservations against the inquiry scope.
    Reserved,
    /// Tender offers an alternative that needs an authority decision.
    Alternative,
    /// Tender is outside the inquiry scope.
    NonCompliant,
}

/// Tender qualification statement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TenderQualification {
    /// Supplier is qualified for the package.
    Qualified,
    /// Supplier qualification is conditional.
    Conditional,
    /// Supplier is not qualified.
    Rejected,
}

/// One tender comparison fact. Competing and corrected facts are preserved.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TenderComparison {
    /// Stable tender control id.
    pub control: ControlId,
    /// Package being tendered.
    pub package: ControlId,
    /// Supplier candidate.
    pub supplier: String,
    /// Scope compliance statement.
    pub scope_compliance: ScopeCompliance,
    /// Supplier reservations.
    pub reservations: Vec<String>,
    /// Supplier alternatives.
    pub alternatives: Vec<String>,
    /// Tender lead time in calendar days.
    pub lead_time_days: u16,
    /// Capacity statement.
    pub capacity: String,
    /// Qualification statement.
    pub qualification: TenderQualification,
    /// Commercial amount.
    pub commercial_amount: CommercialAmount,
    /// Tender evidence.
    pub evidence: Vec<ExternalRef>,
    /// Optional prior tender corrected by this tender fact.
    pub supersedes: Option<ControlId>,
}

impl TenderComparison {
    /// Builds a tender comparison fact.
    pub fn new(
        control: ControlId,
        package: ControlId,
        supplier: impl Into<String>,
        commercial_amount: CommercialAmount,
    ) -> Self {
        Self {
            control,
            package,
            supplier: supplier.into(),
            scope_compliance: ScopeCompliance::Compliant,
            reservations: Vec::new(),
            alternatives: Vec::new(),
            lead_time_days: 0,
            capacity: String::new(),
            qualification: TenderQualification::Qualified,
            commercial_amount,
            evidence: Vec::new(),
            supersedes: None,
        }
    }

    /// Sets scope compliance.
    #[must_use]
    pub fn with_scope_compliance(mut self, compliance: ScopeCompliance) -> Self {
        self.scope_compliance = compliance;
        self
    }

    /// Adds a reservation.
    #[must_use]
    pub fn with_reservation(mut self, reservation: impl Into<String>) -> Self {
        self.reservations.push(reservation.into());
        self
    }

    /// Adds an alternative.
    #[must_use]
    pub fn with_alternative(mut self, alternative: impl Into<String>) -> Self {
        self.alternatives.push(alternative.into());
        self
    }

    /// Sets tender lead time.
    #[must_use]
    pub fn with_lead_time_days(mut self, days: u16) -> Self {
        self.lead_time_days = days;
        self
    }

    /// Sets capacity statement.
    #[must_use]
    pub fn with_capacity(mut self, capacity: impl Into<String>) -> Self {
        self.capacity = capacity.into();
        self
    }

    /// Sets qualification statement.
    #[must_use]
    pub fn with_qualification(mut self, qualification: TenderQualification) -> Self {
        self.qualification = qualification;
        self
    }

    /// Adds tender evidence.
    #[must_use]
    pub fn with_evidence(mut self, evidence: ExternalRef) -> Self {
        self.evidence.push(evidence);
        self
    }

    /// Marks the prior tender corrected by this tender fact.
    #[must_use]
    pub fn supersedes(mut self, tender: ControlId) -> Self {
        self.supersedes = Some(tender);
        self
    }

    /// True when the tender can be included in an award comparison.
    #[must_use]
    pub fn is_comparable(&self) -> bool {
        self.scope_compliance == ScopeCompliance::Compliant
            && self.reservations.is_empty()
            && self.alternatives.is_empty()
            && self.lead_time_days > 0
            && !self.capacity.trim().is_empty()
            && self.qualification == TenderQualification::Qualified
            && !self.evidence.is_empty()
    }
}
