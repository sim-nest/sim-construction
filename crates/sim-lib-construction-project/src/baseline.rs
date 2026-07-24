//! Accepted construction control baselines.

use sim_lib_doc_core::ExternalRef;
use time::Date;

use crate::{
    BaselineId, ConstructionProjectError, ControlId, ProjectBook, ProjectId, ProjectSnapshot,
    Result, RoleId,
};

/// Accepted baseline lane.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BaselineKind {
    /// Scope baseline.
    Scope,
    /// Time baseline.
    Time,
    /// Commercial baseline.
    Commercial,
    /// Obligation baseline.
    Obligations,
    /// Organization baseline.
    Organization,
    /// Reporting baseline.
    Reporting,
}

/// Accepted construction project baseline.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AcceptedBaseline {
    /// Stable baseline identity.
    pub id: BaselineId,
    /// Stable project identity.
    pub project: ProjectId,
    /// Control subject holding the baseline acceptance fact.
    pub control: ControlId,
    /// Baseline lane.
    pub kind: BaselineKind,
    /// Role that accepted the baseline.
    pub accepted_by: RoleId,
    /// Book sequence where the baseline was accepted.
    pub accepted_seq: u64,
    /// Calendar date of acceptance.
    pub accepted_on: Date,
    /// Reference-only evidence for the acceptance.
    pub evidence: Vec<ExternalRef>,
}

impl AcceptedBaseline {
    /// Builds an accepted baseline.
    #[must_use]
    pub fn new(
        id: BaselineId,
        project: ProjectId,
        control: ControlId,
        kind: BaselineKind,
        accepted_by: RoleId,
        accepted_seq: u64,
        accepted_on: Date,
    ) -> Self {
        Self {
            id,
            project,
            control,
            kind,
            accepted_by,
            accepted_seq,
            accepted_on,
            evidence: Vec::new(),
        }
    }

    /// Adds a reference-only evidence link.
    #[must_use]
    pub fn with_evidence(mut self, evidence: ExternalRef) -> Self {
        self.evidence.push(evidence);
        self
    }

    /// Validates the accepted baseline record.
    pub fn validate(&self) -> Result<()> {
        if self.accepted_seq == 0 {
            return Err(ConstructionProjectError::InvalidSequence {
                field: "baseline.accepted_seq",
                sequence: self.accepted_seq,
            });
        }
        if self.evidence.is_empty() {
            return Err(ConstructionProjectError::EmptyCollection(
                "baseline.evidence",
            ));
        }
        Ok(())
    }

    /// Builds a snapshot for comparison, rejecting comparisons before acceptance.
    pub fn comparison_snapshot(
        &self,
        book: &ProjectBook,
        as_of_seq: u64,
    ) -> Result<ProjectSnapshot> {
        self.validate()?;
        if as_of_seq < self.accepted_seq {
            return Err(ConstructionProjectError::StaleBaselineComparison {
                baseline: self.control.clone(),
                accepted_seq: self.accepted_seq,
                as_of_seq,
            });
        }
        book.snapshot_at(as_of_seq)
    }
}
