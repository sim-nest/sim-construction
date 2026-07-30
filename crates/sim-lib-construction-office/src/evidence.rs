//! Construction-to-office evidence composition.

use std::{cmp::Ordering, collections::BTreeSet};

use sim_kernel::{CapabilityName, Cx, Symbol};
use sim_lib_construction_project::{
    ControlId, EvidenceState, ProjectBook, ProjectFact, ProjectId, Visibility,
    construction_project_read_capability,
};
use sim_lib_doc_core::{DocId, Evidence, ExternalRef, LinkRole};
use sim_lib_doc_store::{DocStore, evidence};

use crate::{EvidenceBridgeError, pointer::FactPointer, pointer::construction_fact_ref};

/// Capability required to resolve office evidence references.
pub const OFFICE_EVIDENCE_READ_CAPABILITY: &str = "office.evidence.read";
/// Capability required to attach an office evidence projection.
pub const OFFICE_EVIDENCE_WRITE_CAPABILITY: &str = "office.evidence.write";

/// Builds the office evidence-read capability name.
#[must_use]
pub fn office_evidence_read_capability() -> CapabilityName {
    CapabilityName::new(OFFICE_EVIDENCE_READ_CAPABILITY)
}

/// Builds the office evidence-write capability name.
#[must_use]
pub fn office_evidence_write_capability() -> CapabilityName {
    CapabilityName::new(OFFICE_EVIDENCE_WRITE_CAPABILITY)
}

/// A precise construction relation and its broad office evidence role.
///
/// The precise symbol remains the construction [`ProjectFact`](sim_lib_construction_project::ProjectFact)
/// kind. Only `office_role` is projected into the office evidence store.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceRelation {
    fact_kind: Symbol,
    office_role: LinkRole,
}

impl EvidenceRelation {
    /// Maps an open construction fact kind onto an existing office link role.
    #[must_use]
    pub fn new(fact_kind: Symbol, office_role: LinkRole) -> Self {
        Self {
            fact_kind,
            office_role,
        }
    }

    /// Returns the precise construction fact kind.
    #[must_use]
    pub fn fact_kind(&self) -> &Symbol {
        &self.fact_kind
    }

    /// Returns the broad role stored by the office evidence projection.
    #[must_use]
    pub fn office_role(&self) -> LinkRole {
        self.office_role
    }
}

/// Project-bound disclosure authority for evidence resolution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectEvidenceAccess {
    project: ProjectId,
    visible: Vec<Visibility>,
}

impl ProjectEvidenceAccess {
    /// Builds access to ordinary project-visible facts for one project.
    #[must_use]
    pub fn project(project: ProjectId) -> Self {
        Self {
            project,
            visible: vec![Visibility::Project],
        }
    }

    /// Adds one customer, supplier, reference, or restricted visibility grant.
    #[must_use]
    pub fn allow_visibility(mut self, visibility: Visibility) -> Self {
        if !self.visible.contains(&visibility) {
            self.visible.push(visibility);
        }
        self
    }

    /// Returns the project this access is confined to.
    #[must_use]
    pub fn project_id(&self) -> &ProjectId {
        &self.project
    }

    fn allows(&self, visibility: &Visibility) -> bool {
        self.visible.contains(visibility)
    }
}

/// Complete key for attaching a construction fact reference to an office document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceAttachment {
    /// Project that owns the construction fact.
    pub project: ProjectId,
    /// Construction control supported by the evidence.
    pub control: ControlId,
    /// Immutable project fact sequence carrying the precise relation and reference.
    pub fact_seq: u64,
    /// Existing office document receiving the evidence link.
    pub document: DocId,
    /// Exact reference already retained by the construction fact.
    pub external: ExternalRef,
    /// Precise construction relation and broad office projection role.
    pub relation: EvidenceRelation,
}

impl EvidenceAttachment {
    /// Builds a complete construction-to-office evidence join.
    #[must_use]
    pub fn new(
        project: ProjectId,
        control: ControlId,
        fact_seq: u64,
        document: DocId,
        external: ExternalRef,
        relation: EvidenceRelation,
    ) -> Self {
        Self {
            project,
            control,
            fact_seq,
            document,
            external,
            relation,
        }
    }
}

/// Result of an idempotent evidence attachment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttachOutcome {
    /// A new office projection row was attached.
    Attached,
    /// The identical office projection row already existed.
    Unchanged,
}

/// Resolved construction fact evidence joined to an office document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceLink {
    /// Project that owns the construction fact.
    pub project: ProjectId,
    /// Construction control supported by the evidence.
    pub control: ControlId,
    /// Current project fact sequence carrying the evidence.
    pub fact_seq: u64,
    /// Office document linked to the fact.
    pub document: DocId,
    /// Original reference retained by the construction fact.
    pub external: ExternalRef,
    /// Precise fact kind and broad office role.
    pub relation: EvidenceRelation,
    /// Construction acceptance state; the office link does not alter it.
    pub evidence_state: EvidenceState,
}

