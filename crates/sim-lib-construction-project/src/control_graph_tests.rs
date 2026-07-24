// conformance: construction control graph composition over sim-lib-discrete-graph

use crate::{
    ConstructionProjectError, ControlEdgeKind, ControlGraph, ControlId, ControlNodeKind,
    EvidenceState,
};

#[test]
fn readiness_cycles_are_rejected_but_informational_cycles_are_data() {
    let mut graph = ControlGraph::new();
    let first = control("requirement.first");
    let second = control("requirement.second");
    graph
        .add_node(first.clone(), ControlNodeKind::Requirement)
        .unwrap();
    graph
        .add_node(second.clone(), ControlNodeKind::Requirement)
        .unwrap();
    graph
        .add_edge(first.clone(), second.clone(), ControlEdgeKind::Prerequisite)
        .unwrap();
    graph
        .add_edge(second.clone(), first.clone(), ControlEdgeKind::Blocks)
        .unwrap();
    assert!(matches!(
        graph.validate_readiness(),
        Err(ConstructionProjectError::ControlGraphCycle { .. })
    ));

    let mut informational = ControlGraph::new();
    informational
        .add_node(first.clone(), ControlNodeKind::Requirement)
        .unwrap();
    informational
        .add_node(second.clone(), ControlNodeKind::Requirement)
        .unwrap();
    informational
        .add_edge(
            first.clone(),
            second.clone(),
            ControlEdgeKind::Informational,
        )
        .unwrap();
    informational
        .add_edge(second, first, ControlEdgeKind::Informational)
        .unwrap();
    informational.validate_readiness().unwrap();
}

#[test]
fn analysis_derives_blockers_dependents_cut_and_stable_paths() {
    let mut graph = ControlGraph::new();
    for id in [
        "requirement.alpha",
        "requirement.beta",
        "requirement.gamma",
        "gate.ready",
        "outcome.handover",
    ] {
        graph
            .add_node(control(id), ControlNodeKind::Requirement)
            .unwrap();
    }
    graph
        .add_edge(
            control("requirement.alpha"),
            control("requirement.gamma"),
            ControlEdgeKind::Prerequisite,
        )
        .unwrap();
    graph
        .add_edge(
            control("requirement.beta"),
            control("requirement.gamma"),
            ControlEdgeKind::Prerequisite,
        )
        .unwrap();
    graph
        .add_edge(
            control("requirement.gamma"),
            control("gate.ready"),
            ControlEdgeKind::Blocks,
        )
        .unwrap();
    graph
        .add_edge(
            control("gate.ready"),
            control("outcome.handover"),
            ControlEdgeKind::Produces,
        )
        .unwrap();

    let analysis = graph
        .analyze_target(
            &control("gate.ready"),
            |id| matches!(id.as_str(), "requirement.alpha" | "requirement.beta"),
            |id| {
                if id.as_str() == "requirement.gamma" {
                    (Some(3), EvidenceState::Accepted)
                } else {
                    (None, EvidenceState::Missing)
                }
            },
            |_| None,
        )
        .unwrap();

    assert_eq!(
        ids(&analysis.transitive_blockers),
        vec!["requirement.alpha", "requirement.beta"]
    );
    assert_eq!(ids(&analysis.affected_dependents), vec!["outcome.handover"]);
    assert_eq!(
        ids(&analysis.critical_prerequisite_cut),
        vec!["requirement.alpha", "requirement.beta"]
    );
    assert_eq!(
        analysis.explanation_paths[0]
            .steps
            .iter()
            .map(|step| (
                step.control.as_str(),
                step.edge_kind.map(ControlEdgeKind::label),
                step.current_seq,
                step.evidence_state,
            ))
            .collect::<Vec<_>>(),
        vec![
            ("requirement.alpha", None, None, EvidenceState::Missing),
            (
                "requirement.gamma",
                Some("prerequisite"),
                Some(3),
                EvidenceState::Accepted,
            ),
            ("gate.ready", Some("blocks"), None, EvidenceState::Missing,),
        ]
    );
}

fn control(id: &str) -> ControlId {
    ControlId::new(id).unwrap()
}

fn ids(values: &[ControlId]) -> Vec<&str> {
    values.iter().map(ControlId::as_str).collect()
}
