// conformance: supplier qualification and production handoff readiness

use crate::{
    AwardDecision, AwardDecisionKind, CONSTRUCTION_SUPPLIER_READ_CAPABILITY, CommercialAmount,
    ConstructionProjectError, ControlId, CurrencyCode, DesignControlSet, DesignRelease,
    DesignReleasePurpose, DesignRevision, EvidenceState, EvidenceValidity, OrganizationId,
    PackageHandoff, PackageHandoffControlSet, ProcurementControlSet, QualificationEvidence,
    QualificationRequirement, QualificationStatus, Requirement, RequirementLane, ScopeCompliance,
    SupplierCandidate, SupplierQualificationArea, SupplierQualificationSet, SupplierReference,
    TenderComparison, TenderQualification, WorkPackage,
};
use sim_kernel::Symbol;
use sim_lib_doc_core::ExternalRef;
use time::{Date, Month};

#[test]
fn qualified_supplier_and_accepted_handoff_are_production_ready() {
    let procurement = awarded_procurement()
        .readiness_for(&work_package(), &currency("SEK"), day(20))
        .unwrap();
    let supplier = qualification_set()
        .qualification_for(&org("supplier-alpha"), day(20), day(30))
        .unwrap();
    let design = design_set()
        .readiness_for(package(), DesignReleasePurpose::Production, day(20))
        .unwrap();
    let report = handoff_set()
        .readiness_for(
            &control("handoff.frame"),
            &procurement,
            &supplier,
            &design,
            day(20),
        )
        .unwrap();

    assert_eq!(supplier.status, QualificationStatus::Qualified);
    assert!(report.ready);
    assert!(report.blockers.is_empty());
}

#[test]
fn excessive_subcontract_depth_is_rejected() {
    let result = SupplierQualificationSet::new()
        .with_supplier(
            SupplierReference::new(
                project(),
                org("supplier-tier3"),
                role("installer"),
                role("qs"),
            )
            .under_parent(org("supplier-tier2"), 3, 2)
            .with_evidence(reference("supplier/tier3", "chain")),
        )
        .with_requirement(requirement(
            "qual.insurance",
            SupplierQualificationArea::Insurance,
        ))
        .qualification_for(&org("supplier-tier3"), day(20), day(30));

    assert!(matches!(
        result,
        Err(ConstructionProjectError::SupplierDepthExceeded { .. })
    ));
}

#[test]
fn expired_license_or_insurance_blocks_qualification() {
    let report = qualification_set()
        .with_evidence(
            QualificationEvidence::new(
                org("supplier-alpha"),
                control("qual.license"),
                EvidenceState::Accepted,
                role("qs"),
                day(10),
                "license accepted but expired",
            )
            .with_validity(EvidenceValidity::new(None, Some(day(19))))
            .with_evidence(reference("license/alpha", "expired")),
        )
        .qualification_for(&org("supplier-alpha"), day(20), day(30))
        .unwrap();

    assert_eq!(report.status, QualificationStatus::Conflicted);
    assert!(
        report
            .explanations
            .iter()
            .any(|text| text.contains("qual.license"))
    );
}

#[test]
fn missing_workplace_introduction_blocks_qualification() {
    let report = base_qualification_without_intro()
        .qualification_for(&org("supplier-alpha"), day(20), day(30))
        .unwrap();

    assert_eq!(report.status, QualificationStatus::NotQualified);
    assert!(
        report
            .explanations
            .iter()
            .any(|text| { text == "requirement qual.workplace-introduction is not accepted" })
    );
}

#[test]
fn supplier_substitution_blocks_handoff() {
    let procurement = awarded_procurement()
        .readiness_for(&work_package(), &currency("SEK"), day(20))
        .unwrap();
    let supplier = qualification_set()
        .qualification_for(&org("supplier-alpha"), day(20), day(30))
        .unwrap();
    let design = design_set()
        .readiness_for(package(), DesignReleasePurpose::Production, day(20))
        .unwrap();
    let substituted = PackageHandoffControlSet::new().with_handoff(
        handoff()
            .with_supplier(org("supplier-beta"))
            .with_evidence(reference("handoff/frame", "accepted-substitute")),
    );
    let report = substituted
        .readiness_for(
            &control("handoff.frame"),
            &procurement,
            &supplier,
            &design,
            day(20),
        )
        .unwrap();

    assert!(!report.ready);
    assert!(
        report
            .blockers
            .iter()
            .any(|blocker| blocker.rule == "supplier-substitution")
    );
}

