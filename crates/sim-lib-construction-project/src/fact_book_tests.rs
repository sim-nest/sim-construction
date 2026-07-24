// conformance: construction project fact book snapshots

use crate::{
    ConstructionProjectError, ControlId, EvidenceState, MAX_FACT_BODY_NODES, ProjectBook,
    ProjectFact, ProjectId, ProjectSnapshot, RoleId, SnapshotExplanationKind, snapshot_at,
    snapshot_delta,
};
use sim_kernel::{Expr, Symbol};
use sim_lib_doc_core::ExternalRef;
use time::{Date, Month};

#[test]
fn project_book_replays_facts_in_sequence_order() {
    let writer = writer();
    let project = project();
    let facts = vec![
        fact(2, "decision.fixture", "approve fixture").supersedes(1),
        fact(1, "decision.fixture", "open fixture"),
    ];

    let book = ProjectBook::from_facts(project, writer, facts).unwrap();
    let snapshot = snapshot_at(&book, 2).unwrap();

    assert_eq!(book.last_sequence(), Some(2));
    assert_eq!(
        snapshot
            .current_fact(&control("decision.fixture"))
            .unwrap()
            .body,
        Expr::String("approve fixture".to_owned())
    );
    assert_eq!(snapshot.superseded[&control("decision.fixture")][0].seq, 1);
}

#[test]
fn as_of_snapshot_preserves_prior_state() {
    let mut book = book();
    book.append(fact(1, "decision.fixture", "open fixture"))
        .unwrap();
    book.append(fact(2, "decision.fixture", "approve fixture").supersedes(1))
        .unwrap();

    let as_of = ProjectSnapshot::at(&book, 1).unwrap();
    let current = ProjectSnapshot::at(&book, 2).unwrap();

    assert_eq!(
        as_of
            .current_fact(&control("decision.fixture"))
            .unwrap()
            .body,
        Expr::String("open fixture".to_owned())
    );
    assert_eq!(
        current
            .current_fact(&control("decision.fixture"))
            .unwrap()
            .body,
        Expr::String("approve fixture".to_owned())
    );
}

#[test]
fn snapshot_delta_reports_added_superseded_and_conflicted_subjects() {
    let mut book = book();
    book.append(fact(1, "decision.fixture", "open fixture"))
        .unwrap();
    book.append(fact(2, "decision.fixture", "approve fixture").supersedes(1))
        .unwrap();
    book.append(fact(3, "requirement.access", "first access rule"))
        .unwrap();
    book.append(fact(4, "requirement.access", "competing access rule"))
        .unwrap();

    let delta = snapshot_delta(&book, 1, 4).unwrap();

    assert_eq!(
        delta.added,
        vec![control("decision.fixture"), control("requirement.access"),]
    );
    assert_eq!(delta.superseded, vec![control("decision.fixture")]);
    assert_eq!(delta.conflicted, vec![control("requirement.access")]);
}

#[test]
fn rejected_conflicted_and_superseded_facts_remain_explained() {
    let mut book = book();
    book.append(fact(1, "decision.fixture", "open fixture"))
        .unwrap();
    book.append(
        fact(2, "decision.fixture", "reject fixture").with_evidence_state(EvidenceState::Rejected),
    )
    .unwrap();
    book.append(fact(3, "decision.fixture", "approve fixture").supersedes(1))
        .unwrap();
    book.append(fact(4, "requirement.access", "first access rule"))
        .unwrap();
    book.append(fact(5, "requirement.access", "competing access rule"))
        .unwrap();

    let snapshot = ProjectSnapshot::at(&book, 5).unwrap();
    let explanation_kinds = snapshot
        .explanations
        .iter()
        .map(|row| (row.subject.clone(), row.seq, row.explanation))
        .collect::<Vec<_>>();

    assert!(snapshot.rejected.contains_key(&control("decision.fixture")));
    assert!(snapshot.is_conflicted(&control("requirement.access")));
    assert!(explanation_kinds.contains(&(
        control("decision.fixture"),
        1,
        SnapshotExplanationKind::Superseded
    )));
    assert!(explanation_kinds.contains(&(
        control("decision.fixture"),
        2,
        SnapshotExplanationKind::Rejected
    )));
    assert!(explanation_kinds.contains(&(
        control("requirement.access"),
        4,
        SnapshotExplanationKind::Conflicted
    )));
}

