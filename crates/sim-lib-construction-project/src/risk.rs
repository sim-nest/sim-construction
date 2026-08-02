//! Construction risk and opportunity uncertainty records.
use crate::{BaselineId, ConstructionProjectError, ControlId, ProjectId, Result, RoleId};
use sim_lib_doc_core::ExternalRef;
use time::Date;

/// Kind of uncertainty retained by project control.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum UncertaintyKind {
    /// A possible event with adverse consequences.
    Risk,
    /// A possible event with beneficial consequences.
    Opportunity,
}

/// A rating statement that preserves whether it is qualitative or quantified.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RatingValue {
    /// A label interpreted under the named open rating scheme.
    Qualitative(String),
    /// An exact integer observation with an open unit or scale name.
    Quantified {
        /// Observed value.
        value: i64,
        /// Open unit or scale, such as `percent` or `five-point-scale`.
        unit: String,
    },
}

/// Open, method-bearing likelihood or impact rating.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OpenRating {
    /// Open scheme name; the construction crate does not prescribe a matrix.
    pub scheme: String,
    /// Typed qualitative or quantified statement.
    pub value: RatingValue,
    /// Project fact sequence on which the rating was based.
    pub as_of_seq: u64,
    /// Calendar date on which the rating was made.
    pub as_of_date: Date,
    /// Named assessment method.
    pub method: String,
}

impl OpenRating {
    /// Builds a qualitative rating under an open scheme.
    #[must_use]
    pub fn qualitative(
        scheme: impl Into<String>,
        label: impl Into<String>,
        as_of_seq: u64,
        as_of_date: Date,
        method: impl Into<String>,
    ) -> Self {
        Self {
            scheme: scheme.into(),
            value: RatingValue::Qualitative(label.into()),
            as_of_seq,
            as_of_date,
            method: method.into(),
        }
    }

    /// Builds a quantified rating under an open scheme and unit.
    #[must_use]
    pub fn quantified(
        scheme: impl Into<String>,
        value: i64,
        unit: impl Into<String>,
        as_of_seq: u64,
        as_of_date: Date,
        method: impl Into<String>,
    ) -> Self {
        Self {
            scheme: scheme.into(),
            value: RatingValue::Quantified {
                value,
                unit: unit.into(),
            },
            as_of_seq,
            as_of_date,
            method: method.into(),
        }
    }

    /// Returns whether this rating predates the current uncertainty fact.
    #[must_use]
    pub fn is_stale_at(&self, current_seq: u64) -> bool {
        self.as_of_seq < current_seq
    }

    fn validate(&self, field: &'static str, current_seq: u64) -> Result<()> {
        if self.scheme.trim().is_empty() {
            return Err(ConstructionProjectError::EmptyField(field));
        }
        if self.method.trim().is_empty() {
            return Err(ConstructionProjectError::EmptyField(
                "uncertainty.rating.method",
            ));
        }
        if self.as_of_seq == 0 || self.as_of_seq > current_seq {
            return Err(ConstructionProjectError::InvalidSequence {
                field: "uncertainty.rating.as_of_seq",
                sequence: self.as_of_seq,
            });
        }
        match &self.value {
            RatingValue::Qualitative(label) if label.trim().is_empty() => Err(
                ConstructionProjectError::EmptyField("uncertainty.rating.qualitative"),
            ),
            RatingValue::Quantified { unit, .. } if unit.trim().is_empty() => Err(
                ConstructionProjectError::EmptyField("uncertainty.rating.unit"),
            ),
            _ => Ok(()),
        }
    }
}

/// Current state of an uncertainty response.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ResponseState {
    /// Response is agreed but has not started.
    Planned,
    /// Response work is underway.
    InProgress,
    /// Response completion is retained by a project fact sequence.
    Completed {
        /// Fact sequence accepting response completion.
        fact_seq: u64,
    },
}

impl ResponseState {
    /// Returns whether accountable response work remains open.
    #[must_use]
    pub fn is_open(self) -> bool {
        !matches!(self, Self::Completed { .. })
    }
}

/// Accountable response, trigger, authority, and decision timing.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UncertaintyResponse {
    /// Response strategy and action.
    pub action: String,
    /// Observable trigger statement.
    pub trigger: String,
    /// Fact sequence at which the trigger was crossed.
    pub trigger_crossed_seq: Option<u64>,
    /// Date by which the response is due.
    pub due_on: Date,
    /// Role authorized to decide the response, when assigned.
    pub authority: Option<RoleId>,
    /// Date by which the decision is needed.
    pub decision_due_on: Date,
    /// Calendar lead time required to prepare the decision.
    pub decision_lead_days: u16,
    /// Current response state.
    pub state: ResponseState,
}

