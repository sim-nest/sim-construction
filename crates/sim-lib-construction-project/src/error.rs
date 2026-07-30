//! Error types for construction project-control validation.

use time::Date;

use crate::OrganizationId;
use crate::{
    BaselineId, ChangeId, ControlId, ForecastConsequenceKind, ProjectId, ProjectPhase, RoleId,
    UncertaintyKind, UncertaintyState,
};

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
    /// A policy operation was attempted without the required capability.
    #[error("missing required capability {capability}")]
    MissingCapability {
        /// Required capability name.
        capability: &'static str,
    },
    /// An exception decision was made by a role that lacks matching authority.
    #[error("exception {exception} by {actual} does not match authority {expected}")]
    ExceptionAuthorityMismatch {
        /// Exception control.
        exception: ControlId,
        /// Role authorized by the exception.
        expected: RoleId,
        /// Role carried by the decision.
        actual: RoleId,
    },
    /// An exception expired before the policy evaluation date.
    #[error("exception {exception} expired on {expired_on} before {as_of_date}")]
    ExpiredException {
        /// Exception control.
        exception: ControlId,
        /// Expiry date.
        expired_on: Date,
        /// Policy evaluation date.
        as_of_date: Date,
    },
    /// A non-waivable requirement was covered by an exception.
    #[error("requirement {requirement} is non-waivable but exception {exception} covers it")]
    NonWaivableRequirement {
        /// Non-waivable requirement.
        requirement: ControlId,
        /// Rejected exception.
        exception: ControlId,
    },
    /// A design-control record references a revision that is not present.
    #[error("{kind} {control} references missing design revision {revision}")]
    MissingDesignRevision {
        /// Referencing record kind.
        kind: &'static str,
        /// Referencing control.
        control: ControlId,
        /// Missing revision control.
        revision: ControlId,
    },
    /// More than one current revision exists for an affected control.
    #[error("affected control {affected} has conflicting current design revisions {revisions:?}")]
    ConflictingDesignRevisions {
        /// Affected package, task, or control.
        affected: ControlId,
        /// Current revision controls.
        revisions: Vec<ControlId>,
    },
    /// A design release used the wrong purpose for the requested readiness.
    #[error("release {release} purpose {actual} does not satisfy required purpose {required}")]
    DesignReleasePurposeMismatch {
        /// Release control.
        release: ControlId,
        /// Required purpose.
        required: String,
        /// Actual purpose.
        actual: String,
    },
    /// A release points at a superseded design revision and has not been revalidated.
    #[error(
        "release {release} for revision {revision} is stale after superseding revision {superseding}"
    )]
    StaleDesignRelease {
        /// Stale release control.
        release: ControlId,
        /// Released revision.
        revision: ControlId,
        /// Superseding revision.
        superseding: ControlId,
    },
    /// A release decision was made by a role that lacks matching authority.
    #[error("release {release} by {actual} does not match authority {expected}")]
    DesignReleaseAuthorityMismatch {
        /// Release control.
        release: ControlId,
        /// Authorized release role.
        expected: RoleId,
        /// Actual decision role.
        actual: RoleId,
    },
    /// A non-waivable production blocker was still active.
    #[error("production blocker {blocker} is non-waivable for {target}")]
    NonWaivableProductionBlocker {
        /// Blocked control.
        target: ControlId,
        /// Non-waivable blocker.
        blocker: ControlId,
    },
    /// A control-graph edge named an endpoint that is not present as a node.
    #[error("control graph edge {edge} references missing {endpoint_role} endpoint {endpoint}")]
    ControlGraphMissingEndpoint {
        /// Edge kind.
        edge: &'static str,
        /// Whether the endpoint was the source or target.
        endpoint_role: &'static str,
        /// Missing control id.
        endpoint: ControlId,
    },
    /// A control-graph edge reused an existing source, target, and kind.
    #[error("duplicate control graph edge {kind} from {from} to {target}")]
    DuplicateControlGraphEdge {
        /// Edge kind.
        kind: &'static str,
        /// Source control.
        from: ControlId,
        /// Target control.
        target: ControlId,
    },
    /// A commercial amount used a currency different from the project charter.
    #[error("{field} currency {actual} does not match project charter currency {expected}")]
    CurrencyMismatch {
        /// Field carrying the rejected currency.
        field: &'static str,
        /// Expected project currency.
        expected: String,
        /// Actual currency.
        actual: String,
    },
    /// A commercial amount was zero or negative.
    #[error("{field} amount must be positive")]
    NonPositiveAmount {
        /// Field carrying the rejected amount.
        field: &'static str,
    },
    /// Checked commercial amount arithmetic overflowed.
    #[error("{field} amount arithmetic overflowed")]
    AmountOverflow {
        /// Arithmetic field that overflowed.
        field: &'static str,
    },
    /// A change-chain invariant failed closed.
    #[error("change {change} derivation failed: {reason}")]
    ChangeDerivation {
        /// Stable change identity.
        change: ChangeId,
        /// Stable invariant failure reason.
        reason: &'static str,
    },
    /// A commercial fact included both a summarized parent and its child.
    #[error("change amount double counts parent {parent} and child {child}")]
    ChangeAmountDoubleCount {
        /// Summary component.
        parent: ControlId,
        /// Component already included by the summary.
        child: ControlId,
    },
    /// A closure total did not match the final settlement total.
    #[error(
        "change {change} {side} closure total {closure} does not match settlement total {settlement}"
    )]
    ChangeSettlementMismatch {
        /// Stable change identity.
        change: ChangeId,
        /// Supplier or customer lane.
        side: &'static str,
        /// Exact settlement total.
        settlement: String,
        /// Exact closure total.
        closure: String,
    },
    /// A risk carried opportunity-only state, or an opportunity carried risk-only state.
    #[error("uncertainty {control} kind {kind:?} cannot carry state {state:?}")]
    UncertaintyStateMismatch {
        /// Uncertainty control.
        control: ControlId,
        /// Risk or opportunity kind.
        kind: UncertaintyKind,
        /// Rejected lifecycle state.
        state: UncertaintyState,
    },
    /// A forecast lane carried an incompatible typed value.
    #[error("forecast consequence {consequence} kind {kind:?} has an incompatible value")]
    ForecastValueMismatch {
        /// Forecast consequence control.
        consequence: ControlId,
        /// Forecast lane expecting a different value.
        kind: ForecastConsequenceKind,
    },
    /// Current-fact, baseline, scenario, hierarchy, or schedule derivation failed closed.
    #[error("uncertainty derivation for {control} failed: {reason}")]
    UncertaintyDerivation {
        /// Control being derived.
        control: ControlId,
        /// Stable failure reason.
        reason: &'static str,
    },
    /// A tender references a supplier that is not a package candidate.
    #[error("tender {tender} references supplier {supplier} that is not a package candidate")]
    UnknownTenderSupplier {
        /// Tender control.
        tender: ControlId,
        /// Unknown supplier.
        supplier: String,
    },
    /// A tender references a different work package.
    #[error("tender {tender} package {actual} does not match work package {expected}")]
    TenderPackageMismatch {
        /// Expected work package.
        expected: ControlId,
        /// Actual work package.
        actual: ControlId,
        /// Tender control.
        tender: ControlId,
    },
    /// A tender cannot be compared for an award decision.
    #[error("tender {tender} is not comparable: {reason}")]
    NonComparableTender {
        /// Tender control.
        tender: ControlId,
        /// Reason the tender is not comparable.
        reason: &'static str,
    },
    /// A tender supersession points at a missing tender.
    #[error("tender {tender} corrects missing tender {supersedes}")]
    MissingSupersededTender {
        /// Correcting tender.
        tender: ControlId,
        /// Missing tender.
        supersedes: ControlId,
    },
    /// A tender supersession points at a tender for a different supplier.
    #[error("tender {tender} corrects tender {supersedes} from a different supplier")]
    TenderSupersessionSupplierMismatch {
        /// Correcting tender.
        tender: ControlId,
        /// Superseded tender.
        supersedes: ControlId,
    },
    /// An award was made by a role that lacks package award authority.
    #[error("award {award} by {actual} does not match authority {expected}")]
    AwardAuthorityMismatch {
        /// Award control.
        award: ControlId,
        /// Authorized role.
        expected: RoleId,
        /// Actual decision role.
        actual: RoleId,
    },
    /// An award selected a supplier that is not accepted for award.
    #[error("award {award} selected supplier {supplier} that is not awardable")]
    RejectedSupplierAward {
        /// Award control.
        award: ControlId,
        /// Rejected supplier.
        supplier: String,
    },
    /// An award references a missing tender.
    #[error("award {award} references missing tender {tender}")]
    MissingAwardTender {
        /// Award control.
        award: ControlId,
        /// Missing tender.
        tender: ControlId,
    },
    /// An award selected a tender that was not comparable.
    #[error("award {award} selected non-comparable tender {tender}")]
    AwardTenderNotComparable {
        /// Award control.
        award: ControlId,
        /// Non-comparable tender.
        tender: ControlId,
    },
    /// An award decision was made after the package need date.
    #[error("award {award} decided on {decided_on} after need date {need_date}")]
    AwardAfterNeedDate {
        /// Award control.
        award: ControlId,
        /// Decision date.
        decided_on: Date,
        /// Package need date.
        need_date: Date,
    },
    /// A supplier exceeds the accepted subcontract depth.
    #[error(
        "supplier {supplier} subcontract depth {depth} exceeds accepted depth {max_accepted_depth}"
    )]
    SupplierDepthExceeded {
        /// Supplier organization.
        supplier: OrganizationId,
        /// Actual subcontract depth.
        depth: u8,
        /// Accepted subcontract depth.
        max_accepted_depth: u8,
    },
    /// A supplier qualification record references a missing supplier.
    #[error("supplier qualification references missing supplier {supplier}")]
    UnknownSupplier {
        /// Supplier organization.
        supplier: OrganizationId,
    },
    /// A qualification decision references a missing requirement.
    #[error("qualification decision references missing requirement {requirement}")]
    UnknownQualificationRequirement {
        /// Missing requirement.
        requirement: ControlId,
    },
    /// A qualification decision was made by a role without supplier authority.
    #[error("supplier {supplier} qualification by {actual} does not match authority {expected}")]
    QualificationAuthorityMismatch {
        /// Supplier organization.
        supplier: OrganizationId,
        /// Authorized role.
        expected: RoleId,
        /// Actual role.
        actual: RoleId,
    },
    /// A package handoff record was not present.
    #[error("package handoff {handoff} is missing")]
    MissingPackageHandoff {
        /// Missing handoff control.
        handoff: ControlId,
    },
    /// Handoff package did not match its derived report inputs.
    #[error("handoff {handoff} does not match report package {expected}")]
    HandoffPackageMismatch {
        /// Handoff control.
        handoff: ControlId,
        /// Expected package.
        expected: ControlId,
    },
    /// The canonical Gantt plan rejected the imported schedule.
    #[error("schedule plan validation failed: {reason}")]
    SchedulePlan {
        /// Schedule validation reason.
        reason: String,
    },
    /// The imported schedule plan did not match the accepted baseline plan.
    #[error(
        "schedule plan mismatch: baseline plan {baseline_plan}, imported revision plan {imported_plan}, actual plan {actual_plan}"
    )]
    SchedulePlanMismatch {
        /// Baseline plan id.
        baseline_plan: String,
        /// Imported revision plan id.
        imported_plan: String,
        /// Actual Gantt plan id.
        actual_plan: String,
    },
    /// The imported schedule revision is not the accepted baseline revision.
    #[error(
        "schedule baseline {baseline} accepts revision {accepted_revision}, not imported revision {imported_revision}"
    )]
    ScheduleRevisionMismatch {
        /// Accepted schedule baseline.
        baseline: BaselineId,
        /// Accepted revision.
        accepted_revision: String,
        /// Imported revision.
        imported_revision: String,
    },
    /// A control joined a task id that does not exist in the Gantt plan.
    #[error("schedule join {control} references missing task id {task_id}")]
    MissingScheduleTask {
        /// Construction control id.
        control: ControlId,
        /// Missing Gantt task id.
        task_id: String,
    },
    /// More than one construction control joined the same Gantt task id.
    #[error("duplicate schedule join task id {task_id}")]
    DuplicateScheduleTaskJoin {
        /// Duplicate Gantt task id.
        task_id: String,
    },
    /// A non-informational control-graph cycle would make readiness recursive.
    #[error("control graph has a prohibited readiness cycle: {cycle:?}")]
    ControlGraphCycle {
        /// Stable cycle member ids.
        cycle: Vec<ControlId>,
    },
    /// The canonical graph engine rejected the construction control graph.
    #[error("control graph {operation} failed: {reason}")]
    ControlGraphAlgorithm {
        /// Algorithm operation.
        operation: &'static str,
        /// Error text from the canonical graph engine.
        reason: String,
    },
}
