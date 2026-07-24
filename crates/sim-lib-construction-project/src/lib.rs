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

mod action;
mod authority;
mod baseline;
mod book;
mod charter;
mod control_graph;
mod decision;
mod error;
mod evidence_state;
mod exception;
mod fact;
mod gate;
mod governance;
mod identity;
mod lifecycle;
mod obligation;
mod policy;
mod readiness;
mod requirement;
mod snapshot;

pub use action::{ActionResolution, ActionState, ProjectAction};
pub use authority::{
    CONSTRUCTION_EXCEPTION_CAPABILITY, CONSTRUCTION_PROJECT_ACCEPT_CAPABILITY,
    CONSTRUCTION_PROJECT_READ_CAPABILITY, CONSTRUCTION_PROJECT_WRITE_CAPABILITY,
    CONSTRUCTION_REFERENCE_PUBLISH_CAPABILITY, construction_exception_capability,
    construction_project_accept_capability, construction_project_read_capability,
    construction_project_write_capability, construction_reference_publish_capability,
};
pub use baseline::{AcceptedBaseline, BaselineKind};
pub use book::{DEFAULT_MAX_PROJECT_FACTS, ProjectBook};
pub use charter::{CurrencyCode, PROJECT_CHARTER_KIND, ProjectCharter, ReportingCadence};
pub use control_graph::{
    ControlEdge, ControlEdgeKind, ControlExplanationPath, ControlExplanationStep, ControlGraph,
    ControlGraphAnalysis, ControlGraphProjection, ControlNode, ControlNodeKind,
};
pub use decision::{DecisionResolution, DecisionState, ProjectDecision};
pub use error::{ConstructionProjectError, Result};
pub use evidence_state::{EvidenceState, EvidenceValidity};
pub use exception::{ExceptionDecision, ExceptionScope};
pub use fact::{MAX_FACT_BODY_NODES, MAX_FACT_EVIDENCE_REFS, ProjectFact, expr_node_count};
pub use gate::{GateDecision, GateDecisionKind, GateReport, GateRequirement, PhaseGate};
pub use governance::{
    DueDatePolicy, ProjectGovernance, RoleAssignment, Visibility, VisibilityPolicy,
};
pub use identity::{BaselineId, ControlId, OrganizationId, ProjectId, RoleId};
pub use lifecycle::{LifecyclePolicy, PhaseOverlap, PhaseTransition, ProjectPhase};
pub use obligation::{ObligationPolicy, ProjectObligation};
pub use policy::{GatePolicy, GatePolicyReport, RequirementExplanation};
pub use readiness::{CharterReadiness, evaluate_charter};
pub use requirement::{Requirement, RequirementLane};
pub use snapshot::{
    ProjectDelta, ProjectSnapshot, ProjectSnapshotExplanation, SnapshotExplanationKind,
    snapshot_at, snapshot_delta,
};

/// Cookbook recipes for this lib, embedded at build time.
pub static RECIPES: sim_cookbook::EmbeddedDir =
    include!(concat!(env!("OUT_DIR"), "/cookbook_recipes.rs"));

#[cfg(test)]
mod control_graph_tests;
#[cfg(test)]
mod fact_book_tests;
#[cfg(test)]
mod lifecycle_tests;
#[cfg(test)]
mod obligation_tests;
#[cfg(test)]
mod tests;
