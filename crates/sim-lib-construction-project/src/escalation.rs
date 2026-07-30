//! Deterministic attention recommendations for construction uncertainties.

use std::collections::BTreeMap;

use time::{Date, Duration};

use crate::{
    ConstructionProjectError, ControlId, ExposureReport, ForecastConsequence, ProjectSnapshot,
    Result, RoleId, UncertaintyKind, UncertaintyRecord, UncertaintyState,
};

/// Rule that recommends accountable attention without making a decision.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EscalationReason {
    /// An open response passed its due date.
    OverdueResponse {
        /// Missed response date.
        due_on: Date,
    },
    /// An observed project fact crossed the named trigger.
    CrossedTrigger {
        /// Trigger observation sequence.
        fact_seq: u64,
    },
    /// A current forecast consequence superseded an earlier fact.
    ChangedConsequence {
        /// Changed forecast-consequence control.
        consequence: ControlId,
    },
    /// No response decision authority is assigned.
    MissingAuthority,
    /// The preparation window for an upcoming decision has started.
    DecisionLeadTime {
        /// Date on which preparation had to begin.
        attention_on: Date,
        /// Date on which the decision is needed.
        decision_due_on: Date,
    },
    /// A likelihood or impact rating predates the current uncertainty fact.
    StaleRating,
    /// A forecast consequence affects an accepted critical-path join.
    CriticalPathConsequence,
    /// The risk was realized or opportunity captured and remains under control.
    EventOccurred,
}

/// Relative attention class used only for deterministic queue ordering.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AttentionLevel {
    /// Immediate accountable review.
    Immediate,
    /// High attention in the active control cycle.
    High,
    /// Review and refresh supporting facts.
    Review,
}

/// Attention recommendation; deliberately contains no decision or resolution.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EscalationRecommendation {
    /// Risk or opportunity control needing attention.
    pub uncertainty: ControlId,
    /// Risk or opportunity kind.
    pub kind: UncertaintyKind,
    /// Role accountable for preparing the response.
    pub owner: RoleId,
    /// Assigned decision authority, when present.
    pub recommended_to: Option<RoleId>,
    /// Derived attention class.
    pub attention: AttentionLevel,
    /// Stable ordered reasons for attention.
    pub reasons: Vec<EscalationReason>,
    /// Earliest relevant response or decision date.
    pub attention_due_on: Date,
    /// Human-facing recommendation, not a decision.
    pub recommendation: String,
}

