// conformance: construction project charter readiness

use crate::{EvidenceState, ProjectCharter, ProjectId, RECIPES, RoleId};
use sim_lib_doc_core::ExternalRef;

#[test]
fn accepted_charter_is_ready_with_reference_only_evidence() {
    let charter = ProjectCharter::new(ProjectId::new("reference-center"), "Reference center")
        .with_customer_intent("Deliver a synthetic tenant improvement with safe handover")
        .with_delivery_model("collaboration")
        .with_currency("SEK")
        .accepted_by(RoleId::new("project-chief"), "2026-07-23")
        .with_evidence(ExternalRef::new(
            "doc/synthetic",
            "charter/reference-center",
            Some("rev-a".to_owned()),
            None,
        ));

    let readiness = charter.readiness(7);

    assert!(readiness.is_ready());
    assert_eq!(readiness.state, EvidenceState::Accepted);
    assert_eq!(readiness.evidence_refs, 1);
    assert!(readiness.missing_fields.is_empty());
}

#[test]
fn incomplete_charter_names_missing_fields() {
    let charter = ProjectCharter::new(ProjectId::new("reference-center"), "");

    let readiness = charter.readiness(3);

    assert!(!readiness.is_ready());
    assert_eq!(readiness.state, EvidenceState::Reported);
    assert!(readiness.missing_fields.contains(&"name".to_owned()));
    assert!(
        readiness
            .missing_fields
            .contains(&"customer_intent".to_owned())
    );
    assert!(readiness.missing_fields.contains(&"evidence".to_owned()));
}

#[test]
fn charter_json_round_trips() {
    let charter = ProjectCharter::new(ProjectId::new("reference-center"), "Reference center")
        .with_customer_intent("Deliver a modeled project")
        .with_delivery_model("collaboration")
        .with_currency("SEK")
        .accepted_by(RoleId::new("project-chief"), "2026-07-23")
        .with_evidence(ExternalRef::new(
            "doc/synthetic",
            "charter/reference-center",
            Some("rev-a".to_owned()),
            Some("https://example.invalid/reference-center".to_owned()),
        ));

    let encoded = serde_json::to_string(&charter).unwrap();
    let decoded: ProjectCharter = serde_json::from_str(&encoded).unwrap();

    assert_eq!(decoded, charter);
}

#[test]
fn recipes_export_project_charter() {
    let cards = sim_cookbook::recipes_from_embedded(RECIPES).unwrap();
    assert!(
        cards
            .iter()
            .any(|card| card.id.ends_with("project-charter"))
    );
}
