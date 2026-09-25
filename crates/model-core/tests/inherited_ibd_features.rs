use systems_modeler_core::{
    Connector, ConnectorEnd, ConnectorKind, ElementId, ElementKind, ModelError, Multiplicity,
    Project, RelationshipKind, VisibilityKind,
};

fn block(project: &mut Project, name: &str) -> ElementId {
    project
        .create_element(ElementKind::Block, name, project.root_id)
        .unwrap()
}

fn generalize(project: &mut Project, child: ElementId, parent: ElementId) {
    project
        .create_relationship(
            RelationshipKind::Generalization,
            child,
            parent,
            Some(project.root_id),
        )
        .unwrap();
}

fn typed(
    project: &mut Project,
    kind: ElementKind,
    name: &str,
    owner: ElementId,
    type_id: ElementId,
) -> ElementId {
    project
        .create_typed_feature(kind, name, owner, type_id, Multiplicity::ONE)
        .unwrap()
}

#[test]
fn inherited_parts_and_ports_form_valid_delegation_without_copying_features() {
    let mut project = Project::new("Inherited IBD");
    let base = block(&mut project, "Base");
    let child = block(&mut project, "Child");
    let component_base = block(&mut project, "ComponentBase");
    let component = block(&mut project, "Component");
    let interface = project
        .create_element(ElementKind::InterfaceBlock, "Interface", project.root_id)
        .unwrap();
    generalize(&mut project, child, base);
    generalize(&mut project, component, component_base);
    let part = typed(
        &mut project,
        ElementKind::PartProperty,
        "component",
        base,
        component,
    );
    let boundary = typed(
        &mut project,
        ElementKind::ProxyPort,
        "external",
        base,
        interface,
    );
    let port = typed(
        &mut project,
        ElementKind::ProxyPort,
        "internal",
        component_base,
        interface,
    );
    assert_eq!(
        project.resolve_structural_path(child, &[part]).unwrap(),
        component
    );
    let connector = Connector {
        association_type_id: None,
        end_multiplicities: Default::default(),
        context_id: child,
        kind: ConnectorKind::Delegation,
        source: ConnectorEnd::boundary(boundary),
        target: ConnectorEnd::nested_port(vec![part], port),
    };
    let id = project.create_connector(connector).unwrap();
    project.validate().unwrap();
    let reopened: Project =
        serde_json::from_value(serde_json::to_value(&project).unwrap()).unwrap();
    reopened.validate().unwrap();
    assert_eq!(reopened.element(part).unwrap().owner_id, Some(base));
    assert_eq!(
        reopened.element(port).unwrap().owner_id,
        Some(component_base)
    );
    assert_eq!(
        reopened
            .relationship(id)
            .unwrap()
            .connector
            .as_ref()
            .unwrap()
            .target
            .port_id,
        Some(port)
    );
}

#[test]
fn private_and_unrelated_features_reject_without_mutating_relationships() {
    let mut project = Project::new("Invalid inherited path");
    let base = block(&mut project, "Base");
    let child = block(&mut project, "Child");
    let unrelated = block(&mut project, "Unrelated");
    let component = block(&mut project, "Component");
    generalize(&mut project, child, base);
    let part = typed(
        &mut project,
        ElementKind::PartProperty,
        "part",
        base,
        component,
    );
    let other = typed(
        &mut project,
        ElementKind::PartProperty,
        "other",
        child,
        component,
    );
    project.elements.get_mut(&part).unwrap().visibility = VisibilityKind::Private;
    assert!(!project.has_classifier_feature(child, part).unwrap());
    assert!(project.has_classifier_feature(base, part).unwrap());
    for context in [child, unrelated] {
        let before = serde_json::to_value(&project).unwrap();
        assert_eq!(
            project.resolve_structural_path(context, &[part]),
            Err(ModelError::InvalidConnectorPath(part))
        );
        assert!(
            project
                .create_connector(Connector {
                    association_type_id: None,
                    end_multiplicities: Default::default(),
                    context_id: context,
                    kind: ConnectorKind::Assembly,
                    source: ConnectorEnd::role(part),
                    target: ConnectorEnd::role(other),
                })
                .is_err()
        );
        assert_eq!(serde_json::to_value(&project).unwrap(), before);
    }
    project.elements.get_mut(&part).unwrap().visibility = VisibilityKind::Public;
    assert_eq!(
        project.resolve_structural_path(child, &[part]).unwrap(),
        component
    );
    project.relationships.clear();
    assert!(project.resolve_structural_path(child, &[part]).is_err());
}

#[test]
fn diamond_features_keep_original_identity_and_private_features_remain_local() {
    let mut project = Project::new("Feature membership");
    let base = block(&mut project, "Base");
    let left = block(&mut project, "Left");
    let right = block(&mut project, "Right");
    let child = block(&mut project, "Child");
    for (specific, general) in [(left, base), (right, base), (child, left), (child, right)] {
        generalize(&mut project, specific, general);
    }
    let operation = project
        .create_element(ElementKind::Operation, "run", base)
        .unwrap();
    let hidden = project
        .create_element(ElementKind::Operation, "hidden", base)
        .unwrap();
    project.elements.get_mut(&hidden).unwrap().visibility = VisibilityKind::Private;
    let local = project
        .create_element(ElementKind::Operation, "hidden", child)
        .unwrap();
    project.elements.get_mut(&local).unwrap().visibility = VisibilityKind::Private;
    let before = serde_json::to_value(&project).unwrap();
    let ids: Vec<_> = project
        .classifier_features(child)
        .unwrap()
        .iter()
        .map(|feature| feature.id)
        .collect();
    assert_eq!(ids.len(), 2);
    assert_eq!(ids.iter().filter(|id| **id == operation).count(), 1);
    assert!(ids.contains(&local));
    assert!(!ids.contains(&hidden));
    assert_eq!(serde_json::to_value(&project).unwrap(), before);
}
