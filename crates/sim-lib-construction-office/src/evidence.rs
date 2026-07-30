//! Construction-to-office evidence composition.

use sim_kernel::Symbol;
use sim_lib_doc_core::LinkRole;

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
