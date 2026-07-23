//! Construction project-control records for SIM.
//!
//! This crate owns the narrow construction charter vocabulary: stable project
//! identities, accountable roles, reference-only evidence, and deterministic
//! readiness summaries. It composes document evidence references from
//! `sim-lib-doc-core` instead of storing external project content.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod model;
mod readiness;

pub use model::{
    PROJECT_CHARTER_KIND, PROJECT_CONTROL_CAPABILITY, ProjectCharter, ProjectId, RoleId,
};
pub use readiness::{CharterReadiness, EvidenceState, evaluate_charter};

/// Cookbook recipes for this lib, embedded at build time.
pub static RECIPES: sim_cookbook::EmbeddedDir =
    include!(concat!(env!("OUT_DIR"), "/cookbook_recipes.rs"));

#[cfg(test)]
mod tests;