impl UncertaintyResponse {
    /// Builds a planned response.
    #[must_use]
    pub fn new(
        action: impl Into<String>,
        trigger: impl Into<String>,
        due_on: Date,
        decision_due_on: Date,
        decision_lead_days: u16,
    ) -> Self {
        Self {
            action: action.into(),
            trigger: trigger.into(),
            trigger_crossed_seq: None,
            due_on,
            authority: None,
            decision_due_on,
            decision_lead_days,
            state: ResponseState::Planned,
        }
    }

    /// Assigns the response decision authority.
    #[must_use]
    pub fn with_authority(mut self, authority: RoleId) -> Self {
        self.authority = Some(authority);
        self
    }

    /// Records the fact sequence at which the trigger was crossed.
    #[must_use]
    pub fn trigger_crossed_at(mut self, fact_seq: u64) -> Self {
        self.trigger_crossed_seq = Some(fact_seq);
        self
    }

    /// Sets the current response state.
    #[must_use]
    pub fn with_state(mut self, state: ResponseState) -> Self {
        self.state = state;
        self
    }

    fn validate(&self, current_seq: u64) -> Result<()> {
        if self.action.trim().is_empty() {
            return Err(ConstructionProjectError::EmptyField(
                "uncertainty.response.action",
            ));
        }
        if self.trigger.trim().is_empty() {
            return Err(ConstructionProjectError::EmptyField(
                "uncertainty.response.trigger",
            ));
        }
        if self.decision_lead_days == 0 || self.decision_lead_days > 366 {
            return Err(ConstructionProjectError::InvalidDueDatePolicy {
                field: "uncertainty.response.decision_lead_days",
                max_days: 366,
            });
        }
        if self.decision_due_on > self.due_on {
            return Err(ConstructionProjectError::EmptyField(
                "uncertainty.response.decision_due_on",
            ));
        }
        validate_optional_sequence(
            "uncertainty.response.trigger_crossed_seq",
            self.trigger_crossed_seq,
            current_seq,
        )?;
        if let ResponseState::Completed { fact_seq } = self.state {
            validate_optional_sequence(
                "uncertainty.response.completed_seq",
                Some(fact_seq),
                current_seq,
            )?;
        }
        Ok(())
    }
}

/// Lifecycle state of the uncertain event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum UncertaintyState {
    /// The uncertain event has not occurred.
    Open,
    /// A risk event has occurred and is retained by this fact sequence.
    RiskRealized {
        /// Fact sequence recording realization.
        fact_seq: u64,
    },
    /// An opportunity event has been captured and is retained by this fact sequence.
    OpportunityCaptured {
        /// Fact sequence recording capture.
        fact_seq: u64,
    },
    /// The uncertainty was closed by an accountable fact.
    Closed {
        /// Fact sequence recording closure.
        fact_seq: u64,
    },
}

/// Current risk or opportunity fact used by exposure and escalation derivation.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UncertaintyRecord {
    /// Stable construction project identity.
    pub project: ProjectId,
    /// Stable risk or opportunity control id.
    pub control: ControlId,
    /// Risk or opportunity discriminator.
    pub kind: UncertaintyKind,
    /// Current project fact sequence for this record.
    pub fact_seq: u64,
    /// Accepted baseline against which this uncertainty is controlled.
    pub baseline: BaselineId,
    /// Scenario that must remain separate during exposure aggregation.
    pub scenario: ControlId,
    /// Cause statement.
    pub cause: String,
    /// Uncertain event statement.
    pub uncertain_event: String,
    /// Consequence statement.
    pub consequence: String,
    /// Role accountable for the uncertainty.
    pub owner: RoleId,
    /// Accountable response and trigger.
    pub response: UncertaintyResponse,
    /// Affected project controls.
    pub affected_control_ids: Vec<ControlId>,
    /// Reference-only supporting evidence.
    pub evidence: Vec<ExternalRef>,
    /// Open likelihood rating.
    pub likelihood: OpenRating,
    /// Open impact rating.
    pub impact: OpenRating,
    /// Current uncertainty state.
    pub state: UncertaintyState,
}

