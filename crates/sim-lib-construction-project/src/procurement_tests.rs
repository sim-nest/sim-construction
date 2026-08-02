// conformance: work-package procurement comparison and award control

use crate::{
    AwardDecision, AwardDecisionKind, CommercialAmount, ConstructionProjectError, ControlId,
    CurrencyCode, PackageReadinessReport, ProcurementControlSet, ProcurementStatus,
    ScopeCompliance, SupplierCandidate, TenderComparison, TenderQualification, WorkPackage,
};
use sim_lib_doc_core::ExternalRef;
use time::{Date, Month};

#[test]
fn stable_package_follows_scope_tenders_award_and_need_date() {
    let report = ready_procurement()
        .with_award(
            AwardDecision::new(
                control("award.frame"),
                package(),
                AwardDecisionKind::Award,
                role("project-chief"),
                date(2026, Month::July, 18),
                "best compliant amount and qualified capacity",
            )
            .selects(control("tender.alpha.corrected"))
            .with_evidence(reference("award/frame", "approved")),
        )
        .readiness_for(
            &work_package(),
            &currency("SEK"),
            date(2026, Month::July, 19),
        )
        .unwrap();

    assert!(matches!(
        report.status,
        ProcurementStatus::Awarded { ref supplier, .. } if supplier == "supplier-alpha"
    ));
    assert_eq!(report.comparison.corrected, vec![control("tender.alpha")]);
    assert_eq!(report.comparison.comparable[0].supplier, "supplier-alpha");
    assert_eq!(
        report.comparison.comparable[0]
            .variance_to_target
            .to_decimal_string(),
        "-25000.00"
    );
    assert!(report.interfaces.iter().all(|interface| !interface.exposed));
    assert!(!report.dates.need_date_exposed);
}

#[test]
fn incomplete_inquiry_basis_blocks_readiness() {
    let package = WorkPackage::new(
        project(),
        package(),
        "Frame",
        role("procurement"),
        role("project-chief"),
        date(2026, Month::July, 10),
        date(2026, Month::July, 20),
        date(2026, Month::July, 25),
        amount("100000.00", "SEK"),
    )
    .with_supplier(supplier("supplier-alpha"));

    assert!(matches!(
        package.validate(&currency("SEK")),
        Err(ConstructionProjectError::EmptyCollection(
            "work_package.scope_inclusions"
        ))
    ));
}

#[test]
fn non_comparable_tenders_are_preserved_without_award_authority() {
    let report = ProcurementControlSet::new()
        .with_tender(
            tender("tender.alpha", "supplier-alpha", "90000.00")
                .with_reservation("excludes fire seal")
                .with_scope_compliance(ScopeCompliance::Reserved),
        )
        .readiness_for(
            &work_package(),
            &currency("SEK"),
            date(2026, Month::July, 12),
        )
        .unwrap();

    assert!(matches!(report.status, ProcurementStatus::InquiryReady));
    assert_eq!(
        report.comparison.non_comparable,
        vec![control("tender.alpha")]
    );
}

#[test]
fn mixed_currency_is_rejected_before_comparison() {
    let result = ProcurementControlSet::new()
        .with_tender(tender_currency(
            "tender.alpha",
            "supplier-alpha",
            "90000.00",
            "EUR",
        ))
        .readiness_for(
            &work_package(),
            &currency("SEK"),
            date(2026, Month::July, 12),
        );

    assert!(matches!(
        result,
        Err(ConstructionProjectError::CurrencyMismatch {
            field: "tender.commercial_amount",
            ..
        })
    ));
}

#[test]
fn unauthorized_award_is_rejected() {
    let result = ready_procurement()
        .with_award(
            AwardDecision::new(
                control("award.frame"),
                package(),
                AwardDecisionKind::Award,
                role("procurement"),
                date(2026, Month::July, 18),
                "wrong authority",
            )
            .selects(control("tender.alpha.corrected"))
            .with_evidence(reference("award/frame", "approved")),
        )
        .readiness_for(
            &work_package(),
            &currency("SEK"),
            date(2026, Month::July, 19),
        );

    assert!(matches!(
        result,
        Err(ConstructionProjectError::AwardAuthorityMismatch { .. })
    ));
}