impl EvidenceLink {
    /// Returns whether the construction fact, rather than the office link, is accepted.
    #[must_use]
    pub fn is_accepted(&self) -> bool {
        self.evidence_state == EvidenceState::Accepted
    }
}

/// Idempotently attaches one construction fact reference to an office document.
///
/// The office row points at the immutable construction fact and evidence slot.
/// The original external reference and acceptance state remain on that fact.
pub fn attach_evidence(
    cx: &Cx,
    store: &DocStore,
    book: &ProjectBook,
    access: &ProjectEvidenceAccess,
    attachment: &EvidenceAttachment,
) -> Result<AttachOutcome, EvidenceBridgeError> {
    require_attach_capabilities(cx)?;
    validate_scope(book, access, &attachment.project)?;
    let fact = current_fact(book, attachment.fact_seq)?;
    validate_attachment(access, fact, attachment)?;
    require_document(store, &attachment.document)?;

    let evidence_index = fact
        .evidence
        .iter()
        .position(|reference| reference == &attachment.external)
        .ok_or(EvidenceBridgeError::ReferenceMissing { sequence: fact.seq })?;
    let office_row = Evidence::new(
        attachment.document.clone(),
        construction_fact_ref(fact, evidence_index),
        attachment.relation.office_role(),
        fact.seq,
        attachment.external.version.clone(),
    );
    let existing = office_rows(store, &attachment.document)?
        .into_iter()
        .any(|row| row == office_row);
    evidence::attach(store, &office_row).map_err(store_error)?;
    Ok(if existing {
        AttachOutcome::Unchanged
    } else {
        AttachOutcome::Attached
    })
}

/// Resolves current construction evidence for `control` across known documents.
///
/// Input document order and duplicates do not affect the result. Links are
/// ordered by fact sequence, document id, broad role, and external identity.
/// Superseded links and links for other projects or controls are not returned.
pub fn evidence_for_documents(
    cx: &Cx,
    store: &DocStore,
    book: &ProjectBook,
    access: &ProjectEvidenceAccess,
    control: &ControlId,
    documents: impl IntoIterator<Item = DocId>,
) -> Result<Vec<EvidenceLink>, EvidenceBridgeError> {
    require_query_capabilities(cx)?;
    validate_scope(book, access, book.project())?;

    let documents = documents.into_iter().collect::<BTreeSet<_>>();
    let mut links = Vec::new();
    for document in documents {
        require_document(store, &document)?;
        for office_row in office_rows(store, &document)? {
            let Some(pointer) = FactPointer::parse(&office_row.evidence)? else {
                continue;
            };
            if pointer.project != access.project.as_str() || pointer.control != control.as_str() {
                continue;
            }
            let Some(fact) = book.fact(pointer.sequence) else {
                return Err(EvidenceBridgeError::MissingFact {
                    sequence: pointer.sequence,
                });
            };
            if is_stale(book, fact.seq) {
                continue;
            }
            validate_resolved_fact(access, fact, control, &pointer)?;
            let external = fact
                .evidence
                .get(pointer.evidence_index)
                .cloned()
                .ok_or_else(|| {
                    EvidenceBridgeError::InvalidProjection(format!(
                        "fact {} has no evidence slot {}",
                        fact.seq, pointer.evidence_index
                    ))
                })?;
            if office_row.captured_at_seq != fact.seq {
                return Err(EvidenceBridgeError::InvalidProjection(format!(
                    "office sequence {} does not match fact {}",
                    office_row.captured_at_seq, fact.seq
                )));
            }
            links.push(EvidenceLink {
                project: fact.project.clone(),
                control: fact.subject.clone(),
                fact_seq: fact.seq,
                document: document.clone(),
                external,
                relation: EvidenceRelation::new(fact.kind.clone(), office_row.role),
                evidence_state: fact.evidence_state,
            });
        }
    }
    links.sort_by(compare_links);
    Ok(links)
}

/// Maps a precise design-source relation to the broad office source role.
#[must_use]
pub fn design_source_relation() -> EvidenceRelation {
    relation("design-source", LinkRole::SourceDocument)
}

/// Maps a precise schedule-basis relation to the broad office schedule role.
#[must_use]
pub fn schedule_basis_relation() -> EvidenceRelation {
    relation("schedule-basis", LinkRole::ScheduleReference)
}

/// Maps a precise construction field-issue relation to the broad office issue role.
#[must_use]
pub fn field_issue_relation() -> EvidenceRelation {
    relation("field-issue", LinkRole::ProjectIssue)
}

/// Maps precise commercial support to the broad office accounting role.
#[must_use]
pub fn commercial_support_relation() -> EvidenceRelation {
    relation("commercial-support", LinkRole::AccountingSupport)
}

/// Maps a precise published deliverable to the broad office publication role.
#[must_use]
pub fn published_deliverable_relation() -> EvidenceRelation {
    relation("published-deliverable", LinkRole::PublishedTo)
}

