//! Accountable admission of evidence-backed construction reference claims.

use sim_lib_doc_core::ExternalRef;
use time::Date;

use crate::{ControlId, EvidenceState, ProjectId, RoleId, Visibility};

/// Domain meaning of a proposed reference claim.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum ReferenceClaimKind {
    /// Accepted project lesson.
    Lesson,
    /// Aggregate, chartered people-development outcome.
    PeopleDevelopment,
    /// Chartered property outcome.
    PropertyOutcome,
    /// Chartered city-district outcome.
    CityDistrictOutcome,
    /// Another project-chartered outcome.
    Other,
}

/// A source-minimal claim proposed for a reference pack.
///
/// People claims deliberately have no person, rating, performance-history, or
/// dossier field. They identify aggregate charter and outcome facts only.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ReferenceClaim {
    /// Stable project identity.
    pub project: ProjectId,
    /// Stable claim identity.
    pub id: ControlId,
    /// Claim domain.
    pub kind: ReferenceClaimKind,
    /// Bounded human-readable assertion.
    pub statement: String,
    /// Fact that chartered this outcome or lesson.
    pub charter_fact_seq: u64,
    /// Selected source facts; source artifacts are not copied.
    pub source_fact_sequences: Vec<u64>,
    /// Minimal versioned external references needed to substantiate the claim.
    pub external_refs: Vec<ExternalRef>,
    /// Visibility of the admitted manifest row.
    pub visibility: Visibility,
    /// Whether a current consent clearance is mandatory.
    pub consent_required: bool,
    /// Whether a confidentiality clearance is mandatory.
    pub confidentiality_required: bool,
    /// Optional chartered outcome target that this claim says was achieved.
    pub outcome_target: Option<ControlId>,
}

impl ReferenceClaim {
    /// Builds a claim grounded in its charter fact.
    #[must_use]
    pub fn new(
        project: ProjectId,
        id: ControlId,
        kind: ReferenceClaimKind,
        statement: impl Into<String>,
        charter_fact_seq: u64,
        visibility: Visibility,
    ) -> Self {
        Self {
            project,
            id,
            kind,
            statement: statement.into(),
            charter_fact_seq,
            source_fact_sequences: vec![charter_fact_seq],
            external_refs: Vec::new(),
            visibility,
            consent_required: false,
            confidentiality_required: false,
            outcome_target: None,
        }
    }

    /// Adds a selected supporting fact sequence.
    #[must_use]
    pub fn with_source_fact(mut self, sequence: u64) -> Self {
        self.source_fact_sequences.push(sequence);
        self
    }

    /// Adds a minimal versioned external reference.
    #[must_use]
    pub fn with_external_ref(mut self, reference: ExternalRef) -> Self {
        self.external_refs.push(reference);
        self
    }

    /// Requires current consent evidence.
    #[must_use]
    pub fn requires_consent(mut self) -> Self {
        self.consent_required = true;
        self
    }

    /// Requires current confidentiality clearance.
    #[must_use]
    pub fn requires_confidentiality_clearance(mut self) -> Self {
        self.confidentiality_required = true;
        self
    }

    /// States that the claim depends on one chartered target being achieved.
    #[must_use]
    pub fn asserts_outcome(mut self, target: ControlId) -> Self {
        self.outcome_target = Some(target);
        self
    }
}

/// State of a consent or confidentiality requirement.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DisclosureCondition {
    /// The condition does not apply.
    NotRequired,
    /// Current reference-only evidence satisfies the condition.
    Satisfied(ExternalRef),
    /// The named authority denied the condition.
    Denied(ExternalRef),
    /// Earlier satisfaction was withdrawn.
    Withdrawn(ExternalRef),
}

impl DisclosureCondition {
    pub(crate) fn is_satisfied(&self) -> bool {
        matches!(self, Self::Satisfied(_))
    }

    pub(crate) fn is_withdrawn(&self) -> bool {
        matches!(self, Self::Withdrawn(_))
    }
}

/// Consent and confidentiality clearance selected for one claim.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DisclosureClearance {
    /// Claim being cleared.
    pub claim: ControlId,
    /// Consent state.
    pub consent: DisclosureCondition,
    /// Confidentiality state.
    pub confidentiality: DisclosureCondition,
}

impl DisclosureClearance {
    /// Builds an empty clearance for a claim with no default assumptions.
    #[must_use]
    pub fn new(claim: ControlId) -> Self {
        Self {
            claim,
            consent: DisclosureCondition::NotRequired,
            confidentiality: DisclosureCondition::NotRequired,
        }
    }

    /// Sets consent state.
    #[must_use]
    pub fn with_consent(mut self, condition: DisclosureCondition) -> Self {
        self.consent = condition;
        self
    }

    /// Sets confidentiality state.
    #[must_use]
    pub fn with_confidentiality(mut self, condition: DisclosureCondition) -> Self {
        self.confidentiality = condition;
        self
    }
}

/// Accountable disclosure decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ReferenceDecisionKind {
    /// Approve this exact claim and evidence snapshot.
    Approve,
    /// Reject disclosure.
    Reject,
}

/// Named authority decision over one exact reference claim snapshot.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ReferenceApproval {
    /// Claim being decided.
    pub claim: ControlId,
    /// Stable approving-decision id.
    pub decision: ControlId,
    /// Source fact sequence evaluated.
    pub report_seq: u64,
    /// Later decision sequence.
    pub decision_seq: u64,
    /// Approval or rejection.
    pub outcome: ReferenceDecisionKind,
    /// Named authority making the decision.
    pub decided_by: RoleId,
    /// Reference-only approval evidence.
    pub evidence: Vec<ExternalRef>,
}

