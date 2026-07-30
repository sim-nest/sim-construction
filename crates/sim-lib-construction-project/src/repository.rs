//! Table-backed persistence for append-only construction project books.
//!
//! A repository is bound to one [`ProjectId`] and one authoritative
//! [`RoleId`]. It accepts an injected runtime [`Table`](sim_kernel::Table), and
//! uses nested [`Dir`](sim_kernel::Dir) operations when the injected value
//! provides them. A plain Table stores the same canonical [`TablePath`] as one
//! unqualified symbol key. No backend is selected or constructed here.
//!
//! Table has no canonical compare-and-swap operation. This repository therefore
//! has one authoritative writer per project book and admits contiguous
//! sequences only. It validates a fact and replays its complete predecessor
//! history before the one authoritative `set`. Callers must serialize writes
//! for a project; adding a backend-specific transaction here would create a
//! false portability contract.
//!
//! Reads never enumerate the injected root or list project ids. A requested
//! historical sequence is rebuilt from `facts/1` through `facts/<as-of>`, so a
//! missing or corrupt authoritative fact fails closed. Snapshot projections
//! carry their source sequence and canonical fact-stream content identity.
//! Missing or invalid projections are ignored and regenerated from facts;
//! callers holding write authority also repair the disposable cache.

use std::{
    fmt::Write as _,
    sync::{Arc, Mutex},
};

use sim_kernel::{Cx, Datum, Error, Expr, Symbol, Value};
use sim_table_core::{TablePath, TablePathError};

use crate::{
    AcceptedBaseline, BaselineId, ProjectBook, ProjectBookLayout, ProjectFact, ProjectId,
    ProjectSnapshot, RoleId, citizen, construction_project_read_capability,
    construction_project_write_capability,
};

const SNAPSHOT_PROJECTION: &str = "snapshot";

/// A project-scoped persistent book over one caller-supplied Table or Dir.
#[derive(Clone, Debug)]
pub struct ProjectBookRepository {
    root: Value,
    layout: ProjectBookLayout,
    writer: RoleId,
    hierarchical: bool,
    write_gate: Arc<Mutex<()>>,
}

impl ProjectBookRepository {
    /// Binds an injected Table/Dir to one project and authoritative writer.
    ///
    /// The caller chooses and constructs the backend. The value must implement
    /// the kernel Table contract; Dir support is detected without naming a
    /// concrete backend.
    pub fn new(root: Value, project: ProjectId, writer: RoleId) -> sim_kernel::Result<Self> {
        ProjectId::new(project.as_str()).map_err(Error::from)?;
        RoleId::new(writer.as_str()).map_err(Error::from)?;
        if root.object().as_table_impl().is_none() {
            return Err(Error::Eval(
                "construction project repository requires an injected Table or Dir".to_owned(),
            ));
        }
        let hierarchical = root.object().as_dir().is_some();
        let layout = ProjectBookLayout::new(project).map_err(layout_error)?;
        Ok(Self {
            root,
            layout,
            writer,
            hierarchical,
            write_gate: Arc::new(Mutex::new(())),
        })
    }

    /// Returns the project this repository can access.
    #[must_use]
    pub fn project(&self) -> &ProjectId {
        self.layout.project()
    }

    /// Returns the sole authoritative writer accepted by this repository.
    #[must_use]
    pub fn authoritative_writer(&self) -> &RoleId {
        &self.writer
    }

    /// Returns the injected backend's open symbol without choosing a backend.
    pub fn backend_symbol(&self) -> sim_kernel::Result<Symbol> {
        self.root
            .object()
            .as_table_impl()
            .map(sim_kernel::Table::backend_symbol)
            .ok_or_else(|| Error::Eval("injected project repository Table disappeared".to_owned()))
    }

