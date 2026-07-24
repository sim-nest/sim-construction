//! Project obligations derived from shared construction requirements.

use crate::{EvidenceValidity, ProjectId, Requirement};

/// Mandatory or optional obligation policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ObligationPolicy {
    /// The obligation must be satisfied unless a valid bounded exception applies.
    Mandatory,
    /// The obligation is reported but does not block the gate.
    Optional,
}

/// A project-scoped obligation using the shared requirement shape.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProjectObligation {
    /// Project identity.
    pub project: ProjectId,
    /// Shared requirement.
    pub requirement: Requirement,
    /// Mandatory or optional gate policy.
    pub policy: ObligationPolicy,
    /// Validity window for evidence satisfying this obligation.
    pub evidence_validity: EvidenceValidity,
}

impl ProjectObligation {
    /// Builds a mandatory project obligation.
    #[must_use]
    pub fn mandatory(project: ProjectId, requirement: Requirement) -> Self {
        Self {
            project,
            requirement,
            policy: ObligationPolicy::Mandatory,
            evidence_validity: EvidenceValidity::unbounded(),
        }
    }

    /// Builds an optional project obligation.
    #[must_use]
    pub fn optional(project: ProjectId, requirement: Requirement) -> Self {
        Self {
            project,
            requirement,
            policy: ObligationPolicy::Optional,
            evidence_validity: EvidenceValidity::unbounded(),
        }
    }

    /// Sets the evidence validity window.
    #[must_use]
    pub fn with_evidence_validity(mut self, validity: EvidenceValidity) -> Self {
        self.evidence_validity = validity;
        self
    }

    /// Returns true when the obligation blocks a gate if unsatisfied.
    #[must_use]
    pub fn is_mandatory(&self) -> bool {
        self.policy == ObligationPolicy::Mandatory
    }
}
