//! Reference-only office evidence composition for construction control.
//!
//! The bridge joins construction facts to existing office documents without
//! owning document bytes, an evidence store, or construction acceptance state.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod error;
mod evidence;
mod pointer;

pub use error::EvidenceBridgeError;
pub use evidence::{
    AttachOutcome, EvidenceAttachment, EvidenceLink, EvidenceRelation,
    OFFICE_EVIDENCE_READ_CAPABILITY, OFFICE_EVIDENCE_WRITE_CAPABILITY, ProjectEvidenceAccess,
    attach_evidence, commercial_support_relation, design_source_relation, evidence_for_documents,
    field_issue_relation, office_evidence_read_capability, office_evidence_write_capability,
    published_deliverable_relation, schedule_basis_relation,
};

/// Cookbook recipes for this bridge, embedded at build time.
pub static RECIPES: sim_cookbook::EmbeddedDir =
    include!(concat!(env!("OUT_DIR"), "/cookbook_recipes.rs"));

#[cfg(test)]
mod tests;
