//! Construction control graph projected onto the canonical discrete graph engine.

use std::collections::{BTreeMap, BTreeSet};

use sim_lib_discrete_graph::{
    Directedness, Graph, dijkstra, reachability, strongly_connected_components,
};

use crate::{ConstructionProjectError, ControlId, EvidenceState, Result};

/// Typed construction-control node kinds.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum ControlNodeKind {
    /// Requirement that may be satisfied by accepted evidence.
    Requirement,
    /// Project-scoped obligation derived from a requirement.
    Obligation,
    /// Accountable decision.
    Decision,
    /// Package or work package.
    Package,
    /// Task join or schedule merge point.
    TaskJoin,
    /// Change record.
    Change,
    /// Handover item.
    HandoverItem,
    /// Outcome or delivered result.
    Outcome,
    /// Readiness or lifecycle gate.
    Gate,
    /// Design deliverable revision.
    DesignRevision,
    /// Design review.
    DesignReview,
    /// Request for information.
    Rfi,
    /// Purpose-specific design release.
    DesignRelease,
    /// Authority permit.
    Permit,
    /// Authority inspection.
    Inspection,
    /// Authority obligation.
    AuthorityObligation,
}

/// Stable construction-control graph node.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ControlNode {
    /// Stable control id.
    pub id: ControlId,
    /// Construction-owned node kind.
    pub kind: ControlNodeKind,
}

impl ControlNode {
    /// Builds a typed control node.
    #[must_use]
    pub fn new(id: ControlId, kind: ControlNodeKind) -> Self {
        Self { id, kind }
    }
}

/// Typed construction-control edge kinds.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum ControlEdgeKind {
    /// Source must be ready before target can be ready.
    Prerequisite,
    /// Source blocks target.
    Blocks,
    /// Source affects target but does not itself impose readiness.
    Affects,
    /// Source records a decision over target.
    Decides,
    /// Source package joins target task.
    Joins,
    /// Source changes target.
    Changes,
    /// Source hands over target.
    HandsOver,
    /// Source produces target.
    Produces,
    /// Informational relationship excluded from readiness propagation.
    Informational,
}

impl ControlEdgeKind {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Prerequisite => "prerequisite",
            Self::Blocks => "blocks",
            Self::Affects => "affects",
            Self::Decides => "decides",
            Self::Joins => "joins",
            Self::Changes => "changes",
            Self::HandsOver => "hands-over",
            Self::Produces => "produces",
            Self::Informational => "informational",
        }
    }

    fn propagates_readiness(self) -> bool {
        !matches!(self, Self::Informational)
    }
}

/// Stable construction-control graph edge.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ControlEdge {
    /// Source control id.
    pub source: ControlId,
    /// Target control id.
    pub target: ControlId,
    /// Construction-owned edge kind.
    pub kind: ControlEdgeKind,
}

impl ControlEdge {
    /// Builds a typed control edge.
    #[must_use]
    pub fn new(source: ControlId, target: ControlId, kind: ControlEdgeKind) -> Self {
        Self {
            source,
            target,
            kind,
        }
    }
}

/// Lowered graph with stable mappings to the canonical discrete graph.
#[derive(Clone, Debug, PartialEq)]
pub struct ControlGraphProjection {
    /// Canonical directed graph. Node ids are stable sorted positions.
    pub graph: Graph<ControlNode, ControlEdgeKind>,
    /// Stable control id to canonical node index.
    pub indices: BTreeMap<ControlId, usize>,
}

/// One evidence-aware explanation step along a control path.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ControlExplanationStep {
    /// Control id reached by this step.
    pub control: ControlId,
    /// Current fact sequence at the evaluated snapshot.
    pub current_seq: Option<u64>,
    /// Evidence state used for this control.
    pub evidence_state: EvidenceState,
    /// Edge kind used to enter this control; absent on the path root.
    pub edge_kind: Option<ControlEdgeKind>,
    /// Applied exception, when one covers this control.
    pub exception: Option<ControlId>,
}

/// Stable explanation path through the control graph.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ControlExplanationPath {
    /// Blocking source control.
    pub blocker: ControlId,
    /// Target control being explained.
    pub target: ControlId,
    /// Path steps from blocker to target.
    pub steps: Vec<ControlExplanationStep>,
}

/// Derived graph analysis for one target control.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ControlGraphAnalysis {
    /// Unsatisfied prerequisites that can reach the target.
    pub transitive_blockers: Vec<ControlId>,
    /// Controls downstream from the target.
    pub affected_dependents: Vec<ControlId>,
    /// Minimal unsatisfied prerequisite frontier.
    pub critical_prerequisite_cut: Vec<ControlId>,
    /// Stable explanation paths from blockers to the target.
    pub explanation_paths: Vec<ControlExplanationPath>,
}

/// Construction-owned typed control graph.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ControlGraph {
    /// Typed nodes keyed by stable id.
    pub nodes: BTreeMap<ControlId, ControlNodeKind>,
    /// Typed directed edges.
    pub edges: Vec<ControlEdge>,
}

