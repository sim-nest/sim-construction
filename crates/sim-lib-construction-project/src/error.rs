//! Error types for construction project-control validation.

use crate::RoleId;

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
}
