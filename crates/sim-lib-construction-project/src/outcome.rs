//! Sustainability, certification, climate, reuse, and place outcome controls.

use std::collections::{BTreeMap, BTreeSet};

use sim_kernel::Symbol;
use sim_lib_doc_core::ExternalRef;
use time::Date;

use crate::outcome_symbol as symbol_text;
use crate::{
    ConstructionProjectError, ControlId, EvidenceState, EvidenceValidity, ProjectId, Result, RoleId,
};

/// A SIM Shape registered by an installed method, certification, material, or
/// outcome package.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct RegisteredOutcomeShape {
    /// Open category, scheme, material-system, or calculation-method symbol.
    #[serde(with = "symbol_text")]
    pub symbol: Symbol,
    /// Shape symbol that an installed specialist package registers.
    #[serde(with = "symbol_text")]
    pub shape: Symbol,
}

impl RegisteredOutcomeShape {
    /// Builds an open symbol and its registered Shape symbol.
    #[must_use]
    pub fn new(symbol: Symbol, shape: Symbol) -> Self {
        Self { symbol, shape }
    }
}

/// A domain quantity retained in its source representation.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DomainQuantity {
    /// Source magnitude text, exactly as reported by the source method.
    pub source_value: String,
    /// Explicit unit symbol from the source method.
    #[serde(with = "symbol_text")]
    pub unit: Symbol,
}

impl DomainQuantity {
    /// Builds a source-retained quantity.
    #[must_use]
    pub fn new(source_value: impl Into<String>, unit: Symbol) -> Self {
        Self {
            source_value: source_value.into(),
            unit,
        }
    }

    fn validate(&self, field: &'static str) -> Result<()> {
        if self.source_value.trim().is_empty() {
            return Err(ConstructionProjectError::EmptyField(field));
        }
        Ok(())
    }
}

/// Boundary for a climate, certification, reuse, or place outcome assertion.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OutcomeBoundary {
    /// Open boundary symbol.
    #[serde(with = "symbol_text")]
    pub kind: Symbol,
    /// Source-retained boundary description or version text.
    pub source_scope: String,
    /// External source defining the boundary.
    pub source_ref: ExternalRef,
}

impl OutcomeBoundary {
    /// Builds an explicit source-defined boundary.
    #[must_use]
    pub fn new(kind: Symbol, source_scope: impl Into<String>, source_ref: ExternalRef) -> Self {
        Self {
            kind,
            source_scope: source_scope.into(),
            source_ref,
        }
    }
}

/// Open calculation, certification, or disclosure method reference.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OutcomeMethod {
    /// Open method symbol.
    #[serde(with = "symbol_text")]
    pub method: Symbol,
    /// Method version retained from the source.
    pub version: String,
    /// Registered Shape supplied by the installed specialist package.
    #[serde(with = "symbol_text")]
    pub shape: Symbol,
    /// Source or calculator reference that owns the method result.
    pub source_ref: ExternalRef,
}

impl OutcomeMethod {
    /// Builds a method reference without implementing the method locally.
    #[must_use]
    pub fn new(
        method: Symbol,
        version: impl Into<String>,
        shape: Symbol,
        source_ref: ExternalRef,
    ) -> Self {
        Self {
            method,
            version: version.into(),
            shape,
            source_ref,
        }
    }
}

/// Target class for construction sustainability and place controls.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum OutcomeTargetKind {
    /// Certification scheme target.
    Certification,
    /// Climate boundary and budget target.
    Climate,
    /// Energy or resource efficiency target.
    Efficiency,
    /// Reuse target.
    Reuse,
    /// Waste target.
    Waste,
    /// Responsible materials or supply-chain target.
    ResponsibleMaterials,
    /// Quality target.
    Quality,
    /// Safety target.
    Safety,
    /// Work-environment target.
    WorkEnvironment,
    /// Property or city-district outcome target.
    Place,
}

