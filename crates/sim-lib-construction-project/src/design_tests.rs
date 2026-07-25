// conformance: design release, RFI, permit, and authority readiness control

use crate::{
    AuthorityObligation, AuthorityObligationState, ConstructionProjectError, ControlId,
    DesignControlSet, DesignRelease, DesignReleasePurpose, DesignReview, DesignReviewState,
    DesignRevision, EvidenceState, EvidenceValidity, PermitRecord, PermitState, ProjectId,
    RfiRecord, RfiState, RoleId,
};
use sim_lib_doc_core::ExternalRef;
use time::{Date, Month};

#[test]
fn accepted_production_release_makes_package_design_ready() {
    let report = ready_set()
        .readiness_for(package(), DesignReleasePurpose::Production, today())
        .unwrap();

    assert!(report.ready);
    assert_eq!(report.current_revisions, vec![revision_a()]);
    assert_eq!(report.releases, vec![release("release.prod")]);
    assert!(report.blockers.is_empty());
}

#[test]
fn conflicting_current_revisions_are_rejected() {
    let result = DesignControlSet::new()
        .with_revision(accepted_revision("design.a", "A"))
        .with_revision(accepted_revision("design.b", "B"))
        .with_release(accepted_release("release.prod", "design.a"))
        .readiness_for(package(), DesignReleasePurpose::Production, today());

    assert!(matches!(
        result,
        Err(ConstructionProjectError::ConflictingDesignRevisions { .. })
    ));
}

#[test]
fn answered_but_unaccepted_rfi_blocks_readiness_through_graph_path() {
    let report = ready_set()
        .with_rfi(
            RfiRecord::new(
                project(),
                control("rfi.fire-rating"),
                role("designer"),
                today(),
            )
            .with_state(RfiState::Answered)
            .with_evidence_state(EvidenceState::Evidenced)
            .affects(package())
            .with_external_ref(reference("rfi/fire-rating", "answered")),
        )
        .readiness_for(package(), DesignReleasePurpose::Production, today())
        .unwrap();

    assert!(!report.ready);
    assert_eq!(report.blockers[0].control.as_str(), "rfi.fire-rating");
    assert_eq!(
        report.blockers[0].reason,
        "RFI is answered but not accepted"
    );
    assert_eq!(
        report.blockers[0].paths[0]
            .steps
            .iter()
            .map(|step| step.control.as_str())
            .collect::<Vec<_>>(),
        vec!["rfi.fire-rating", "package.frame"]
    );
}

#[test]
fn release_for_wrong_purpose_blocks_production_readiness() {
    let report = DesignControlSet::new()
        .with_revision(accepted_revision("design.a", "A"))
        .with_release(
            DesignRelease::new(
                project(),
                release("release.procurement"),
                revision_a(),
                "A",
                DesignReleasePurpose::Procurement,
                role("designer"),
                role("project-chief"),
                role("project-chief"),
                today(),
            )
            .with_evidence_state(EvidenceState::Accepted)
            .affects(package())
            .with_external_ref(reference("release/procurement", "accepted")),
        )
        .readiness_for(package(), DesignReleasePurpose::Production, today())
        .unwrap();

    assert!(!report.ready);
    assert_eq!(report.blockers[0].rule, "release-purpose");
}

#[test]
fn expired_permit_blocks_authority_readiness() {
    let report = ready_set()
        .with_permit(
            PermitRecord::new(
                project(),
                control("permit.fire"),
                role("authority-lead"),
                today(),
            )
            .with_state(PermitState::Granted)
            .with_evidence_state(EvidenceState::Accepted)
            .with_validity(EvidenceValidity::new(
                None,
                Some(Date::from_calendar_date(2026, Month::July, 22).unwrap()),
            ))
            .affects(package())
            .with_external_ref(reference("permit/fire", "granted")),
        )
        .readiness_for(package(), DesignReleasePurpose::Production, today())
        .unwrap();

    assert!(!report.ready);
    assert_eq!(report.blockers[0].evidence_state, EvidenceState::Expired);
    assert_eq!(report.blockers[0].reason, "permit is expired");
}

