use systems_modeler_core::{
    Connector, ConnectorEnd, ConnectorKind, ElementKind, ItemFlow, Multiplicity, Project,
    RelationshipKind,
};

#[test]
fn deleted_contexts_retire_only_their_specialized_repositories_and_identities() {
    use std::collections::HashSet;
    use systems_modeler_core::{ActivityRepository, BehaviorRepository, BehaviorSemanticId};
    let mut project = Project::new("Contexts");
    let context = project
        .create_element(ElementKind::Block, "Deleted", project.root_id)
        .unwrap();
    let kept = project
        .create_element(ElementKind::Block, "Kept", project.root_id)
        .unwrap();
    let mut behavior = BehaviorRepository::default();
    let machine = behavior
        .create_state_machine(&project, context, "Old")
        .unwrap();
    let other = behavior
        .create_state_machine(&project, kept, "Kept")
        .unwrap();
    let region = behavior.state_machines[&machine].regions[0].id;
    behavior
        .external_ids
        .insert("old-region".into(), BehaviorSemanticId::Region(region));
    let mut activity = ActivityRepository::default();
    let old_activity = activity
        .create_activity(&project, project.root_id, Some(context), "Old")
        .unwrap();
    let kept_activity = activity
        .create_activity(&project, project.root_id, Some(kept), "Kept")
        .unwrap();
    let deleted = HashSet::from([context]);
    behavior.remove_deleted_contexts(&deleted);
    activity.remove_deleted_contexts(&deleted);
    assert!(!behavior.state_machines.contains_key(&machine));
    assert!(behavior.state_machines.contains_key(&other));
    assert!(behavior.external_ids.is_empty());
    assert!(!activity.activities.contains_key(&old_activity));
    assert!(activity.activities.contains_key(&kept_activity));
    let candidate = project.stage_connected_element_deletion(context).unwrap();
    behavior.validate(&candidate.project).unwrap();
    activity.validate(&candidate.project).unwrap();
}

#[test]
fn deleting_usage_removes_relationships_and_flows_but_keeps_type_and_other_usage() {
    let mut project = Project::new("Deletion");
    let owner = project.root_id;
    let system = project
        .create_element(ElementKind::Block, "System", owner)
        .unwrap();
    let component = project
        .create_element(ElementKind::Block, "Component", owner)
        .unwrap();
    let signal = project
        .create_element(ElementKind::Signal, "Signal", owner)
        .unwrap();
    let (composition, part) = project
        .create_composition(system, component, "part", Multiplicity::ONE, Some(owner))
        .unwrap();
    let sibling = project
        .create_typed_feature(
            ElementKind::PartProperty,
            "other",
            system,
            component,
            Multiplicity::ONE,
        )
        .unwrap();
    let source = ConnectorEnd::role(part);
    let target = ConnectorEnd::role(sibling);
    let connector = project
        .create_connector(Connector {
            context_id: system,
            kind: ConnectorKind::Assembly,
            source: source.clone(),
            target: target.clone(),
        })
        .unwrap();
    let flow = project
        .create_item_flow(ItemFlow {
            connector_id: connector,
            source,
            target,
            conveyed_item_ids: vec![signal],
        })
        .unwrap();
    let before = serde_json::to_value(&project).unwrap();
    let deletion = project.stage_connected_element_deletion(part).unwrap();
    assert_eq!(serde_json::to_value(&project).unwrap(), before);
    assert_eq!(deletion.elements.len(), 1);
    for id in [composition, connector, flow] {
        assert!(deletion.relationships.contains(&id));
    }
    for id in [system, component, signal, sibling] {
        assert!(deletion.project.element(id).is_ok());
    }
    assert!(deletion.project.element(part).is_err());
    deletion.project.validate().unwrap();
}

#[test]
fn deletion_removes_owned_subtree_and_traceability_but_protects_root_and_external_types() {
    let mut project = Project::new("Deletion");
    let root = project.root_id;
    let package = project
        .create_element(ElementKind::Package, "Package", root)
        .unwrap();
    let block = project
        .create_element(ElementKind::Block, "Block", package)
        .unwrap();
    let requirement = project
        .create_requirement("Need", "R1", "Text", root)
        .unwrap();
    let satisfy = project
        .create_relationship(RelationshipKind::Satisfy, block, requirement, Some(root))
        .unwrap();
    let deletion = project.stage_connected_element_deletion(package).unwrap();
    assert!(deletion.elements.contains(&block));
    assert!(deletion.relationships.contains(&satisfy));
    assert!(deletion.project.element(requirement).is_ok());
    assert!(project.stage_connected_element_deletion(root).is_err());
    let external = project
        .create_element(ElementKind::Block, "External", root)
        .unwrap();
    project
        .create_typed_feature(
            ElementKind::PartProperty,
            "shared",
            external,
            block,
            Multiplicity::ONE,
        )
        .unwrap();
    let before = serde_json::to_value(&project).unwrap();
    assert!(project.stage_connected_element_deletion(package).is_err());
    assert_eq!(serde_json::to_value(project).unwrap(), before);
}
