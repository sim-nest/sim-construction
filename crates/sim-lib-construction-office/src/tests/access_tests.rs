use super::*;

#[test]
fn restricted_visibility_requires_the_exact_project_visibility_grant() {
    let mut fixture = Fixture::new("project-a");
    let relation = design_source_relation();
    let external = external("design/restricted");
    let restricted = Visibility::Restricted(Symbol::qualified("project", "confidential-design"));
    fixture.append(fact(
        1,
        fixture.project.clone(),
        "design.restricted",
        &relation,
        external.clone(),
        EvidenceState::Accepted,
        restricted.clone(),
    ));
    let document = fixture.save_document("office/restricted-register");
    let attachment = attachment(
        &fixture.project,
        "design.restricted",
        1,
        &document,
        &external,
        relation,
    );

    assert!(matches!(
        fixture.attach(&attachment).unwrap_err(),
        EvidenceBridgeError::VisibilityDenied { sequence: 1, .. }
    ));
    fixture.access = fixture.access.clone().allow_visibility(restricted);
    fixture.attach(&attachment).unwrap();
    assert_eq!(
        fixture
            .query("design.restricted", [document])
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn project_scopes_isolate_identical_sequences_controls_and_references() {
    let mut cx = authorized_context();
    let store = DocStore::create(Path::new(":memory:")).unwrap();
    let document = save_document(&mut cx, &store, "office/shared-register");
    let relation = schedule_basis_relation();
    let external = external("schedule/shared");
    let mut book_a = book("project-a");
    let mut book_b = book("project-b");
    book_a
        .append(fact(
            1,
            project("project-a"),
            "schedule.main",
            &relation,
            external.clone(),
            EvidenceState::Accepted,
            Visibility::Project,
        ))
        .unwrap();
    book_b
        .append(fact(
            1,
            project("project-b"),
            "schedule.main",
            &relation,
            external.clone(),
            EvidenceState::Accepted,
            Visibility::Project,
        ))
        .unwrap();
    let access_a = ProjectEvidenceAccess::project(project("project-a"));
    let access_b = ProjectEvidenceAccess::project(project("project-b"));

    attach_evidence(
        &cx,
        &store,
        &book_a,
        &access_a,
        &attachment(
            &project("project-a"),
            "schedule.main",
            1,
            &document,
            &external,
            relation.clone(),
        ),
    )
    .unwrap();
    attach_evidence(
        &cx,
        &store,
        &book_b,
        &access_b,
        &attachment(
            &project("project-b"),
            "schedule.main",
            1,
            &document,
            &external,
            relation,
        ),
    )
    .unwrap();

    let links_a = evidence_for_documents(
        &cx,
        &store,
        &book_a,
        &access_a,
        &control("schedule.main"),
        [document.clone()],
    )
    .unwrap();
    let links_b = evidence_for_documents(
        &cx,
        &store,
        &book_b,
        &access_b,
        &control("schedule.main"),
        [document],
    )
    .unwrap();
    assert_eq!(links_a.len(), 1);
    assert_eq!(links_b.len(), 1);
    assert_eq!(links_a[0].project, project("project-a"));
    assert_eq!(links_b[0].project, project("project-b"));
}

#[test]
fn construction_and_office_capabilities_are_independently_required() {
    let mut fixture = Fixture::new("project-a");
    let relation = published_deliverable_relation();
    let external = external("publication/closeout");
    fixture.append(fact(
        1,
        fixture.project.clone(),
        "publication.closeout",
        &relation,
        external.clone(),
        EvidenceState::Accepted,
        Visibility::Project,
    ));
    let document = fixture.save_document("office/closeout");
    let attachment = attachment(
        &fixture.project,
        "publication.closeout",
        1,
        &document,
        &external,
        relation,
    );
    let mut denied = context();

    assert_capability(
        attach_evidence(
            &denied,
            &fixture.store,
            &fixture.book,
            &fixture.access,
            &attachment,
        )
        .unwrap_err(),
        construction_project_read_capability(),
    );
    denied.grant(construction_project_read_capability());
    assert_capability(
        attach_evidence(
            &denied,
            &fixture.store,
            &fixture.book,
            &fixture.access,
            &attachment,
        )
        .unwrap_err(),
        office_evidence_read_capability(),
    );
    denied.grant(office_evidence_read_capability());
    assert_capability(
        attach_evidence(
            &denied,
            &fixture.store,
            &fixture.book,
            &fixture.access,
            &attachment,
        )
        .unwrap_err(),
        office_evidence_write_capability(),
    );
}
