//! Deterministic readiness checks for construction charters.

use crate::ProjectCharter;

/// Evidence state used by construction control summaries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EvidenceState {
    /// No usable project information is present.
    Missing,
    /// Some information is reported, but mandatory proof is incomplete.
    Reported,
    /// Evidence is present but not accepted by an accountable role.
    Evidenced,
    /// Mandatory information is current, evidenced, and accepted.
    Accepted,
    /// Evidence has been rejected by an accountable role.
    Rejected,
    /// Evidence is present but outside its valid window.
    Expired,
    /// Competing accepted records require human resolution.
    Conflicted,
}

/// Readiness result for a project charter at an event sequence.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CharterReadiness {
    /// Sequence used to derive this summary.
    pub as_of_seq: u64,
    /// Current evidence state.
    pub state: EvidenceState,
    /// Mandatory field names that are still missing.
    pub missing_fields: Vec<String>,
    /// Number of reference-only evidence links attached to the charter.
    pub evidence_refs: usize,
}

impl CharterReadiness {
    /// Returns true when the charter can support downstream planning work.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.state == EvidenceState::Accepted && self.missing_fields.is_empty()
    }
}

impl ProjectCharter {
    /// Evaluates this charter at an event sequence.
    #[must_use]
    pub fn readiness(&self, as_of_seq: u64) -> CharterReadiness {
        evaluate_charter(self, as_of_seq)
    }
}

/// Evaluates the minimum accepted-charter fields.
#[must_use]
pub fn evaluate_charter(charter: &ProjectCharter, as_of_seq: u64) -> CharterReadiness {
    let mut missing_fields = Vec::new();
    require_text(&charter.project.0, "project", &mut missing_fields);
    require_text(&charter.name, "name", &mut missing_fields);
    require_text(
        &charter.customer_intent,
        "customer_intent",
        &mut missing_fields,
    );
    require_text(
        &charter.delivery_model,
        "delivery_model",
        &mut missing_fields,
    );
    require_text(&charter.currency, "currency", &mut missing_fields);
    if charter.accepted_by.is_none() {
        missing_fields.push("accepted_by".to_owned());
    }
    if charter
        .accepted_on
        .as_deref()
        .is_none_or(|accepted_on| accepted_on.trim().is_empty())
    {
        missing_fields.push("accepted_on".to_owned());
    }
    if charter.evidence.is_empty() {
        missing_fields.push("evidence".to_owned());
    }

    let state = if missing_fields.is_empty() {
        EvidenceState::Accepted
    } else if charter.evidence.is_empty() {
        EvidenceState::Reported
    } else {
        EvidenceState::Evidenced
    };

    CharterReadiness {
        as_of_seq,
        state,
        missing_fields,
        evidence_refs: charter.evidence.len(),
    }
}

fn require_text(value: &str, field: &str, missing_fields: &mut Vec<String>) {
    if value.trim().is_empty() {
        missing_fields.push(field.to_owned());
    }
}