/// Chartered target that can be traced to method, boundary, role, and evidence.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SustainabilityTarget {
    /// Project identity.
    pub project: ProjectId,
    /// Stable target id.
    pub id: ControlId,
    /// Target class.
    pub kind: OutcomeTargetKind,
    /// Open outcome category symbol.
    pub category: RegisteredOutcomeShape,
    /// Target title.
    pub title: String,
    /// Target value retained in source representation.
    pub target: DomainQuantity,
    /// Optional baseline value retained in source representation.
    pub baseline: Option<DomainQuantity>,
    /// Required method family.
    pub method: OutcomeMethod,
    /// Required boundary.
    pub boundary: OutcomeBoundary,
    /// Role responsible for producing evidence.
    pub responsible: RoleId,
    /// Date by which evidence is due.
    pub due_on: Option<Date>,
    /// Evidence validity policy.
    pub evidence_validity: EvidenceValidity,
    /// Source references that chartered the target.
    pub source_refs: Vec<ExternalRef>,
    /// True when a reference-published claim may be made for this target.
    pub reference_claim_allowed: bool,
}

/// Required fields for a chartered sustainability target.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SustainabilityTargetSpec {
    /// Project identity.
    pub project: ProjectId,
    /// Stable target id.
    pub id: ControlId,
    /// Target class.
    pub kind: OutcomeTargetKind,
    /// Open outcome category symbol and registered Shape.
    pub category: RegisteredOutcomeShape,
    /// Target title.
    pub title: String,
    /// Source-retained target quantity.
    pub target: DomainQuantity,
    /// Required method family.
    pub method: OutcomeMethod,
    /// Required boundary.
    pub boundary: OutcomeBoundary,
    /// Responsible role.
    pub responsible: RoleId,
}

impl SustainabilityTarget {
    /// Builds a chartered target.
    #[must_use]
    pub fn new(spec: SustainabilityTargetSpec) -> Self {
        Self {
            project: spec.project,
            id: spec.id,
            kind: spec.kind,
            category: spec.category,
            title: spec.title,
            target: spec.target,
            baseline: None,
            method: spec.method,
            boundary: spec.boundary,
            responsible: spec.responsible,
            due_on: None,
            evidence_validity: EvidenceValidity::unbounded(),
            source_refs: Vec::new(),
            reference_claim_allowed: false,
        }
    }

    /// Adds a baseline quantity.
    #[must_use]
    pub fn with_baseline(mut self, baseline: DomainQuantity) -> Self {
        self.baseline = Some(baseline);
        self
    }

    /// Adds a source reference.
    #[must_use]
    pub fn with_source_ref(mut self, source_ref: ExternalRef) -> Self {
        self.source_refs.push(source_ref);
        self
    }

    /// Sets the evidence due date.
    #[must_use]
    pub fn due_on(mut self, due_on: Date) -> Self {
        self.due_on = Some(due_on);
        self
    }

    /// Sets the evidence validity window.
    #[must_use]
    pub fn with_evidence_validity(mut self, validity: EvidenceValidity) -> Self {
        self.evidence_validity = validity;
        self
    }

    /// Allows reference-publication when a current accepted measurement exists.
    #[must_use]
    pub fn allow_reference_claim(mut self) -> Self {
        self.reference_claim_allowed = true;
        self
    }

    /// Validates the local target shape.
    pub fn validate(&self) -> Result<()> {
        if self.title.trim().is_empty() {
            return Err(ConstructionProjectError::EmptyField(
                "sustainability_target.title",
            ));
        }
        self.target.validate("sustainability_target.target")?;
        if let Some(baseline) = &self.baseline {
            baseline.validate("sustainability_target.baseline")?;
        }
        Ok(())
    }
}

/// Whether a reported outcome may be disclosed outside project control.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DisclosureState {
    /// Disclosure review is not complete.
    Pending,
    /// Disclosure is accepted for the named reference claim.
    Accepted,
    /// Disclosure has been rejected.
    Rejected,
    /// The outcome is project-internal only.
    Restricted,
}

/// Whether a record is a baseline, forecast, measured result, certificate, or substitution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum OutcomeRecordKind {
    /// Baseline from a chartered method.
    Baseline,
    /// Forecast or estimate.
    Forecast,
    /// Measurement or verified specialist result.
    Measurement,
    /// Certification status from a scheme.
    Certificate,
    /// Reuse or material-system substitution.
    ReuseSubstitution,
}

