// conformance: construction field-control facts

use sim_lib_doc_core::ExternalRef;
use time::{Date, Month};

use crate::{
    CorrectiveAction, Defect, EFFECT_LEDGER_BACKEND, EvidenceState, FieldItem, FieldItemImport,
    FieldItemImportOutcome, FieldItemKind, FieldItemReference, FieldItemState, FieldLane,
    FieldSeverity, IncidentEscalation, InspectionPoint, InspectionResult, ProjectBook,
    ProjectIncident, ProjectObservation, QualityDeviation, import_field_item, safety_first_rollup,
};

#[test]
fn field_control_types_share_accountability_without_payload_copying() {
    let observation = ProjectObservation::new(
        item(
            "observation.deck",
            FieldItemKind::Observation,
            FieldLane::Progress,
        ),
        day(10),
        "Deck edge protection incomplete",
    );
    observation.validate().unwrap();

    let deviation = QualityDeviation {
        field_item: item(
            "deviation.frame",
            FieldItemKind::Deviation,
            FieldLane::Quality,
        ),
        requirement: control("requirement.frame-tolerance"),
        description: "Measured frame exceeds accepted tolerance".to_owned(),
    };
    deviation.validate().unwrap();

    let inspection = InspectionPoint::new(item(
        "inspection.frame",
        FieldItemKind::InspectionPoint,
        FieldLane::Quality,
    ));
    assert!(inspection.blocks_production());
    inspection.validate().unwrap();

    let defect = Defect {
        field_item: item("defect.door", FieldItemKind::Defect, FieldLane::Quality),
        detected_on: day(10),
    };
    defect.validate().unwrap();

    let reference =
        FieldItemReference::new("site/dalux", "items/item-1", Some("rev-2".to_owned())).unwrap();
    let external = reference.as_external_ref();
    assert_eq!(external.backend, "site/dalux");
    assert_eq!(external.external_id, "items/item-1");
    assert_eq!(external.web_url, None);
}

#[test]
fn incident_and_corrective_closure_require_accountable_evidence() {
    let incident = ProjectIncident::new(
        item(
            "incident.access",
            FieldItemKind::Incident,
            FieldLane::Safety,
        )
        .non_waivable(),
        day(10),
    )
    .with_escalation(IncidentEscalation {
        escalated_to: role("project-chief"),
        escalated_on: day(10),
        reason: "Stop affected access route".to_owned(),
        evidence: vec![evidence("incident/access/escalation")],
    });
    assert!(incident.requires_escalation());
    incident.validate().unwrap();

    let corrective = CorrectiveAction {
        field_item: item(
            "corrective.access",
            FieldItemKind::CorrectiveAction,
            FieldLane::Safety,
        )
        .with_state(FieldItemState::Closed)
        .with_evidence_state(EvidenceState::Accepted)
        .with_evidence(evidence("corrective/access/accepted")),
        corrects: vec![control("incident.access")],
        accepted_by: Some(role("safety-lead")),
    };
    assert!(corrective.has_accepted_evidence());
    corrective.validate().unwrap();
}

#[test]
fn critical_incident_fails_closed_until_escalated() {
    let incident = ProjectIncident::new(
        item("incident.fall", FieldItemKind::Incident, FieldLane::Safety)
            .with_severity(FieldSeverity::Critical),
        day(10),
    );
    assert!(incident.requires_escalation());
    assert!(incident.validate().is_err());

    let escalated = incident.with_escalation(IncidentEscalation {
        escalated_to: role("project-chief"),
        escalated_on: day(10),
        reason: "Stop work and secure the opening".to_owned(),
        evidence: vec![evidence("incident/fall/escalation")],
    });
    escalated.validate().unwrap();
}

#[test]
fn passed_inspection_needs_accepted_evidence() {
    let passed = InspectionPoint::new(
        item(
            "test.pressure",
            FieldItemKind::TestPoint,
            FieldLane::Quality,
        )
        .with_state(FieldItemState::Closed)
        .with_evidence_state(EvidenceState::Accepted)
        .with_evidence(evidence("test/pressure/accepted")),
    )
    .with_result(InspectionResult::Passed, role("quality-lead"));

    assert!(!passed.blocks_production());
    passed.validate().unwrap();
}

#[test]
fn rejected_inspection_remains_a_production_blocker() {
    let rejected = InspectionPoint::new(
        item(
            "inspection.fire-seal",
            FieldItemKind::InspectionPoint,
            FieldLane::Quality,
        )
        .with_state(FieldItemState::Rejected)
        .with_evidence_state(EvidenceState::Rejected)
        .with_evidence(evidence("inspection/fire-seal/rejected")),
    )
    .with_result(InspectionResult::Rejected, role("quality-lead"));

    assert!(rejected.blocks_production());
    rejected.validate().unwrap();
}

#[test]
fn open_defect_becomes_overdue_but_closed_defect_does_not() {
    let defect = Defect {
        field_item: item("defect.door", FieldItemKind::Defect, FieldLane::Quality)
            .with_state(FieldItemState::Open),
        detected_on: day(10),
    };
    assert!(defect.is_overdue(day(20)));

    let closed = Defect {
        field_item: defect
            .field_item
            .with_state(FieldItemState::Closed)
            .with_evidence_state(EvidenceState::Accepted)
            .with_evidence(evidence("defect/door/accepted")),
        detected_on: defect.detected_on,
    };
    assert!(!closed.is_overdue(day(20)));
    closed.validate().unwrap();
}