#[test]
fn invalid_sequences_and_supersession_edges_fail_closed() {
    let duplicate = ProjectBook::from_facts(
        project(),
        writer(),
        vec![
            fact(1, "decision.fixture", "open fixture"),
            fact(1, "decision.other", "other fixture"),
        ],
    );
    assert!(matches!(
        duplicate,
        Err(ConstructionProjectError::DuplicateSequence { sequence: 1 })
    ));

    let mut book = book();
    book.append(fact(1, "decision.fixture", "open fixture"))
        .unwrap();

    assert!(matches!(
        book.append(fact(2, "decision.fixture", "bad cycle").supersedes(2)),
        Err(ConstructionProjectError::InvalidSupersession { .. })
    ));
    assert!(matches!(
        book.append(fact(100, "decision.fixture", "missing").supersedes(99)),
        Err(ConstructionProjectError::MissingSupersededFact { .. })
    ));
    assert!(matches!(
        book.append(fact(2, "decision.other", "wrong subject").supersedes(1)),
        Err(ConstructionProjectError::SupersessionSubjectMismatch { .. })
    ));

    book.append(fact(2, "decision.fixture", "approve fixture").supersedes(1))
        .unwrap();
    assert!(matches!(
        book.append(fact(3, "decision.fixture", "fork fixture").supersedes(1)),
        Err(ConstructionProjectError::SupersessionFork { .. })
    ));
}

#[test]
fn project_book_enforces_project_writer_and_bounds() {
    let mut book = ProjectBook::new(project(), writer()).with_max_facts(1);
    book.append(fact(1, "decision.fixture", "open fixture"))
        .unwrap();
    assert!(matches!(
        book.append(fact(2, "decision.other", "other fixture")),
        Err(ConstructionProjectError::FactLimitExceeded { max: 1 })
    ));

    let wrong_project = ProjectFact::new(
        1,
        ProjectId::new("other-project").unwrap(),
        control("decision.fixture"),
        fact_kind(),
        accepted_on(),
        writer(),
        Expr::String("open fixture".to_owned()),
    );
    assert!(matches!(
        ProjectBook::new(project(), writer()).append(wrong_project),
        Err(ConstructionProjectError::ProjectMismatch { .. })
    ));

    let wrong_writer = ProjectFact::new(
        1,
        project(),
        control("decision.fixture"),
        fact_kind(),
        accepted_on(),
        RoleId::new("supplier-lead").unwrap(),
        Expr::String("open fixture".to_owned()),
    );
    assert!(matches!(
        ProjectBook::new(project(), writer()).append(wrong_writer),
        Err(ConstructionProjectError::WriterMismatch { .. })
    ));

    let oversized_body = ProjectFact::new(
        1,
        project(),
        control("decision.fixture"),
        fact_kind(),
        accepted_on(),
        writer(),
        Expr::Vector(vec![Expr::Nil; MAX_FACT_BODY_NODES]),
    );
    assert!(matches!(
        ProjectBook::new(project(), writer()).append(oversized_body),
        Err(ConstructionProjectError::FactBodyTooLarge { .. })
    ));
}

#[test]
fn explanation_order_is_byte_stable() {
    let mut book = book();
    book.append(fact(1, "z.subject", "z")).unwrap();
    book.append(fact(2, "a.subject", "a")).unwrap();
    book.append(fact(3, "a.subject", "a correction").supersedes(2))
        .unwrap();

    let snapshot = ProjectSnapshot::at(&book, 3).unwrap();
    let encoded = serde_json::to_string(&snapshot.explanations).unwrap();

    assert_eq!(
        encoded,
        r#"[{"subject":"a.subject","seq":2,"kind":"construction/fact","evidence_state":"Accepted","explanation":"Superseded","related_seq":3},{"subject":"a.subject","seq":3,"kind":"construction/fact","evidence_state":"Accepted","explanation":"Current","related_seq":null},{"subject":"z.subject","seq":1,"kind":"construction/fact","evidence_state":"Accepted","explanation":"Current","related_seq":null}]"#
    );
}

fn book() -> ProjectBook {
    ProjectBook::new(project(), writer())
}

fn fact(seq: u64, subject: &str, body: &str) -> ProjectFact {
    ProjectFact::new(
        seq,
        project(),
        control(subject),
        fact_kind(),
        accepted_on(),
        writer(),
        Expr::String(body.to_owned()),
    )
    .with_evidence(evidence_ref(seq))
}

fn project() -> ProjectId {
    ProjectId::new("reference-center").unwrap()
}

fn writer() -> RoleId {
    RoleId::new("project-chief").unwrap()
}

fn control(id: &str) -> ControlId {
    ControlId::new(id).unwrap()
}

fn fact_kind() -> Symbol {
    Symbol::qualified("construction", "fact")
}

fn evidence_ref(seq: u64) -> ExternalRef {
    ExternalRef::new(
        "doc/synthetic",
        format!("fact/reference-center/{seq}"),
        Some(format!("rev-{seq}")),
        None,
    )
}

fn accepted_on() -> Date {
    Date::from_calendar_date(2026, Month::July, 23).unwrap()
}