impl ControlGraph {
    /// Builds an empty control graph.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds or confirms a typed node.
    pub fn add_node(&mut self, id: ControlId, kind: ControlNodeKind) -> Result<()> {
        if let Some(existing) = self.nodes.get(&id) {
            if *existing != kind {
                return Err(ConstructionProjectError::DuplicateId {
                    kind: "control_graph.node",
                    id: id.to_string(),
                });
            }
            return Ok(());
        }
        self.nodes.insert(id, kind);
        Ok(())
    }

    /// Adds a typed edge after validating both endpoints.
    pub fn add_edge(
        &mut self,
        source: ControlId,
        target: ControlId,
        kind: ControlEdgeKind,
    ) -> Result<()> {
        if !self.nodes.contains_key(&source) {
            return Err(ConstructionProjectError::ControlGraphMissingEndpoint {
                edge: kind.label(),
                endpoint_role: "source",
                endpoint: source,
            });
        }
        if !self.nodes.contains_key(&target) {
            return Err(ConstructionProjectError::ControlGraphMissingEndpoint {
                edge: kind.label(),
                endpoint_role: "target",
                endpoint: target,
            });
        }
        if self
            .edges
            .iter()
            .any(|edge| edge.source == source && edge.target == target && edge.kind == kind)
        {
            return Err(ConstructionProjectError::DuplicateControlGraphEdge {
                kind: kind.label(),
                from: source,
                target,
            });
        }
        self.edges.push(ControlEdge::new(source, target, kind));
        Ok(())
    }

    /// Lowers stable construction ids into a canonical directed graph.
    pub fn project(&self, readiness_only: bool) -> Result<ControlGraphProjection> {
        let nodes = self
            .nodes
            .iter()
            .map(|(id, kind)| ControlNode::new(id.clone(), *kind))
            .collect::<Vec<_>>();
        let indices = nodes
            .iter()
            .enumerate()
            .map(|(index, node)| (node.id.clone(), index))
            .collect::<BTreeMap<_, _>>();
        let mut graph = Graph::with_nodes(nodes, Directedness::Directed);
        let mut sorted_edges = self.edges.clone();
        sorted_edges.sort_by(|left, right| {
            left.source
                .cmp(&right.source)
                .then(left.target.cmp(&right.target))
                .then(left.kind.cmp(&right.kind))
        });
        for edge in sorted_edges
            .into_iter()
            .filter(|edge| !readiness_only || edge.kind.propagates_readiness())
        {
            let source = *indices.get(&edge.source).ok_or_else(|| {
                ConstructionProjectError::ControlGraphMissingEndpoint {
                    edge: edge.kind.label(),
                    endpoint_role: "source",
                    endpoint: edge.source.clone(),
                }
            })?;
            let target = *indices.get(&edge.target).ok_or_else(|| {
                ConstructionProjectError::ControlGraphMissingEndpoint {
                    edge: edge.kind.label(),
                    endpoint_role: "target",
                    endpoint: edge.target.clone(),
                }
            })?;
            graph.add_edge(source, target, edge.kind).map_err(|error| {
                ConstructionProjectError::ControlGraphAlgorithm {
                    operation: "add_edge",
                    reason: error.to_string(),
                }
            })?;
        }
        graph
            .validate()
            .map_err(|error| ConstructionProjectError::ControlGraphAlgorithm {
                operation: "validate",
                reason: error.to_string(),
            })?;
        Ok(ControlGraphProjection { graph, indices })
    }

    /// Rejects prohibited readiness cycles while allowing informational cycles.
    pub fn validate_readiness(&self) -> Result<()> {
        let projection = self.project(true)?;
        let components = strongly_connected_components(&projection.graph).map_err(|error| {
            ConstructionProjectError::ControlGraphAlgorithm {
                operation: "strongly_connected_components",
                reason: error.to_string(),
            }
        })?;
        for component in components {
            let self_loop = component.len() == 1
                && projection
                    .graph
                    .edges
                    .iter()
                    .any(|edge| edge.source == component[0] && edge.target == component[0]);
            if component.len() > 1 || self_loop {
                let cycle = component
                    .into_iter()
                    .map(|node| projection.graph.nodes[node].id.clone())
                    .collect::<Vec<_>>();
                return Err(ConstructionProjectError::ControlGraphCycle { cycle });
            }
        }
        Ok(())
    }

