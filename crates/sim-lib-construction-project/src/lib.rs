//! Construction project-control records for SIM.
//!
//! This crate owns construction project identity, charter, governance,
//! append-only fact books, deterministic as-of snapshots, capability,
//! reference-only evidence, and readiness records. It stores stable role and
//! organization references rather than personnel profiles, and it composes
//! document evidence references from `sim-lib-doc-core` instead of storing
//! external project content.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod authority;
mod book;
mod charter;
mod error;
mod fact;
mod governance;
mod identity;
mod readiness;
mod snapshot;

pub use authority::{
    CONSTRUCTION_EXCEPTION_CAPABILITY, CONSTRUCTION_PROJECT_ACCEPT_CAPABILITY,
    CONSTRUCTION_PROJECT_READ_CAPABILITY, CONSTRUCTION_PROJECT_WRITE_CAPABILITY,
    CONSTRUCTION_REFERENCE_PUBLISH_CAPABILITY, construction_exception_capability,
    construction_project_accept_capability, construction_project_read_capability,
    construction_project_write_capability, construction_reference_publish_capability,
};
pub use book::{DEFAULT_MAX_PROJECT_FACTS, ProjectBook};
pub use charter::{CurrencyCode, PROJECT_CHARTER_KIND, ProjectCharter, ReportingCadence};
pub use error::{ConstructionProjectError, Result};
pub use fact::{MAX_FACT_BODY_NODES, MAX_FACT_EVIDENCE_REFS, ProjectFact, expr_node_count};
pub use governance::{
    DueDatePolicy, ProjectGovernance, RoleAssignment, Visibility, VisibilityPolicy,
};
pub use identity::{BaselineId, ControlId, OrganizationId, ProjectId, RoleId};
pub use readiness::{CharterReadiness, EvidenceState, evaluate_charter};
pub use snapshot::{
    ProjectDelta, ProjectSnapshot, ProjectSnapshotExplanation, SnapshotExplanationKind,
    snapshot_at, snapshot_delta,
};

/// Cookbook recipes for this lib, embedded at build time.
pub static RECIPES: sim_cookbook::EmbeddedDir =
    include!(concat!(env!("OUT_DIR"), "/cookbook_recipes.rs"));

#[cfg(test)]
mod fact_book_tests;
#[cfg(test)]
mod tests;
