//! External field-item correlation into append-only project facts.

use sim_kernel::{Expr, Symbol};
use sim_lib_doc_core::ExternalRef;
use time::Date;

use crate::{
    ConstructionProjectError, EvidenceState, FieldItem, FieldItemKind, FieldItemReference,
    FieldItemState, FieldLane, FieldSeverity, ProjectBook, ProjectFact, Result,
};

/// Backend namespace used for references into the kernel effect ledger.
pub const EFFECT_LEDGER_BACKEND: &str = "effect/ledger";

/// Source-minimized field-item import.
///
/// The external system retains payloads, attachments, credentials, and URLs.
/// This record admits only stable correlation, bounded source state, the
/// project-control fields, and the effect-ledger reference for the read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldItemImport {
    /// Accountable project field item produced by the import.
    pub field_item: FieldItem,
    /// Stable source and external item id.
    pub source: FieldItemReference,
    /// Bounded source workflow state.
    pub source_state: String,
    /// Reference to the site operation in the kernel effect ledger.
    pub effect_ref: ExternalRef,
}

impl FieldItemImport {
    /// Builds a minimized external field-item import.
    pub fn new(
        field_item: FieldItem,
        source: FieldItemReference,
        source_state: impl Into<String>,
        effect_ref: ExternalRef,
    ) -> Result<Self> {
        let value = Self {
            field_item,
            source,
            source_state: source_state.into(),
            effect_ref,
        };
        value.validate()?;
        Ok(value)
    }

    /// Validates the import boundary.
    pub fn validate(&self) -> Result<()> {
        if self.field_item.kind != FieldItemKind::ExternalReference {
            return Err(ConstructionProjectError::EmptyField(
                "field_item_import.external_reference_kind",
            ));
        }
        self.field_item.validate()?;
        validate_source_state(&self.source_state)?;
        if self.effect_ref.backend != EFFECT_LEDGER_BACKEND
            || self.effect_ref.external_id.trim().is_empty()
            || self.effect_ref.web_url.is_some()
        {
            return Err(ConstructionProjectError::EmptyField(
                "field_item_import.effect_ref",
            ));
        }
        Ok(())
    }
}

/// Result of importing an external field item into a project book.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldItemImportOutcome {
    /// The source version, source state, and project-control fields were already imported.
    Duplicate {
        /// Existing fact sequence.
        existing_seq: u64,
    },
    /// A new fact was appended.
    Appended {
        /// Appended fact sequence.
        seq: u64,
        /// Prior import sequence superseded by a changed source or control state.
        supersedes: Option<u64>,
    },
}

/// Imports one minimized external field item into the append-only fact book.
///
/// Import is idempotent for identical source and project-control state. A
/// changed source version, source state, or local control field becomes a
/// superseding fact. Imported facts remain `Reported`; source presence alone
/// never turns vendor data into accepted construction evidence.
pub fn import_field_item(
    book: &mut ProjectBook,
    next_seq: u64,
    effective_on: Date,
    imported: &FieldItemImport,
) -> Result<FieldItemImportOutcome> {
    imported.validate()?;
    if imported.field_item.project != *book.project() {
        return Err(ConstructionProjectError::ProjectMismatch {
            expected: book.project().clone(),
            actual: imported.field_item.project.clone(),
        });
    }

    let body = import_body(imported);
    let prior = current_import(book, &imported.source);
    if let Some(prior) = prior
        && prior.body == body
    {
        return Ok(FieldItemImportOutcome::Duplicate {
            existing_seq: prior.seq,
        });
    }

    let supersedes = prior.map(|fact| fact.seq);
    let mut fact = ProjectFact::new(
        next_seq,
        book.project().clone(),
        imported.field_item.control.clone(),
        Symbol::qualified("construction", "field-item-import"),
        effective_on,
        book.authoritative_writer().clone(),
        body,
    )
    .with_evidence(imported.source.as_external_ref())
    .with_evidence(imported.effect_ref.clone())
    .with_evidence_state(EvidenceState::Reported);
    if let Some(prior_seq) = supersedes {
        fact = fact.supersedes(prior_seq);
    }
    book.append(fact)?;
    Ok(FieldItemImportOutcome::Appended {
        seq: next_seq,
        supersedes,
    })
}

fn current_import<'a>(
    book: &'a ProjectBook,
    source: &FieldItemReference,
) -> Option<&'a ProjectFact> {
    book.facts()
        .filter(|fact| {
            fact.kind == Symbol::qualified("construction", "field-item-import")
                && fact.evidence.iter().any(|reference| {
                    reference.backend == source.source
                        && reference.external_id == source.external_id
                })
        })
        .last()
}

fn import_body(imported: &FieldItemImport) -> Expr {
    let item = &imported.field_item;
    Expr::Map(vec![
        field("source", Expr::String(imported.source.source.clone())),
        field(
            "external-id",
            Expr::String(imported.source.external_id.clone()),
        ),
        field(
            "source-version",
            imported
                .source
                .version
                .clone()
                .map_or(Expr::Nil, Expr::String),
        ),
        field("source-state", Expr::String(imported.source_state.clone())),
        field(
            "severity",
            Expr::Symbol(Symbol::new(severity_label(item.severity))),
        ),
        field("lane", Expr::Symbol(Symbol::new(lane_label(item.lane)))),
        field(
            "responsible-role",
            Expr::String(item.responsible_role.to_string()),
        ),
        field(
            "due-on",
            item.due_on
                .map(|date| Expr::String(date.to_string()))
                .unwrap_or(Expr::Nil),
        ),
        field(
            "affected-controls",
            Expr::Vector(
                item.affected_control_ids
                    .iter()
                    .map(|control| Expr::String(control.to_string()))
                    .collect(),
            ),
        ),
        field("state", Expr::Symbol(Symbol::new(state_label(item.state)))),
        field("non-waivable", Expr::Bool(item.non_waivable)),
    ])
}

fn field(name: &str, value: Expr) -> (Expr, Expr) {
    (Expr::Symbol(Symbol::new(name)), value)
}

fn validate_source_state(state: &str) -> Result<()> {
    if state.is_empty()
        || state.len() > 64
        || !state
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        return Err(ConstructionProjectError::EmptyField(
            "field_item_import.source_state",
        ));
    }
    Ok(())
}

const fn severity_label(severity: FieldSeverity) -> &'static str {
    match severity {
        FieldSeverity::Imminent => "imminent",
        FieldSeverity::Critical => "critical",
        FieldSeverity::Major => "major",
        FieldSeverity::Moderate => "moderate",
        FieldSeverity::Minor => "minor",
        FieldSeverity::Information => "information",
    }
}

const fn lane_label(lane: FieldLane) -> &'static str {
    match lane {
        FieldLane::Safety => "safety",
        FieldLane::WorkEnvironment => "work-environment",
        FieldLane::Progress => "progress",
        FieldLane::Quality => "quality",
        FieldLane::Environment => "environment",
        FieldLane::Convenience => "convenience",
    }
}

const fn state_label(state: FieldItemState) -> &'static str {
    match state {
        FieldItemState::Reported => "reported",
        FieldItemState::Open => "open",
        FieldItemState::InProgress => "in-progress",
        FieldItemState::Blocked => "blocked",
        FieldItemState::Corrected => "corrected",
        FieldItemState::Closed => "closed",
        FieldItemState::Rejected => "rejected",
    }
}