#[test]
fn conflicting_qualification_is_reported() {
    let report = qualification_set()
        .with_evidence(
            evidence("qual.insurance", SupplierQualificationArea::Insurance)
                .with_evidence(reference("insurance/alpha", "second")),
        )
        .qualification_for(&org("supplier-alpha"), day(20), day(30))
        .unwrap();

    assert_eq!(report.status, QualificationStatus::Conflicted);
}

#[test]
fn ready_to_award_is_not_ready_to_produce() {
    let procurement = ProcurementControlSet::new()
        .with_tender(tender("tender.alpha", "supplier-alpha", "95000.00"))
        .readiness_for(&work_package(), &currency("SEK"), day(20))
        .unwrap();
    let supplier = qualification_set()
        .qualification_for(&org("supplier-alpha"), day(20), day(30))
        .unwrap();
    let design = design_set()
        .readiness_for(package(), DesignReleasePurpose::Production, day(20))
        .unwrap();
    let report = handoff_set()
        .readiness_for(
            &control("handoff.frame"),
            &procurement,
            &supplier,
            &design,
            day(20),
        )
        .unwrap();

    assert!(!report.ready);
    assert!(report.blockers.iter().any(|blocker| {
        blocker.rule == "award" && blocker.reason == "ready to award is not ready to produce"
    }));
}

#[test]
fn supplier_evidence_requires_supplier_read_capability() {
    let evidence = evidence("qual.insurance", SupplierQualificationArea::Insurance)
        .with_evidence(reference("insurance/alpha", "accepted"));

    assert!(matches!(
        evidence.evidence(&[]),
        Err(ConstructionProjectError::MissingCapability { capability })
            if capability == CONSTRUCTION_SUPPLIER_READ_CAPABILITY
    ));
    assert_eq!(
        evidence
            .evidence(&[CONSTRUCTION_SUPPLIER_READ_CAPABILITY.to_owned()])
            .unwrap()
            .len(),
        1
    );
}

fn qualification_set() -> SupplierQualificationSet {
    base_qualification_without_intro().with_evidence(
        evidence(
            "qual.workplace-introduction",
            SupplierQualificationArea::WorkplaceIntroduction,
        )
        .with_evidence(reference("intro/alpha", "accepted")),
    )
}

fn base_qualification_without_intro() -> SupplierQualificationSet {
    [
        (
            "qual.economic-standing",
            SupplierQualificationArea::EconomicStanding,
        ),
        (
            "qual.responsible-business",
            SupplierQualificationArea::ResponsibleBusinessHumanRights,
        ),
        (
            "qual.collective-arrangement",
            SupplierQualificationArea::CollectiveArrangements,
        ),
        (
            "qual.license",
            SupplierQualificationArea::CompetenceLicenses,
        ),
        ("qual.insurance", SupplierQualificationArea::Insurance),
        (
            "qual.safety-training",
            SupplierQualificationArea::SafetyTraining,
        ),
        (
            "qual.workplace-introduction",
            SupplierQualificationArea::WorkplaceIntroduction,
        ),
        (
            "qual.risk-assessment",
            SupplierQualificationArea::RiskAssessment,
        ),
        (
            "qual.work-preparation",
            SupplierQualificationArea::WorkPreparation,
        ),
        ("qual.equipment", SupplierQualificationArea::Equipment),
        ("qual.materials", SupplierQualificationArea::Materials),
        (
            "qual.quality-environment",
            SupplierQualificationArea::QualityEnvironment,
        ),
        ("qual.staffing", SupplierQualificationArea::Staffing),
        ("qual.logistics", SupplierQualificationArea::Logistics),
        (
            "qual.meeting-participation",
            SupplierQualificationArea::MeetingParticipation,
        ),
    ]
    .into_iter()
    .filter(|(id, _)| *id != "qual.workplace-introduction")
    .fold(
        SupplierQualificationSet::new()
            .with_supplier(
                SupplierReference::new(
                    project(),
                    org("supplier-alpha"),
                    role("installer"),
                    role("qs"),
                )
                .with_validity(EvidenceValidity::new(None, Some(day(31))))
                .with_evidence(reference("supplier/alpha", "project-ref")),
            )
            .with_requirement(requirement(
                "qual.workplace-introduction",
                SupplierQualificationArea::WorkplaceIntroduction,
            )),
        |set, (id, area)| {
            set.with_requirement(requirement(id, area))
                .with_evidence(evidence(id, area).with_evidence(reference(id, "accepted")))
        },
    )
}

