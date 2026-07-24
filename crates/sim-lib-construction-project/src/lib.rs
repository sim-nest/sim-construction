//! Construction project-control records for SIM.
//!
//! This crate owns construction project identity, charter, governance,
//! capability, reference-only evidence, and deterministic readiness records. It
//! stores stable role and organization references rather than personnel
//! profiles, and it composes document evidence references from
//! `sim-lib-doc-core` instead of storing external project content.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod authority;
mod charter;
mod error;
mod governance;
mod identity;
mod readiness;

pub use authority::{
    CONSTRUCTION_EXCEPTION_CAPABILITY, CONSTRUCTION_PROJECT_ACCEPT_CAPABILITY,
    CONSTRUCTION_PROJECT_READ_CAPABILITY, CONSTRUCTION_PROJECT_WRITE_CAPABILITY,
    CONSTRUCTION_REFERENCE_PUBLISH_CAPABILITY, construction_exception_capability,
    construction_project_accept_capability, construction_project_read_capability,
    construction_project_write_capability, construction_reference_publish_capability,
};
pub use charter::{CurrencyCode, PROJECT_CHARTER_KIND, ProjectCharter, ReportingCadence};
pub use error::{ConstructionProjectError, Result};
pub use governance::{
    DueDatePolicy, ProjectGovernance, RoleAssignment, Visibility, VisibilityPolicy,
};
pub use identity::{BaselineId, ControlId, OrganizationId, ProjectId, RoleId};
pub use readiness::{CharterReadiness, EvidenceState, evaluate_charter};

/// Cookbook recipes for this lib, embedded at build time.
pub static RECIPES: sim_cookbook::EmbeddedDir =
    include!(concat!(env!("OUT_DIR"), "/cookbook_recipes.rs"));

#[cfg(test)]
mod tests;