    /// Analyzes blockers and dependents for `target` using canonical algorithms.
    pub fn analyze_target<F, G, H>(
        &self,
        target: &ControlId,
        mut blocks: F,
        mut evidence_state: G,
        mut exception: H,
    ) -> Result<ControlGraphAnalysis>
    where
        F: FnMut(&ControlId) -> bool,
        G: FnMut(&ControlId) -> (Option<u64>, EvidenceState),
        H: FnMut(&ControlId) -> Option<ControlId>,
    {
        self.validate_readiness()?;
        let projection = self.project(true)?;
        let target_index = *projection.indices.get(target).ok_or_else(|| {
            ConstructionProjectError::ControlGraphMissingEndpoint {
                edge: "analysis-target",
                endpoint_role: "target",
                endpoint: target.clone(),
            }
        })?;
        let reachable = reachability(&projection.graph).map_err(|error| {
            ConstructionProjectError::ControlGraphAlgorithm {
                operation: "reachability",
                reason: error.to_string(),
            }
        })?;

        let mut blockers = Vec::new();
        let mut dependents = Vec::new();
        for (control, index) in &projection.indices {
            if *index == target_index {
                continue;
            }
            let reaches_target = reachable.get(*index, target_index).map_err(|error| {
                ConstructionProjectError::ControlGraphAlgorithm {
                    operation: "reachability-read",
                    reason: error.to_string(),
                }
            })?;
            if reaches_target.0 && blocks(control) {
                blockers.push(control.clone());
            }
            let reached_from_target = reachable.get(target_index, *index).map_err(|error| {
                ConstructionProjectError::ControlGraphAlgorithm {
                    operation: "dependents-read",
                    reason: error.to_string(),
                }
            })?;
            if reached_from_target.0 {
                dependents.push(control.clone());
            }
        }

        let blocker_set = blockers.iter().cloned().collect::<BTreeSet<_>>();
        let mut cut = blockers
            .iter()
            .filter(|candidate| {
                let candidate_index = projection.indices[*candidate];
                !blocker_set.iter().any(|other| {
                    other != *candidate
                        && reachable
                            .get(projection.indices[other], candidate_index)
                            .map(|cell| cell.0)
                            .unwrap_or(false)
                })
            })
            .cloned()
            .collect::<Vec<_>>();

        blockers.sort();
        dependents.sort();
        cut.sort();
        let explanation_paths = blockers
            .iter()
            .map(|blocker| {
                self.explanation_path(
                    &projection,
                    blocker,
                    target,
                    &mut evidence_state,
                    &mut exception,
                )
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(ControlGraphAnalysis {
            transitive_blockers: blockers,
            affected_dependents: dependents,
            critical_prerequisite_cut: cut,
            explanation_paths,
        })
    }

    fn explanation_path<G, H>(
        &self,
        projection: &ControlGraphProjection,
        blocker: &ControlId,
        target: &ControlId,
        evidence_state: &mut G,
        exception: &mut H,
    ) -> Result<ControlExplanationPath>
    where
        G: FnMut(&ControlId) -> (Option<u64>, EvidenceState),
        H: FnMut(&ControlId) -> Option<ControlId>,
    {
        let source = projection.indices[blocker];
        let target_index = projection.indices[target];
        let weighted = self.weighted_projection()?;
        let paths = dijkstra(&weighted.graph, source).map_err(|error| {
            ConstructionProjectError::ControlGraphAlgorithm {
                operation: "dijkstra",
                reason: error.to_string(),
            }
        })?;
        let mut indices = Vec::new();
        let mut cursor = Some(target_index);
        while let Some(index) = cursor {
            indices.push(index);
            if index == source {
                break;
            }
            cursor = paths.predecessors[index];
        }
        indices.reverse();

        let mut steps = Vec::new();
        for (position, index) in indices.iter().enumerate() {
            let control = projection.graph.nodes[*index].id.clone();
            let (current_seq, evidence_state_value) = evidence_state(&control);
            let edge_kind = if position == 0 {
                None
            } else {
                edge_kind_between(&projection.graph, indices[position - 1], *index)
            };
            steps.push(ControlExplanationStep {
                exception: exception(&control),
                control,
                current_seq,
                evidence_state: evidence_state_value,
                edge_kind,
            });
        }
        Ok(ControlExplanationPath {
            blocker: blocker.clone(),
            target: target.clone(),
            steps,
        })
    }

    fn weighted_projection(&self) -> Result<WeightedControlGraphProjection> {
        let projection = self.project(true)?;
        let mut graph = Graph::with_nodes(projection.graph.nodes.clone(), Directedness::Directed);
        for edge in &projection.graph.edges {
            graph
                .add_edge(edge.source, edge.target, 1)
                .map_err(|error| ConstructionProjectError::ControlGraphAlgorithm {
                    operation: "weighted-add-edge",
                    reason: error.to_string(),
                })?;
        }
        Ok(WeightedControlGraphProjection { graph })
    }
}

#[derive(Clone, Debug, PartialEq)]
struct WeightedControlGraphProjection {
    graph: Graph<ControlNode, u64>,
}

fn edge_kind_between(
    graph: &Graph<ControlNode, ControlEdgeKind>,
    source: usize,
    target: usize,
) -> Option<ControlEdgeKind> {
    graph
        .edges
        .iter()
        .filter(|edge| edge.source == source && edge.target == target)
        .map(|edge| edge.weight)
        .min()
}
