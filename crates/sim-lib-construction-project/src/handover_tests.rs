// conformance: handover hierarchy reuses stable construction control graph ids and edges

use crate::{
    ConstructionProjectError, ControlEdgeKind, ControlId, HandoverControlKind, HandoverHierarchy,
    ProjectId,
};

#[test]
fn hierarchy_uses_typed_common_graph_nodes_and_member_edges() {
    let mut hierarchy = hierarchy();
    add(
        &mut hierarchy,
        "milestone.first-use",
        HandoverControlKind::ContractualMilestone,
    );
    add(&mut hierarchy, "area.east", HandoverControlKind::Area);
    add(
        &mut hierarchy,
        "system.ventilation",
        HandoverControlKind::System,
    );
    add(
        &mut hierarchy,
        "package.controls",
        HandoverControlKind::WorkPackage,
    );
    add(
        &mut hierarchy,
        "assets.ahu",
        HandoverControlKind::AssetGroup,
    );

    hierarchy
        .add_member(control("area.east"), control("milestone.first-use"))
        .unwrap();
    hierarchy
        .add_member(control("system.ventilation"), control("area.east"))
        .unwrap();
    hierarchy
        .add_member(control("package.controls"), control("system.ventilation"))
        .unwrap();
    hierarchy
        .add_member(control("assets.ahu"), control("system.ventilation"))
        .unwrap();

    assert_eq!(
        hierarchy.scope(&control("milestone.first-use")).unwrap(),
        vec![
            control("area.east"),
            control("assets.ahu"),
            control("milestone.first-use"),
            control("package.controls"),
            control("system.ventilation"),
        ]
    );
    assert!(
        hierarchy
            .control_graph()
            .edges
            .iter()
            .all(|edge| edge.kind == ControlEdgeKind::MemberOf)
    );
}

#[test]
fn one_system_can_roll_into_multiple_areas_without_a_second_tree() {
    let mut hierarchy = hierarchy();
    add(&mut hierarchy, "area.east", HandoverControlKind::Area);
    add(&mut hierarchy, "area.west", HandoverControlKind::Area);
    add(
        &mut hierarchy,
        "system.fire-alarm",
        HandoverControlKind::System,
    );

    hierarchy
        .add_member(control("system.fire-alarm"), control("area.east"))
        .unwrap();
    hierarchy
        .add_member(control("system.fire-alarm"), control("area.west"))
        .unwrap();

    assert_eq!(
        hierarchy.direct_parents(&control("system.fire-alarm")),
        vec![control("area.east"), control("area.west")]
    );
    assert_eq!(
        hierarchy.leaves(&control("area.east")).unwrap(),
        vec![control("system.fire-alarm")]
    );
}

#[test]
fn member_cycles_are_rejected_without_mutating_the_hierarchy() {
    let mut hierarchy = hierarchy();
    add(
        &mut hierarchy,
        "system.primary",
        HandoverControlKind::System,
    );
    add(
        &mut hierarchy,
        "system.secondary",
        HandoverControlKind::System,
    );
    hierarchy
        .add_member(control("system.secondary"), control("system.primary"))
        .unwrap();

    let result = hierarchy.add_member(control("system.primary"), control("system.secondary"));

    assert!(matches!(
        result,
        Err(ConstructionProjectError::ControlGraphCycle { .. })
    ));
    assert!(
        hierarchy
            .direct_parents(&control("system.primary"))
            .is_empty()
    );
}

fn hierarchy() -> HandoverHierarchy {
    HandoverHierarchy::new(ProjectId::new("project.handover").unwrap())
}

fn add(hierarchy: &mut HandoverHierarchy, id: &str, kind: HandoverControlKind) {
    hierarchy.add_control(control(id), kind).unwrap();
}

fn control(id: &str) -> ControlId {
    ControlId::new(id).unwrap()
}
