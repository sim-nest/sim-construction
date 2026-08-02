//! Capability-derived visibility filtering before office projection.

use sim_kernel::{CapabilityName, Cx};
use sim_lib_construction_project::{
    ProjectBook, ProjectFact, Visibility, construction_project_read_capability,
    construction_supplier_read_capability,
};

use crate::OfficePackError;

/// Stable policy marker embedded in every generated pack.
pub const OFFICE_PACK_VISIBILITY_POLICY: &str = "construction-office/cx-capability-filter-v1";
/// Capability that admits customer-visible facts into an office pack.
pub const OFFICE_CUSTOMER_READ_CAPABILITY: &str = "construction.office.customer.read";
/// Capability that admits reference-candidate facts into an office pack.
pub const OFFICE_REFERENCE_READ_CAPABILITY: &str = "construction.office.reference.read";

const RESTRICTED_PREFIX: &str = "construction.office.restricted.";

/// Returns the capability required for customer-visible pack facts.
#[must_use]
pub fn office_customer_read_capability() -> CapabilityName {
    CapabilityName::new(OFFICE_CUSTOMER_READ_CAPABILITY)
}

/// Returns the capability required for reference-candidate pack facts.
#[must_use]
pub fn office_reference_read_capability() -> CapabilityName {
    CapabilityName::new(OFFICE_REFERENCE_READ_CAPABILITY)
}

/// Returns the exact capability required for one restricted visibility symbol.
#[must_use]
pub fn office_restricted_read_capability(visibility: &sim_kernel::Symbol) -> CapabilityName {
    CapabilityName::new(format!(
        "{RESTRICTED_PREFIX}{}",
        visibility.as_qualified_str()
    ))
}

pub(crate) fn visible_book(
    cx: &Cx,
    book: &ProjectBook,
    through: u64,
) -> Result<ProjectBook, OfficePackError> {
    cx.require(&construction_project_read_capability())?;
    let visible_sequences = book
        .facts()
        .filter(|fact| fact.seq <= through && visible(cx, &fact.visibility))
        .map(|fact| fact.seq)
        .collect::<std::collections::BTreeSet<_>>();
    let facts = book
        .facts()
        .filter(|fact| visible_sequences.contains(&fact.seq))
        .cloned()
        .map(|mut fact| {
            if fact
                .supersedes
                .is_some_and(|prior| !visible_sequences.contains(&prior))
            {
                fact.supersedes = None;
            }
            fact
        })
        .collect::<Vec<ProjectFact>>();
    ProjectBook::from_facts(
        book.project().clone(),
        book.authoritative_writer().clone(),
        facts,
    )
    .map_err(OfficePackError::from)
}

fn visible(cx: &Cx, visibility: &Visibility) -> bool {
    let required = match visibility {
        Visibility::Project => return true,
        Visibility::Customer => office_customer_read_capability(),
        Visibility::Supplier => construction_supplier_read_capability(),
        Visibility::ReferenceCandidate => office_reference_read_capability(),
        Visibility::Restricted(symbol) => office_restricted_read_capability(symbol),
    };
    cx.capabilities().contains(&required)
}
