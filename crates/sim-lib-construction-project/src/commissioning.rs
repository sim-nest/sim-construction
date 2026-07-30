//! Evidence requirements for commissioning and handover.

use std::collections::BTreeSet;

use crate::{
    ConstructionProjectError, ControlEdgeKind, ControlGraph, ControlId, ControlNodeKind,
    HandoverHierarchy, ObligationPolicy, ProjectId, ProjectObligation, Result,
};

/// Kind of leaf requirement controlled through commissioning and handover.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum CommissioningRequirementKind {
    /// Planned commissioning activity or functional sequence.
    Activity,
    /// Inspection point.
    Inspection,
    /// Test point, including a superseding retest.
    Test,
    /// Defect that must be closed or accountably excepted.
    Defect,
    /// Operations and maintenance deliverable retained by an external store.
    OperationsMaintenanceDeliverable,
    /// As-built deliverable retained by an external store.
    AsBuiltDeliverable,
    /// Operator, maintainer, or customer training.
    Training,
    /// Certificate with an evidence validity window.
    Certification,
    /// Closure controlled by a public or delegated authority.
    AuthorityClosure,
    /// Customer acceptance evidence.
    CustomerAcceptance,
    /// Explicit remaining-work item.
    RemainingWork,
}

/// One typed commissioning requirement over the shared obligation shape.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CommissioningRequirement {
    /// Commissioning or handover kind.
    pub kind: CommissioningRequirementKind,
    /// Shared project obligation, including requirement, evidence, and policy.
    pub obligation: ProjectObligation,
    /// Leaf or aggregate handover controls to which this requirement applies.
    pub targets: Vec<ControlId>,
    /// True when failure has a critical consequence.
    pub critical: bool,
}

impl CommissioningRequirement {
    /// Builds a typed requirement for one handover control.
    ///
    /// Authority closure is always made non-waivable. All other policy remains
    /// explicit on the supplied common obligation.
    #[must_use]
    pub fn new(
        kind: CommissioningRequirementKind,
        mut obligation: ProjectObligation,
        target: ControlId,
    ) -> Self {
        if kind == CommissioningRequirementKind::AuthorityClosure {
            obligation.requirement.non_waivable = true;
            obligation.policy = ObligationPolicy::Mandatory;
        }
        Self {
            kind,
            obligation,
            targets: vec![target],
            critical: false,
        }
    }

    /// Adds another handover control affected by the same requirement.
    #[must_use]
    pub fn with_target(mut self, target: ControlId) -> Self {
        self.targets.push(target);
        self
    }

    /// Marks the requirement as critical.
    #[must_use]
    pub fn critical(mut self) -> Self {
        self.critical = true;
        self
    }

    /// Returns the stable common requirement id.
    #[must_use]
    pub fn id(&self) -> &ControlId {
        &self.obligation.requirement.id
    }

    /// Validates commissioning-specific evidence and target rules.
    pub fn validate(&self, project: &ProjectId, hierarchy: &HandoverHierarchy) -> Result<()> {
        if &self.obligation.project != project {
            return Err(ConstructionProjectError::ProjectMismatch {
                expected: project.clone(),
                actual: self.obligation.project.clone(),
            });
        }
        self.obligation.requirement.validate()?;
        if !self.obligation.requirement.evidence_required {
            return Err(ConstructionProjectError::EmptyCollection(
                "commissioning_requirement.required_evidence",
            ));
        }
        if self.obligation.requirement.evidence_kinds.is_empty() {
            return Err(ConstructionProjectError::EmptyCollection(
                "commissioning_requirement.evidence_kinds",
            ));
        }
        if self.targets.is_empty() {
            return Err(ConstructionProjectError::EmptyCollection(
                "commissioning_requirement.targets",
            ));
        }
        let mut seen = BTreeSet::new();
        for target in &self.targets {
            if hierarchy.kind(target).is_none() {
                return Err(ConstructionProjectError::ControlGraphMissingEndpoint {
                    edge: "commissioning-target",
                    endpoint_role: "target",
                    endpoint: target.clone(),
                });
            }
            if !seen.insert(target) {
                return Err(ConstructionProjectError::DuplicateId {
                    kind: "commissioning_requirement.target",
                    id: target.to_string(),
                });
            }
        }
        if self.kind == CommissioningRequirementKind::AuthorityClosure
            && (!self.obligation.requirement.non_waivable
                || self.obligation.policy != ObligationPolicy::Mandatory)
        {
            return Err(
                ConstructionProjectError::NonWaivableCommissioningRequirement {
                    requirement: self.id().clone(),
                },
            );
        }
        Ok(())
    }
}

/// Typed commissioning requirements projected onto one handover hierarchy.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CommissioningControlSet {
    /// Stable project identity.
    pub project: ProjectId,
    /// Typed commissioning requirements.
    pub requirements: Vec<CommissioningRequirement>,
}

impl CommissioningControlSet {
    /// Starts an empty commissioning control set.
    #[must_use]
    pub fn new(project: ProjectId) -> Self {
        Self {
            project,
            requirements: Vec::new(),
        }
    }

    /// Adds one typed requirement.
    #[must_use]
    pub fn with_requirement(mut self, requirement: CommissioningRequirement) -> Self {
        self.requirements.push(requirement);
        self
    }

    /// Validates ids, evidence shapes, targets, and dependency endpoints.
    pub fn validate(&self, hierarchy: &HandoverHierarchy) -> Result<()> {
        if hierarchy.project() != &self.project {
            return Err(ConstructionProjectError::ProjectMismatch {
                expected: self.project.clone(),
                actual: hierarchy.project().clone(),
            });
        }
        if self.requirements.is_empty() {
            return Err(ConstructionProjectError::EmptyCollection(
                "commissioning.requirements",
            ));
        }
        let mut seen = BTreeSet::new();
        for requirement in &self.requirements {
            requirement.validate(&self.project, hierarchy)?;
            if !seen.insert(requirement.id()) {
                return Err(ConstructionProjectError::DuplicateId {
                    kind: "commissioning_requirement",
                    id: requirement.id().to_string(),
                });
            }
        }
        self.control_graph(hierarchy)?.validate_readiness()
    }

    /// Projects requirements, dependencies, targets, and membership onto the
    /// common construction control graph.
    pub fn control_graph(&self, hierarchy: &HandoverHierarchy) -> Result<ControlGraph> {
        let mut graph = hierarchy.control_graph().clone();
        for requirement in &self.requirements {
            graph.add_node(requirement.id().clone(), ControlNodeKind::Requirement)?;
        }
        for requirement in &self.requirements {
            for dependency in &requirement.obligation.requirement.dependencies {
                graph.add_edge(
                    dependency.clone(),
                    requirement.id().clone(),
                    ControlEdgeKind::Prerequisite,
                )?;
            }
            for target in &requirement.targets {
                graph.add_edge(
                    requirement.id().clone(),
                    target.clone(),
                    ControlEdgeKind::Prerequisite,
                )?;
            }
        }
        Ok(graph)
    }
}
