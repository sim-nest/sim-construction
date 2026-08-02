// conformance: sustainability, certification, climate, reuse, and place outcomes

use sim_kernel::Symbol;
use sim_lib_doc_core::ExternalRef;
use time::{Date, Month};

use crate::{
    ControlId, DisclosureState, DomainQuantity, EvidenceState, EvidenceValidity, OutcomeBlocker,
    OutcomeBoundary, OutcomeMethod, OutcomeRecord, OutcomeRecordKind, OutcomeRecordSpec,
    OutcomeTargetKind, OutcomeVariance, ProjectId, RegisteredOutcomeShape, RoleId,
    SustainabilityTarget, SustainabilityTargetSpec, evaluate_outcomes,
};

#[test]
fn mixed_methods_and_boundaries_are_not_compared_or_verified() {
    let project = project();
    let target = target("climate.a1-a5")
        .with_source_ref(ext("charter/climate"))
        .allow_reference_claim();
    let record = record(
        "calc.forecast",
        "climate.a1-a5",
        OutcomeRecordKind::Measurement,
    )
    .with_evidence_state(EvidenceState::Accepted)
    .with_source_ref(ext("calculator/result"))
    .with_disclosure(DisclosureState::Accepted);
    let mut mismatched = record;
    mismatched.method.version = "v2".to_owned();
    mismatched.boundary.source_scope = "A1-A4".to_owned();

    let report = evaluate_outcomes(
        project,
        &[target],
        &[mismatched],
        date(2026, Month::July, 25),
    )
    .unwrap();
    let target_report = &report.targets[0];

    assert!(!target_report.covered);
    assert_eq!(
        target_report.variance,
        OutcomeVariance::NotComparable {
            reason: "method".to_owned()
        }
    );
    assert!(
        target_report
            .blockers
            .iter()
            .any(|blocker| matches!(blocker, OutcomeBlocker::MethodMismatch(_)))
    );
    assert!(
        target_report
            .blockers
            .iter()
            .any(|blocker| matches!(blocker, OutcomeBlocker::BoundaryMismatch(_)))
    );
}

#[test]
fn superseded_calculations_do_not_support_current_claims() {
    let project = project();
    let target = target("reuse.steel")
        .with_source_ref(ext("charter/reuse"))
        .allow_reference_claim();
    let stale = record("calc.old", "reuse.steel", OutcomeRecordKind::Measurement)
        .with_evidence_state(EvidenceState::Accepted)
        .with_source_ref(ext("calculator/old"))
        .with_disclosure(DisclosureState::Accepted);
    let current = record("calc.new", "reuse.steel", OutcomeRecordKind::Measurement)
        .with_evidence_state(EvidenceState::Accepted)
        .with_source_ref(ext("calculator/new"))
        .with_disclosure(DisclosureState::Accepted)
        .supersedes(id("calc.old"));

    let report = evaluate_outcomes(
        project,
        &[target],
        &[stale, current],
        date(2026, Month::July, 25),
    )
    .unwrap();

    assert_eq!(report.targets[0].current_record, Some(id("calc.new")));
    assert!(report.targets[0].reference_claim_admissible);
}

#[test]
fn expired_certificate_blocks_gate_but_keeps_provenance() {
    let project = project();
    let target = target("cert.breeam")
        .with_source_ref(ext("charter/certification"))
        .allow_reference_claim();
    let certificate = record(
        "certificate.expired",
        "cert.breeam",
        OutcomeRecordKind::Certificate,
    )
    .with_evidence_state(EvidenceState::Accepted)
    .with_validity(EvidenceValidity::new(
        Some(date(2025, Month::January, 1)),
        Some(date(2026, Month::January, 1)),
    ))
    .with_source_ref(ext("scheme/certificate"))
    .with_disclosure(DisclosureState::Accepted);

    let report = evaluate_outcomes(
        project,
        &[target],
        &[certificate],
        date(2026, Month::July, 25),
    )
    .unwrap();

    assert!(!report.gates_clear);
    assert!(
        report.targets[0]
            .blockers
            .iter()
            .any(|blocker| matches!(blocker, OutcomeBlocker::ExpiredEvidence(_)))
    );
}

