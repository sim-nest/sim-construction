//! Error types for construction project-control validation.

use crate::{ControlId, ProjectId, ProjectPhase, RoleId};

/// Result alias for construction project-control validation.
pub type Result<T> = std::result::Result<T, ConstructionProjectError>;

/// Validation error for construction project-control records.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ConstructionProjectError {
    /// A stable identifier was empty.
    #[error("{kind} id is empty")]
    EmptyId {
        /// Identifier kind.
        kind: &'static str,
    },
    /// A stable identifier exceeded the bounded identifier length.
    #[error("{kind} id {value:?} exceeds {max} characters")]
    IdTooLong {
        /// Identifier kind.
        kind: &'static str,
        /// Rejected identifier text.
        value: String,
        /// Maximum accepted character count.
        max: usize,
    },
    /// A stable identifier contained whitespace or trim-sensitive text.
    #[error("{kind} id {value:?} contains whitespace or trim-sensitive text")]
    WhitespaceAmbiguousId {
        /// Identifier kind.
        kind: &'static str,
        /// Rejected identifier text.
        value: String,
    },
    /// A stable identifier contained a path separator.
    #[error("{kind} id {value:?} contains a reserved slash")]
    SlashBearingId {
        /// Identifier kind.
        kind: &'static str,
        /// Rejected identifier text.
        value: String,
    },
    /// A stable identifier used a reserved sentinel value.
    #[error("{kind} id {value:?} is reserved")]
    ReservedId {
        /// Identifier kind.
        kind: &'static str,
        /// Rejected identifier text.
        value: String,
    },
    /// A stable identifier contained unsupported characters.
    #[error("{kind} id {value:?} contains unsupported characters")]
    UnsupportedIdCharacters {
        /// Identifier kind.
        kind: &'static str,
        /// Rejected identifier text.
        value: String,
    },
    /// A project currency was not in the supported ISO 4217 project set.
    #[error("currency {0:?} is not a supported project currency")]
    UnknownCurrency(String),
    /// A required text field was empty.
    #[error("field {0} must not be empty")]
    EmptyField(&'static str),
    /// A required text list was empty.
    #[error("field {0} must contain at least one item")]
    EmptyCollection(&'static str),
    /// A charter had no project objectives.
    #[error("project charter must contain at least one objective")]
    EmptyObjectives,
    /// A due-date policy had an invalid day count.
    #[error("due-date policy field {field} must be between 1 and {max_days} days")]
    InvalidDueDatePolicy {
        /// Invalid due-date policy field.
        field: &'static str,
        /// Maximum accepted day count.
        max_days: u16,
    },
    /// A record reused a stable identifier within the same control scope.
    #[error("duplicate {kind} id {id}")]
    DuplicateId {
        /// Identifier kind.
        kind: &'static str,
        /// Duplicate identifier text.
        id: String,
    },
    /// A role escalated to a role not present in the governance record.
    #[error("role {role} escalates to missing role {target}")]
    MissingEscalationTarget {
        /// Role containing the bad escalation edge.
        role: RoleId,
        /// Missing escalation target.
        target: RoleId,
    },
    /// Role escalation edges formed an authority cycle.
    #[error("role escalation path contains a cycle starting at {0}")]
    AuthorityCycle(RoleId),
    /// A restricted visibility symbol was not allowed by the visibility policy.
    #[error("restricted visibility {symbol} is not allowed by the visibility policy")]
    RestrictedVisibilityDenied {
        /// Rejected restricted visibility symbol.
        symbol: String,
    },
    /// A serialized SIM symbol could not be parsed.
    #[error("symbol {value:?} is invalid: {reason}")]
    InvalidSymbol {
        /// Rejected symbol text.
        value: String,
        /// Reason the symbol text is not accepted.
        reason: String,
    },
    /// A fact sequence or snapshot sequence was outside the accepted range.
    #[error("{field} sequence {sequence} is invalid")]
    InvalidSequence {
        /// Invalid sequence field.
        field: &'static str,
        /// Rejected sequence.
        sequence: u64,
    },
    /// A fact append attempted to move backward or reuse the current tail.
    #[error("fact sequence {next_sequence} is not after current tail {last_sequence}")]
    OutOfOrderSequence {
        /// Current highest sequence in the book.
        last_sequence: u64,
        /// Rejected appended sequence.
        next_sequence: u64,
    },
    /// More than one fact used the same sequence number.
    #[error("duplicate fact sequence {sequence}")]
    DuplicateSequence {
        /// Reused sequence.
        sequence: u64,
    },
    /// A fact belonged to a different project than the book.
    #[error("fact project {actual} does not match project book {expected}")]
    ProjectMismatch {
        /// Project owned by the book.
        expected: ProjectId,
        /// Project carried by the fact.
        actual: ProjectId,
    },
    /// A fact was written by a role other than the book's authoritative writer.
    #[error("fact writer {actual} does not match authoritative writer {expected}")]
    WriterMismatch {
        /// Authoritative writer for the book.
        expected: RoleId,
        /// Actor role carried by the fact.
        actual: RoleId,
    },
    /// A project book exceeded its configured fact count bound.
    #[error("project book exceeds {max} facts")]
    FactLimitExceeded {
        /// Maximum fact count.
        max: usize,
    },
    /// A fact body exceeded the expression node bound.
    #[error("fact {sequence} body has {nodes} expression nodes, above {max}")]
    FactBodyTooLarge {
        /// Fact sequence.
        sequence: u64,
        /// Counted expression nodes.
        nodes: usize,
        /// Maximum accepted node count.
        max: usize,
    },
    /// A fact exceeded the reference-only evidence count bound.
    #[error("fact {sequence} carries {count} evidence references, above {max}")]
    EvidenceLimitExceeded {
        /// Fact sequence.
        sequence: u64,
        /// Evidence reference count.
        count: usize,
        /// Maximum accepted evidence reference count.
        max: usize,
    },
    /// A superseding fact pointed at a missing prior sequence.
    #[error("fact {sequence} supersedes missing sequence {supersedes}")]
    MissingSupersededFact {
        /// Superseding fact sequence.
        sequence: u64,
        /// Missing prior sequence.
        supersedes: u64,
    },
    /// A supersession edge was not a valid backward correction edge.
    #[error("fact {sequence} cannot supersede {supersedes}: {reason}")]
    InvalidSupersession {
        /// Superseding fact sequence.
        sequence: u64,
        /// Referenced prior sequence.
        supersedes: u64,
        /// Reason the edge was rejected.
        reason: &'static str,
    },
    /// A superseding fact targeted a different subject than its predecessor.
    #[error(
        "fact {sequence} subject {actual} does not match superseded {supersedes} subject {expected}"
    )]
    SupersessionSubjectMismatch {
        /// Superseding fact sequence.
        sequence: u64,
        /// Referenced prior sequence.
        supersedes: u64,
        /// Subject on the superseded fact.
        expected: ControlId,
        /// Subject on the superseding fact.
        actual: ControlId,
    },
    /// More than one fact tried to supersede the same prior sequence.
    #[error("sequence {supersedes} is already superseded by {existing}, not {attempted}")]
    SupersessionFork {
        /// Prior sequence that already has a correction edge.
        supersedes: u64,
        /// Existing superseding sequence.
        existing: u64,
        /// Rejected superseding sequence.
        attempted: u64,
    },
    /// A delta query used an inverted sequence range.
    #[error("snapshot range {from_seq}..={through_seq} is invalid")]
    InvalidSnapshotRange {
        /// Start sequence.
        from_seq: u64,
        /// End sequence.
        through_seq: u64,
    },
    /// A project phase transition moved backward without an accountable decision.
    #[error("phase regression from {from:?} to {to:?} requires an explicit decision")]
    PhaseRegressionRequiresDecision {
        /// Phase being left.
        from: ProjectPhase,
        /// Earlier phase being entered.
        to: ProjectPhase,
    },
    /// A baseline comparison used a sequence older than the accepted baseline.
    #[error(
        "baseline {baseline} accepted at sequence {accepted_seq} cannot compare stale sequence {as_of_seq}"
    )]
    StaleBaselineComparison {
        /// Baseline control identifier.
        baseline: ControlId,
        /// Sequence accepting the baseline.
        accepted_seq: u64,
        /// Rejected comparison sequence.
        as_of_seq: u64,
    },
    /// A control reference was not present in the derived snapshot.
    #[error("control {control} is not present in snapshot sequence {as_of_seq}")]
    OrphanControlRef {
        /// Missing control reference.
        control: ControlId,
        /// Snapshot sequence used for validation.
        as_of_seq: u64,
    },
    /// An accountable item was closed without a resolution fact.
    #[error("{kind} {control} is closed without a resolution fact")]
    MissingResolutionFact {
        /// Accountable item kind.
        kind: &'static str,
        /// Closed item control.
        control: ControlId,
    },
    /// A gate decision was made by a role that lacks matching authority.
    #[error("gate {gate} approval by {actual} does not match authority {expected}")]
    ApprovalAuthorityMismatch {
        /// Gate control.
        gate: ControlId,
        /// Role authorized by the gate.
        expected: RoleId,
        /// Role carried by the decision.
        actual: RoleId,
    },
    /// A project decision was closed by a role that lacks matching authority.
    #[error("decision {decision} resolution by {actual} does not match authority {expected}")]
    DecisionAuthorityMismatch {
        /// Decision control.
        decision: ControlId,
        /// Role authorized by the decision.
        expected: RoleId,
        /// Role carried by the resolution.
        actual: RoleId,
    },
    /// A gate decision pointed at a different gate than the report.
    #[error("gate decision for {actual} does not match report gate {expected}")]
    GateMismatch {
        /// Gate named by the report.
        expected: ControlId,
        /// Gate named by the decision.
        actual: ControlId,
    },
    /// A gate approval used a different sequence than the derived report.
    #[error(
        "gate {gate} approval sequence {decision_seq} does not match report sequence {report_seq}"
    )]
    GateSequenceMismatch {
        /// Gate control.
        gate: ControlId,
        /// Sequence on the report.
        report_seq: u64,
        /// Sequence on the decision.
        decision_seq: u64,
    },
    /// A gate approval tried to approve an unready derived report.
    #[error("gate {gate} cannot be approved because the derived report is not ready")]
    GateReportNotReady {
        /// Gate control.
        gate: ControlId,
    },
}
