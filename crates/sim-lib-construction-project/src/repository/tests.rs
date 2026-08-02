use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use sim_kernel::{Cx, DefaultFactory, EagerPolicy, Error, Expr, Symbol, Value};
use sim_lib_doc_core::ExternalRef;
use sim_table_fs::{FsDir, table_fs_read_capability, table_fs_write_capability};
use sim_table_hash::HashTable;
use time::{Date, Month};

use super::{ProjectBookRepository, decode_projection};
use crate::{
    AcceptedBaseline, BaselineId, BaselineKind, ConstructionProjectLib, ControlId, ProjectFact,
    ProjectId, RoleId, construction_project_read_capability, construction_project_write_capability,
};

#[test]
fn hash_table_runs_the_complete_project_book_conformance_suite() {
    let mut cx = context();
    grant_project_authority(&mut cx);
    let root = cx.factory().opaque(Arc::new(HashTable::new())).unwrap();
    run_conformance(&mut cx, root);
}

#[test]
fn filesystem_dir_runs_the_same_project_book_conformance_suite() {
    let temporary = TempRoot::new();
    let mut cx = filesystem_context();
    grant_project_authority(&mut cx);
    let root = cx
        .factory()
        .opaque(Arc::new(FsDir::open(temporary.path.clone()).unwrap()))
        .unwrap();
    run_conformance(&mut cx, root);
}

#[test]
fn filesystem_dir_reopens_and_rebuilds_historical_state() {
    let temporary = TempRoot::new();
    let mut cx = filesystem_context();
    grant_project_authority(&mut cx);
    let first_root = cx
        .factory()
        .opaque(Arc::new(FsDir::open(temporary.path.clone()).unwrap()))
        .unwrap();
    let first = repository(first_root);
    first
        .append_fact(&mut cx, fact(1, "scope", "original"))
        .unwrap();
    first
        .append_fact(&mut cx, fact(2, "scope", "corrected").supersedes(1))
        .unwrap();
    first.read_snapshot(&mut cx, 2).unwrap();

    let reopened_root = cx
        .factory()
        .opaque(Arc::new(FsDir::open(temporary.path.clone()).unwrap()))
        .unwrap();
    let reopened = repository(reopened_root);
    let historical = reopened.read_snapshot(&mut cx, 1).unwrap();
    let current = reopened.read_snapshot(&mut cx, 2).unwrap();

    assert_eq!(
        historical.current[&control("scope")].body,
        Expr::String("original".to_owned())
    );
    assert_eq!(
        current.current[&control("scope")].body,
        Expr::String("corrected".to_owned())
    );
}

#[test]
fn construction_read_and_write_authority_are_independent() {
    let mut owner = context();
    let root = owner.factory().opaque(Arc::new(HashTable::new())).unwrap();
    let repo = repository(root.clone());

    let write_error = repo
        .append_fact(&mut owner, fact(1, "scope", "original"))
        .unwrap_err();
    assert!(matches!(
        write_error,
        Error::CapabilityDenied { capability }
            if capability == construction_project_write_capability()
    ));
    let read_error = repo.read_book(&mut owner, 0).unwrap_err();
    assert!(matches!(
        read_error,
        Error::CapabilityDenied { capability }
            if capability == construction_project_read_capability()
    ));

    owner.grant(construction_project_write_capability());
    repo.append_fact(&mut owner, fact(1, "scope", "original"))
        .unwrap();

    let mut reader = context();
    reader.grant(construction_project_read_capability());
    assert_eq!(repository(root).read_book(&mut reader, 1).unwrap().len(), 1);
}

#[test]
fn backend_capability_errors_are_propagated_unchanged() {
    let temporary = TempRoot::new();
    let mut owner = filesystem_context();
    grant_project_authority(&mut owner);
    let root = owner
        .factory()
        .opaque(Arc::new(FsDir::open(temporary.path.clone()).unwrap()))
        .unwrap();
    let repo = repository(root.clone());
    repo.append_fact(&mut owner, fact(1, "scope", "original"))
        .unwrap();

    let mut denied = context();
    denied.grant(construction_project_read_capability());
    let error = repository(root).read_book(&mut denied, 1).unwrap_err();
    assert!(matches!(
        error,
        Error::CapabilityDenied { capability }
            if capability == table_fs_read_capability()
    ));
}