    /// Appends an authoritative fact after Shape/domain, identity, sequence,
    /// supersession, reference, writer, project, and write-capability checks.
    ///
    /// The caller must serialize this operation with every other writer for the
    /// same project book.
    pub fn append_fact(&self, cx: &mut Cx, fact: ProjectFact) -> sim_kernel::Result<()> {
        cx.require(&construction_project_write_capability())?;
        let _write = self.write_gate.lock().map_err(|_| {
            Error::Eval("construction project repository writer lock poisoned".to_owned())
        })?;
        fact.validate_bounds().map_err(Error::from)?;
        self.require_project(&fact.project)?;
        if fact.actor_role != self.writer {
            return Err(crate::ConstructionProjectError::WriterMismatch {
                expected: self.writer.clone(),
                actual: fact.actor_role.clone(),
            }
            .into());
        }
        if fact.seq == 0 {
            return Err(crate::ConstructionProjectError::InvalidSequence {
                field: "fact.seq",
                sequence: 0,
            }
            .into());
        }

        let path = self.layout.fact(fact.seq).map_err(layout_error)?;
        if self.read_path(cx, &path)?.is_some() {
            return Err(
                crate::ConstructionProjectError::DuplicateSequence { sequence: fact.seq }.into(),
            );
        }

        let mut book = self.rebuild(cx, fact.seq - 1)?;
        book.append(fact.clone()).map_err(Error::from)?;
        let value = cx.factory().expr(citizen::encode_semantic(&fact)?)?;
        self.write_path(cx, &path, value)
    }

    /// Rebuilds a project book through `as_of` from authoritative fact paths.
    ///
    /// This requires only `construction.project.read`; it never lists other
    /// projects or even the project directory.
    pub fn read_book(&self, cx: &mut Cx, as_of: u64) -> sim_kernel::Result<ProjectBook> {
        cx.require(&construction_project_read_capability())?;
        self.rebuild(cx, as_of)
    }

    /// Rebuilds a historical snapshot and verifies its disposable cache.
    ///
    /// The authoritative fact stream is always read first. A missing, corrupt,
    /// stale, or content-mismatched cache is never trusted. If the caller also
    /// holds `construction.project.write`, the regenerated projection replaces
    /// the bad cache; read-only callers still receive the rebuilt snapshot.
    pub fn read_snapshot(&self, cx: &mut Cx, as_of: u64) -> sim_kernel::Result<ProjectSnapshot> {
        cx.require(&construction_project_read_capability())?;
        let book = self.rebuild(cx, as_of)?;
        let snapshot = book.snapshot_at(as_of).map_err(Error::from)?;
        let source_identity = fact_stream_identity(&book)?;
        let path = self
            .layout
            .projection(SNAPSHOT_PROJECTION, as_of)
            .map_err(layout_error)?;
        let cached = self
            .read_path(cx, &path)?
            .and_then(|value| decode_projection(cx, &value).ok());
        let valid = cached.is_some_and(|cached| {
            cached.source_sequence == as_of
                && cached.source_identity == source_identity
                && cached.snapshot == snapshot
        });
        if !valid
            && cx
                .capabilities()
                .contains(&construction_project_write_capability())
        {
            let projection = SnapshotProjection {
                source_sequence: as_of,
                source_identity,
                snapshot: snapshot.clone(),
            };
            let value = cx.factory().expr(citizen::encode_semantic(&projection)?)?;
            self.write_path(cx, &path, value)?;
        }
        Ok(snapshot)
    }

    /// Stores an accepted baseline expression in the authoritative baseline
    /// lane after project, record, and write-capability checks.
    pub fn write_baseline(
        &self,
        cx: &mut Cx,
        baseline: &AcceptedBaseline,
    ) -> sim_kernel::Result<()> {
        cx.require(&construction_project_write_capability())?;
        baseline.validate().map_err(Error::from)?;
        self.require_project(&baseline.project)?;
        let path = self.layout.baseline(&baseline.id).map_err(layout_error)?;
        let value = cx.factory().expr(citizen::encode_semantic(baseline)?)?;
        self.write_path(cx, &path, value)
    }