/// A reported outcome, retained as a reference-backed claim and not a local calculation.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OutcomeRecord {
    /// Project identity.
    pub project: ProjectId,
    /// Stable record id.
    pub id: ControlId,
    /// Target this record supports.
    pub target: ControlId,
    /// Record kind.
    pub kind: OutcomeRecordKind,
    /// Reported value retained in source representation.
    pub quantity: DomainQuantity,
    /// Method or certification scheme that owns the calculation.
    pub method: OutcomeMethod,
    /// Boundary used by the source calculation.
    pub boundary: OutcomeBoundary,
    /// Responsible reporting role.
    pub responsible: RoleId,
    /// Reporting date.
    pub reported_on: Date,
    /// Evidence state for this record.
    pub evidence_state: EvidenceState,
    /// Optional validity window for certificates or measured results.
    pub validity: EvidenceValidity,
    /// External references carrying the source calculation or certificate.
    pub source_refs: Vec<ExternalRef>,
    /// Evidence references reviewed by the project.
    pub evidence_refs: Vec<ExternalRef>,
    /// Optional record superseded by this record.
    pub supersedes: Option<ControlId>,
    /// Disclosure decision for reference publication.
    pub disclosure: DisclosureState,
}

/// Required fields for an externally owned outcome record.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OutcomeRecordSpec {
    /// Project identity.
    pub project: ProjectId,
    /// Stable record id.
    pub id: ControlId,
    /// Target this record supports.
    pub target: ControlId,
    /// Record kind.
    pub kind: OutcomeRecordKind,
    /// Source-retained quantity.
    pub quantity: DomainQuantity,
    /// External method owner.
    pub method: OutcomeMethod,
    /// Source-defined boundary.
    pub boundary: OutcomeBoundary,
    /// Responsible role.
    pub responsible: RoleId,
    /// Reporting date.
    pub reported_on: Date,
}

impl OutcomeRecord {
    /// Builds an outcome record.
    #[must_use]
    pub fn new(spec: OutcomeRecordSpec) -> Self {
        Self {
            project: spec.project,
            id: spec.id,
            target: spec.target,
            kind: spec.kind,
            quantity: spec.quantity,
            method: spec.method,
            boundary: spec.boundary,
            responsible: spec.responsible,
            reported_on: spec.reported_on,
            evidence_state: EvidenceState::Reported,
            validity: EvidenceValidity::unbounded(),
            source_refs: Vec::new(),
            evidence_refs: Vec::new(),
            supersedes: None,
            disclosure: DisclosureState::Pending,
        }
    }

    /// Sets the evidence state.
    #[must_use]
    pub fn with_evidence_state(mut self, state: EvidenceState) -> Self {
        self.evidence_state = state;
        self
    }

    /// Sets the validity window.
    #[must_use]
    pub fn with_validity(mut self, validity: EvidenceValidity) -> Self {
        self.validity = validity;
        self
    }

    /// Adds a source reference.
    #[must_use]
    pub fn with_source_ref(mut self, source_ref: ExternalRef) -> Self {
        self.source_refs.push(source_ref);
        self
    }

    /// Adds reviewed evidence.
    #[must_use]
    pub fn with_evidence_ref(mut self, evidence_ref: ExternalRef) -> Self {
        self.evidence_refs.push(evidence_ref);
        self
    }

    /// Marks a prior record superseded.
    #[must_use]
    pub fn supersedes(mut self, prior: ControlId) -> Self {
        self.supersedes = Some(prior);
        self
    }

    /// Sets disclosure state.
    #[must_use]
    pub fn with_disclosure(mut self, disclosure: DisclosureState) -> Self {
        self.disclosure = disclosure;
        self
    }

    pub(crate) fn validate(&self) -> Result<()> {
        self.quantity.validate("outcome_record.quantity")?;
        Ok(())
    }
}

/// Variance derivation without unit conversion.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum OutcomeVariance {
    /// Current measurement exactly matches the target source value and unit.
    OnTarget,
    /// Current measurement has the same unit but a different source value.
    ReportedDifferent {
        /// Target source-retained value.
        target: DomainQuantity,
        /// Current source-retained value.
        current: DomainQuantity,
    },
    /// Current measurement uses a different unit, method, or boundary.
    NotComparable {
        /// Reason comparison is intentionally withheld.
        reason: String,
    },
    /// No current accepted measurement is available.
    NoCurrentMeasurement,
}