#[test]
fn authority_hold_blocks_readiness() {
    let report = ready_set()
        .with_permit(
            PermitRecord::new(
                project(),
                control("permit.traffic"),
                role("authority-lead"),
                today(),
            )
            .with_state(PermitState::Hold)
            .with_evidence_state(EvidenceState::Reported)
            .affects(package())
            .with_external_ref(reference("permit/traffic", "hold")),
        )
        .readiness_for(package(), DesignReleasePurpose::Production, today())
        .unwrap();

    assert!(!report.ready);
    assert_eq!(report.blockers[0].reason, "authority permit is on hold");
}

#[test]
fn superseded_drawing_makes_dependent_release_stale_until_revalidated() {
    let stale = DesignControlSet::new()
        .with_revision(accepted_revision("design.a", "A"))
        .with_revision(accepted_revision("design.b", "B").supersedes(revision_a()))
        .with_release(accepted_release("release.prod", "design.a"))
        .readiness_for(package(), DesignReleasePurpose::Production, today());

    assert!(matches!(
        stale,
        Err(ConstructionProjectError::StaleDesignRelease { .. })
    ));

    let revalidated = DesignControlSet::new()
        .with_revision(accepted_revision("design.a", "A"))
        .with_revision(accepted_revision("design.b", "B").supersedes(revision_a()))
        .with_release(
            accepted_release("release.prod", "design.a").revalidated_against(revision_b()),
        )
        .readiness_for(package(), DesignReleasePurpose::Production, today())
        .unwrap();

    assert!(revalidated.ready);
}

#[test]
fn non_waivable_production_blocker_is_rejected() {
    let result = ready_set()
        .with_authority_obligation(
            AuthorityObligation::new(
                project(),
                control("authority.stop"),
                role("authority-lead"),
                today(),
            )
            .with_state(AuthorityObligationState::Hold)
            .with_evidence_state(EvidenceState::Reported)
            .non_waivable()
            .affects(package())
            .with_external_ref(reference("authority/stop", "hold")),
        )
        .readiness_for(package(), DesignReleasePurpose::Production, today());

    assert!(matches!(
        result,
        Err(ConstructionProjectError::NonWaivableProductionBlocker { .. })
    ));
}

#[test]
fn accepted_review_decision_is_part_of_design_control() {
    let report = ready_set()
        .with_review(
            DesignReview::new(
                project(),
                control("review.design-a"),
                revision_a(),
                role("reviewer"),
                today(),
            )
            .with_state(DesignReviewState::Accepted)
            .with_evidence_state(EvidenceState::Accepted)
            .affects(package())
            .with_external_ref(reference("review/design-a", "accepted")),
        )
        .readiness_for(package(), DesignReleasePurpose::Production, today())
        .unwrap();

    assert!(report.ready);
}

fn ready_set() -> DesignControlSet {
    DesignControlSet::new()
        .with_revision(accepted_revision("design.a", "A"))
        .with_release(accepted_release("release.prod", "design.a"))
}

fn accepted_revision(control_id: &str, revision: &str) -> DesignRevision {
    DesignRevision::new(
        project(),
        control(control_id),
        revision,
        role("designer"),
        today(),
    )
    .with_evidence_state(EvidenceState::Accepted)
    .affects(package())
    .with_external_ref(reference(control_id, revision))
}

fn accepted_release(control_id: &str, revision_id: &str) -> DesignRelease {
    DesignRelease::new(
        project(),
        release(control_id),
        control(revision_id),
        "A",
        DesignReleasePurpose::Production,
        role("designer"),
        role("project-chief"),
        role("project-chief"),
        today(),
    )
    .with_evidence_state(EvidenceState::Accepted)
    .affects(package())
    .with_external_ref(reference(control_id, "accepted"))
}

fn project() -> ProjectId {
    ProjectId::new("reference-center").unwrap()
}

fn package() -> ControlId {
    control("package.frame")
}

fn revision_a() -> ControlId {
    control("design.a")
}

fn revision_b() -> ControlId {
    control("design.b")
}

fn release(id: &str) -> ControlId {
    control(id)
}

fn control(id: &str) -> ControlId {
    ControlId::new(id).unwrap()
}

fn role(id: &str) -> RoleId {
    RoleId::new(id).unwrap()
}

fn today() -> Date {
    Date::from_calendar_date(2026, Month::July, 23).unwrap()
}

fn reference(id: &str, version: &str) -> ExternalRef {
    ExternalRef::new("doc/synthetic", id, Some(version.to_owned()), None)
}