#[test]
fn reuse_substitution_can_cover_target_without_local_unit_conversion() {
    let project = project();
    let mut target = target("reuse.beams")
        .with_source_ref(ext("charter/reuse-beams"))
        .allow_reference_claim();
    target.target = DomainQuantity::new("12", symbol("unit/pieces"));
    let mut substitution = record(
        "reuse.substitution",
        "reuse.beams",
        OutcomeRecordKind::ReuseSubstitution,
    )
    .with_evidence_state(EvidenceState::Accepted)
    .with_source_ref(ext("materials/substitution"))
    .with_disclosure(DisclosureState::Accepted);
    substitution.quantity = DomainQuantity::new("12", symbol("unit/pieces"));

    let report = evaluate_outcomes(
        project,
        &[target],
        &[substitution],
        date(2026, Month::July, 25),
    )
    .unwrap();

    assert!(report.gates_clear);
    assert_eq!(report.targets[0].variance, OutcomeVariance::OnTarget);
}

#[test]
fn missing_source_reference_blocks_reference_claim() {
    let project = project();
    let target = target("climate.operational").allow_reference_claim();
    let record = record(
        "calc.no-source",
        "climate.operational",
        OutcomeRecordKind::Measurement,
    )
    .with_evidence_state(EvidenceState::Accepted)
    .with_disclosure(DisclosureState::Accepted);

    let report =
        evaluate_outcomes(project, &[target], &[record], date(2026, Month::July, 25)).unwrap();

    assert!(!report.targets[0].reference_claim_admissible);
    assert!(
        report.targets[0]
            .blockers
            .iter()
            .any(|blocker| matches!(blocker, OutcomeBlocker::MissingTargetSourceRef(_)))
    );
    assert!(
        report.targets[0]
            .blockers
            .iter()
            .any(|blocker| matches!(blocker, OutcomeBlocker::MissingRecordSourceRef(_)))
    );
}

#[test]
fn disclosure_rejection_prevents_outcome_reference_claim() {
    let project = project();
    let target = target("place.social-value")
        .with_source_ref(ext("charter/place"))
        .allow_reference_claim();
    let record = record(
        "place.measurement",
        "place.social-value",
        OutcomeRecordKind::Measurement,
    )
    .with_evidence_state(EvidenceState::Accepted)
    .with_source_ref(ext("outcome/place"))
    .with_disclosure(DisclosureState::Rejected);

    let report =
        evaluate_outcomes(project, &[target], &[record], date(2026, Month::July, 25)).unwrap();

    assert!(!report.targets[0].reference_claim_admissible);
    assert!(
        report.targets[0]
            .blockers
            .iter()
            .any(|blocker| matches!(blocker, OutcomeBlocker::DisclosureRejected(_)))
    );
}

fn target(id_text: &str) -> SustainabilityTarget {
    SustainabilityTarget::new(SustainabilityTargetSpec {
        project: project(),
        id: id(id_text),
        kind: OutcomeTargetKind::Climate,
        category: RegisteredOutcomeShape::new(
            symbol("construction-outcome/climate"),
            symbol("shape/climate"),
        ),
        title: "Climate budget".to_owned(),
        target: DomainQuantity::new("100", symbol("unit/kg-co2e")),
        method: method(),
        boundary: boundary(),
        responsible: role("sustainability-lead"),
    })
}

fn record(id_text: &str, target: &str, kind: OutcomeRecordKind) -> OutcomeRecord {
    OutcomeRecord::new(OutcomeRecordSpec {
        project: project(),
        id: id(id_text),
        target: id(target),
        kind,
        quantity: DomainQuantity::new("100", symbol("unit/kg-co2e")),
        method: method(),
        boundary: boundary(),
        responsible: role("sustainability-lead"),
        reported_on: date(2026, Month::July, 20),
    })
}

fn method() -> OutcomeMethod {
    OutcomeMethod::new(
        symbol("method/en15978"),
        "v1",
        symbol("shape/en15978"),
        ext("methods/en15978"),
    )
}

fn boundary() -> OutcomeBoundary {
    OutcomeBoundary::new(symbol("boundary/lifecycle"), "A1-A5", ext("boundary/a1-a5"))
}

fn project() -> ProjectId {
    ProjectId::new("reference-center").unwrap()
}

fn id(value: &str) -> ControlId {
    ControlId::new(value).unwrap()
}

fn role(value: &str) -> RoleId {
    RoleId::new(value).unwrap()
}

fn symbol(value: &str) -> Symbol {
    let (namespace, name) = value.split_once('/').unwrap();
    Symbol::qualified(namespace, name)
}

fn ext(id: &str) -> ExternalRef {
    ExternalRef::new("doc/synthetic", id, Some("rev-a".to_owned()), None)
}

fn date(year: i32, month: Month, day: u8) -> Date {
    Date::from_calendar_date(year, month, day).unwrap()
}
