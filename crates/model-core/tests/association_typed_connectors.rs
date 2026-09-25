use systems_modeler_core::{
    AggregationKind, Connector, ConnectorEnd, ConnectorKind, ElementKind, Multiplicity, Project,
    RelationshipId, RelationshipKind,
};

fn fixture() -> (Project, Connector) {
    let mut project = Project::new("Generic typed connections");
    let root = project.root_id;
    let context = project
        .create_element(ElementKind::Block, "Installation", root)
        .unwrap();
    let source_type = project
        .create_element(ElementKind::Block, "Producer", root)
        .unwrap();
    let specific = project
        .create_element(ElementKind::Block, "SpecializedProducer", root)
        .unwrap();
    let target_type = project
        .create_element(ElementKind::Block, "Consumer", root)
        .unwrap();
    project
        .create_relationship(
            RelationshipKind::Generalization,
            specific,
            source_type,
            Some(root),
        )
        .unwrap();
    let source = project
        .create_typed_feature(
            ElementKind::PartProperty,
            "supply",
            context,
            specific,
            Multiplicity::new(4, Some(4)).unwrap(),
        )
        .unwrap();
    let target = project
        .create_typed_feature(
            ElementKind::PartProperty,
            "load",
            context,
            target_type,
            Multiplicity::ONE,
        )
        .unwrap();
    let type_id = project
        .create_association(
            Some(root),
            vec![
                Project::association_end(
                    source_type,
                    "provider",
                    Multiplicity::new(0, None).unwrap(),
                    true,
                    AggregationKind::None,
                ),
                Project::association_end(
                    target_type,
                    "client",
                    Multiplicity::new(1, Some(3)).unwrap(),
                    true,
                    AggregationKind::None,
                ),
            ],
        )
        .unwrap();
    let connector = Connector {
        context_id: context,
        kind: ConnectorKind::Assembly,
        source: ConnectorEnd::role(source),
        target: ConnectorEnd::role(target),
        association_type_id: Some(type_id),
        end_multiplicities: [Multiplicity::ONE, Multiplicity::new(2, Some(3)).unwrap()],
    };
    (project, connector)
}

#[test]
fn runtime_realizes_only_links_consistent_with_explicit_connector_end_bounds() {
    use systems_modeler_core::{
        StructuralRuntime, StructuralRuntimeConfiguration, StructuralRuntimeError,
    };
    let (mut project, mut connector) = fixture();
    let context = connector.context_id;
    connector.end_multiplicities = [Multiplicity::new(4, Some(4)).unwrap(), Multiplicity::ONE];
    let id = project.create_connector(connector).unwrap();
    let before = serde_json::to_value(&project).unwrap();
    let configuration = StructuralRuntimeConfiguration::default();
    let runtime = StructuralRuntime::build(&project, context, &configuration).unwrap();
    assert_eq!(runtime.connector_links.len(), 4);
    let repeated = StructuralRuntime::build(&project, context, &configuration).unwrap();
    assert_eq!(
        runtime
            .connector_links
            .iter()
            .map(|link| link.id)
            .collect::<Vec<_>>(),
        repeated
            .connector_links
            .iter()
            .map(|link| link.id)
            .collect::<Vec<_>>()
    );
    assert_eq!(serde_json::to_value(&project).unwrap(), before);
    project
        .relationships
        .get_mut(&id)
        .unwrap()
        .connector
        .as_mut()
        .unwrap()
        .end_multiplicities[0] = Multiplicity::ONE;
    project.validate().unwrap();
    let before_failure = serde_json::to_value(&project).unwrap();
    let error = StructuralRuntime::build(&project, context, &configuration).unwrap_err();
    assert!(matches!(
        error,
        StructuralRuntimeError::ConnectorEndCardinality { count: 4, .. }
    ));
    assert_eq!(serde_json::to_value(&project).unwrap(), before_failure);
}