fn requirement(id: &str, area: SupplierQualificationArea) -> QualificationRequirement {
    QualificationRequirement::new(
        area,
        crate::ProjectObligation::mandatory(
            project(),
            Requirement::new(
                control(id),
                RequirementLane::new(Symbol::qualified("construction-supplier", id)),
                format!("{area:?}"),
                role("package-owner"),
                role("qs"),
            )
            .with_source_ref(reference(id, "requirement"))
            .with_evidence_kind(Symbol::qualified("construction-evidence", "external-ref")),
        ),
    )
}

fn evidence(id: &str, _area: SupplierQualificationArea) -> QualificationEvidence {
    QualificationEvidence::new(
        org("supplier-alpha"),
        control(id),
        EvidenceState::Accepted,
        role("qs"),
        day(10),
        format!("{id} accepted"),
    )
    .with_validity(EvidenceValidity::new(None, Some(day(31))))
}

fn handoff_set() -> PackageHandoffControlSet {
    PackageHandoffControlSet::new().with_handoff(
        handoff()
            .accepts_responsibility(day(20), "supplier accepts production responsibility")
            .with_evidence(reference("handoff/frame", "accepted")),
    )
}

fn handoff() -> PackageHandoff {
    PackageHandoff::new(
        control("handoff.frame"),
        package(),
        control("award.frame"),
        org("supplier-alpha"),
        control("release.prod"),
        control("material.frame"),
        5,
        day(30),
        role("site-manager"),
    )
    .accepts_responsibility(day(20), "supplier accepts production responsibility")
}

fn awarded_procurement() -> ProcurementControlSet {
    ProcurementControlSet::new()
        .with_tender(tender("tender.alpha", "supplier-alpha", "95000.00"))
        .with_award(
            AwardDecision::new(
                control("award.frame"),
                package(),
                AwardDecisionKind::Award,
                role("project-chief"),
                day(18),
                "qualified tender accepted",
            )
            .selects(control("tender.alpha"))
            .with_evidence(reference("award/frame", "accepted")),
        )
}

fn tender(id: &str, supplier: &str, value: &str) -> TenderComparison {
    TenderComparison::new(control(id), package(), supplier, amount(value, "SEK"))
        .with_lead_time_days(5)
        .with_capacity("reserved")
        .with_scope_compliance(ScopeCompliance::Compliant)
        .with_qualification(TenderQualification::Qualified)
        .with_evidence(reference(id, "evaluation"))
}

fn work_package() -> WorkPackage {
    WorkPackage::new(
        project(),
        package(),
        "Frame",
        role("procurement"),
        role("project-chief"),
        day(5),
        day(15),
        day(30),
        amount("100000.00", "SEK"),
    )
    .includes("frame supply")
    .requires_design_input(control("design.frame"))
    .with_supplier(
        SupplierCandidate::new("supplier-alpha", "qualified")
            .with_evidence(reference("supplier/alpha", "candidate")),
    )
    .with_supplier(
        SupplierCandidate::new("supplier-beta", "alternate")
            .with_evidence(reference("supplier/beta", "candidate")),
    )
    .with_evidence(reference("package/frame", "basis"))
}

fn design_set() -> DesignControlSet {
    DesignControlSet::new()
        .with_revision(
            DesignRevision::new(
                project(),
                control("design.frame"),
                "A",
                role("designer"),
                day(10),
            )
            .with_evidence_state(EvidenceState::Accepted)
            .affects(package())
            .with_external_ref(reference("design/frame", "A")),
        )
        .with_release(
            DesignRelease::new(
                project(),
                control("release.prod"),
                control("design.frame"),
                "A",
                DesignReleasePurpose::Production,
                role("designer"),
                role("project-chief"),
                role("project-chief"),
                day(16),
            )
            .with_evidence_state(EvidenceState::Accepted)
            .affects(package())
            .with_external_ref(reference("release/prod", "accepted")),
        )
}

fn amount(value: &str, currency_code: &str) -> CommercialAmount {
    CommercialAmount::parse(value, currency(currency_code)).unwrap()
}

fn currency(value: &str) -> CurrencyCode {
    CurrencyCode::new(value).unwrap()
}

fn project() -> crate::ProjectId {
    crate::ProjectId::new("reference-center").unwrap()
}

fn package() -> ControlId {
    control("package.frame")
}

fn control(value: &str) -> ControlId {
    ControlId::new(value).unwrap()
}

fn org(value: &str) -> OrganizationId {
    OrganizationId::new(value).unwrap()
}

fn role(value: &str) -> crate::RoleId {
    crate::RoleId::new(value).unwrap()
}

fn day(day: u8) -> Date {
    Date::from_calendar_date(2026, Month::July, day).unwrap()
}

fn reference(id: &str, version: &str) -> ExternalRef {
    ExternalRef::new("doc/synthetic", id, Some(version.to_owned()), None)
}
