use systems_modeler_core::{ElementKind, Project, RelationshipKind};

#[test]
fn nested_requirements_preserve_identity_through_rename_and_reparent() {
    let mut project = Project::new("System");
    let root = project.root_id;
    let system = project.create_requirement("System", "R-1", "System text", root).unwrap();
    let subsystem = project.create_requirement("Subsystem", "R-2", "Subsystem text", system).unwrap();
    let component = project.create_requirement("Component", "R-3", "Component text", subsystem).unwrap();
    let id = project.element(component).unwrap().external_id.clone();
    project.validate().unwrap();
    assert_eq!(project.qualified_name(component).unwrap(), "System::System::Subsystem::Component");
    project.rename_element(subsystem, "Unit").unwrap();
    project.move_element(component, system).unwrap();
    project.validate().unwrap();
    assert_eq!(project.element(component).unwrap().external_id, id);
    assert_eq!(project.qualified_name(component).unwrap(), "System::System::Component");
    assert_eq!(project.children(system).count(), 2);
}

#[test]
fn illegal_requirement_ownership_and_cycles_preserve_the_model() {
    let mut project = Project::new("System");
    let root = project.root_id;
    let parent = project.create_requirement("Parent", "R-1", "Parent text", root).unwrap();
    let child = project.create_requirement("Child", "R-2", "Child text", parent).unwrap();
    let block = project.create_element(ElementKind::Block, "Block", root).unwrap();
    let before = serde_json::to_value(&project).unwrap();
    assert!(project.move_element(parent, child).is_err());
    assert!(project.move_element(child, child).is_err());
    assert!(project.move_element(child, block).is_err());
    assert!(project.create_element(ElementKind::Block, "Invalid", parent).is_err());
    assert!(project.create_requirement("Duplicate", "R-2", "Invalid", parent).is_err());
    assert_eq!(serde_json::to_value(&project).unwrap(), before);
    project.validate().unwrap();
}

#[test]
fn deleting_a_compound_requirement_retires_descendants_and_incident_traceability() {
    let mut project = Project::new("System");
    let root = project.root_id;
    let parent = project.create_requirement("Parent", "R-1", "Parent text", root).unwrap();
    let child = project.create_requirement("Child", "R-2", "Child text", parent).unwrap();
    let leaf = project.create_requirement("Leaf", "R-3", "Leaf text", child).unwrap();
    let block = project.create_element(ElementKind::Block, "Implementation", root).unwrap();
    let link = project.create_relationship(RelationshipKind::Satisfy, block, leaf, Some(root)).unwrap();
    project.delete_element(parent).unwrap();
    for id in [parent, child, leaf] {
        assert!(project.element(id).is_err());
    }
    assert!(project.relationship(link).is_err());
    assert!(project.element(block).is_ok());
    project.validate().unwrap();
}
