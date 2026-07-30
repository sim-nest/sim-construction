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
mod award;
mod baseline;
mod bid;
mod book;
mod change;
mod change_exposure;
mod change_settlement;
mod change_validation;
mod charter;
mod collaboration;
mod commercial;
mod commissioning;
mod control_graph;
mod decision;
mod design;
mod error;
mod escalation;
mod evidence_state;
mod exception;
mod exposure;
mod fact;
mod field_import;
mod field_item;
mod forecast;
mod gate;
mod governance;
mod handoff;
mod handover;
mod identity;
mod incident;
mod inspection;
mod intent;
mod lifecycle;
mod lookahead;
mod obligation;
mod observation;
mod opportunity;
mod outcome;
mod outcome_symbol;
mod permit;
mod policy;
mod procurement;
mod production_plan;
mod quality;
mod readiness;
mod release;
mod requirement;
mod review;
mod rfi;
mod risk;
mod schedule_join;
mod schedule_status;
mod snapshot;
mod supplier;
mod tender;
mod work_package;

pub use action::{ActionResolution, ActionState, ProjectAction};
pub use authority::{
    CONSTRUCTION_EXCEPTION_CAPABILITY, CONSTRUCTION_PROJECT_ACCEPT_CAPABILITY,
    CONSTRUCTION_PROJECT_READ_CAPABILITY, CONSTRUCTION_PROJECT_WRITE_CAPABILITY,
    CONSTRUCTION_REFERENCE_PUBLISH_CAPABILITY, CONSTRUCTION_SUPPLIER_READ_CAPABILITY,
    construction_exception_capability, construction_project_accept_capability,
    construction_project_read_capability, construction_project_write_capability,
    construction_reference_publish_capability, construction_supplier_read_capability,
};
pub use award::{AwardDecision, AwardDecisionKind};
pub use baseline::{AcceptedBaseline, BaselineKind};
pub use bid::{BidDecision, BidDecisionKind, OfferBasisReport};
pub use book::{DEFAULT_MAX_PROJECT_FACTS, ProjectBook};
pub use change::{
    ChangeDirection, ChangeFact, ChangeRecord, ChangeScheduleImpact, ChangeStage, ChangeStatus,
    ContractualBasis,
};
pub use change_exposure::{ChangeControlSet, ChangeExposureReport, ChangeExposureView};
pub use change_settlement::ChangeSettlementView;
pub use charter::{CurrencyCode, PROJECT_CHARTER_KIND, ProjectCharter, ReportingCadence};
pub use collaboration::{CollaborationCharter, CollaborationReadinessReport};
pub use commercial::{
    ChangeAmountComponent, CommercialEvidenceSource, CommercialSide, ReferencedAmount,
    ReferencedAmountEvidence,
};
pub use commissioning::{
    CommissioningControlSet, CommissioningRequirement, CommissioningRequirementKind,
};
pub use control_graph::{
    ControlEdge, ControlEdgeKind, ControlExplanationPath, ControlExplanationStep, ControlGraph,
    ControlGraphAnalysis, ControlGraphProjection, ControlNode, ControlNodeKind,
};
pub use decision::{DecisionResolution, DecisionState, ProjectDecision};
pub use design::{DesignBlocker, DesignControlSet, DesignReadinessReport, DesignRevision};
pub use error::{ConstructionProjectError, Result};
pub use escalation::{
    AttentionLevel, EscalationReason, EscalationRecommendation, derive_escalation_queue,
};
pub use evidence_state::{EvidenceState, EvidenceValidity};
pub use exception::{ExceptionDecision, ExceptionScope};
pub use exposure::{
    ExposureAnnotation, ExposureBucket, ExposureQueueItem, ExposureReport, derive_exposure,
};
pub use fact::{MAX_FACT_BODY_NODES, MAX_FACT_EVIDENCE_REFS, ProjectFact, expr_node_count};
pub use field_import::{
    EFFECT_LEDGER_BACKEND, FieldItemImport, FieldItemImportOutcome, import_field_item,
};
pub use field_item::{
    FieldItem, FieldItemKind, FieldItemReference, FieldItemState, FieldLane, FieldRollupEntry,
    FieldSeverity, safety_first_rollup,
};
pub use forecast::{ForecastBasis, ForecastConsequence, ForecastConsequenceKind, ForecastValue};
pub use gate::{GateDecision, GateDecisionKind, GateReport, GateRequirement, PhaseGate};
pub use governance::{
    DueDatePolicy, ProjectGovernance, RoleAssignment, Visibility, VisibilityPolicy,
};
pub use handoff::{
    HandoffReadinessReport, PackageHandoff, PackageHandoffBlocker, PackageHandoffControlSet,
};
pub use handover::{HandoverControlKind, HandoverHierarchy};
pub use identity::{BaselineId, ChangeId, ControlId, OrganizationId, ProjectId, RoleId};
pub use incident::{IncidentEscalation, ProjectIncident};
pub use inspection::{InspectionPoint, InspectionResult};
pub use intent::{
    ConstructionVariant, CustomerIntent, CustomerIntentAcceptance, IntentCoverageReport,
    IntentField,
};
pub use lifecycle::{LifecyclePolicy, PhaseOverlap, PhaseTransition, ProjectPhase};
pub use lookahead::{
    AcceptedTaskWindow, LookaheadWindow, ProductionActivity, ProductionActivityReadiness,
    ProductionCommitment, ProductionConstraint, ProductionReadinessSnapshot,
    ProductionReadinessState, ProductionTaskMovement,
};
pub use obligation::{ObligationPolicy, ProjectObligation};
pub use observation::ProjectObservation;
pub use opportunity::{OpportunityRecord, OpportunitySource};
pub use outcome::{
    DisclosureState, DomainQuantity, OutcomeBlocker, OutcomeBoundary, OutcomeControlReport,
    OutcomeMethod, OutcomeRecord, OutcomeRecordKind, OutcomeRecordSpec, OutcomeTargetKind,
    OutcomeTargetReport, OutcomeVariance, RegisteredOutcomeShape, SustainabilityTarget,
    SustainabilityTargetSpec, evaluate_outcomes,
};
pub use permit::{
    AuthorityObligation, AuthorityObligationState, InspectionRecord, InspectionState, PermitRecord,
    PermitState,
};
pub use policy::{GatePolicy, GatePolicyReport, RequirementExplanation};
pub use procurement::{
    AwardConsequence, InterfaceExposure, PackageReadinessReport, ProcurementComparison,
    ProcurementControlSet, ProcurementDateReport, ProcurementStatus, TenderEvaluation,
};
pub use production_plan::ProductionPlan;
pub use quality::{CorrectiveAction, Defect, QualityDeviation};
pub use readiness::{CharterReadiness, evaluate_charter};
pub use release::{DesignRelease, DesignReleasePurpose};
pub use requirement::{Requirement, RequirementLane};
pub use review::{DesignReview, DesignReviewState};
pub use rfi::{RfiRecord, RfiState};
pub use risk::{
    OpenRating, RatingValue, ResponseState, UncertaintyKind, UncertaintyRecord,
    UncertaintyResponse, UncertaintyState,
};
pub use schedule_join::{
    ScheduleBaseline, ScheduleJoinKind, SchedulePlanRevision, ScheduleTaskJoin, ScheduleTaskJoinSet,
};
pub use schedule_status::{
    ScheduleControlState, ScheduleExplanationKind, ScheduleImpactExplanation, ScheduleStatusReport,
    explain_schedule_impact,
};
pub use snapshot::{
    ProjectDelta, ProjectSnapshot, ProjectSnapshotExplanation, SnapshotExplanationKind,
    snapshot_at, snapshot_delta,
};
pub use supplier::{
    QualificationEvidence, QualificationRequirement, QualificationStatus,
    SupplierQualificationArea, SupplierQualificationReport, SupplierQualificationSet,
    SupplierReference,
};
pub use tender::{ScopeCompliance, TenderComparison, TenderQualification};
pub use work_package::{CommercialAmount, SupplierCandidate, WorkPackage};

/// Cookbook recipes for this lib, embedded at build time.
pub static RECIPES: sim_cookbook::EmbeddedDir =
    include!(concat!(env!("OUT_DIR"), "/cookbook_recipes.rs"));

#[cfg(test)]
mod change_test_chain;
#[cfg(test)]
mod change_tests;
#[cfg(test)]
mod control_graph_tests;
#[cfg(test)]
mod design_tests;
#[cfg(test)]
mod escalation_tests;
#[cfg(test)]
mod exposure_tests;
#[cfg(test)]
mod fact_book_tests;
#[cfg(test)]
mod field_control_tests;
#[cfg(test)]
mod handover_tests;
#[cfg(test)]
mod lifecycle_tests;
#[cfg(test)]
mod obligation_tests;
#[cfg(test)]
mod opportunity_tests;
#[cfg(test)]
mod outcome_tests;
#[cfg(test)]
mod procurement_tests;
#[cfg(test)]
mod production_plan_tests;
#[cfg(test)]
mod risk_tests;
#[cfg(test)]
mod schedule_tests;
#[cfg(test)]
mod supplier_tests;
#[cfg(test)]
mod tests;
