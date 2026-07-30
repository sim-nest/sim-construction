//! Shared field-control records and safety-first project rollups.

use sim_lib_doc_core::ExternalRef;
use time::Date;

use crate::{ConstructionProjectError, ControlId, EvidenceState, ProjectId, Result, RoleId};

/// Kind of field reality represented by a project-control item.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FieldItemKind {
    /// General production observation.
    Observation,
    /// Deviation from an accepted requirement or method.
    Deviation,
    /// Safety, work-environment, environmental, quality, or production incident.
    Incident,
    /// Planned or performed inspection point.
    InspectionPoint,
    /// Planned or performed test point.
    TestPoint,
    /// Confirmed defect.
    Defect,
    /// Action intended to correct a field condition.
    CorrectiveAction,
    /// Reference to an item retained by an external field system.
    ExternalReference,
}

/// Project-control lane used for field-item reporting and rollup.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FieldLane {
    /// Immediate physical safety.
    Safety,
    /// Occupational health and work environment.
    WorkEnvironment,
    /// Production progress and readiness.
    Progress,
    /// Quality and conformance.
    Quality,
    /// Environmental performance or harm.
    Environment,
    /// Convenience or presentation without a production blocker.
    Convenience,
}

/// Consequence severity carried independently from the field-item lane.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum FieldSeverity {
    /// Harm is occurring or can occur before the normal control cycle reacts.
    Imminent,
    /// Critical consequence requiring immediate accountable attention.
    Critical,
    /// Major consequence that blocks or materially impairs controlled work.
    Major,
    /// Moderate consequence within the normal control cycle.
    Moderate,
    /// Minor consequence with bounded local impact.
    Minor,
    /// Informational signal with no present adverse consequence.
    Information,
}

/// Lifecycle state shared by field-control items.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FieldItemState {
    /// Newly reported and awaiting triage.
    Reported,
    /// Accepted into active control.
    Open,
    /// Work is in progress.
    InProgress,
    /// Work or acceptance is blocked.
    Blocked,
    /// A correction is reported but awaits acceptance.
    Corrected,
    /// Accountable control is closed.
    Closed,
    /// The report or proposed result was rejected.
    Rejected,
}

impl FieldItemState {
    /// Returns whether the item remains active in a field-control rollup.
    #[must_use]
    pub fn is_active(self) -> bool {
        !matches!(self, Self::Closed)
    }
}

/// Common accountable record carried by every field-control item.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FieldItem {
    /// Stable construction project identity.
    pub project: ProjectId,
    /// Stable control id for this item.
    pub control: ControlId,
    /// Field reality represented by this item.
    pub kind: FieldItemKind,
    /// Consequence severity, independent from lane and requirement policy.
    pub severity: FieldSeverity,
    /// Project-control lane.
    pub lane: FieldLane,
    /// Role accountable for the next state transition.
    pub responsible_role: RoleId,
    /// Calendar due date, when the item has one.
    pub due_on: Option<Date>,
    /// Project controls affected by this item.
    pub affected_control_ids: Vec<ControlId>,
    /// Current item state.
    pub state: FieldItemState,
    /// Current evidence state.
    pub evidence_state: EvidenceState,
    /// True when the governing requirement cannot be waived.
    pub non_waivable: bool,
    /// Reference-only supporting evidence.
    pub evidence: Vec<ExternalRef>,
}

impl FieldItem {
    /// Builds a reported field-control item.
    #[must_use]
    pub fn new(
        project: ProjectId,
        control: ControlId,
        kind: FieldItemKind,
        severity: FieldSeverity,
        lane: FieldLane,
        responsible_role: RoleId,
    ) -> Self {
        Self {
            project,
            control,
            kind,
            severity,
            lane,
            responsible_role,
            due_on: None,
            affected_control_ids: Vec::new(),
            state: FieldItemState::Reported,
            evidence_state: EvidenceState::Reported,
            non_waivable: false,
            evidence: Vec::new(),
        }
    }

    /// Sets the due date.
    #[must_use]
    pub fn due_on(mut self, due_on: Date) -> Self {
        self.due_on = Some(due_on);
        self
    }

    /// Adds an affected project control.
    #[must_use]
    pub fn affects(mut self, control: ControlId) -> Self {
        self.affected_control_ids.push(control);
        self
    }

    /// Sets the item state.
    #[must_use]
    pub fn with_state(mut self, state: FieldItemState) -> Self {
        self.state = state;
        self
    }

    /// Sets consequence severity.
    #[must_use]
    pub fn with_severity(mut self, severity: FieldSeverity) -> Self {
        self.severity = severity;
        self
    }

    /// Sets the evidence state.
    #[must_use]
    pub fn with_evidence_state(mut self, evidence_state: EvidenceState) -> Self {
        self.evidence_state = evidence_state;
        self
    }

