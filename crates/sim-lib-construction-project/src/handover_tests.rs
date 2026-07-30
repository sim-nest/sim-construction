// conformance: handover hierarchy reuses stable construction control graph ids and edges

use crate::{
    CommissioningControlSet, CommissioningRequirement, CommissioningRequirementKind,
    ConstructionProjectError, ControlEdgeKind, ControlId, HandoverControlKind, HandoverHierarchy,
    ObligationPolicy, ProjectId, ProjectObligation, Requirement, RequirementLane, RoleId,
};
use sim_kernel::Symbol;
use sim_lib_doc_core::ExternalRef;

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

#[test]
fn every_commissioning_kind_uses_shared_evidence_requirements() {
    let mut hierarchy = hierarchy();
    add(
        &mut hierarchy,
        "system.heating",
        HandoverControlKind::System,
    );
    let kinds = [
        CommissioningRequirementKind::Activity,
        CommissioningRequirementKind::Inspection,
        CommissioningRequirementKind::Test,
        CommissioningRequirementKind::Defect,
        CommissioningRequirementKind::OperationsMaintenanceDeliverable,
        CommissioningRequirementKind::AsBuiltDeliverable,
        CommissioningRequirementKind::Training,
        CommissioningRequirementKind::Certification,
        CommissioningRequirementKind::AuthorityClosure,
        CommissioningRequirementKind::CustomerAcceptance,
        CommissioningRequirementKind::RemainingWork,
    ];
    let controls = kinds.into_iter().enumerate().fold(
        CommissioningControlSet::new(project()),
        |controls, (index, kind)| {
            controls.with_requirement(commissioning_requirement(
                kind,
                &format!("requirement.handover-{index}"),
            ))
        },
    );

    controls.validate(&hierarchy).unwrap();
    assert_eq!(controls.requirements.len(), 11);
    assert!(controls.requirements.iter().all(|item| {
        item.obligation.requirement.evidence_required
            && !item.obligation.requirement.evidence_kinds.is_empty()
            && !item.obligation.requirement.source_refs.is_empty()
    }));
    let authority = controls
        .requirements
        .iter()
        .find(|item| item.kind == CommissioningRequirementKind::AuthorityClosure)
        .unwrap();
    assert!(authority.obligation.requirement.non_waivable);
    assert_eq!(authority.obligation.policy, ObligationPolicy::Mandatory);
}

#[test]
fn commissioning_rejects_optional_evidence_and_unknown_targets() {
    let mut hierarchy = hierarchy();
    add(
        &mut hierarchy,
        "system.heating",
        HandoverControlKind::System,
    );
    let mut missing_evidence = commissioning_requirement(
        CommissioningRequirementKind::Training,
        "requirement.training",
    );
    missing_evidence
        .obligation
        .requirement
        .evidence_kinds
        .clear();
    let result = CommissioningControlSet::new(project())
        .with_requirement(missing_evidence)
        .validate(&hierarchy);
    assert!(matches!(
        result,
        Err(ConstructionProjectError::EmptyCollection(
            "commissioning_requirement.evidence_kinds"
        ))
    ));

    let mut unknown_target = commissioning_requirement(
        CommissioningRequirementKind::Test,
        "requirement.unknown-target",
    );
    unknown_target.targets = vec![control("system.unknown")];
    let result = CommissioningControlSet::new(project())
        .with_requirement(unknown_target)
        .validate(&hierarchy);
    assert!(matches!(
        result,
        Err(ConstructionProjectError::ControlGraphMissingEndpoint {
            edge: "commissioning-target",
            ..
        })
    ));
}

fn hierarchy() -> HandoverHierarchy {
    HandoverHierarchy::new(project())
}

fn project() -> ProjectId {
    ProjectId::new("project.handover").unwrap()
}

fn add(hierarchy: &mut HandoverHierarchy, id: &str, kind: HandoverControlKind) {
    hierarchy.add_control(control(id), kind).unwrap();
}

fn control(id: &str) -> ControlId {
    ControlId::new(id).unwrap()
}

fn commissioning_requirement(
    kind: CommissioningRequirementKind,
    id: &str,
) -> CommissioningRequirement {
    let requirement = Requirement::new(
        control(id),
        RequirementLane::new(Symbol::qualified("construction", "handover")),
        format!("{kind:?} evidence"),
        RoleId::new("role.commissioning-lead").unwrap(),
        RoleId::new("role.customer").unwrap(),
    )
    .with_evidence_kind(Symbol::qualified("construction", "commissioning-record"))
    .with_source_ref(ExternalRef::new(
        "doc/synthetic",
        id,
        Some("v1".to_owned()),
        None,
    ));
    CommissioningRequirement::new(
        kind,
        ProjectObligation::mandatory(project(), requirement),
        control("system.heating"),
    )
}
