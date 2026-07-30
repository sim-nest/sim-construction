use std::{path::Path, sync::Arc};

use sim_kernel::{CapabilityName, Cx, DefaultFactory, Error, Expr, NoopEvalPolicy, Symbol};
use sim_lib_construction_project::{
    ControlId, EvidenceState, ProjectBook, ProjectFact, ProjectId, RoleId, Visibility,
    construction_project_read_capability,
};
use sim_lib_doc_core::{Doc, DocId, DocKind, ExternalRef, LinkRole};
use sim_lib_doc_store::{DocStore, evidence};
use time::{Date, Month};

use crate::{
    AttachOutcome, EvidenceAttachment, EvidenceBridgeError, ProjectEvidenceAccess, attach_evidence,
    commercial_support_relation, design_source_relation, evidence_for_documents,
    field_issue_relation, office_evidence_read_capability, office_evidence_write_capability,
    published_deliverable_relation, schedule_basis_relation,
};

// conformance: construction fact evidence composes with office documents.

mod access_tests;

#[test]
fn precise_construction_relations_map_to_existing_office_roles() {
    let mappings = [
        (design_source_relation(), LinkRole::SourceDocument),
        (schedule_basis_relation(), LinkRole::ScheduleReference),
        (field_issue_relation(), LinkRole::ProjectIssue),
        (commercial_support_relation(), LinkRole::AccountingSupport),
        (published_deliverable_relation(), LinkRole::PublishedTo),
    ];

    for (relation, expected) in mappings {
        assert_eq!(relation.office_role(), expected);
        assert_eq!(
            relation.fact_kind().namespace.as_deref(),
            Some("construction-evidence")
        );
    }
}

#[test]
fn construction_office_evidence_recipe_is_embedded() {
    let recipes = sim_cookbook::recipes_from_embedded(crate::RECIPES).unwrap();

    assert!(
        recipes
            .iter()
            .any(|recipe| recipe.id.ends_with("construction-office-evidence"))
    );
}

#[test]
fn attach_is_idempotent_and_query_restores_the_precise_fact_relation() {
    let mut fixture = Fixture::new("project-a");
    let relation = design_source_relation();
    let external = external("design/revision-7");
    fixture.append(fact(
        1,
        fixture.project.clone(),
        "design.wall",
        &relation,
        external.clone(),
        EvidenceState::Accepted,
        Visibility::Project,
    ));
    let document = fixture.save_document("office/design-register");
    let attachment = attachment(
        &fixture.project,
        "design.wall",
        1,
        &document,
        &external,
        relation.clone(),
    );

    assert_eq!(
        fixture.attach(&attachment).unwrap(),
        AttachOutcome::Attached
    );
    assert_eq!(
        fixture.attach(&attachment).unwrap(),
        AttachOutcome::Unchanged
    );
    assert_eq!(
        evidence::evidence_for(&fixture.store, &document)
            .unwrap()
            .len(),
        1
    );

    let links = fixture.query("design.wall", [document]).unwrap();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].relation.fact_kind(), relation.fact_kind());
    assert_eq!(links[0].external, external);
    assert!(links[0].is_accepted());
}

