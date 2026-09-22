use systems_modeler_core::{ElementKind, ModelError, Project};

#[test]
fn whole_project_validation_rejects_a_missing_root() {
    let mut project = Project::new("Vehicle");
    let root = project.root_id;
    project.elements.remove(&root);

    assert_eq!(
        project.validate(),
        Err(ModelError::MissingProjectRoot(root))
    );
}

#[test]
fn whole_project_validation_rejects_a_non_model_root() {
    let mut project = Project::new("Vehicle");
    let root = project.root_id;
    project.elements.get_mut(&root).unwrap().kind = ElementKind::Package;

    assert_eq!(
        project.validate(),
        Err(ModelError::InvalidProjectRootKind {
            id: root,
            kind: ElementKind::Package,
        })
    );
}

#[test]
fn whole_project_validation_rejects_an_owned_root() {
    let mut project = Project::new("Vehicle");
    let root = project.root_id;
    let owner = project
        .create_element(ElementKind::Package, "Invalid owner", root)
        .unwrap();
    project.elements.get_mut(&root).unwrap().owner_id = Some(owner);

    assert_eq!(
        project.validate(),
        Err(ModelError::ProjectRootHasOwner(root))
    );
}
