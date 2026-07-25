//! Purpose-specific design release decisions.

use sim_lib_doc_core::ExternalRef;
use time::Date;

use crate::{ControlId, EvidenceState, ProjectId, RoleId};

/// Purpose served by a design release.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum DesignReleasePurpose {
    /// Release for procurement.
    Procurement,
    /// Release for production or construction work.
    Production,
    /// Release for handover.
    Handover,
    /// Open project-specific purpose symbol.
    Open(String),
}

impl DesignReleasePurpose {
    /// Returns the stable purpose text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Procurement => "procurement",
            Self::Production => "production",
            Self::Handover => "handover",
            Self::Open(symbol) => symbol.as_str(),
        }
    }
}

/// Accountable release decision for a named design revision and purpose.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DesignRelease {
    /// Stable construction project identity.
    pub project: ProjectId,
    /// Release control id.
    pub control: ControlId,
    /// Released design revision control id.
    pub design_revision: ControlId,
    /// Human-facing revision name recorded by the release decision.
    pub revision: String,
    /// Release purpose.
    pub purpose: DesignReleasePurpose,
    /// Role responsible for preparing the release.
    pub responsible_role: RoleId,
    /// Role authorized to release.
    pub release_authority: RoleId,
    /// Role that made the release decision.
    pub released_by: RoleId,
    /// Date the release is needed.
    pub need_date: Date,
    /// Evidence state for the release decision.
    pub evidence_state: EvidenceState,
    /// Package, task, or control ids affected by the release.
    pub affected_control_ids: Vec<ControlId>,
    /// Explicit revalidation against a superseding revision, when present.
    pub revalidated_against: Option<ControlId>,
    /// Reference-only release evidence.
    pub external_refs: Vec<ExternalRef>,
}

impl DesignRelease {
    /// Builds an accountable design release decision.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        project: ProjectId,
        control: ControlId,
        design_revision: ControlId,
        revision: impl Into<String>,
        purpose: DesignReleasePurpose,
        responsible_role: RoleId,
        release_authority: RoleId,
        released_by: RoleId,
        need_date: Date,
    ) -> Self {
        Self {
            project,
            control,
            design_revision,
            revision: revision.into(),
            purpose,
            responsible_role,
            release_authority,
            released_by,
            need_date,
            evidence_state: EvidenceState::Reported,
            affected_control_ids: Vec::new(),
            revalidated_against: None,
            external_refs: Vec::new(),
        }
    }

    /// Sets the evidence state.
    #[must_use]
    pub fn with_evidence_state(mut self, evidence_state: EvidenceState) -> Self {
        self.evidence_state = evidence_state;
        self
    }

    /// Adds an affected package, task, or control id.
    #[must_use]
    pub fn affects(mut self, control: ControlId) -> Self {
        self.affected_control_ids.push(control);
        self
    }

    /// Marks this release explicitly revalidated against a newer revision.
    #[must_use]
    pub fn revalidated_against(mut self, revision: ControlId) -> Self {
        self.revalidated_against = Some(revision);
        self
    }

    /// Adds a reference-only release record.
    #[must_use]
    pub fn with_external_ref(mut self, external_ref: ExternalRef) -> Self {
        self.external_refs.push(external_ref);
        self
    }
}
