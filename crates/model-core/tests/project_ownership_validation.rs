use systems_modeler_core::{ElementId, ElementKind, ModelError, Project};

fn package(project: &mut Project, owner: ElementId, name: &str) -> ElementId {
    project
        .create_element(ElementKind::Package, name, owner)
        .unwrap()
}

#[test]
fn whole_project_ownership_accepts_nested_libraries_and_features() {
    let mut project = Project::new("Vehicle");
    let root = project.root_id;
    let structure = package(&mut project, root, "Structure");
    let library = project
        .create_element(ElementKind::ModelLibrary, "Common", structure)
        .unwrap();
    let block = project
        .create_element(ElementKind::Block, "System", library)
        .unwrap();
    project
        .create_element(ElementKind::Operation, "Operate", block)
        .unwrap();

    assert_eq!(project.validate(), Ok(()));
    assert_eq!(
        project.qualified_name(block).unwrap(),
        "Vehicle::Structure::Common::System"
    );
}

#[test]
fn whole_project_ownership_rejects_self_ownership_without_mutation() {
    let mut project = Project::new("Vehicle");
    let root = project.root_id;
    let id = package(&mut project, root, "Cycle");
    project.element_mut(id).unwrap().owner_id = Some(id);
    let before = serde_json::to_value(&project).unwrap();

    assert_eq!(
        project.validate(),
        Err(ModelError::CyclicProjectOwnership(id))
    );
    assert_eq!(serde_json::to_value(&project).unwrap(), before);
}

#[test]
fn whole_project_ownership_rejects_a_cycle_with_attached_descendants() {
    let mut project = Project::new("Vehicle");
    let root = project.root_id;
    let first = package(&mut project, root, "First");
    let second = package(&mut project, first, "Second");
    let third = package(&mut project, second, "Third");
    project
        .create_element(ElementKind::Block, "Descendant", third)
        .unwrap();
    project.element_mut(first).unwrap().owner_id = Some(third);
    let before = serde_json::to_value(&project).unwrap();

    assert!(matches!(
        project.validate(),
        Err(ModelError::CyclicProjectOwnership(id)) if [first, second, third].contains(&id)
    ));
    assert_eq!(serde_json::to_value(&project).unwrap(), before);
}

#[test]
fn whole_project_ownership_rejects_an_unowned_block() {
    let mut project = Project::new("Vehicle");
    let id = project
        .create_element(ElementKind::Block, "Orphan", project.root_id)
        .unwrap();
    project.element_mut(id).unwrap().owner_id = None;

    assert_eq!(project.validate(), Err(ModelError::UnownedProjectElement(id)));
}

#[test]
fn whole_project_ownership_rejects_a_disconnected_subtree() {
    let mut project = Project::new("Vehicle");
    let root = project.root_id;
    let detached = package(&mut project, root, "Detached");
    let child = package(&mut project, detached, "Child");
    project
        .create_element(ElementKind::Block, "System", child)
        .unwrap();
    project.element_mut(detached).unwrap().owner_id = None;

    assert_eq!(
        project.validate(),
        Err(ModelError::UnownedProjectElement(detached))
    );
}

#[test]
fn whole_project_ownership_rejects_a_missing_owner() {
    let mut project = Project::new("Vehicle");
    let root = project.root_id;
    let id = package(&mut project, root, "Orphan");
    let missing = ElementId::new();
    project.element_mut(id).unwrap().owner_id = Some(missing);

    assert_eq!(project.validate(), Err(ModelError::ElementNotFound(missing)));
}

#[test]
fn whole_project_ownership_accepts_deep_containment_without_recursion() {
    let mut project = Project::new("Vehicle");
    let mut owner = project.root_id;
    for _ in 0..4096 {
        owner = package(&mut project, owner, "Nested");
    }
    for _ in 0..256 {
        project
            .create_element(ElementKind::Block, "Leaf", owner)
            .unwrap();
    }

    assert_eq!(project.validate(), Ok(()));
}
