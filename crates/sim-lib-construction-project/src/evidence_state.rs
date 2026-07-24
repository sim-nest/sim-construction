//! Shared evidence states and validity windows for construction controls.

use time::Date;

/// Evidence state used by construction control summaries.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
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

impl EvidenceState {
    /// Returns true when this state can satisfy an evidence-required obligation.
    #[must_use]
    pub fn satisfies_required_evidence(self) -> bool {
        self == Self::Accepted
    }
}

/// Optional validity window for evidence-backed construction facts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EvidenceValidity {
    /// First date on which the evidence is valid.
    pub valid_from: Option<Date>,
    /// Last date on which the evidence remains valid.
    pub valid_until: Option<Date>,
}

impl EvidenceValidity {
    /// Builds an unbounded validity window.
    #[must_use]
    pub fn unbounded() -> Self {
        Self {
            valid_from: None,
            valid_until: None,
        }
    }

    /// Builds a bounded validity window.
    #[must_use]
    pub fn new(valid_from: Option<Date>, valid_until: Option<Date>) -> Self {
        Self {
            valid_from,
            valid_until,
        }
    }

    /// Returns true when `date` falls inside the validity window.
    #[must_use]
    pub fn contains(self, date: Date) -> bool {
        self.valid_from.is_none_or(|from| date >= from)
            && self.valid_until.is_none_or(|until| date <= until)
    }
}
