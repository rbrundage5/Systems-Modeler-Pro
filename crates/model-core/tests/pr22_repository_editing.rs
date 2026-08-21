use systems_modeler_core::{ElementKind, ModelError, Project};

#[test]
fn repository_reparenting_preserves_valid_containment() {
    let mut project = Project::new("Repository Editing");
    let source = project
        .create_element(ElementKind::Package, "Source", project.root_id)
        .unwrap();
    let target = project
        .create_element(ElementKind::Package, "Target", project.root_id)
        .unwrap();
    let block = project
        .create_element(ElementKind::Block, "Controller", source)
        .unwrap();

    project.move_element(block, target).unwrap();

    assert_eq!(project.element(block).unwrap().owner_id, Some(target));
    assert!(project.children(target).any(|element| element.id == block));
    assert!(!project.children(source).any(|element| element.id == block));
    project.validate().unwrap();
}

#[test]
fn repository_reparenting_rejects_root_moves_and_cycles() {
    let mut project = Project::new("Repository Editing");
    let parent = project
        .create_element(ElementKind::Package, "Parent", project.root_id)
        .unwrap();
    let child = project
        .create_element(ElementKind::Package, "Child", parent)
        .unwrap();

    assert_eq!(
        project.move_element(project.root_id, parent),
        Err(ModelError::ProtectedProjectRoot(project.root_id))
    );
    assert_eq!(
        project.move_element(parent, child),
        Err(ModelError::OwnershipCycle {
            element_id: parent,
            new_owner_id: child,
        })
    );
    assert_eq!(project.element(parent).unwrap().owner_id, Some(project.root_id));
    assert_eq!(project.element(child).unwrap().owner_id, Some(parent));
}
