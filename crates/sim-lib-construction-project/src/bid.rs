//! Bid/no-bid control records for construction project control.

use sim_lib_doc_core::ExternalRef;
use time::Date;

use crate::{ConstructionProjectError, ControlId, ProjectId, Result, RoleId};

/// Accountable bid decision outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BidDecisionKind {
    /// Submit a bid.
    Bid,
    /// Decline the opportunity.
    NoBid,
    /// Submit only under named conditions.
    ConditionalBid,
}

/// Bid/no-bid decision with control basis references.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BidDecision {
    /// Stable construction project identity.
    pub project: ProjectId,
    /// Bid decision control id.
    pub control: ControlId,
    /// Customer intent control used as the demand basis.
    pub intent: ControlId,
    /// Role accountable for the decision.
    pub authority: RoleId,
    /// Role that recorded the decision.
    pub decided_by: RoleId,
    /// Capacity and resource view.
    pub capacity_view: Vec<String>,
    /// Material risks.
    pub risks: Vec<String>,
    /// Material opportunities.
    pub opportunities: Vec<String>,
    /// Bid assumptions.
    pub assumptions: Vec<String>,
    /// Offer validity date, when an offer is made.
    pub valid_until: Option<Date>,
    /// Price basis reference-only evidence.
    pub price_basis: Vec<ExternalRef>,
    /// Schedule basis reference-only evidence.
    pub schedule_basis: Vec<ExternalRef>,
    /// Decision outcome.
    pub decision: BidDecisionKind,
    /// Reference-only evidence for the accountable decision.
    pub evidence: Vec<ExternalRef>,
}

impl BidDecision {
    /// Builds a bid decision.
    #[must_use]
    pub fn new(
        project: ProjectId,
        control: ControlId,
        intent: ControlId,
        authority: RoleId,
        decided_by: RoleId,
        decision: BidDecisionKind,
    ) -> Self {
        Self {
            project,
            control,
            intent,
            authority,
            decided_by,
            capacity_view: Vec::new(),
            risks: Vec::new(),
            opportunities: Vec::new(),
            assumptions: Vec::new(),
            valid_until: None,
            price_basis: Vec::new(),
            schedule_basis: Vec::new(),
            decision,
            evidence: Vec::new(),
        }
    }

    /// Adds one capacity/resource note.
    #[must_use]
    pub fn with_capacity_view(mut self, value: impl Into<String>) -> Self {
        self.capacity_view.push(value.into());
        self
    }

    /// Adds one material risk.
    #[must_use]
    pub fn with_risk(mut self, value: impl Into<String>) -> Self {
        self.risks.push(value.into());
        self
    }

    /// Adds one material opportunity.
    #[must_use]
    pub fn with_opportunity(mut self, value: impl Into<String>) -> Self {
        self.opportunities.push(value.into());
        self
    }

    /// Adds one bid assumption.
    #[must_use]
    pub fn with_assumption(mut self, value: impl Into<String>) -> Self {
        self.assumptions.push(value.into());
        self
    }

    /// Sets offer validity.
    #[must_use]
    pub fn valid_until(mut self, date: Date) -> Self {
        self.valid_until = Some(date);
        self
    }

    /// Adds a price basis reference.
    #[must_use]
    pub fn with_price_basis(mut self, evidence: ExternalRef) -> Self {
        self.price_basis.push(evidence);
        self
    }

    /// Adds a schedule basis reference.
    #[must_use]
    pub fn with_schedule_basis(mut self, evidence: ExternalRef) -> Self {
        self.schedule_basis.push(evidence);
        self
    }

    /// Adds decision evidence.
    #[must_use]
    pub fn with_evidence(mut self, evidence: ExternalRef) -> Self {
        self.evidence.push(evidence);
        self
    }

    /// Validates authority, accountable basis, and offer-basis evidence.
    pub fn validate(&self) -> Result<()> {
        if self.decided_by != self.authority {
            return Err(ConstructionProjectError::DecisionAuthorityMismatch {
                decision: self.control.clone(),
                expected: self.authority.clone(),
                actual: self.decided_by.clone(),
            });
        }
        validate_non_empty(&self.capacity_view, "bid.capacity_view")?;
        validate_non_empty(&self.risks, "bid.risks")?;
        validate_texts(&self.opportunities, "bid.opportunities")?;
        validate_texts(&self.assumptions, "bid.assumptions")?;
        if self.evidence.is_empty() {
            return Err(ConstructionProjectError::EmptyCollection("bid.evidence"));
        }
        if matches!(
            self.decision,
            BidDecisionKind::Bid | BidDecisionKind::ConditionalBid
        ) {
            if self.valid_until.is_none() {
                return Err(ConstructionProjectError::EmptyField("bid.valid_until"));
            }
            if self.price_basis.is_empty() {
                return Err(ConstructionProjectError::EmptyCollection("bid.price_basis"));
            }
            if self.schedule_basis.is_empty() {
                return Err(ConstructionProjectError::EmptyCollection(
                    "bid.schedule_basis",
                ));
            }
        }
        Ok(())
    }

    /// Derives whether the offer basis is still usable.
    pub fn offer_basis_report(&self, as_of_date: Date) -> Result<OfferBasisReport> {
        self.validate()?;
        let expired = self
            .valid_until
            .is_some_and(|valid_until| as_of_date > valid_until);
        Ok(OfferBasisReport {
            bid: self.control.clone(),
            as_of_date,
            expired,
            valid_until: self.valid_until,
            has_price_basis: !self.price_basis.is_empty(),
            has_schedule_basis: !self.schedule_basis.is_empty(),
        })
    }
}

/// Derived offer-basis status.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OfferBasisReport {
    /// Bid decision control id.
    pub bid: ControlId,
    /// Date used for derivation.
    pub as_of_date: Date,
    /// True when the offer basis is past validity.
    pub expired: bool,
    /// Offer validity date.
    pub valid_until: Option<Date>,
    /// True when price basis evidence is attached.
    pub has_price_basis: bool,
    /// True when schedule basis evidence is attached.
    pub has_schedule_basis: bool,
}

fn validate_non_empty(values: &[String], field: &'static str) -> Result<()> {
    if values.is_empty() {
        return Err(ConstructionProjectError::EmptyCollection(field));
    }
    validate_texts(values, field)
}

fn validate_texts(values: &[String], field: &'static str) -> Result<()> {
    if values.iter().any(|value| value.trim().is_empty()) {
        return Err(ConstructionProjectError::EmptyField(field));
    }
    Ok(())
}
