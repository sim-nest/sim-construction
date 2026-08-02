//! System, area, package, asset-group, and milestone handover hierarchy.

use std::collections::BTreeSet;

use crate::{
    ConstructionProjectError, ControlEdgeKind, ControlGraph, ControlId, ControlNodeKind, ProjectId,
    Result,
};

/// Kind of stable control participating in a handover hierarchy.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum HandoverControlKind {
    /// Functional system, including a system spanning several areas.
    System,
    /// Geographic or functional handover area.
    Area,
    /// Contracted work package.
    WorkPackage,
    /// Group of assets commissioned and handed over together.
    AssetGroup,
    /// Contractual milestone collecting handover scope.
    ContractualMilestone,
}

impl HandoverControlKind {
    fn node_kind(self) -> ControlNodeKind {
        match self {
            Self::System => ControlNodeKind::HandoverSystem,
            Self::Area => ControlNodeKind::HandoverArea,
            Self::WorkPackage => ControlNodeKind::HandoverWorkPackage,
            Self::AssetGroup => ControlNodeKind::HandoverAssetGroup,
            Self::ContractualMilestone => ControlNodeKind::HandoverMilestone,
        }
    }

    fn from_node_kind(kind: ControlNodeKind) -> Option<Self> {
        match kind {
            ControlNodeKind::HandoverSystem => Some(Self::System),
            ControlNodeKind::HandoverArea => Some(Self::Area),
            ControlNodeKind::HandoverWorkPackage => Some(Self::WorkPackage),
            ControlNodeKind::HandoverAssetGroup => Some(Self::AssetGroup),
            ControlNodeKind::HandoverMilestone => Some(Self::ContractualMilestone),
            _ => None,
        }
    }
}

/// Handover scope backed by the common construction control graph.
///
/// Membership is a directed acyclic graph rather than a tree: one system can
/// belong to several areas, packages can cross systems, and a contractual
/// milestone can collect either. Member edges point from member to parent.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HandoverHierarchy {
    project: ProjectId,
    graph: ControlGraph,
}

impl HandoverHierarchy {
    /// Starts an empty handover hierarchy for a project.
    #[must_use]
    pub fn new(project: ProjectId) -> Self {
        Self {
            project,
            graph: ControlGraph::new(),
        }
    }

    /// Returns the project that owns this hierarchy.
    #[must_use]
    pub fn project(&self) -> &ProjectId {
        &self.project
    }

    /// Borrows the canonical construction control graph.
    #[must_use]
    pub fn control_graph(&self) -> &ControlGraph {
        &self.graph
    }

    /// Adds or confirms a stable, typed handover control.
    pub fn add_control(&mut self, id: ControlId, kind: HandoverControlKind) -> Result<()> {
        self.graph.add_node(id, kind.node_kind())
    }

    /// Adds a typed member-to-parent edge and rejects hierarchy cycles.
    pub fn add_member(&mut self, member: ControlId, parent: ControlId) -> Result<()> {
        self.require_handover_control(&member, "member")?;
        self.require_handover_control(&parent, "parent")?;

        let mut candidate = self.graph.clone();
        candidate.add_edge(member, parent, ControlEdgeKind::MemberOf)?;
        candidate.validate_readiness()?;
        self.graph = candidate;
        Ok(())
    }

    /// Returns the declared kind of a handover control.
    #[must_use]
    pub fn kind(&self, control: &ControlId) -> Option<HandoverControlKind> {
        self.graph
            .nodes
            .get(control)
            .and_then(|kind| HandoverControlKind::from_node_kind(*kind))
    }

    /// Returns direct members in stable control-id order.
    #[must_use]
    pub fn direct_members(&self, parent: &ControlId) -> Vec<ControlId> {
        self.graph
            .edges
            .iter()
            .filter(|edge| edge.kind == ControlEdgeKind::MemberOf && &edge.target == parent)
            .map(|edge| edge.source.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    /// Returns direct parents in stable control-id order.
    #[must_use]
    pub fn direct_parents(&self, member: &ControlId) -> Vec<ControlId> {
        self.graph
            .edges
            .iter()
            .filter(|edge| edge.kind == ControlEdgeKind::MemberOf && &edge.source == member)
            .map(|edge| edge.target.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    /// Returns a control and all transitive members in stable order.
    pub fn scope(&self, root: &ControlId) -> Result<Vec<ControlId>> {
        self.require_handover_control(root, "scope")?;
        let mut scope = BTreeSet::from([root.clone()]);
        let mut frontier = vec![root.clone()];
        while let Some(parent) = frontier.pop() {
            for member in self.direct_members(&parent) {
                if scope.insert(member.clone()) {
                    frontier.push(member);
                }
            }
        }
        Ok(scope.into_iter().collect())
    }

    /// Returns controls with no declared members inside the selected scope.
    pub fn leaves(&self, root: &ControlId) -> Result<Vec<ControlId>> {
        Ok(self
            .scope(root)?
            .into_iter()
            .filter(|control| self.direct_members(control).is_empty())
            .collect())
    }

    fn require_handover_control(
        &self,
        control: &ControlId,
        endpoint_role: &'static str,
    ) -> Result<()> {
        if self.kind(control).is_none() {
            return Err(ConstructionProjectError::ControlGraphMissingEndpoint {
                edge: "handover-member",
                endpoint_role,
                endpoint: control.clone(),
            });
        }
        Ok(())
    }
}