    /// Reads one accepted baseline by exact id without enumerating projects.
    pub fn read_baseline(
        &self,
        cx: &mut Cx,
        id: &BaselineId,
    ) -> sim_kernel::Result<Option<AcceptedBaseline>> {
        cx.require(&construction_project_read_capability())?;
        let path = self.layout.baseline(id).map_err(layout_error)?;
        self.read_path(cx, &path)?
            .map(|value| {
                let expr = value.object().as_expr(cx)?;
                let baseline: AcceptedBaseline =
                    citizen::decode_semantic(&expr, "authoritative baseline")?;
                baseline.validate().map_err(Error::from)?;
                self.require_project(&baseline.project)?;
                if baseline.id != *id {
                    return Err(Error::Eval(format!(
                        "construction authoritative baseline {} carries id {}",
                        id, baseline.id
                    )));
                }
                Ok(baseline)
            })
            .transpose()
    }

    /// Stores one version-neutral policy expression under an exact safe id.
    pub fn write_policy(&self, cx: &mut Cx, id: &str, policy: Expr) -> sim_kernel::Result<()> {
        cx.require(&construction_project_write_capability())?;
        let path = self.layout.policy(id).map_err(layout_error)?;
        let value = cx.factory().expr(policy)?;
        self.write_path(cx, &path, value)
    }

    /// Reads one policy expression by exact id without directory enumeration.
    pub fn read_policy(&self, cx: &mut Cx, id: &str) -> sim_kernel::Result<Option<Expr>> {
        cx.require(&construction_project_read_capability())?;
        let path = self.layout.policy(id).map_err(layout_error)?;
        self.read_path(cx, &path)?
            .map(|value| value.object().as_expr(cx))
            .transpose()
    }

    fn require_project(&self, actual: &ProjectId) -> sim_kernel::Result<()> {
        if actual == self.project() {
            Ok(())
        } else {
            Err(crate::ConstructionProjectError::ProjectMismatch {
                expected: self.project().clone(),
                actual: actual.clone(),
            }
            .into())
        }
    }

    fn rebuild(&self, cx: &mut Cx, as_of: u64) -> sim_kernel::Result<ProjectBook> {
        let count = usize::try_from(as_of).unwrap_or(usize::MAX);
        if count > crate::DEFAULT_MAX_PROJECT_FACTS {
            return Err(crate::ConstructionProjectError::FactLimitExceeded {
                max: crate::DEFAULT_MAX_PROJECT_FACTS,
            }
            .into());
        }
        let mut book = ProjectBook::new(self.project().clone(), self.writer.clone());
        for sequence in 1..=as_of {
            let path = self.layout.fact(sequence).map_err(layout_error)?;
            let value = self.read_path(cx, &path)?.ok_or_else(|| {
                Error::Eval(format!(
                    "construction authoritative fact {sequence} is missing for project {}",
                    self.project()
                ))
            })?;
            let fact = citizen::decode_fact_value(cx, &value).map_err(|error| {
                Error::Eval(format!(
                    "construction authoritative fact {sequence} is corrupt for project {}: {error}",
                    self.project()
                ))
            })?;
            if fact.seq != sequence {
                return Err(Error::Eval(format!(
                    "construction authoritative fact path {sequence} carries sequence {}",
                    fact.seq
                )));
            }
            book.append(fact).map_err(Error::from)?;
        }
        Ok(book)
    }

    fn read_path(&self, cx: &mut Cx, path: &TablePath) -> sim_kernel::Result<Option<Value>> {
        if !self.hierarchical {
            let table = self.root.object().as_table_impl().ok_or_else(|| {
                Error::Eval("injected project repository Table disappeared".to_owned())
            })?;
            let key = Symbol::new(path.join());
            return if table.has(cx, key.clone())? {
                table.get(cx, key).map(Some)
            } else {
                Ok(None)
            };
        }
        let (parents, leaf) = split_leaf(path)?;
        let Some(table_value) = self.open_dirs(cx, parents, false)? else {
            return Ok(None);
        };
        let table = table_value.object().as_table_impl().ok_or_else(|| {
            Error::Eval(format!(
                "construction path parent for {path} is not a Table"
            ))
        })?;
        let key = Symbol::new(leaf);
        if table.has(cx, key.clone())? {
            table.get(cx, key).map(Some)
        } else {
            Ok(None)
        }
    }

