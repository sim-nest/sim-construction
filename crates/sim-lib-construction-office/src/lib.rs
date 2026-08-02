//! Visibility-safe office composition for construction control.
//!
//! The bridge joins construction facts to existing office documents without
//! owning document bytes, an evidence store, or construction acceptance state.
//! Role-cadence packs project filtered snapshots into the existing office
//! `Doc`, `Sheet`, and `Deck` values and present them through the suite surface.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod cadence;
mod error;
mod evidence;
mod pointer;
mod projection;
mod visibility;

pub use cadence::{OfficePackRequest, PackCadence, PackControl, PackSection};
pub use error::{EvidenceBridgeError, OfficePackError};
pub use evidence::{
    AttachOutcome, EvidenceAttachment, EvidenceLink, EvidenceRelation,
    OFFICE_EVIDENCE_READ_CAPABILITY, OFFICE_EVIDENCE_WRITE_CAPABILITY, ProjectEvidenceAccess,
    attach_evidence, commercial_support_relation, design_source_relation, evidence_for_documents,
    field_issue_relation, office_evidence_read_capability, office_evidence_write_capability,
    published_deliverable_relation, schedule_basis_relation,
};
pub use projection::{OfficePack, project_office_pack};
pub use visibility::{
    OFFICE_CUSTOMER_READ_CAPABILITY, OFFICE_PACK_VISIBILITY_POLICY,
    OFFICE_REFERENCE_READ_CAPABILITY, office_customer_read_capability,
    office_reference_read_capability, office_restricted_read_capability,
};

/// Cookbook recipes for this bridge, embedded at build time.
pub static RECIPES: sim_cookbook::EmbeddedDir =
    include!(concat!(env!("OUT_DIR"), "/cookbook_recipes.rs"));

#[cfg(test)]
mod tests;