/// Target-level blocker derived from explicit records.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum OutcomeBlocker {
    /// Target has no source reference.
    MissingTargetSourceRef(ControlId),
    /// Current record has no source reference.
    MissingRecordSourceRef(ControlId),
    /// Required evidence is overdue.
    OverdueEvidence {
        /// Target whose evidence is overdue.
        target: ControlId,
        /// Due date.
        due_on: Date,
    },
    /// No current accepted measurement exists.
    MissingAcceptedMeasurement(ControlId),
    /// Current method differs from the chartered method.
    MethodMismatch(ControlId),
    /// Current boundary differs from the chartered boundary.
    BoundaryMismatch(ControlId),
    /// Certificate or measurement validity has expired.
    ExpiredEvidence(ControlId),
    /// Current evidence is not accepted.
    EvidenceNotAccepted {
        /// Record with insufficient evidence.
        record: ControlId,
        /// Evidence state.
        state: EvidenceState,
    },
    /// Reference claim is not allowed by target policy.
    ReferenceClaimNotAllowed(ControlId),
    /// Reference claim disclosure was not accepted.
    DisclosureRejected(ControlId),
}

/// Per-target sustainability outcome derivation.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OutcomeTargetReport {
    /// Target id.
    pub target: ControlId,
    /// Current non-superseded record, when present.
    pub current_record: Option<ControlId>,
    /// Forecast records retained separately from verified outcomes.
    pub forecasts: Vec<ControlId>,
    /// True when a current accepted measurement covers the target.
    pub covered: bool,
    /// Derived variance without unit conversion.
    pub variance: OutcomeVariance,
    /// Blockers for gates depending on this target.
    pub blockers: Vec<OutcomeBlocker>,
    /// True when the current record can support a reference-published claim.
    pub reference_claim_admissible: bool,
}

/// Sustainability and place outcome report.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OutcomeControlReport {
    /// Project identity.
    pub project: ProjectId,
    /// Date used for due-date and validity checks.
    pub as_of: Date,
    /// Per-target reports.
    pub targets: Vec<OutcomeTargetReport>,
    /// True when no target carries a gate blocker.
    pub gates_clear: bool,
}

