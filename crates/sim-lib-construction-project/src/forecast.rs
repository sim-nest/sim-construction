//! Method-bearing forecast-consequence facts for construction control.

use sim_lib_doc_core::ExternalRef;
use time::Date;

use crate::{BaselineId, CommercialAmount, ConstructionProjectError, ControlId, ProjectId, Result};

/// Construction outcome lane affected by an uncertain event.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum ForecastConsequenceKind {
    /// Schedule interval consequence.
    Time,
    /// Exact commercial amount consequence.
    Amount,
    /// Physical safety consequence.
    Safety,
    /// Product or process quality consequence.
    Quality,
    /// Environmental consequence.
    Environment,
    /// Sustainability target consequence.
    Sustainability,
    /// People or organization consequence.
    People,
    /// Property, site, or neighborhood consequence.
    Place,
    /// Customer outcome consequence.
    Customer,
}

/// Typed value of a forecast consequence.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ForecastValue {
    /// Inclusive local interval affected by the event.
    TimeInterval {
        /// Forecast interval start.
        start_on: Date,
        /// Forecast interval finish.
        finish_on: Date,
    },
    /// Exact fixed-decimal amount and currency.
    Amount(CommercialAmount),
    /// Qualitative consequence statement.
    Qualitative(String),
    /// Quantified non-monetary consequence under an open unit.
    Quantified {
        /// Observed or forecast integer value.
        value: i64,
        /// Open unit name.
        unit: String,
    },
}

/// Method and accepted as-of basis for a forecast consequence.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ForecastBasis {
    /// Accepted project baseline used by the method.
    pub baseline: BaselineId,
    /// Named forecasting or assessment method.
    pub method: String,
    /// Project fact sequence used by the method.
    pub as_of_seq: u64,
    /// Calendar date represented by the method input.
    pub as_of_date: Date,
}

impl ForecastBasis {
    /// Builds explicit forecast method and as-of provenance.
    #[must_use]
    pub fn new(
        baseline: BaselineId,
        method: impl Into<String>,
        as_of_seq: u64,
        as_of_date: Date,
    ) -> Self {
        Self {
            baseline,
            method: method.into(),
            as_of_seq,
            as_of_date,
        }
    }

    fn validate(&self, fact_seq: u64) -> Result<()> {
        if self.method.trim().is_empty() {
            return Err(ConstructionProjectError::EmptyField(
                "forecast.basis.method",
            ));
        }
        if self.as_of_seq == 0 || self.as_of_seq > fact_seq {
            return Err(ConstructionProjectError::InvalidSequence {
                field: "forecast.basis.as_of_seq",
                sequence: self.as_of_seq,
            });
        }
        Ok(())
    }
}

/// Current method-bearing forecast consequence fact.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ForecastConsequence {
    /// Stable construction project identity.
    pub project: ProjectId,
    /// Stable forecast-consequence control id.
    pub control: ControlId,
    /// Risk or opportunity control producing this consequence.
    pub uncertainty: ControlId,
    /// Current project fact sequence for this consequence.
    pub fact_seq: u64,
    /// Scenario kept separate during exposure aggregation.
    pub scenario: ControlId,
    /// Outcome lane.
    pub kind: ForecastConsequenceKind,
    /// Typed forecast value.
    pub value: ForecastValue,
    /// Explicit method and as-of basis.
    pub basis: ForecastBasis,
    /// Project controls affected by the consequence.
    pub affected_control_ids: Vec<ControlId>,
    /// Parent summary containing this consequence, when present.
    pub parent: Option<ControlId>,
    /// Child consequences summarized by this fact.
    pub summarizes: Vec<ControlId>,
    /// Correlation group retained as an annotation rather than probability math.
    pub correlation: Option<ControlId>,
    /// Reference-only supporting evidence.
    pub evidence: Vec<ExternalRef>,
}

impl ForecastConsequence {
    /// Builds a method-bearing forecast-consequence fact.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        project: ProjectId,
        control: ControlId,
        uncertainty: ControlId,
        fact_seq: u64,
        scenario: ControlId,
        kind: ForecastConsequenceKind,
        value: ForecastValue,
        basis: ForecastBasis,
    ) -> Self {
        Self {
            project,
            control,
            uncertainty,
            fact_seq,
            scenario,
            kind,
            value,
            basis,
            affected_control_ids: Vec::new(),
            parent: None,
            summarizes: Vec::new(),
            correlation: None,
            evidence: Vec::new(),
        }
    }

    /// Adds an affected project control.
    #[must_use]
    pub fn affects(mut self, control: ControlId) -> Self {
        self.affected_control_ids.push(control);
        self
    }

    /// Marks this consequence as a child of a higher-level summary.
    #[must_use]
    pub fn with_parent(mut self, parent: ControlId) -> Self {
        self.parent = Some(parent);
        self
    }

    /// Declares a child consequence represented by this parent summary.
    #[must_use]
    pub fn summarizes(mut self, child: ControlId) -> Self {
        self.summarizes.push(child);
        self
    }

    /// Adds a correlation group without converting it to a probability.
    #[must_use]
    pub fn correlated_with(mut self, group: ControlId) -> Self {
        self.correlation = Some(group);
        self
    }

    /// Adds reference-only supporting evidence.
    #[must_use]
    pub fn with_evidence(mut self, evidence: ExternalRef) -> Self {
        self.evidence.push(evidence);
        self
    }

    /// Validates kind/value agreement, method provenance, and required evidence.
    pub fn validate(&self) -> Result<()> {
        if self.fact_seq == 0 {
            return Err(ConstructionProjectError::InvalidSequence {
                field: "forecast.fact_seq",
                sequence: self.fact_seq,
            });
        }
        self.basis.validate(self.fact_seq)?;
        if self.affected_control_ids.is_empty() {
            return Err(ConstructionProjectError::EmptyCollection(
                "forecast.affected_control_ids",
            ));
        }
        if self.evidence.is_empty() {
            return Err(ConstructionProjectError::EmptyCollection(
                "forecast.evidence",
            ));
        }
        if self.parent.is_some() && !self.summarizes.is_empty() {
            return Err(ConstructionProjectError::UncertaintyDerivation {
                control: self.control.clone(),
                reason: "a forecast consequence cannot be both parent and child",
            });
        }
        match (&self.kind, &self.value) {
            (
                ForecastConsequenceKind::Time,
                ForecastValue::TimeInterval {
                    start_on,
                    finish_on,
                },
            ) if start_on <= finish_on => Ok(()),
            (ForecastConsequenceKind::Amount, ForecastValue::Amount(amount))
                if amount.amount.0 > 0 =>
            {
                Ok(())
            }
            (
                ForecastConsequenceKind::Safety
                | ForecastConsequenceKind::Quality
                | ForecastConsequenceKind::Environment
                | ForecastConsequenceKind::Sustainability
                | ForecastConsequenceKind::People
                | ForecastConsequenceKind::Place
                | ForecastConsequenceKind::Customer,
                ForecastValue::Qualitative(statement),
            ) if !statement.trim().is_empty() => Ok(()),
            (
                ForecastConsequenceKind::Safety
                | ForecastConsequenceKind::Quality
                | ForecastConsequenceKind::Environment
                | ForecastConsequenceKind::Sustainability
                | ForecastConsequenceKind::People
                | ForecastConsequenceKind::Place
                | ForecastConsequenceKind::Customer,
                ForecastValue::Quantified { unit, .. },
            ) if !unit.trim().is_empty() => Ok(()),
            _ => Err(ConstructionProjectError::ForecastValueMismatch {
                consequence: self.control.clone(),
                kind: self.kind,
            }),
        }
    }
}