#[test]
fn optional_absent_typed_endpoint_does_not_fabricate_runtime_links() {
    use systems_modeler_core::{StructuralRuntime, StructuralRuntimeConfiguration};
    let (mut project, mut connector) = fixture();
    let context = connector.context_id;
    project
        .element_mut(connector.source.role_id)
        .unwrap()
        .multiplicity = Some(Multiplicity::new(0, Some(0)).unwrap());
    connector.end_multiplicities = [Multiplicity::new(0, Some(1)).unwrap(), Multiplicity::ONE];
    project.create_connector(connector).unwrap();
    let runtime = StructuralRuntime::build(
        &project,
        context,
        &StructuralRuntimeConfiguration::default(),
    )
    .unwrap();
    assert!(runtime.connector_links.is_empty());
}

#[test]
fn association_types_accept_distinct_subtyped_roles_and_independent_multiplicities() {
    let (mut project, connector) = fixture();
    let id = project.create_connector(connector.clone()).unwrap();
    project.validate().unwrap();
    assert_eq!(
        project.relationship(id).unwrap().connector.as_ref(),
        Some(&connector)
    );
    assert_eq!(
        project
            .element(connector.source.role_id)
            .unwrap()
            .multiplicity
            .unwrap()
            .lower,
        4
    );
    let restored: Project =
        serde_json::from_str(&serde_json::to_string(&project).unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(
        restored.relationship(id).unwrap().connector.as_ref(),
        Some(&connector)
    );
}

#[test]
fn typed_connector_rejects_order_type_arity_and_multiplicity_without_mutation() {
    let (project, connector) = fixture();
    let type_id = connector.association_type_id.unwrap();
    let before = serde_json::to_value(&project).unwrap();
    let mut cases = Vec::new();
    let mut swapped = connector.clone();
    std::mem::swap(&mut swapped.source, &mut swapped.target);
    cases.push(swapped);
    let mut missing = connector.clone();
    missing.association_type_id = Some(RelationshipId::new());
    cases.push(missing);
    for multiplicity in [
        Multiplicity::new(0, Some(3)).unwrap(),
        Multiplicity::new(1, None).unwrap(),
        Multiplicity::new(1, Some(4)).unwrap(),
    ] {
        let mut broad = connector.clone();
        broad.end_multiplicities[1] = multiplicity;
        cases.push(broad);
    }
    for invalid in cases {
        let mut candidate = project.clone();
        assert!(candidate.create_connector(invalid).is_err());
        assert_eq!(serde_json::to_value(candidate).unwrap(), before);
    }
    let mut wrong_kind = project.clone();
    wrong_kind.relationships.get_mut(&type_id).unwrap().kind = RelationshipKind::Dependency;
    assert!(wrong_kind.validate_connector(&connector).is_err());
    let mut no_ends = project.clone();
    no_ends
        .relationships
        .get_mut(&type_id)
        .unwrap()
        .association_ends
        .clear();
    assert!(no_ends.validate_connector(&connector).is_err());
}

#[test]
fn old_connector_json_keeps_identity_and_gets_untyped_unit_multiplicities() {
    let (_, connector) = fixture();
    let mut value = serde_json::to_value(&connector).unwrap();
    value.as_object_mut().unwrap().remove("association_type_id");
    value.as_object_mut().unwrap().remove("end_multiplicities");
    let restored: Connector = serde_json::from_value(value).unwrap();
    assert_eq!(restored.context_id, connector.context_id);
    assert_eq!(restored.source, connector.source);
    assert_eq!(restored.target, connector.target);
    assert_eq!(restored.association_type_id, None);
    assert_eq!(restored.end_multiplicities, [Multiplicity::ONE; 2]);
}

#[test]
fn association_changes_and_deletion_are_validated_through_all_dependents() {
    let (mut project, connector) = fixture();
    let type_id = connector.association_type_id.unwrap();
    project.create_connector(connector).unwrap();
    let before = serde_json::to_value(&project).unwrap();
    let mut deleted = project.clone();
    deleted.relationships.remove(&type_id);
    assert!(
        deleted
            .validate()
            .unwrap_err()
            .to_string()
            .contains("retype")
    );
    let mut narrowed = project.clone();
    narrowed
        .relationships
        .get_mut(&type_id)
        .unwrap()
        .association_ends[1]
        .multiplicity = Multiplicity::ONE;
    assert!(narrowed.validate().is_err());
    assert_eq!(serde_json::to_value(project).unwrap(), before);
}