#[test]
fn missing_document_fails_closed_before_an_office_row_is_written() {
    let mut fixture = Fixture::new("project-a");
    let relation = design_source_relation();
    let external = external("design/revision-7");
    fixture.append(fact(
        1,
        fixture.project.clone(),
        "design.wall",
        &relation,
        external.clone(),
        EvidenceState::Accepted,
        Visibility::Project,
    ));
    let missing = DocId::new("office/missing");
    let error = fixture
        .attach(&attachment(
            &fixture.project,
            "design.wall",
            1,
            &missing,
            &external,
            relation,
        ))
        .unwrap_err();

    assert!(matches!(
        error,
        EvidenceBridgeError::MissingDocument { document }
            if document == "office/missing"
    ));
    assert!(
        evidence::evidence_for(&fixture.store, &missing)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn superseded_fact_cannot_be_attached_or_resolved_as_current() {
    let mut fixture = Fixture::new("project-a");
    let relation = field_issue_relation();
    let old_external = external("issue/old");
    fixture.append(fact(
        1,
        fixture.project.clone(),
        "issue.door",
        &relation,
        old_external.clone(),
        EvidenceState::Evidenced,
        Visibility::Project,
    ));
    let document = fixture.save_document("office/issue-register");
    let old_attachment = attachment(
        &fixture.project,
        "issue.door",
        1,
        &document,
        &old_external,
        relation.clone(),
    );
    fixture.attach(&old_attachment).unwrap();

    fixture.append(
        fact(
            2,
            fixture.project.clone(),
            "issue.door",
            &relation,
            external("issue/current"),
            EvidenceState::Accepted,
            Visibility::Project,
        )
        .supersedes(1),
    );

    assert!(matches!(
        fixture.attach(&old_attachment).unwrap_err(),
        EvidenceBridgeError::StaleFact { sequence: 1 }
    ));
    assert!(fixture.query("issue.door", [document]).unwrap().is_empty());
}

#[test]
fn rejected_fact_stays_rejected_after_office_resolution() {
    let mut fixture = Fixture::new("project-a");
    let relation = commercial_support_relation();
    let external = external("invoice/rejected-4");
    fixture.append(fact(
        1,
        fixture.project.clone(),
        "commercial.invoice-4",
        &relation,
        external.clone(),
        EvidenceState::Rejected,
        Visibility::Project,
    ));
    let document = fixture.save_document("office/commercial-review");
    fixture
        .attach(&attachment(
            &fixture.project,
            "commercial.invoice-4",
            1,
            &document,
            &external,
            relation,
        ))
        .unwrap();

    let links = fixture.query("commercial.invoice-4", [document]).unwrap();
    assert_eq!(links[0].evidence_state, EvidenceState::Rejected);
    assert!(!links[0].is_accepted());
}

#[test]
fn query_order_is_deterministic_across_document_input_order_and_duplicates() {
    let mut fixture = Fixture::new("project-a");
    let document_b = fixture.save_document("office/z-register");
    let document_a = fixture.save_document("office/a-register");
    let rows = [
        (
            1,
            "design.wall",
            design_source_relation(),
            external("design/1"),
            document_b.clone(),
        ),
        (
            2,
            "design.wall",
            commercial_support_relation(),
            external("accounting/2"),
            document_a.clone(),
        ),
        (
            3,
            "design.wall",
            schedule_basis_relation(),
            external("schedule/3"),
            document_a.clone(),
        ),
    ];
    for (seq, control, relation, external, document) in &rows {
        fixture.append(fact(
            *seq,
            fixture.project.clone(),
            control,
            relation,
            external.clone(),
            EvidenceState::Accepted,
            Visibility::Project,
        ));
        fixture
            .attach(&attachment(
                &fixture.project,
                control,
                *seq,
                document,
                external,
                relation.clone(),
            ))
            .unwrap();
    }

    let first = fixture
        .query(
            "design.wall",
            [document_b.clone(), document_a.clone(), document_a.clone()],
        )
        .unwrap();
    let second = fixture
        .query("design.wall", [document_a, document_b])
        .unwrap();
    assert_eq!(first, second);
    assert_eq!(
        first.iter().map(|link| link.fact_seq).collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
}

#[test]
fn relation_and_external_reference_must_come_from_the_fact() {
    let mut fixture = Fixture::new("project-a");
    let relation = design_source_relation();
    let retained = external("design/1");
    fixture.append(fact(
        1,
        fixture.project.clone(),
        "design.wall",
        &relation,
        retained.clone(),
        EvidenceState::Accepted,
        Visibility::Project,
    ));
    let document = fixture.save_document("office/design");

    let wrong_relation = attachment(
        &fixture.project,
        "design.wall",
        1,
        &document,
        &retained,
        schedule_basis_relation(),
    );
    assert!(matches!(
        fixture.attach(&wrong_relation).unwrap_err(),
        EvidenceBridgeError::RelationMismatch { sequence: 1, .. }
    ));

    let unretained = external("design/not-retained");
    let missing_reference = attachment(
        &fixture.project,
        "design.wall",
        1,
        &document,
        &unretained,
        relation,
    );
    assert!(matches!(
        fixture.attach(&missing_reference).unwrap_err(),
        EvidenceBridgeError::ReferenceMissing { sequence: 1 }
    ));
}

struct Fixture {
    cx: Cx,
    store: DocStore,
    project: ProjectId,
    book: ProjectBook,
    access: ProjectEvidenceAccess,
}

impl Fixture {
    fn new(project_name: &str) -> Self {
        let project = project(project_name);
        Self {
            cx: authorized_context(),
            store: DocStore::create(Path::new(":memory:")).unwrap(),
            book: ProjectBook::new(project.clone(), role()),
            access: ProjectEvidenceAccess::project(project.clone()),
            project,
        }
    }

    fn append(&mut self, fact: ProjectFact) {
        self.book.append(fact).unwrap();
    }

    fn save_document(&mut self, id: &str) -> DocId {
        save_document(&mut self.cx, &self.store, id)
    }

    fn attach(
        &self,
        attachment: &EvidenceAttachment,
    ) -> Result<AttachOutcome, EvidenceBridgeError> {
        attach_evidence(&self.cx, &self.store, &self.book, &self.access, attachment)
    }

    fn query(
        &self,
        control_id: &str,
        documents: impl IntoIterator<Item = DocId>,
    ) -> Result<Vec<crate::EvidenceLink>, EvidenceBridgeError> {
        evidence_for_documents(
            &self.cx,
            &self.store,
            &self.book,
            &self.access,
            &control(control_id),
            documents,
        )
    }
}

fn context() -> Cx {
    Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory))
}

fn authorized_context() -> Cx {
    let mut cx = context();
    cx.grant(construction_project_read_capability());
    cx.grant(office_evidence_read_capability());
    cx.grant(office_evidence_write_capability());
    cx
}

fn assert_capability(error: EvidenceBridgeError, expected: CapabilityName) {
    assert!(matches!(
        error,
        EvidenceBridgeError::Capability(Error::CapabilityDenied { capability })
            if capability == expected
    ));
}

fn save_document(cx: &mut Cx, store: &DocStore, id: &str) -> DocId {
    let id = DocId::new(id);
    let body = cx
        .factory()
        .string("reviewable office projection".to_owned())
        .unwrap();
    store
        .save_doc(&Doc::new(
            DocKind::new("report"),
            id.clone(),
            body,
            Vec::new(),
        ))
        .unwrap();
    id
}

fn fact(
    seq: u64,
    project: ProjectId,
    control_id: &str,
    relation: &crate::EvidenceRelation,
    external: ExternalRef,
    state: EvidenceState,
    visibility: Visibility,
) -> ProjectFact {
    ProjectFact::new(
        seq,
        project,
        control(control_id),
        relation.fact_kind().clone(),
        Date::from_calendar_date(2026, Month::July, 30).unwrap(),
        role(),
        Expr::String("construction evidence relation".to_owned()),
    )
    .with_evidence(external)
    .with_evidence_state(state)
    .with_visibility(visibility)
}

fn attachment(
    project: &ProjectId,
    control_id: &str,
    fact_seq: u64,
    document: &DocId,
    external: &ExternalRef,
    relation: crate::EvidenceRelation,
) -> EvidenceAttachment {
    EvidenceAttachment::new(
        project.clone(),
        control(control_id),
        fact_seq,
        document.clone(),
        external.clone(),
        relation,
    )
}

fn external(id: &str) -> ExternalRef {
    ExternalRef::new("doc/synthetic", id, Some("revision-a".to_owned()), None)
}

fn project(id: &str) -> ProjectId {
    ProjectId::new(id).unwrap()
}

fn control(id: &str) -> ControlId {
    ControlId::new(id).unwrap()
}

fn role() -> RoleId {
    RoleId::new("project-chief").unwrap()
}

fn book(project_id: &str) -> ProjectBook {
    ProjectBook::new(project(project_id), role())
}
