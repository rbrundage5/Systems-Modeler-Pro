use systems_modeler_core::{ElementKind, ModelError, Project, RelationshipKind, VisibilityKind};

fn package_project() -> (
    Project,
    systems_modeler_core::ElementId,
    systems_modeler_core::ElementId,
    systems_modeler_core::ElementId,
) {
    let mut project = Project::new("Vehicle Architecture");
    let importing = project
        .create_element(ElementKind::Package, "Vehicle", project.root_id)
        .unwrap();
    let imported = project
        .create_element(ElementKind::Package, "Powertrain", project.root_id)
        .unwrap();
    let library = project
        .create_element(ElementKind::ModelLibrary, "Standard Types", project.root_id)
        .unwrap();
    (project, importing, imported, library)
}

#[test]
fn packages_and_model_libraries_are_first_class_namespaces() {
    let (project, package, _, library) = package_project();
    assert_eq!(project.element(package).unwrap().kind, ElementKind::Package);
    assert!(project.element(package).unwrap().is_namespace());
    assert_eq!(
        project.element(library).unwrap().kind,
        ElementKind::ModelLibrary
    );
    assert!(project.element(library).unwrap().is_namespace());
    project.validate().unwrap();
}

#[test]
fn package_import_preserves_ownership_visibility_and_stable_ids() {
    let (mut project, importing, imported, _) = package_project();
    let imported_owner = project.element(imported).unwrap().owner_id;
    let imported_id = imported;
    let relationship = project
        .create_package_import(importing, imported, VisibilityKind::Private)
        .unwrap();

    let package_import = project.relationship(relationship).unwrap();
    assert_eq!(package_import.kind, RelationshipKind::PackageImport);
    assert_eq!(package_import.owner_id, Some(importing));
    assert_eq!(package_import.visibility, VisibilityKind::Private);
    assert_eq!(project.element(imported).unwrap().owner_id, imported_owner);
    assert_eq!(project.element(imported).unwrap().id, imported_id);
    project.validate().unwrap();
}

#[test]
fn element_import_supports_public_private_visibility_and_alias() {
    let (mut project, importing, _, _) = package_project();
    let block = project
        .create_element(ElementKind::Block, "Controller", project.root_id)
        .unwrap();
    let original_owner = project.element(block).unwrap().owner_id;
    let relationship = project
        .create_element_import(
            importing,
            block,
            VisibilityKind::Public,
            Some("ControlType".into()),
        )
        .unwrap();
    let element_import = project.relationship(relationship).unwrap();
    assert_eq!(element_import.kind, RelationshipKind::ElementImport);
    assert_eq!(element_import.alias.as_deref(), Some("ControlType"));
    assert_eq!(element_import.visibility, VisibilityKind::Public);
    assert_eq!(project.element(block).unwrap().owner_id, original_owner);

    assert_eq!(
        project.create_element_import(
            importing,
            project.root_id,
            VisibilityKind::Private,
            Some("not a valid alias".into()),
        ),
        Err(ModelError::InvalidElementImportAlias(
            "not a valid alias".into()
        ))
    );
    project.validate().unwrap();
}

#[test]
fn package_merge_requires_package_endpoints_and_does_not_reparent() {
    let (mut project, receiving, merged, _) = package_project();
    let merged_owner = project.element(merged).unwrap().owner_id;
    let relationship = project.create_package_merge(receiving, merged).unwrap();
    assert_eq!(
        project.relationship(relationship).unwrap().kind,
        RelationshipKind::PackageMerge
    );
    assert_eq!(project.element(merged).unwrap().owner_id, merged_owner);

    let block = project
        .create_element(ElementKind::Block, "Invalid target", project.root_id)
        .unwrap();
    assert!(matches!(
        project.create_package_merge(receiving, block),
        Err(ModelError::InvalidPackageRelationshipEndpoints { .. })
    ));
    project.validate().unwrap();
}

#[test]
fn package_relationships_reject_self_links_and_equivalent_duplicates_by_name() {
    let (mut project, importing, imported, _) = package_project();
    project
        .create_package_import(importing, imported, VisibilityKind::Public)
        .unwrap();

    let duplicate = project
        .create_package_import(importing, imported, VisibilityKind::Private)
        .unwrap_err();
    assert!(matches!(
        &duplicate,
        ModelError::DuplicatePackageRelationship { .. }
    ));
    let message = duplicate.to_string();
    assert!(message.contains("Vehicle"));
    assert!(message.contains("Powertrain"));
    assert!(!message.contains(&importing.to_string()));

    let self_import = project
        .create_package_import(importing, importing, VisibilityKind::Public)
        .unwrap_err();
    assert!(matches!(
        &self_import,
        ModelError::SelfPackageRelationship { .. }
    ));
    assert!(self_import.to_string().contains("Vehicle"));
}

#[test]
fn reconnect_validation_rejects_duplicates_and_accepts_a_new_target() {
    let (mut project, importing, imported, library) = package_project();
    let first = project
        .create_package_import(importing, imported, VisibilityKind::Public)
        .unwrap();
    let second = project
        .create_package_import(importing, library, VisibilityKind::Private)
        .unwrap();

    project.relationships.get_mut(&second).unwrap().target_id = imported;
    assert!(matches!(
        project.validate(),
        Err(ModelError::DuplicatePackageRelationship { .. })
    ));
    project.relationships.get_mut(&second).unwrap().target_id = library;
    project.validate().unwrap();

    project.relationships.get_mut(&first).unwrap().target_id = library;
    assert!(matches!(
        project.validate(),
        Err(ModelError::DuplicatePackageRelationship { .. })
    ));
}

#[test]
fn package_level_dependency_is_validated_as_package_semantics() {
    let (mut project, source, target, library) = package_project();
    project
        .create_relationship(RelationshipKind::Dependency, source, target, Some(source))
        .unwrap();
    assert!(matches!(
        project.create_relationship(RelationshipKind::Dependency, source, target, Some(source)),
        Err(ModelError::DuplicatePackageRelationship { .. })
    ));
    let comment = project
        .create_element(ElementKind::Comment, "Invalid dependency target", library)
        .unwrap();
    assert!(matches!(
        project.create_relationship(RelationshipKind::Dependency, source, comment, Some(source)),
        Err(ModelError::InvalidPackageRelationshipEndpoints { .. })
    ));
    project.validate().unwrap();
}
