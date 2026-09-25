use systems_modeler_core::{ElementKind, ModelError, Project, RelationshipKind, VisibilityKind};

#[test]
fn pr47_native_use_case_and_namespace_relationship_rules_remain_authoritative() {
    let mut project = Project::new("PR47 Core");
    let root = project.root_id;
    let uc_pkg = project
        .create_element(ElementKind::Package, "UseCases", root)
        .unwrap();
    let base = project
        .create_element(ElementKind::UseCase, "Base", uc_pkg)
        .unwrap();
    project.element_mut(base).unwrap().extension_points = vec!["point".into()];
    let extension = project
        .create_element(ElementKind::UseCase, "Extension", uc_pkg)
        .unwrap();
    let included = project
        .create_element(ElementKind::UseCase, "Included", uc_pkg)
        .unwrap();
    let include = project
        .create_relationship(RelationshipKind::Include, base, included, Some(uc_pkg))
        .unwrap();
    assert_eq!(project.relationship(include).unwrap().source_id, base);
    let extend = project
        .create_relationship(RelationshipKind::Extend, extension, base, Some(uc_pkg))
        .unwrap();
    project
        .update_extend_relationship(extend, Some("guard".into()), Some("point".into()))
        .unwrap();
    assert!(matches!(
        project.create_relationship(RelationshipKind::Include, base, base, Some(uc_pkg)),
        Err(ModelError::SelfUseCaseRelationship)
    ));
    assert!(matches!(
        project.update_extend_relationship(extend, None, Some("missing".into())),
        Err(ModelError::ExtensionPointNotFound { .. })
    ));

    let vehicle = project
        .create_element(ElementKind::Package, "Vehicle", root)
        .unwrap();
    let common = project
        .create_element(ElementKind::Package, "Common", root)
        .unwrap();
    let signal = project
        .create_element(ElementKind::Signal, "Command", common)
        .unwrap();
    let pi = project
        .create_package_import(vehicle, common, VisibilityKind::Private)
        .unwrap();
    let ei = project
        .create_element_import(vehicle, signal, VisibilityKind::Public, Some("Cmd".into()))
        .unwrap();
    let pm = project
        .create_relationship(
            RelationshipKind::PackageMerge,
            vehicle,
            common,
            Some(vehicle),
        )
        .unwrap();
    assert_eq!(project.relationship(pi).unwrap().owner_id, Some(vehicle));
    assert_eq!(
        project.relationship(ei).unwrap().alias.as_deref(),
        Some("Cmd")
    );
    assert_eq!(project.relationship(pm).unwrap().owner_id, Some(vehicle));
    assert!(
        project
            .create_relationship(
                RelationshipKind::PackageImport,
                vehicle,
                common,
                Some(vehicle)
            )
            .is_err()
    );
    project.validate().unwrap();
}