#[test]
fn corrective_action_needs_evidence_and_accepting_role() {
    let field_item = item(
        "corrective.fire-seal",
        FieldItemKind::CorrectiveAction,
        FieldLane::Quality,
    )
    .with_state(FieldItemState::Closed)
    .with_evidence_state(EvidenceState::Accepted)
    .with_evidence(evidence("corrective/fire-seal/photo"));
    let missing_authority = CorrectiveAction {
        field_item: field_item.clone(),
        corrects: vec![control("inspection.fire-seal")],
        accepted_by: None,
    };
    assert!(!missing_authority.has_accepted_evidence());
    assert!(missing_authority.validate().is_err());

    let accepted = CorrectiveAction {
        field_item,
        corrects: vec![control("inspection.fire-seal")],
        accepted_by: Some(role("quality-lead")),
    };
    assert!(accepted.has_accepted_evidence());
    accepted.validate().unwrap();
}

#[test]
fn rollup_puts_imminent_safety_and_non_waivable_controls_first() {
    let items = vec![
        item(
            "progress.critical",
            FieldItemKind::Observation,
            FieldLane::Progress,
        )
        .with_state(FieldItemState::Blocked)
        .with_severity(FieldSeverity::Critical),
        item(
            "convenience.non-waivable",
            FieldItemKind::Deviation,
            FieldLane::Convenience,
        )
        .with_state(FieldItemState::Open)
        .with_severity(FieldSeverity::Information)
        .non_waivable(),
        item(
            "safety.imminent",
            FieldItemKind::Incident,
            FieldLane::Safety,
        )
        .with_state(FieldItemState::Blocked)
        .with_severity(FieldSeverity::Imminent),
        item(
            "safety.closed",
            FieldItemKind::CorrectiveAction,
            FieldLane::Safety,
        )
        .with_state(FieldItemState::Closed)
        .with_evidence_state(EvidenceState::Accepted)
        .with_evidence(evidence("safety/closed")),
        item(
            "environment.major",
            FieldItemKind::Incident,
            FieldLane::Environment,
        )
        .with_state(FieldItemState::Open),
    ];

    let controls = safety_first_rollup(&items, day(20))
        .into_iter()
        .map(|row| row.control.to_string())
        .collect::<Vec<_>>();

    assert_eq!(
        controls,
        vec![
            "safety.imminent",
            "convenience.non-waivable",
            "progress.critical",
            "environment.major",
            "safety.closed",
        ]
    );
}

#[test]
fn external_import_is_idempotent_and_changed_source_state_supersedes() {
    let mut book = ProjectBook::new(
        crate::ProjectId::new("reference-center").unwrap(),
        role("project-chief"),
    );
    let first = dalux_import("open", "2026-07-10T10:00:00Z");

    assert_eq!(
        import_field_item(&mut book, 1, day(10), &first).unwrap(),
        FieldItemImportOutcome::Appended {
            seq: 1,
            supersedes: None,
        }
    );
    assert_eq!(
        import_field_item(&mut book, 2, day(10), &first).unwrap(),
        FieldItemImportOutcome::Duplicate { existing_seq: 1 }
    );
    assert_eq!(book.len(), 1);

    let changed = dalux_import("closed", "2026-07-10T11:00:00Z");
    assert_eq!(
        import_field_item(&mut book, 2, day(10), &changed).unwrap(),
        FieldItemImportOutcome::Appended {
            seq: 2,
            supersedes: Some(1),
        }
    );

    let current = book.snapshot_at(2).unwrap();
    let fact = current
        .current_fact(&control("field.dalux-item-1"))
        .unwrap();
    assert_eq!(fact.evidence_state, EvidenceState::Reported);
    assert_eq!(fact.evidence.len(), 2);
    assert!(
        fact.evidence
            .iter()
            .all(|reference| reference.web_url.is_none())
    );
    let debug = format!("{fact:?}");
    for excluded in [
        "attachment",
        "bearer",
        "token",
        "web_url: Some",
        "Door review",
    ] {
        assert!(!debug.contains(excluded), "fact leaked {excluded:?}");
    }
}

fn item(id: &str, kind: FieldItemKind, lane: FieldLane) -> FieldItem {
    FieldItem::new(
        crate::ProjectId::new("reference-center").unwrap(),
        control(id),
        kind,
        FieldSeverity::Major,
        lane,
        role("site-manager"),
    )
    .due_on(day(12))
    .affects(control("package.frame"))
}

fn dalux_import(state: &str, version: &str) -> FieldItemImport {
    FieldItemImport::new(
        item(
            "field.dalux-item-1",
            FieldItemKind::ExternalReference,
            FieldLane::Quality,
        )
        .with_state(FieldItemState::Open),
        FieldItemReference::new("site/dalux", "items/item-1", Some(version.to_owned())).unwrap(),
        state,
        ExternalRef::new(
            EFFECT_LEDGER_BACKEND,
            format!("dalux/read/{version}"),
            None,
            None,
        ),
    )
    .unwrap()
}

fn control(id: &str) -> crate::ControlId {
    crate::ControlId::new(id).unwrap()
}

fn role(id: &str) -> crate::RoleId {
    crate::RoleId::new(id).unwrap()
}

fn evidence(id: &str) -> ExternalRef {
    ExternalRef::new("doc/synthetic", id, Some("accepted".to_owned()), None)
}

fn day(day: u8) -> Date {
    Date::from_calendar_date(2026, Month::July, day).unwrap()
}