fn run_conformance(cx: &mut Cx, root: Value) {
    let repo = repository(root.clone());
    let lib = ConstructionProjectLib::with_project_book(root, project(), writer()).unwrap();
    assert_eq!(
        lib.project_books().unwrap().backend_symbol().unwrap(),
        repo.backend_symbol().unwrap()
    );

    let non_monotone = repo
        .append_fact(cx, fact(2, "scope", "sequence gap"))
        .unwrap_err();
    assert!(non_monotone.to_string().contains("fact 1 is missing"));

    let mut bad_reference = fact(1, "scope", "invalid reference");
    bad_reference.evidence[0].backend.clear();
    let reference_error = repo.append_fact(cx, bad_reference).unwrap_err();
    assert!(
        reference_error
            .to_string()
            .contains("fact.evidence.backend")
    );

    let wrong_project = ProjectFact::new(
        1,
        ProjectId::new("project-other").unwrap(),
        control("scope"),
        kind(),
        effective_on(),
        writer(),
        Expr::String("wrong project".to_owned()),
    )
    .with_evidence(evidence(1));
    assert!(
        repo.append_fact(cx, wrong_project)
            .unwrap_err()
            .to_string()
            .contains("does not match project book")
    );

    repo.append_fact(cx, fact(1, "scope", "original")).unwrap();
    assert!(
        repo.append_fact(cx, fact(1, "scope", "duplicate"))
            .unwrap_err()
            .to_string()
            .contains("duplicate fact sequence 1")
    );
    assert!(
        repo.append_fact(cx, fact(3, "scope", "sequence gap"))
            .unwrap_err()
            .to_string()
            .contains("fact 2 is missing")
    );
    repo.append_fact(cx, fact(2, "scope", "corrected").supersedes(1))
        .unwrap();

    let historical = repo.read_book(cx, 1).unwrap();
    let current = repo.read_book(cx, 2).unwrap();
    assert_eq!(historical.len(), 1);
    assert_eq!(
        historical.fact(1).unwrap().body,
        Expr::String("original".to_owned())
    );
    assert_eq!(
        current.snapshot_at(2).unwrap().current[&control("scope")].body,
        Expr::String("corrected".to_owned())
    );

    let baseline = AcceptedBaseline::new(
        BaselineId::new("scope-main").unwrap(),
        project(),
        control("scope"),
        BaselineKind::Scope,
        writer(),
        2,
        effective_on(),
    )
    .with_evidence(evidence(2));
    repo.write_baseline(cx, &baseline).unwrap();
    assert_eq!(
        repo.read_baseline(cx, &baseline.id).unwrap(),
        Some(baseline)
    );
    let policy = Expr::Map(vec![(
        Expr::Symbol(Symbol::new("non-waivable")),
        Expr::Bool(true),
    )]);
    repo.write_policy(cx, "gate-main", policy.clone()).unwrap();
    assert_eq!(repo.read_policy(cx, "gate-main").unwrap(), Some(policy));

    let snapshot = repo.read_snapshot(cx, 2).unwrap();
    assert_eq!(snapshot.through_seq, 2);
    let projection_path = repo.layout.projection("snapshot", 2).unwrap();
    let partial = cx
        .factory()
        .expr(Expr::Map(vec![(
            Expr::Symbol(Symbol::new("source_sequence")),
            Expr::String("interrupted".to_owned()),
        )]))
        .unwrap();
    repo.write_path(cx, &projection_path, partial).unwrap();

    let regenerated = repo.read_snapshot(cx, 2).unwrap();
    assert_eq!(regenerated, snapshot);
    let repaired = repo.read_path(cx, &projection_path).unwrap().unwrap();
    let repaired = decode_projection(cx, &repaired).unwrap();
    assert_eq!(repaired.source_sequence, 2);
    assert_eq!(repaired.snapshot, snapshot);

    let fact_path = repo.layout.fact(1).unwrap();
    let corrupt = cx
        .factory()
        .expr(Expr::String("truncated fact write".to_owned()))
        .unwrap();
    repo.write_path(cx, &fact_path, corrupt).unwrap();
    assert!(
        repo.read_book(cx, 2)
            .unwrap_err()
            .to_string()
            .contains("authoritative fact 1 is corrupt")
    );
    repo.delete_path(cx, &fact_path).unwrap();
    assert!(
        repo.read_book(cx, 2)
            .unwrap_err()
            .to_string()
            .contains("authoritative fact 1 is missing")
    );
}

fn repository(root: Value) -> ProjectBookRepository {
    ProjectBookRepository::new(root, project(), writer()).unwrap()
}

fn context() -> Cx {
    Cx::new(Arc::new(EagerPolicy), Arc::new(DefaultFactory))
}

fn filesystem_context() -> Cx {
    let mut cx = context();
    let codec = sim_codec_lisp::LispCodecLib::new(cx.registry_mut().fresh_codec_id()).unwrap();
    cx.load_lib(&codec).unwrap();
    cx.grant(table_fs_read_capability());
    cx.grant(table_fs_write_capability());
    cx
}

fn grant_project_authority(cx: &mut Cx) {
    cx.grant(construction_project_read_capability());
    cx.grant(construction_project_write_capability());
}

fn fact(sequence: u64, subject: &str, body: &str) -> ProjectFact {
    ProjectFact::new(
        sequence,
        project(),
        control(subject),
        kind(),
        effective_on(),
        writer(),
        Expr::String(body.to_owned()),
    )
    .with_evidence(evidence(sequence))
}

fn project() -> ProjectId {
    ProjectId::new("project-alpha").unwrap()
}

fn writer() -> RoleId {
    RoleId::new("project-chief").unwrap()
}

fn control(id: &str) -> ControlId {
    ControlId::new(id).unwrap()
}

fn kind() -> Symbol {
    Symbol::qualified("construction", "fact")
}

fn evidence(sequence: u64) -> ExternalRef {
    ExternalRef::new(
        "doc/synthetic",
        format!("project-alpha/fact/{sequence}"),
        Some(format!("rev-{sequence}")),
        None,
    )
}

fn effective_on() -> Date {
    Date::from_calendar_date(2026, Month::July, 30).unwrap()
}

struct TempRoot {
    path: PathBuf,
}

impl TempRoot {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "sim-construction-project-book-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        Self { path }
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        if self.path.exists() {
            std::fs::remove_dir_all(&self.path).expect("remove owned temporary project book");
        }
    }
}