impl UncertaintyRecord {
    /// Builds an open risk record.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn risk(
        project: ProjectId,
        control: ControlId,
        fact_seq: u64,
        baseline: BaselineId,
        scenario: ControlId,
        cause: impl Into<String>,
        uncertain_event: impl Into<String>,
        consequence: impl Into<String>,
        owner: RoleId,
        response: UncertaintyResponse,
        likelihood: OpenRating,
        impact: OpenRating,
    ) -> Self {
        Self::new(
            project,
            control,
            UncertaintyKind::Risk,
            fact_seq,
            baseline,
            scenario,
            cause,
            uncertain_event,
            consequence,
            owner,
            response,
            likelihood,
            impact,
        )
    }

    /// Builds an open opportunity record.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn opportunity(
        project: ProjectId,
        control: ControlId,
        fact_seq: u64,
        baseline: BaselineId,
        scenario: ControlId,
        cause: impl Into<String>,
        uncertain_event: impl Into<String>,
        consequence: impl Into<String>,
        owner: RoleId,
        response: UncertaintyResponse,
        likelihood: OpenRating,
        impact: OpenRating,
    ) -> Self {
        Self::new(
            project,
            control,
            UncertaintyKind::Opportunity,
            fact_seq,
            baseline,
            scenario,
            cause,
            uncertain_event,
            consequence,
            owner,
            response,
            likelihood,
            impact,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn new(
        project: ProjectId,
        control: ControlId,
        kind: UncertaintyKind,
        fact_seq: u64,
        baseline: BaselineId,
        scenario: ControlId,
        cause: impl Into<String>,
        uncertain_event: impl Into<String>,
        consequence: impl Into<String>,
        owner: RoleId,
        response: UncertaintyResponse,
        likelihood: OpenRating,
        impact: OpenRating,
    ) -> Self {
        Self {
            project,
            control,
            kind,
            fact_seq,
            baseline,
            scenario,
            cause: cause.into(),
            uncertain_event: uncertain_event.into(),
            consequence: consequence.into(),
            owner,
            response,
            affected_control_ids: Vec::new(),
            evidence: Vec::new(),
            likelihood,
            impact,
            state: UncertaintyState::Open,
        }
    }

    /// Adds an affected project control.
    #[must_use]
    pub fn affects(mut self, control: ControlId) -> Self {
        self.affected_control_ids.push(control);
        self
    }

    /// Adds reference-only supporting evidence.
    #[must_use]
    pub fn with_evidence(mut self, evidence: ExternalRef) -> Self {
        self.evidence.push(evidence);
        self
    }

    /// Sets the current lifecycle state.
    #[must_use]
    pub fn with_state(mut self, state: UncertaintyState) -> Self {
        self.state = state;
        self
    }

    /// Returns whether either open rating predates the current fact.
    #[must_use]
    pub fn has_stale_rating(&self) -> bool {
        self.likelihood.is_stale_at(self.fact_seq) || self.impact.is_stale_at(self.fact_seq)
    }

    /// Validates required uncertainty, rating, response, and evidence fields.
    pub fn validate(&self) -> Result<()> {
        if self.fact_seq == 0 {
            return Err(ConstructionProjectError::InvalidSequence {
                field: "uncertainty.fact_seq",
                sequence: self.fact_seq,
            });
        }
        for (field, value) in [
            ("uncertainty.cause", self.cause.as_str()),
            ("uncertainty.uncertain_event", self.uncertain_event.as_str()),
            ("uncertainty.consequence", self.consequence.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(ConstructionProjectError::EmptyField(field));
            }
        }
        if self.affected_control_ids.is_empty() {
            return Err(ConstructionProjectError::EmptyCollection(
                "uncertainty.affected_control_ids",
            ));
        }
        if self.evidence.is_empty() {
            return Err(ConstructionProjectError::EmptyCollection(
                "uncertainty.evidence",
            ));
        }
        self.response.validate(self.fact_seq)?;
        self.likelihood
            .validate("uncertainty.likelihood.scheme", self.fact_seq)?;
        self.impact
            .validate("uncertainty.impact.scheme", self.fact_seq)?;
        match (self.kind, self.state) {
            (UncertaintyKind::Risk, UncertaintyState::OpportunityCaptured { .. })
            | (UncertaintyKind::Opportunity, UncertaintyState::RiskRealized { .. }) => {
                return Err(ConstructionProjectError::UncertaintyStateMismatch {
                    control: self.control.clone(),
                    kind: self.kind,
                    state: self.state,
                });
            }
            _ => {}
        }
        let state_seq = match self.state {
            UncertaintyState::Open => None,
            UncertaintyState::RiskRealized { fact_seq }
            | UncertaintyState::OpportunityCaptured { fact_seq }
            | UncertaintyState::Closed { fact_seq } => Some(fact_seq),
        };
        validate_optional_sequence("uncertainty.state.fact_seq", state_seq, self.fact_seq)
    }
}

fn validate_optional_sequence(
    field: &'static str,
    sequence: Option<u64>,
    current_seq: u64,
) -> Result<()> {
    if sequence.is_some_and(|sequence| sequence == 0 || sequence > current_seq) {
        return Err(ConstructionProjectError::InvalidSequence {
            field,
            sequence: sequence.unwrap_or_default(),
        });
    }
    Ok(())
}
