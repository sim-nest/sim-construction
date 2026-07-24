//! Ordered lifecycle vocabulary and data-backed phase policy.

use crate::{ConstructionProjectError, ControlId, ProjectId, Result, RoleId};

/// Ordered construction project lifecycle phase.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum ProjectPhase {
    /// An opportunity is being qualified.
    Opportunity,
    /// Customer and supplier collaboration shape is being formed.
    Collaboration,
    /// The project is mobilizing people, systems, and controls.
    Mobilization,
    /// Design work is being coordinated.
    Design,
    /// Procurement work is being committed.
    Procurement,
    /// Production is being prepared.
    ProductionPreparation,
    /// Production work is being executed.
    Production,
    /// Handover is being accepted.
    Handover,
    /// Closeout records and lessons are being finalized.
    Closeout,
}

impl ProjectPhase {
    /// Returns the stable order index for the lifecycle vocabulary.
    #[must_use]
    pub const fn order(self) -> u8 {
        match self {
            Self::Opportunity => 0,
            Self::Collaboration => 1,
            Self::Mobilization => 2,
            Self::Design => 3,
            Self::Procurement => 4,
            Self::ProductionPreparation => 5,
            Self::Production => 6,
            Self::Handover => 7,
            Self::Closeout => 8,
        }
    }
}

/// Explicit overlap allowance between two ordered phases.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PhaseOverlap {
    /// Earlier lifecycle phase.
    pub from: ProjectPhase,
    /// Later lifecycle phase.
    pub to: ProjectPhase,
    /// Policy reason allowing overlap.
    pub reason: String,
}

impl PhaseOverlap {
    /// Builds a phase-overlap policy row.
    #[must_use]
    pub fn new(from: ProjectPhase, to: ProjectPhase, reason: impl Into<String>) -> Self {
        Self {
            from,
            to,
            reason: reason.into(),
        }
    }
}

/// Data-backed lifecycle policy for a construction project.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LifecyclePolicy {
    /// Ordered lifecycle vocabulary used by the project.
    pub phases: Vec<ProjectPhase>,
    /// Explicit overlap allowances.
    pub overlaps: Vec<PhaseOverlap>,
}

impl LifecyclePolicy {
    /// Builds the standard construction lifecycle policy.
    #[must_use]
    pub fn standard() -> Self {
        Self {
            phases: vec![
                ProjectPhase::Opportunity,
                ProjectPhase::Collaboration,
                ProjectPhase::Mobilization,
                ProjectPhase::Design,
                ProjectPhase::Procurement,
                ProjectPhase::ProductionPreparation,
                ProjectPhase::Production,
                ProjectPhase::Handover,
                ProjectPhase::Closeout,
            ],
            overlaps: Vec::new(),
        }
    }

    /// Adds an explicit overlap allowance.
    #[must_use]
    pub fn with_overlap(mut self, overlap: PhaseOverlap) -> Self {
        self.overlaps.push(overlap);
        self
    }

    /// Returns whether the two phases may be active together.
    #[must_use]
    pub fn allows_overlap(&self, left: ProjectPhase, right: ProjectPhase) -> bool {
        self.overlaps.iter().any(|overlap| {
            (overlap.from == left && overlap.to == right)
                || (overlap.from == right && overlap.to == left)
        })
    }
}

/// Accountable project phase transition.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PhaseTransition {
    /// Stable construction project identity.
    pub project: ProjectId,
    /// Control id for the transition fact.
    pub control: ControlId,
    /// Previous phase.
    pub from: ProjectPhase,
    /// Next phase.
    pub to: ProjectPhase,
    /// Role accountable for recording the transition.
    pub recorded_by: RoleId,
    /// Explicit decision authorizing regression, when moving backward.
    pub regression_decision: Option<ControlId>,
}

impl PhaseTransition {
    /// Builds an accountable phase transition.
    #[must_use]
    pub fn new(
        project: ProjectId,
        control: ControlId,
        from: ProjectPhase,
        to: ProjectPhase,
        recorded_by: RoleId,
    ) -> Self {
        Self {
            project,
            control,
            from,
            to,
            recorded_by,
            regression_decision: None,
        }
    }

    /// Names the explicit decision that authorizes a phase regression.
    #[must_use]
    pub fn with_regression_decision(mut self, decision: ControlId) -> Self {
        self.regression_decision = Some(decision);
        self
    }

    /// Rejects phase regression unless it names an explicit decision.
    pub fn validate(&self) -> Result<()> {
        if self.to.order() < self.from.order() && self.regression_decision.is_none() {
            return Err(ConstructionProjectError::PhaseRegressionRequiresDecision {
                from: self.from,
                to: self.to,
            });
        }
        Ok(())
    }
}