/// Evaluates targets and records without implementing a calculator,
/// certification engine, ESG database, or unit-conversion framework.
pub fn evaluate_outcomes(
    project: ProjectId,
    targets: &[SustainabilityTarget],
    records: &[OutcomeRecord],
    as_of: Date,
) -> Result<OutcomeControlReport> {
    let mut reports = Vec::with_capacity(targets.len());
    let mut superseded = BTreeSet::new();
    for record in records {
        record.validate()?;
        if let Some(prior) = &record.supersedes {
            superseded.insert(prior.clone());
        }
    }

    let mut records_by_target: BTreeMap<&ControlId, Vec<&OutcomeRecord>> = BTreeMap::new();
    for record in records
        .iter()
        .filter(|record| record.project == project && !superseded.contains(&record.id))
    {
        records_by_target
            .entry(&record.target)
            .or_default()
            .push(record);
    }

    for target in targets.iter().filter(|target| target.project == project) {
        target.validate()?;
        let target_records = records_by_target
            .get(&target.id)
            .map_or(&[][..], Vec::as_slice);
        let forecasts = target_records
            .iter()
            .filter(|record| record.kind == OutcomeRecordKind::Forecast)
            .map(|record| record.id.clone())
            .collect();
        let current = target_records
            .iter()
            .filter(|record| {
                matches!(
                    record.kind,
                    OutcomeRecordKind::Measurement
                        | OutcomeRecordKind::Certificate
                        | OutcomeRecordKind::ReuseSubstitution
                )
            })
            .max_by_key(|record| record.reported_on);

        let mut blockers = Vec::new();
        if target.source_refs.is_empty() {
            blockers.push(OutcomeBlocker::MissingTargetSourceRef(target.id.clone()));
        }
        if target.due_on.is_some_and(|due_on| due_on < as_of) && current.is_none() {
            blockers.push(OutcomeBlocker::OverdueEvidence {
                target: target.id.clone(),
                due_on: target.due_on.expect("checked above"),
            });
        }

        let (current_record, covered, variance, reference_claim_admissible) =
            if let Some(record) = current {
                if record.source_refs.is_empty() {
                    blockers.push(OutcomeBlocker::MissingRecordSourceRef(record.id.clone()));
                }
                if record.method.method != target.method.method
                    || record.method.version != target.method.version
                {
                    blockers.push(OutcomeBlocker::MethodMismatch(record.id.clone()));
                }
                if record.boundary.kind != target.boundary.kind
                    || record.boundary.source_scope != target.boundary.source_scope
                {
                    blockers.push(OutcomeBlocker::BoundaryMismatch(record.id.clone()));
                }
                if !record.validity.contains(as_of) || !target.evidence_validity.contains(as_of) {
                    blockers.push(OutcomeBlocker::ExpiredEvidence(record.id.clone()));
                }
                if !record.evidence_state.satisfies_required_evidence() {
                    blockers.push(OutcomeBlocker::EvidenceNotAccepted {
                        record: record.id.clone(),
                        state: record.evidence_state,
                    });
                }

                let variance = if record.method.method != target.method.method
                    || record.method.version != target.method.version
                {
                    OutcomeVariance::NotComparable {
                        reason: "method".to_owned(),
                    }
                } else if record.boundary.kind != target.boundary.kind
                    || record.boundary.source_scope != target.boundary.source_scope
                {
                    OutcomeVariance::NotComparable {
                        reason: "boundary".to_owned(),
                    }
                } else if record.quantity.unit != target.target.unit {
                    OutcomeVariance::NotComparable {
                        reason: "unit".to_owned(),
                    }
                } else if record.quantity.source_value == target.target.source_value {
                    OutcomeVariance::OnTarget
                } else {
                    OutcomeVariance::ReportedDifferent {
                        target: target.target.clone(),
                        current: record.quantity.clone(),
                    }
                };

                let mut reference_claim_admissible = target.reference_claim_allowed
                    && record.evidence_state.satisfies_required_evidence()
                    && record.disclosure == DisclosureState::Accepted
                    && record.validity.contains(as_of)
                    && record.quantity.unit == target.target.unit
                    && record.method.method == target.method.method
                    && record.method.version == target.method.version
                    && record.boundary.kind == target.boundary.kind
                    && record.boundary.source_scope == target.boundary.source_scope
                    && !record.source_refs.is_empty();

                if !target.reference_claim_allowed {
                    blockers.push(OutcomeBlocker::ReferenceClaimNotAllowed(target.id.clone()));
                    reference_claim_admissible = false;
                }
                if matches!(
                    record.disclosure,
                    DisclosureState::Rejected | DisclosureState::Restricted
                ) {
                    blockers.push(OutcomeBlocker::DisclosureRejected(record.id.clone()));
                    reference_claim_admissible = false;
                }

                (
                    Some(record.id.clone()),
                    record.evidence_state.satisfies_required_evidence()
                        && blockers.iter().all(|blocker| {
                            !matches!(
                                blocker,
                                OutcomeBlocker::MethodMismatch(_)
                                    | OutcomeBlocker::BoundaryMismatch(_)
                                    | OutcomeBlocker::ExpiredEvidence(_)
                                    | OutcomeBlocker::EvidenceNotAccepted { .. }
                                    | OutcomeBlocker::MissingRecordSourceRef(_)
                            )
                        }),
                    variance,
                    reference_claim_admissible,
                )
            } else {
                blockers.push(OutcomeBlocker::MissingAcceptedMeasurement(
                    target.id.clone(),
                ));
                (None, false, OutcomeVariance::NoCurrentMeasurement, false)
            };

        reports.push(OutcomeTargetReport {
            target: target.id.clone(),
            current_record,
            forecasts,
            covered,
            variance,
            blockers,
            reference_claim_admissible,
        });
    }

    let gates_clear = reports
        .iter()
        .all(|report| report.blockers.is_empty() && report.covered);
    Ok(OutcomeControlReport {
        project,
        as_of,
        targets: reports,
        gates_clear,
    })
}