    fn write_path(&self, cx: &mut Cx, path: &TablePath, value: Value) -> sim_kernel::Result<()> {
        if !self.hierarchical {
            let table = self.root.object().as_table_impl().ok_or_else(|| {
                Error::Eval("injected project repository Table disappeared".to_owned())
            })?;
            return table.set(cx, Symbol::new(path.join()), value);
        }
        let (parents, leaf) = split_leaf(path)?;
        let table_value = self
            .open_dirs(cx, parents, true)?
            .expect("create=true always returns a Table value");
        let table = table_value.object().as_table_impl().ok_or_else(|| {
            Error::Eval(format!(
                "construction path parent for {path} is not a Table"
            ))
        })?;
        table.set(cx, Symbol::new(leaf), value)
    }

    #[cfg(test)]
    fn delete_path(&self, cx: &mut Cx, path: &TablePath) -> sim_kernel::Result<Value> {
        if !self.hierarchical {
            let table = self.root.object().as_table_impl().ok_or_else(|| {
                Error::Eval("injected project repository Table disappeared".to_owned())
            })?;
            return table.del(cx, Symbol::new(path.join()));
        }
        let (parents, leaf) = split_leaf(path)?;
        let table_value = self.open_dirs(cx, parents, false)?.ok_or_else(|| {
            Error::Eval(format!("construction path parent for {path} is missing"))
        })?;
        let table = table_value.object().as_table_impl().ok_or_else(|| {
            Error::Eval(format!(
                "construction path parent for {path} is not a Table"
            ))
        })?;
        table.del(cx, Symbol::new(leaf))
    }

    fn open_dirs(
        &self,
        cx: &mut Cx,
        segments: &[String],
        create: bool,
    ) -> sim_kernel::Result<Option<Value>> {
        let mut current = self.root.clone();
        for segment in segments {
            let dir = current.object().as_dir().ok_or_else(|| {
                Error::Eval(format!(
                    "construction path component {segment:?} is not a Dir"
                ))
            })?;
            let name = Symbol::new(segment.clone());
            current = match dir.opendir(cx, name.clone())? {
                Some(value) => value,
                None if create => dir.mkdir(cx, name)?,
                None => return Ok(None),
            };
        }
        Ok(Some(current))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct SnapshotProjection {
    source_sequence: u64,
    source_identity: String,
    snapshot: ProjectSnapshot,
}

fn decode_projection(cx: &mut Cx, value: &Value) -> sim_kernel::Result<SnapshotProjection> {
    let expr = value.object().as_expr(cx)?;
    citizen::decode_semantic(&expr, "snapshot projection")
}

fn fact_stream_identity(book: &ProjectBook) -> sim_kernel::Result<String> {
    let facts = book
        .facts()
        .map(citizen::encode_semantic)
        .collect::<sim_kernel::Result<Vec<_>>>()?;
    let datum = Datum::try_from(Expr::Vector(facts))?;
    let id = datum.content_id()?;
    let mut identity = id.algorithm.as_qualified_str();
    identity.push(':');
    for byte in id.bytes {
        write!(identity, "{byte:02x}")
            .map_err(|error| Error::Eval(format!("content identity formatting failed: {error}")))?;
    }
    Ok(identity)
}

fn split_leaf(path: &TablePath) -> sim_kernel::Result<(&[String], String)> {
    let (leaf, parents) = path.segments().split_last().ok_or_else(|| {
        Error::Eval("construction repository cannot address Table root".to_owned())
    })?;
    Ok((parents, leaf.clone()))
}

fn layout_error(error: TablePathError) -> Error {
    Error::Eval(format!(
        "construction project repository rejected TablePath component: {error:?}"
    ))
}

#[cfg(test)]
#[path = "repository/tests.rs"]
mod tests;