fn relation(name: &str, office_role: LinkRole) -> EvidenceRelation {
    EvidenceRelation::new(
        Symbol::qualified("construction-evidence".to_owned(), name.to_owned()),
        office_role,
    )
}

fn require_attach_capabilities(cx: &Cx) -> Result<(), EvidenceBridgeError> {
    cx.require_all(&[
        construction_project_read_capability(),
        office_evidence_read_capability(),
        office_evidence_write_capability(),
    ])?;
    Ok(())
}

fn require_query_capabilities(cx: &Cx) -> Result<(), EvidenceBridgeError> {
    cx.require_all(&[
        construction_project_read_capability(),
        office_evidence_read_capability(),
    ])?;
    Ok(())
}

fn validate_scope(
    book: &ProjectBook,
    access: &ProjectEvidenceAccess,
    project: &ProjectId,
) -> Result<(), EvidenceBridgeError> {
    for actual in [book.project(), project] {
        if actual != access.project_id() {
            return Err(EvidenceBridgeError::ProjectScopeMismatch {
                expected: access.project.clone(),
                actual: actual.clone(),
            });
        }
    }
    Ok(())
}

fn current_fact(book: &ProjectBook, sequence: u64) -> Result<&ProjectFact, EvidenceBridgeError> {
    let fact = book
        .fact(sequence)
        .ok_or(EvidenceBridgeError::MissingFact { sequence })?;
    if is_stale(book, sequence) {
        return Err(EvidenceBridgeError::StaleFact { sequence });
    }
    Ok(fact)
}

fn validate_attachment(
    access: &ProjectEvidenceAccess,
    fact: &ProjectFact,
    attachment: &EvidenceAttachment,
) -> Result<(), EvidenceBridgeError> {
    if fact.project != attachment.project {
        return Err(EvidenceBridgeError::ProjectScopeMismatch {
            expected: attachment.project.clone(),
            actual: fact.project.clone(),
        });
    }
    if fact.subject != attachment.control {
        return Err(EvidenceBridgeError::ControlMismatch {
            sequence: fact.seq,
            expected: attachment.control.clone(),
            actual: fact.subject.clone(),
        });
    }
    if fact.kind != *attachment.relation.fact_kind() {
        return Err(EvidenceBridgeError::RelationMismatch {
            sequence: fact.seq,
            expected: attachment.relation.fact_kind().as_qualified_str(),
            actual: fact.kind.as_qualified_str(),
        });
    }
    require_visibility(access, fact)
}

fn validate_resolved_fact(
    access: &ProjectEvidenceAccess,
    fact: &ProjectFact,
    control: &ControlId,
    pointer: &FactPointer<'_>,
) -> Result<(), EvidenceBridgeError> {
    if fact.project.as_str() != pointer.project {
        return Err(EvidenceBridgeError::InvalidProjection(format!(
            "fact {} belongs to project {}, not {}",
            fact.seq, fact.project, pointer.project
        )));
    }
    if &fact.subject != control || fact.subject.as_str() != pointer.control {
        return Err(EvidenceBridgeError::InvalidProjection(format!(
            "fact {} belongs to control {}, not {}",
            fact.seq, fact.subject, pointer.control
        )));
    }
    require_visibility(access, fact)
}

fn require_visibility(
    access: &ProjectEvidenceAccess,
    fact: &ProjectFact,
) -> Result<(), EvidenceBridgeError> {
    if access.allows(&fact.visibility) {
        Ok(())
    } else {
        Err(EvidenceBridgeError::VisibilityDenied {
            sequence: fact.seq,
            visibility: fact.visibility.label(),
        })
    }
}

fn require_document(store: &DocStore, document: &DocId) -> Result<(), EvidenceBridgeError> {
    if store.load_doc(document).map_err(store_error)?.is_some() {
        Ok(())
    } else {
        Err(EvidenceBridgeError::MissingDocument {
            document: document.as_str().to_owned(),
        })
    }
}

fn office_rows(store: &DocStore, document: &DocId) -> Result<Vec<Evidence>, EvidenceBridgeError> {
    evidence::evidence_for(store, document).map_err(store_error)
}

fn store_error(error: impl std::fmt::Display) -> EvidenceBridgeError {
    EvidenceBridgeError::Store(error.to_string())
}

fn is_stale(book: &ProjectBook, sequence: u64) -> bool {
    book.facts()
        .any(|candidate| candidate.supersedes == Some(sequence))
}

fn compare_links(left: &EvidenceLink, right: &EvidenceLink) -> Ordering {
    left.fact_seq
        .cmp(&right.fact_seq)
        .then_with(|| left.document.cmp(&right.document))
        .then_with(|| {
            left.relation
                .office_role()
                .cmp(&right.relation.office_role())
        })
        .then_with(|| left.external.backend.cmp(&right.external.backend))
        .then_with(|| left.external.external_id.cmp(&right.external.external_id))
        .then_with(|| left.external.version.cmp(&right.external.version))
        .then_with(|| left.external.web_url.cmp(&right.external.web_url))
}
