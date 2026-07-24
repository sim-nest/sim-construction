//! Deterministic readiness checks for construction charters.

use crate::{EvidenceState, ProjectCharter};

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
    require_text(charter.project.as_str(), "project", &mut missing_fields);
    require_text(charter.control.as_str(), "control", &mut missing_fields);
    require_text(&charter.name, "name", &mut missing_fields);
    require_text(
        &charter.customer_outcome,
        "customer_outcome",
        &mut missing_fields,
    );
    require_text(
        &charter.procurement_form,
        "procurement_form",
        &mut missing_fields,
    );
    require_list(
        &charter.property_constraints,
        "property_constraints",
        &mut missing_fields,
    );
    require_list(
        &charter.product_constraints,
        "product_constraints",
        &mut missing_fields,
    );
    require_list(&charter.objectives, "objectives", &mut missing_fields);
    require_list(
        &charter.non_negotiables,
        "non_negotiables",
        &mut missing_fields,
    );
    require_list(
        &charter.target_outcomes,
        "target_outcomes",
        &mut missing_fields,
    );
    require_list(
        &charter.reference_criteria,
        "reference_criteria",
        &mut missing_fields,
    );
    require_text(charter.currency.as_str(), "currency", &mut missing_fields);
    if charter.accepted_by.is_none() {
        missing_fields.push("accepted_by".to_owned());
    }
    if charter.accepted_on.is_none() {
        missing_fields.push("accepted_on".to_owned());
    }
    if charter.source_refs.is_empty() {
        missing_fields.push("source_refs".to_owned());
    }

    let state = if missing_fields.is_empty() {
        EvidenceState::Accepted
    } else if charter.source_refs.is_empty() {
        EvidenceState::Reported
    } else {
        EvidenceState::Evidenced
    };

    CharterReadiness {
        as_of_seq,
        state,
        missing_fields,
        evidence_refs: charter.source_refs.len(),
    }
}

fn require_text(value: &str, field: &str, missing_fields: &mut Vec<String>) {
    if value.trim().is_empty() {
        missing_fields.push(field.to_owned());
    }
}

fn require_list(values: &[String], field: &str, missing_fields: &mut Vec<String>) {
    if values.is_empty() || values.iter().any(|value| value.trim().is_empty()) {
        missing_fields.push(field.to_owned());
    }
}
