//! Role and cadence vocabulary for construction office packs.

use sim_lib_construction_project::{BaselineId, ControlId, RoleId};
use time::Date;

/// A supported construction review horizon.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackCadence {
    /// Daily production control.
    Daily,
    /// Weekly project-chief review.
    Weekly,
    /// Monthly or lifecycle-gate review.
    MonthlyGate,
    /// System and project handover review.
    Handover,
    /// Commercial and delivery closeout review.
    Closeout,
    /// Reference-candidate review.
    ReferenceReview,
}

impl PackCadence {
    /// Returns the stable cadence id.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Daily => "daily",
            Self::Weekly => "weekly",
            Self::MonthlyGate => "monthly-gate",
            Self::Handover => "handover",
            Self::Closeout => "closeout",
            Self::ReferenceReview => "reference-review",
        }
    }

    pub(crate) fn includes(self, section: PackSection) -> bool {
        use PackCadence as C;
        use PackSection as S;
        match section {
            S::Decisions | S::SafetyLegalBlockers | S::EvidenceExceptions => true,
            S::CriticalSchedule | S::Readiness => !matches!(self, C::ReferenceReview),
            S::Procurement => matches!(self, C::Daily | C::Weekly | C::MonthlyGate | C::Handover),
            S::RiskChangeEconomy => {
                matches!(self, C::Weekly | C::MonthlyGate | C::Closeout)
            }
            S::Handover => !matches!(self, C::Daily),
            S::Sustainability | S::Outcomes => {
                matches!(
                    self,
                    C::MonthlyGate | C::Handover | C::Closeout | C::ReferenceReview
                )
            }
        }
    }
}

/// Priority-ordered content section in an office pack.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PackSection {
    /// Decisions requiring accountable action.
    Decisions,
    /// Non-waivable safety, work-environment, authority, and legal blockers.
    SafetyLegalBlockers,
    /// Unknown, reported, conflicted, rejected, or expired evidence.
    EvidenceExceptions,
    /// Critical-path and need-date schedule controls.
    CriticalSchedule,
    /// Near-term production and gate readiness.
    Readiness,
    /// Procurement, award, supplier, and material controls.
    Procurement,
    /// Risk, change, exposure, forecast, and final-economy controls.
    RiskChangeEconomy,
    /// Commissioning, handover, completion, and acceptance controls.
    Handover,
    /// Certification, climate, reuse, waste, and place controls.
    Sustainability,
    /// Safety, quality, people, place, and reference outcomes.
    Outcomes,
}

impl PackSection {
    /// Returns the stable section id.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Decisions => "decisions",
            Self::SafetyLegalBlockers => "safety-legal-blockers",
            Self::EvidenceExceptions => "evidence-exceptions",
            Self::CriticalSchedule => "critical-schedule",
            Self::Readiness => "readiness",
            Self::Procurement => "procurement",
            Self::RiskChangeEconomy => "risk-change-economy",
            Self::Handover => "handover",
            Self::Sustainability => "sustainability",
            Self::Outcomes => "outcomes",
        }
    }

    pub(crate) const ORDERED: [Self; 10] = [
        Self::Decisions,
        Self::SafetyLegalBlockers,
        Self::EvidenceExceptions,
        Self::CriticalSchedule,
        Self::Readiness,
        Self::Procurement,
        Self::RiskChangeEconomy,
        Self::Handover,
        Self::Sustainability,
        Self::Outcomes,
    ];
}

/// One authoritative project control selected for a role pack.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackControl {
    /// Stable source control id.
    pub control: ControlId,
    /// Cadence section that owns the control when its evidence is current.
    pub section: PackSection,
    /// Whether this control must be accepted before the aggregate can be accepted.
    pub mandatory: bool,
}

impl PackControl {
    /// Selects a mandatory source control for a pack section.
    #[must_use]
    pub fn mandatory(control: ControlId, section: PackSection) -> Self {
        Self {
            control,
            section,
            mandatory: true,
        }
    }

    /// Selects an informative source control that does not determine the aggregate.
    #[must_use]
    pub fn optional(control: ControlId, section: PackSection) -> Self {
        Self {
            control,
            section,
            mandatory: false,
        }
    }
}

/// Complete deterministic request for one role-cadence office pack.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OfficePackRequest {
    /// Review horizon.
    pub cadence: PackCadence,
    /// Accountable audience role.
    pub role: RoleId,
    /// Inclusive project sequence to review.
    pub as_of_seq: u64,
    /// Calendar date used to interpret the review.
    pub as_of_date: Date,
    /// Caller-supplied deterministic generation marker.
    pub generated_at: String,
    /// Accepted baselines used by the review.
    pub accepted_baselines: Vec<BaselineId>,
    /// Authoritative controls selected for the audience.
    pub controls: Vec<PackControl>,
    /// Optional prior meeting sequence used to flag changed controls.
    pub changed_since_seq: Option<u64>,
}

impl OfficePackRequest {
    /// Starts a deterministic pack request.
    #[must_use]
    pub fn new(
        cadence: PackCadence,
        role: RoleId,
        as_of_seq: u64,
        as_of_date: Date,
        generated_at: impl Into<String>,
    ) -> Self {
        Self {
            cadence,
            role,
            as_of_seq,
            as_of_date,
            generated_at: generated_at.into(),
            accepted_baselines: Vec::new(),
            controls: Vec::new(),
            changed_since_seq: None,
        }
    }

    /// Adds an accepted baseline id.
    #[must_use]
    pub fn with_baseline(mut self, baseline: BaselineId) -> Self {
        self.accepted_baselines.push(baseline);
        self
    }

    /// Adds one source control selection.
    #[must_use]
    pub fn with_control(mut self, control: PackControl) -> Self {
        self.controls.push(control);
        self
    }

    /// Flags controls changed after the prior meeting sequence.
    #[must_use]
    pub fn changed_since(mut self, sequence: u64) -> Self {
        self.changed_since_seq = Some(sequence);
        self
    }
}