/// Derives a deterministic high-attention queue from the current exposure report.
pub fn derive_escalation_queue(
    snapshot: &ProjectSnapshot,
    exposure: &ExposureReport,
    uncertainties: &[UncertaintyRecord],
    consequences: &[ForecastConsequence],
    as_of_date: Date,
) -> Result<Vec<EscalationRecommendation>> {
    if exposure.as_of_seq != snapshot.through_seq {
        return Err(ConstructionProjectError::UncertaintyDerivation {
            control: ControlId::new(exposure.schedule_baseline.as_str())
                .expect("baseline ids are valid control ids"),
            reason: "exposure report does not match the current snapshot",
        });
    }
    let mut exposure_by_id = BTreeMap::new();
    for item in &exposure.queue {
        if exposure_by_id
            .insert(item.uncertainty.clone(), item)
            .is_some()
        {
            return Err(ConstructionProjectError::DuplicateId {
                kind: "exposure_queue.uncertainty",
                id: item.uncertainty.as_str().to_owned(),
            });
        }
    }
    let mut consequence_by_id = BTreeMap::new();
    for consequence in consequences {
        consequence.validate()?;
        validate_current(
            snapshot,
            &consequence.project,
            &consequence.control,
            consequence.fact_seq,
        )?;
        if consequence_by_id
            .insert(consequence.control.clone(), consequence)
            .is_some()
        {
            return Err(ConstructionProjectError::DuplicateId {
                kind: "escalation.consequence",
                id: consequence.control.as_str().to_owned(),
            });
        }
    }
    let mut queue = Vec::new();

    for uncertainty in uncertainties {
        uncertainty.validate()?;
        validate_current(
            snapshot,
            &uncertainty.project,
            &uncertainty.control,
            uncertainty.fact_seq,
        )?;
        if matches!(uncertainty.state, UncertaintyState::Closed { .. }) {
            continue;
        }
        let exposure_item = exposure_by_id.get(&uncertainty.control).ok_or_else(|| {
            ConstructionProjectError::UncertaintyDerivation {
                control: uncertainty.control.clone(),
                reason: "current uncertainty is absent from the exposure queue",
            }
        })?;
        if exposure_item.kind != uncertainty.kind
            || exposure_item.state != uncertainty.state
            || exposure_item.baseline != uncertainty.baseline
        {
            return Err(ConstructionProjectError::UncertaintyDerivation {
                control: uncertainty.control.clone(),
                reason: "uncertainty and exposure queue facts do not match",
            });
        }

        let response_open = uncertainty.response.state.is_open();
        let mut reasons = Vec::new();
        if response_open && uncertainty.response.due_on < as_of_date {
            reasons.push(EscalationReason::OverdueResponse {
                due_on: uncertainty.response.due_on,
            });
        }
        if response_open && let Some(fact_seq) = uncertainty.response.trigger_crossed_seq {
            reasons.push(EscalationReason::CrossedTrigger { fact_seq });
        }
        for consequence_id in &exposure_item.consequences {
            let consequence = consequence_by_id.get(consequence_id).ok_or_else(|| {
                ConstructionProjectError::UncertaintyDerivation {
                    control: consequence_id.clone(),
                    reason: "exposure queue references a missing consequence",
                }
            })?;
            if consequence.uncertainty != uncertainty.control {
                return Err(ConstructionProjectError::UncertaintyDerivation {
                    control: consequence.control.clone(),
                    reason: "exposure consequence belongs to another uncertainty",
                });
            }
            if snapshot
                .superseded
                .get(&consequence.control)
                .is_some_and(|prior| !prior.is_empty())
            {
                reasons.push(EscalationReason::ChangedConsequence {
                    consequence: consequence.control.clone(),
                });
            }
        }
        if response_open && uncertainty.response.authority.is_none() {
            reasons.push(EscalationReason::MissingAuthority);
        }
        let attention_on = uncertainty.response.decision_due_on
            - Duration::days(i64::from(uncertainty.response.decision_lead_days));
        if response_open && attention_on <= as_of_date {
            reasons.push(EscalationReason::DecisionLeadTime {
                attention_on,
                decision_due_on: uncertainty.response.decision_due_on,
            });
        }
        if exposure_item.stale_rating {
            reasons.push(EscalationReason::StaleRating);
        }
        if exposure_item.critical_path {
            reasons.push(EscalationReason::CriticalPathConsequence);
        }
        if matches!(
            uncertainty.state,
            UncertaintyState::RiskRealized { .. } | UncertaintyState::OpportunityCaptured { .. }
        ) {
            reasons.push(EscalationReason::EventOccurred);
        }
        if reasons.is_empty() {
            continue;
        }
        reasons.sort_by_key(reason_rank);
        reasons.dedup();
        let attention = attention_level(&reasons);
        queue.push(EscalationRecommendation {
            uncertainty: uncertainty.control.clone(),
            kind: uncertainty.kind,
            owner: uncertainty.owner.clone(),
            recommended_to: uncertainty.response.authority.clone(),
            attention,
            reasons,
            attention_due_on: uncertainty
                .response
                .due_on
                .min(uncertainty.response.decision_due_on),
            recommendation: "review current evidence and obtain an accountable decision".to_owned(),
        });
    }

    queue.sort_by(|left, right| {
        attention_rank(left.attention)
            .cmp(&attention_rank(right.attention))
            .then(left.attention_due_on.cmp(&right.attention_due_on))
            .then(left.uncertainty.cmp(&right.uncertainty))
    });
    Ok(queue)
}

fn validate_current(
    snapshot: &ProjectSnapshot,
    project: &crate::ProjectId,
    control: &ControlId,
    fact_seq: u64,
) -> Result<()> {
    if project != &snapshot.project
        || snapshot
            .current_fact(control)
            .is_none_or(|fact| fact.seq != fact_seq)
    {
        return Err(ConstructionProjectError::UncertaintyDerivation {
            control: control.clone(),
            reason: "escalation input does not match the current project fact",
        });
    }
    Ok(())
}

fn attention_level(reasons: &[EscalationReason]) -> AttentionLevel {
    if reasons.iter().any(|reason| {
        matches!(
            reason,
            EscalationReason::CrossedTrigger { .. }
                | EscalationReason::CriticalPathConsequence
                | EscalationReason::EventOccurred
        )
    }) {
        AttentionLevel::Immediate
    } else if reasons
        .iter()
        .any(|reason| !matches!(reason, EscalationReason::StaleRating))
    {
        AttentionLevel::High
    } else {
        AttentionLevel::Review
    }
}

fn attention_rank(attention: AttentionLevel) -> u8 {
    match attention {
        AttentionLevel::Immediate => 0,
        AttentionLevel::High => 1,
        AttentionLevel::Review => 2,
    }
}

fn reason_rank(reason: &EscalationReason) -> u8 {
    match reason {
        EscalationReason::EventOccurred => 0,
        EscalationReason::CrossedTrigger { .. } => 1,
        EscalationReason::CriticalPathConsequence => 2,
        EscalationReason::OverdueResponse { .. } => 3,
        EscalationReason::ChangedConsequence { .. } => 4,
        EscalationReason::MissingAuthority => 5,
        EscalationReason::DecisionLeadTime { .. } => 6,
        EscalationReason::StaleRating => 7,
    }
}
