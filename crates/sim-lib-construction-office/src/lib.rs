//! Reference-only office evidence composition for construction control.
//!
//! The bridge joins construction facts to existing office documents without
//! owning document bytes, an evidence store, or construction acceptance state.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod evidence;

pub use evidence::{
    EvidenceRelation, commercial_support_relation, design_source_relation, field_issue_relation,
    published_deliverable_relation, schedule_basis_relation,
};

#[cfg(test)]
mod tests;
