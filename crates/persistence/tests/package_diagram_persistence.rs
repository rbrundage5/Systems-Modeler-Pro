use systems_modeler_core::{ElementKind, Project, RelationshipKind, VisibilityKind};
use systems_modeler_persistence::ProjectDatabase;

#[test]
fn package_semantics_visibility_alias_and_ownership_round_trip() {
    let mut project = Project::new("Package persistence");
    let importing = project
        .create_element(ElementKind::Package, "Vehicle", project.root_id)
        .unwrap();
    let imported = project
        .create_element(ElementKind::Package, "Powertrain", project.root_id)
        .unwrap();
    let library = project
        .create_element(ElementKind::ModelLibrary, "Standard Types", project.root_id)
        .unwrap();
    let imported_owner = project.element(imported).unwrap().owner_id;
    let library_owner = project.element(library).unwrap().owner_id;

    let package_import = project
        .create_package_import(importing, imported, VisibilityKind::Private)
        .unwrap();
    let element_import = project
        .create_element_import(
            importing,
            library,
            VisibilityKind::Public,
            Some("Types".into()),
        )
        .unwrap();
    project.validate().unwrap();

    let mut database = ProjectDatabase::open_in_memory().unwrap();
    database.save_project(&project).unwrap();
    let restored = database.load_project(project.id).unwrap();
    restored.validate().unwrap();

    assert_eq!(restored.element(imported).unwrap().owner_id, imported_owner);
    assert_eq!(restored.element(library).unwrap().owner_id, library_owner);
    assert_eq!(
        restored.relationship(package_import).unwrap().visibility,
        VisibilityKind::Private
    );
    let restored_element_import = restored.relationship(element_import).unwrap();
    assert_eq!(
        restored_element_import.kind,
        RelationshipKind::ElementImport
    );
    assert_eq!(restored_element_import.alias.as_deref(), Some("Types"));
    assert_eq!(
        restored.relationship(package_import).unwrap().kind,
        RelationshipKind::PackageImport
    );
}