impl ReferenceApproval {
    /// Builds a decision over one exact source-fact snapshot.
    #[must_use]
    pub fn new(
        claim: ControlId,
        decision: ControlId,
        report_seq: u64,
        decision_seq: u64,
        outcome: ReferenceDecisionKind,
        decided_by: RoleId,
    ) -> Self {
        Self {
            claim,
            decision,
            report_seq,
            decision_seq,
            outcome,
            decided_by,
            evidence: Vec::new(),
        }
    }

    /// Adds reference-only decision evidence.
    #[must_use]
    pub fn with_evidence(mut self, evidence: ExternalRef) -> Self {
        self.evidence.push(evidence);
        self
    }
}

/// Reason a proposed claim cannot enter the reference manifest.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ReferenceAdmissionBlocker {
    /// A named source sequence is absent.
    MissingSourceFact(u64),
    /// A source fact is not accepted.
    SourceFactNotAccepted {
        /// Source sequence.
        sequence: u64,
        /// Current evidence state.
        state: EvidenceState,
    },
    /// A source fact was superseded or is not current.
    SourceFactNotCurrent {
        /// Proposed source sequence.
        sequence: u64,
        /// Current sequence for the same subject, when any.
        current: Option<u64>,
    },
    /// Competing source facts make the claim conflicted.
    SourceFactConflicted(u64),
    /// Required consent has no current satisfaction.
    ConsentUnsatisfied,
    /// Earlier consent was withdrawn.
    ConsentWithdrawn,
    /// Restricted evidence lacks confidentiality clearance.
    ConfidentialityUnsatisfied,
    /// The claimed chartered target is missing, blocked, or short.
    OutcomeShortfall(ControlId),
    /// No accountable approval exists.
    MissingApproval,
    /// The decision rejected disclosure.
    ApprovalRejected,
    /// The decision was made by a different role.
    ApprovalAuthorityMismatch {
        /// Expected authority.
        expected: RoleId,
        /// Actual deciding role.
        actual: RoleId,
    },
    /// Approval is not bound to the exact snapshot and closeout order.
    ApprovalSequenceMismatch,
    /// Approval carries no reference-only evidence.
    MissingApprovalEvidence,
}

/// Admission result for one proposed claim.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ReferenceClaimAdmission {
    /// Stable claim identity.
    pub claim: ControlId,
    /// Deterministic blockers.
    pub blockers: Vec<ReferenceAdmissionBlocker>,
    /// True only when this claim can enter the manifest.
    pub admitted: bool,
}

/// Result of evaluating a whole reference pack.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ReferenceAdmissionReport {
    /// Per-claim results in stable claim-id order.
    pub claims: Vec<ReferenceClaimAdmission>,
    /// Immutable manifest, created only when every requested claim is admitted.
    pub manifest: Option<ReferenceManifest>,
}

/// Immutable manifest of admitted reference claims.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ReferenceManifest {
    pub(crate) project: ProjectId,
    pub(crate) as_of_date: Date,
    pub(crate) closeout_decision: ControlId,
    pub(crate) claims: Vec<ReferenceManifestClaim>,
}

impl ReferenceManifest {
    /// Returns the project represented by this manifest.
    #[must_use]
    pub fn project(&self) -> &ProjectId {
        &self.project
    }

    /// Returns the disclosure as-of date.
    #[must_use]
    pub fn as_of_date(&self) -> Date {
        self.as_of_date
    }

    /// Returns the accountable closeout decision preceding admission.
    #[must_use]
    pub fn closeout_decision(&self) -> &ControlId {
        &self.closeout_decision
    }

    /// Returns immutable claim rows.
    #[must_use]
    pub fn claims(&self) -> &[ReferenceManifestClaim] {
        &self.claims
    }
}

/// Immutable manifest row containing references, never copied source artifacts.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ReferenceManifestClaim {
    pub(crate) claim_id: ControlId,
    pub(crate) source_fact_sequences: Vec<u64>,
    pub(crate) external_refs: Vec<ExternalRef>,
    pub(crate) as_of_date: Date,
    pub(crate) visibility: Visibility,
    pub(crate) approving_decision: ControlId,
}

impl ReferenceManifestClaim {
    /// Returns the stable claim id.
    #[must_use]
    pub fn claim_id(&self) -> &ControlId {
        &self.claim_id
    }

    /// Returns selected source fact sequences.
    #[must_use]
    pub fn source_fact_sequences(&self) -> &[u64] {
        &self.source_fact_sequences
    }

    /// Returns minimal versioned external references.
    #[must_use]
    pub fn external_refs(&self) -> &[ExternalRef] {
        &self.external_refs
    }

    /// Returns the admission as-of date.
    #[must_use]
    pub fn as_of_date(&self) -> Date {
        self.as_of_date
    }

    /// Returns admitted visibility.
    #[must_use]
    pub fn visibility(&self) -> &Visibility {
        &self.visibility
    }

    /// Returns the named approving decision.
    #[must_use]
    pub fn approving_decision(&self) -> &ControlId {
        &self.approving_decision
    }
}

/// Candidate pack evaluated after accountable closeout.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ReferencePackAdmission {
    /// Stable project identity.
    pub project: ProjectId,
    /// Inclusive source-fact sequence.
    pub as_of_seq: u64,
    /// Disclosure as-of date.
    pub as_of_date: Date,
    /// Named authority allowed to approve disclosure.
    pub approving_authority: RoleId,
    /// Proposed claims.
    pub claims: Vec<ReferenceClaim>,
    /// Claim-specific consent and confidentiality states.
    pub clearances: Vec<DisclosureClearance>,
    /// Accountable claim decisions.
    pub approvals: Vec<ReferenceApproval>,
}
