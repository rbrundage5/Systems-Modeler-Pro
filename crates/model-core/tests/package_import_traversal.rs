use systems_modeler_core::{
    ElementId, ElementKind, NamespaceResolutionError, Project, RelationshipId, VisibilityKind,
};

fn package(project: &mut Project, name: &str) -> ElementId {
    project.create_element(ElementKind::Package, name, project.root_id).unwrap()
}

fn import(project: &mut Project, source: ElementId, target: ElementId) -> RelationshipId {
    project.create_package_import(source, target, VisibilityKind::Public).unwrap()
}

#[test]
fn repeated_diamonds_visit_each_export_once_without_expanding_every_path() {
    let mut project = Project::new("Shared libraries");
    let consumer = package(&mut project, "Consumer");
    let mut previous = vec![consumer];
    for level in 0..35 {
        let next: Vec<_> = (0..2)
            .map(|side| package(&mut project, &format!("Layer {level} {side}")))
            .collect();
        for source in previous {
            for target in &next {
                import(&mut project, source, *target);
            }
        }
        previous = next;
    }
    let library = package(&mut project, "Library");
    let member = project.create_element(ElementKind::Signal, "Payload", library).unwrap();
    for source in previous {
        import(&mut project, source, library);
        project.create_element_import(source, member, VisibilityKind::Public, Some("Data".into())).unwrap();
    }
    // An import cycle must not bring local bindings back into the export walk.
    import(&mut project, library, consumer);
    project.validate().unwrap();
    let before = serde_json::to_value(&project).unwrap();
    assert_eq!(project.resolve_name(consumer, "Payload").unwrap(), member);
    assert_eq!(project.resolve_name(consumer, "Data").unwrap(), member);
    let visible = project.visible_members(consumer).unwrap();
    assert_eq!(visible.iter().filter(|id| **id == member).count(), 1);
    let reopened: Project = serde_json::from_value(before.clone()).unwrap();
    assert_eq!(reopened.visible_members(consumer).unwrap(), visible);
    assert_eq!(serde_json::to_value(&project).unwrap(), before);
}

#[test]
fn chained_imports_do_not_use_the_call_stack_or_rescan_each_library() {
    let mut project = Project::new("Deep imports");
    let libraries: Vec<_> = (0..20_001)
        .map(|index| package(&mut project, &format!("Library {index}")))
        .collect();
    let first = import(&mut project, libraries[0], libraries[1]);
    let template = project.relationship(first).unwrap().clone();
    for index in 1..20_000 {
        let mut edge = template.clone();
        edge.id = RelationshipId::new();
        edge.external_id = edge.id.to_string();
        edge.source_id = libraries[index];
        edge.target_id = libraries[index + 1];
        edge.owner_id = Some(libraries[index]);
        project.relationships.insert(edge.id, edge);
    }
    let member = project.create_element(ElementKind::Signal, "Payload", libraries[20_000]).unwrap();
    assert_eq!(project.resolve_name(libraries[0], "Payload").unwrap(), member);
    assert_eq!(project.visible_members(libraries[0]).unwrap(), vec![member]);
}

#[test]
fn visibility_alias_ambiguity_and_owned_precedence_survive_fresh_queries() {
    let mut project = Project::new("Visibility");
    let consumer = package(&mut project, "Consumer");
    let middle = package(&mut project, "Middle");
    let library = package(&mut project, "Library");
    let other = package(&mut project, "Other");
    let member = project.create_element(ElementKind::Signal, "Payload", library).unwrap();
    let other_member = project.create_element(ElementKind::Signal, "Payload", other).unwrap();
    import(&mut project, consumer, middle);
    let edge = project.create_package_import(middle, library, VisibilityKind::Private).unwrap();
    assert_eq!(project.resolve_name(middle, "Payload").unwrap(), member);
    assert!(matches!(project.resolve_name(consumer, "Payload"), Err(NamespaceResolutionError::NotFound { .. })));
    project.relationships.get_mut(&edge).unwrap().visibility = VisibilityKind::Public;
    assert_eq!(project.resolve_name(consumer, "Payload").unwrap(), member);
    project.rename_element(member, "Renamed").unwrap();
    assert!(project.resolve_name(consumer, "Payload").is_err());
    assert_eq!(project.resolve_name(consumer, "Renamed").unwrap(), member);
    project.elements.get_mut(&member).unwrap().visibility = VisibilityKind::Private;
    assert!(project.resolve_name(consumer, "Renamed").is_err());
    project.elements.get_mut(&member).unwrap().visibility = VisibilityKind::Public;
    project.create_element_import(middle, member, VisibilityKind::Public, Some("Alias".into())).unwrap();
    project.create_element_import(middle, other_member, VisibilityKind::Public, Some("Alias".into())).unwrap();
    assert!(matches!(project.resolve_name(consumer, "Alias"), Err(NamespaceResolutionError::Ambiguous { candidates, .. }) if candidates.len() == 2));
    let local = project.create_element(ElementKind::Signal, "Alias", consumer).unwrap();
    assert_eq!(project.resolve_name(consumer, "Alias").unwrap(), local);
}

#[test]
fn malformed_missing_and_non_namespace_targets_do_not_hide_valid_exports() {
    let mut project = Project::new("Defensive imports");
    let consumer = package(&mut project, "Consumer");
    let library = package(&mut project, "Library");
    let member = project.create_element(ElementKind::Signal, "Payload", library).unwrap();
    let id = import(&mut project, consumer, library);
    let template = project.relationship(id).unwrap().clone();
    for target in [ElementId::new(), member] {
        let mut edge = template.clone();
        edge.id = RelationshipId::new();
        edge.external_id = edge.id.to_string();
        edge.target_id = target;
        project.relationships.insert(edge.id, edge);
    }
    assert_eq!(project.resolve_name(consumer, "Payload").unwrap(), member);
    assert!(project.validate().is_err());
}