#[test]
fn award_after_need_date_is_rejected() {
    let result = ready_procurement()
        .with_award(
            AwardDecision::new(
                control("award.frame"),
                package(),
                AwardDecisionKind::Award,
                role("project-chief"),
                date(2026, Month::July, 26),
                "late award",
            )
            .selects(control("tender.alpha.corrected"))
            .with_evidence(reference("award/frame", "approved")),
        )
        .readiness_for(
            &work_package(),
            &currency("SEK"),
            date(2026, Month::July, 26),
        );

    assert!(matches!(
        result,
        Err(ConstructionProjectError::AwardAfterNeedDate { .. })
    ));
}

#[test]
fn rejected_supplier_cannot_be_awarded() {
    let package = work_package().with_supplier(supplier("supplier-rejected").rejected("failed QA"));
    let result = ProcurementControlSet::new()
        .with_tender(tender("tender.rejected", "supplier-rejected", "88000.00"))
        .with_award(
            AwardDecision::new(
                control("award.frame"),
                package.control.clone(),
                AwardDecisionKind::Award,
                role("project-chief"),
                date(2026, Month::July, 18),
                "not allowed",
            )
            .selects(control("tender.rejected"))
            .with_evidence(reference("award/frame", "approved")),
        )
        .readiness_for(&package, &currency("SEK"), date(2026, Month::July, 19));

    assert!(matches!(
        result,
        Err(ConstructionProjectError::RejectedSupplierAward { .. })
    ));
}

#[test]
fn corrected_tender_evidence_replaces_current_comparison_without_losing_prior_fact() {
    let comparison = ready_procurement()
        .compare(&work_package(), &currency("SEK"))
        .unwrap();

    assert_eq!(comparison.corrected, vec![control("tender.alpha")]);
    assert_eq!(
        comparison
            .comparable
            .iter()
            .map(|item| item.tender.as_str())
            .collect::<Vec<_>>(),
        vec!["tender.alpha.corrected", "tender.beta"]
    );
}

#[test]
fn award_ready_report_names_overdue_decisions_and_exposed_interfaces() {
    let report: PackageReadinessReport = ready_procurement()
        .readiness_for(
            &work_package(),
            &currency("SEK"),
            date(2026, Month::July, 24),
        )
        .unwrap();

    assert!(matches!(report.status, ProcurementStatus::AwardReady));
    assert!(report.dates.award_overdue);
    assert!(!report.dates.need_date_exposed);
    assert!(report.interfaces.iter().all(|interface| interface.exposed));
}

fn ready_procurement() -> ProcurementControlSet {
    ProcurementControlSet::new()
        .with_tender(tender("tender.alpha", "supplier-alpha", "100000.00"))
        .with_tender(
            tender("tender.alpha.corrected", "supplier-alpha", "95000.00")
                .supersedes(control("tender.alpha")),
        )
        .with_tender(tender("tender.beta", "supplier-beta", "105000.00"))
}

fn work_package() -> WorkPackage {
    WorkPackage::new(
        project(),
        package(),
        "Frame work package",
        role("procurement"),
        role("project-chief"),
        date(2026, Month::July, 10),
        date(2026, Month::July, 20),
        date(2026, Month::July, 25),
        amount("120000.00", "SEK"),
    )
    .includes("glulam frame supply")
    .includes("site installation")
    .excludes("foundation anchors")
    .requires_design_input(control("design.frame"))
    .exposes_interface(control("interface.foundation"))
    .exposes_interface(control("interface.facade"))
    .with_supplier(supplier("supplier-alpha"))
    .with_supplier(supplier("supplier-beta"))
    .with_evidence(reference("inquiry/frame", "basis-a"))
}

fn tender(id: &str, supplier: &str, value: &str) -> TenderComparison {
    tender_currency(id, supplier, value, "SEK")
}

fn tender_currency(id: &str, supplier: &str, value: &str, currency_code: &str) -> TenderComparison {
    TenderComparison::new(
        control(id),
        package(),
        supplier,
        amount(value, currency_code),
    )
    .with_lead_time_days(21)
    .with_capacity("capacity reserved for need date")
    .with_qualification(TenderQualification::Qualified)
    .with_evidence(reference(id, "evaluation"))
}

fn supplier(id: &str) -> SupplierCandidate {
    SupplierCandidate::new(id, "prequalified").with_evidence(reference(id, "candidate"))
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

fn role(value: &str) -> crate::RoleId {
    crate::RoleId::new(value).unwrap()
}

fn date(year: i32, month: Month, day: u8) -> Date {
    Date::from_calendar_date(year, month, day).unwrap()
}

fn reference(id: &str, version: &str) -> ExternalRef {
    ExternalRef::new("doc/synthetic", id, Some(version.to_owned()), None)
}