    /// Marks the governing requirement as non-waivable.
    #[must_use]
    pub fn non_waivable(mut self) -> Self {
        self.non_waivable = true;
        self
    }

    /// Adds a reference-only evidence link.
    #[must_use]
    pub fn with_evidence(mut self, evidence: ExternalRef) -> Self {
        self.evidence.push(evidence);
        self
    }

    /// Returns whether the item is overdue and remains active.
    #[must_use]
    pub fn is_overdue(&self, as_of: Date) -> bool {
        self.state.is_active() && self.due_on.is_some_and(|due_on| due_on < as_of)
    }

    /// Validates the common field-control invariants.
    pub fn validate(&self) -> Result<()> {
        if self.affected_control_ids.is_empty() {
            return Err(ConstructionProjectError::EmptyCollection(
                "field_item.affected_control_ids",
            ));
        }
        if matches!(
            self.state,
            FieldItemState::Corrected | FieldItemState::Closed
        ) && (self.evidence.is_empty() || !self.evidence_state.satisfies_required_evidence())
        {
            return Err(ConstructionProjectError::EmptyCollection(
                "field_item.accepted_evidence",
            ));
        }
        Ok(())
    }
}

/// Stable correlation to an externally retained field item.
///
/// The type deliberately has no payload, attachment, credential, or URL field.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct FieldItemReference {
    /// Stable source namespace, normally a registered site id.
    pub source: String,
    /// Source-local stable item id.
    pub external_id: String,
    /// Optional source revision or state marker.
    pub version: Option<String>,
}

impl FieldItemReference {
    /// Builds a stable field-item correlation.
    pub fn new(
        source: impl Into<String>,
        external_id: impl Into<String>,
        version: Option<String>,
    ) -> Result<Self> {
        let source = source.into();
        let external_id = external_id.into();
        if source.trim().is_empty() {
            return Err(ConstructionProjectError::EmptyField(
                "field_item_reference.source",
            ));
        }
        if external_id.trim().is_empty() {
            return Err(ConstructionProjectError::EmptyField(
                "field_item_reference.external_id",
            ));
        }
        Ok(Self {
            source,
            external_id,
            version,
        })
    }

    /// Projects the correlation as a URL-free external reference.
    #[must_use]
    pub fn as_external_ref(&self) -> ExternalRef {
        ExternalRef::new(
            self.source.clone(),
            self.external_id.clone(),
            self.version.clone(),
            None,
        )
    }
}

/// One derived safety-first rollup row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldRollupEntry {
    /// Stable field-item control id.
    pub control: ControlId,
    /// Item kind.
    pub kind: FieldItemKind,
    /// Item lane.
    pub lane: FieldLane,
    /// Item severity.
    pub severity: FieldSeverity,
    /// Whether the governing requirement is non-waivable.
    pub non_waivable: bool,
    /// Whether the item is overdue as of the rollup date.
    pub overdue: bool,
    /// Calendar due date, when the item has one.
    pub due_on: Option<Date>,
    /// Current item state.
    pub state: FieldItemState,
}

/// Builds a deterministic safety-first field-control rollup.
#[must_use]
pub fn safety_first_rollup(items: &[FieldItem], as_of: Date) -> Vec<FieldRollupEntry> {
    let mut rows = items
        .iter()
        .map(|item| FieldRollupEntry {
            control: item.control.clone(),
            kind: item.kind,
            lane: item.lane,
            severity: item.severity,
            non_waivable: item.non_waivable,
            overdue: item.is_overdue(as_of),
            due_on: item.due_on,
            state: item.state,
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| rollup_key(left).cmp(&rollup_key(right)));
    rows
}

fn rollup_key(row: &FieldRollupEntry) -> (u8, u8, u8, u8, Option<Date>, &ControlId) {
    (
        priority_group(row),
        u8::from(!row.non_waivable),
        severity_rank(row.severity),
        u8::from(!row.overdue),
        row.due_on,
        &row.control,
    )
}

fn priority_group(row: &FieldRollupEntry) -> u8 {
    if row.state.is_active()
        && row.severity == FieldSeverity::Imminent
        && matches!(row.lane, FieldLane::Safety | FieldLane::WorkEnvironment)
    {
        return 0;
    }
    if row.state.is_active() && row.non_waivable {
        return 1;
    }
    if !row.state.is_active() {
        return 8;
    }
    match row.lane {
        FieldLane::Safety => 2,
        FieldLane::WorkEnvironment => 3,
        FieldLane::Progress => 4,
        FieldLane::Quality => 5,
        FieldLane::Environment => 6,
        FieldLane::Convenience => 7,
    }
}

const fn severity_rank(severity: FieldSeverity) -> u8 {
    match severity {
        FieldSeverity::Imminent => 0,
        FieldSeverity::Critical => 1,
        FieldSeverity::Major => 2,
        FieldSeverity::Moderate => 3,
        FieldSeverity::Minor => 4,
        FieldSeverity::Information => 5,
    }
}
