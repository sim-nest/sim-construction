// conformance: construction project charter readiness

use crate::{
    CONSTRUCTION_EXCEPTION_CAPABILITY, CONSTRUCTION_PROJECT_ACCEPT_CAPABILITY,
    CONSTRUCTION_PROJECT_READ_CAPABILITY, CONSTRUCTION_PROJECT_WRITE_CAPABILITY,
    CONSTRUCTION_REFERENCE_PUBLISH_CAPABILITY, CONSTRUCTION_SUPPLIER_READ_CAPABILITY,
    ConstructionProjectError, ControlId, CurrencyCode, DueDatePolicy, EvidenceState,
    OrganizationId, ProjectCharter, ProjectGovernance, ProjectId, RECIPES, ReportingCadence,
    RoleAssignment, RoleId, Visibility, VisibilityPolicy, construction_exception_capability,
    construction_project_accept_capability, construction_project_read_capability,
    construction_project_write_capability, construction_reference_publish_capability,
    construction_supplier_read_capability,
};
use sim_kernel::Symbol;
use sim_lib_doc_core::ExternalRef;
use time::{Date, Month};

#[test]
fn accepted_charter_is_ready_with_reference_only_evidence() {
    let charter = accepted_charter();

    let readiness = charter.readiness(7);

    charter.validate().unwrap();
    assert!(readiness.is_ready());
    assert_eq!(readiness.state, EvidenceState::Accepted);
    assert_eq!(readiness.evidence_refs, 1);
    assert!(readiness.missing_fields.is_empty());
}

#[test]
fn incomplete_charter_names_missing_fields() {
    let charter = ProjectCharter::new(
        ProjectId::new("reference-center").unwrap(),
        ControlId::new("control.reference-center").unwrap(),
        "",
        CurrencyCode::new("SEK").unwrap(),
    );

    let readiness = charter.readiness(3);

    assert!(!readiness.is_ready());
    assert_eq!(readiness.state, EvidenceState::Reported);
    assert!(readiness.missing_fields.contains(&"name".to_owned()));
    assert!(
        readiness
            .missing_fields
            .contains(&"customer_outcome".to_owned())
    );
    assert!(readiness.missing_fields.contains(&"source_refs".to_owned()));
}

#[test]
fn stable_ids_reject_ambiguous_values() {
    for value in [
        "",
        " reference",
        "reference center",
        "reference/center",
        "unknown",
    ] {
        assert!(ProjectId::new(value).is_err());
    }
    let oversized = "x".repeat(97);
    assert!(ProjectId::new(oversized).is_err());

    let control = ControlId::new("control.reference-center").unwrap();
    assert_eq!(control.as_str(), "control.reference-center");
}

#[test]
fn unknown_currency_is_rejected() {
    assert_eq!(
        CurrencyCode::new("ZZZ"),
        Err(ConstructionProjectError::UnknownCurrency("ZZZ".to_owned()))
    );
    assert_eq!(CurrencyCode::new("SEK").unwrap().as_str(), "SEK");
}

#[test]
fn empty_objectives_are_rejected() {
    let mut charter = accepted_charter();
    charter.objectives.clear();

    assert_eq!(
        charter.validate(),
        Err(ConstructionProjectError::EmptyObjectives)
    );
}

#[test]
fn governance_validates_roles_and_visibility() {
    let visibility = Symbol::new("commercial-confidential");
    let governance = ProjectGovernance::new(
        ProjectId::new("reference-center").unwrap(),
        DueDatePolicy::new(5, 2).unwrap(),
    )
    .with_visibility_policy(VisibilityPolicy::project().with_restricted(visibility.clone()))
    .with_role(
        RoleAssignment::new(
            RoleId::new("project-chief").unwrap(),
            OrganizationId::new("customer-org").unwrap(),
            Visibility::Restricted(visibility),
        )
        .with_responsibility("Accept charter baseline")
        .may_decide(Symbol::new("accept-charter")),
    )
    .with_role(
        RoleAssignment::new(
            RoleId::new("supplier-lead").unwrap(),
            OrganizationId::new("supplier-org").unwrap(),
            Visibility::Supplier,
        )
        .with_responsibility("Maintain supplier delivery commitments")
        .may_decide(Symbol::new("raise-exception"))
        .escalates_to(RoleId::new("project-chief").unwrap()),
    );

    governance.validate().unwrap();
}

