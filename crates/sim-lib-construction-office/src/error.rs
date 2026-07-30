//! Errors from construction-to-office evidence composition.

use sim_lib_construction_project::{ControlId, ProjectId};

/// Error reported by construction-to-office evidence composition.
#[derive(Debug, thiserror::Error)]
pub enum EvidenceBridgeError {
    /// A required construction or office capability is absent.
    #[error(transparent)]
    Capability(#[from] sim_kernel::Error),
    /// The office document/evidence store failed.
    #[error("office evidence store failed: {0}")]
    Store(String),
    /// The access token, book, and request do not name the same project.
    #[error("evidence project {actual} is outside authorized project {expected}")]
    ProjectScopeMismatch {
        /// Authorized project.
        expected: ProjectId,
        /// Project on the book or request.
        actual: ProjectId,
    },
    /// The requested fact sequence is absent.
    #[error("construction fact sequence {sequence} is missing")]
    MissingFact {
        /// Missing fact sequence.
        sequence: u64,
    },
    /// The requested control differs from the fact subject.
    #[error("fact {sequence} belongs to control {actual}, not {expected}")]
    ControlMismatch {
        /// Fact sequence.
        sequence: u64,
        /// Requested control.
        expected: ControlId,
        /// Fact subject.
        actual: ControlId,
    },
    /// A newer fact supersedes the requested fact.
    #[error("construction fact sequence {sequence} is stale")]
    StaleFact {
        /// Superseded fact sequence.
        sequence: u64,
    },
    /// The requested precise relation differs from the fact kind.
    #[error("fact {sequence} kind {actual} does not match evidence relation {expected}")]
    RelationMismatch {
        /// Fact sequence.
        sequence: u64,
        /// Requested precise relation.
        expected: String,
        /// Fact's precise relation.
        actual: String,
    },
    /// The requested external reference is not retained by the fact.
    #[error("fact {sequence} does not retain the requested external reference")]
    ReferenceMissing {
        /// Fact sequence.
        sequence: u64,
    },
    /// The office document does not exist in the supplied store.
    #[error("office document {document} is missing")]
    MissingDocument {
        /// Missing document.
        document: String,
    },
    /// The caller lacks the fact's project disclosure visibility.
    #[error("visibility {visibility} is denied for construction fact {sequence}")]
    VisibilityDenied {
        /// Fact sequence.
        sequence: u64,
        /// Denied visibility label.
        visibility: String,
    },
    /// A stored construction fact reference is malformed or inconsistent.
    #[error("invalid construction evidence projection: {0}")]
    InvalidProjection(String),
}