#[test]
fn governance_rejects_duplicate_missing_and_cyclic_authority() {
    let duplicate = ProjectGovernance::new(
        ProjectId::new("reference-center").unwrap(),
        DueDatePolicy::new(5, 2).unwrap(),
    )
    .with_role(
        RoleAssignment::new(
            RoleId::new("project-chief").unwrap(),
            OrganizationId::new("customer-org").unwrap(),
            Visibility::Project,
        )
        .with_responsibility("Accept charter"),
    )
    .with_role(
        RoleAssignment::new(
            RoleId::new("project-chief").unwrap(),
            OrganizationId::new("supplier-org").unwrap(),
            Visibility::Supplier,
        )
        .with_responsibility("Maintain plan"),
    );

    assert!(matches!(
        duplicate.validate(),
        Err(ConstructionProjectError::DuplicateId { kind: "role", .. })
    ));

    let missing = ProjectGovernance::new(
        ProjectId::new("reference-center").unwrap(),
        DueDatePolicy::new(5, 2).unwrap(),
    )
    .with_role(
        RoleAssignment::new(
            RoleId::new("supplier-lead").unwrap(),
            OrganizationId::new("supplier-org").unwrap(),
            Visibility::Supplier,
        )
        .with_responsibility("Maintain plan")
        .escalates_to(RoleId::new("project-chief").unwrap()),
    );

    assert!(matches!(
        missing.validate(),
        Err(ConstructionProjectError::MissingEscalationTarget { .. })
    ));

    let cyclic = ProjectGovernance::new(
        ProjectId::new("reference-center").unwrap(),
        DueDatePolicy::new(5, 2).unwrap(),
    )
    .with_role(
        RoleAssignment::new(
            RoleId::new("project-chief").unwrap(),
            OrganizationId::new("customer-org").unwrap(),
            Visibility::Project,
        )
        .with_responsibility("Accept charter")
        .escalates_to(RoleId::new("supplier-lead").unwrap()),
    )
    .with_role(
        RoleAssignment::new(
            RoleId::new("supplier-lead").unwrap(),
            OrganizationId::new("supplier-org").unwrap(),
            Visibility::Supplier,
        )
        .with_responsibility("Maintain plan")
        .escalates_to(RoleId::new("project-chief").unwrap()),
    );

    assert!(matches!(
        cyclic.validate(),
        Err(ConstructionProjectError::AuthorityCycle(_))
    ));
}

#[test]
fn governance_rejects_unlisted_restricted_visibility() {
    let governance = ProjectGovernance::new(
        ProjectId::new("reference-center").unwrap(),
        DueDatePolicy::new(5, 2).unwrap(),
    )
    .with_role(
        RoleAssignment::new(
            RoleId::new("project-chief").unwrap(),
            OrganizationId::new("customer-org").unwrap(),
            Visibility::Restricted(Symbol::new("commercial-confidential")),
        )
        .with_responsibility("Accept charter"),
    );

    assert!(matches!(
        governance.validate(),
        Err(ConstructionProjectError::RestrictedVisibilityDenied { .. })
    ));
}

#[test]
fn charter_json_round_trips() {
    let charter = accepted_charter();

    let encoded = serde_json::to_string(&charter).unwrap();
    let decoded: ProjectCharter = serde_json::from_str(&encoded).unwrap();

    assert_eq!(decoded, charter);
}

#[test]
fn capability_constants_build_kernel_capability_names() {
    assert_eq!(
        construction_project_read_capability().as_str(),
        CONSTRUCTION_PROJECT_READ_CAPABILITY
    );
    assert_eq!(
        construction_project_write_capability().as_str(),
        CONSTRUCTION_PROJECT_WRITE_CAPABILITY
    );
    assert_eq!(
        construction_project_accept_capability().as_str(),
        CONSTRUCTION_PROJECT_ACCEPT_CAPABILITY
    );
    assert_eq!(
        construction_exception_capability().as_str(),
        CONSTRUCTION_EXCEPTION_CAPABILITY
    );
    assert_eq!(
        construction_reference_publish_capability().as_str(),
        CONSTRUCTION_REFERENCE_PUBLISH_CAPABILITY
    );
    assert_eq!(
        construction_supplier_read_capability().as_str(),
        CONSTRUCTION_SUPPLIER_READ_CAPABILITY
    );
}

#[test]
fn recipes_export_project_charter() {
    let cards = sim_cookbook::recipes_from_embedded(RECIPES).unwrap();
    assert!(
        cards
            .iter()
            .any(|card| card.id.ends_with("project-charter"))
    );
    assert!(cards.iter().any(|card| card.id.ends_with("what-changed")));
    assert!(
        cards
            .iter()
            .any(|card| card.id.ends_with("mobilization-gate"))
    );
    assert!(cards.iter().any(|card| card.id.ends_with("late-decision")));
    assert!(
        cards
            .iter()
            .any(|card| card.id.ends_with("mixed-obligation-gate"))
    );
    assert!(
        cards
            .iter()
            .any(|card| card.id.ends_with("customer-intent-to-collaboration"))
    );
    assert!(
        cards
            .iter()
            .any(|card| card.id.ends_with("supplier-to-production-handoff"))
    );
}

fn accepted_charter() -> ProjectCharter {
    ProjectCharter::new(
        ProjectId::new("reference-center").unwrap(),
        ControlId::new("control.reference-center").unwrap(),
        "Reference center",
        CurrencyCode::new("SEK").unwrap(),
    )
    .with_baseline(crate::BaselineId::new("baseline.reference-center").unwrap())
    .with_customer_outcome("Deliver a synthetic tenant improvement with safe handover")
    .with_property_constraint("Existing shell boundary is fixed")
    .with_product_constraint("Handover pack must be referenceable")
    .with_procurement_form("collaboration")
    .with_objective("Open the reference center safely")
    .with_non_negotiable("No personal profiles in the charter")
    .with_target_outcome("Accepted customer handover")
    .with_reference_criterion("Evidence can support a public reference candidate")
    .with_reporting_cadence(ReportingCadence::new("weekly", 7).unwrap())
    .accepted_by(RoleId::new("project-chief").unwrap(), accepted_on())
    .with_source_ref(ExternalRef::new(
        "doc/synthetic",
        "charter/reference-center",
        Some("rev-a".to_owned()),
        Some("https://example.invalid/reference-center".to_owned()),
    ))
}

fn accepted_on() -> Date {
    Date::from_calendar_date(2026, Month::July, 23).unwrap()
}
