use systems_modeler_core::{
    Connector, ConnectorEnd, ConnectorKind, ElementKind, Multiplicity, Project,
};
use systems_modeler_persistence::ProjectDatabase;

#[test]
fn pr44_native_database_round_trip_preserves_complete_connector_payload() {
    let mut project = Project::new("PR44 Connector Persistence");
    let context = project
        .create_element(ElementKind::Block, "Vehicle", project.root_id)
        .unwrap();
    let controller_type = project
        .create_element(ElementKind::Block, "Controller", project.root_id)
        .unwrap();
    let interface = project
        .create_element(
            ElementKind::InterfaceBlock,
            "CommandInterface",
            project.root_id,
        )
        .unwrap();
    let left = project
        .create_typed_feature(
            ElementKind::PartProperty,
            "leftController",
            context,
            controller_type,
            Multiplicity::ONE,
        )
        .unwrap();
    let right = project
        .create_typed_feature(
            ElementKind::ReferenceProperty,
            "rightController",
            context,
            controller_type,
            Multiplicity::ONE,
        )
        .unwrap();
    let port = project
        .create_typed_feature(
            ElementKind::ProxyPort,
            "command",
            controller_type,
            interface,
            Multiplicity::ONE,
        )
        .unwrap();
    let connector = project
        .create_connector(Connector {
            association_type_id: None,
            end_multiplicities: Default::default(),
            context_id: context,
            kind: ConnectorKind::Assembly,
            source: ConnectorEnd::nested_port(vec![left], port),
            target: ConnectorEnd::nested_port(vec![right], port),
        })
        .unwrap();
    {
        let relationship = project.relationships.get_mut(&connector).unwrap();
        relationship.external_id = "catia:pr44::CONN-1".into();
        relationship.name = "controller command bus".into();
        relationship.documentation = "Imported connector topology".into();
    }
    project.validate().unwrap();

    let association = project
        .create_association(
            Some(project.root_id),
            vec![
                Project::association_end(
                    interface,
                    "sender",
                    Multiplicity::new(0, None).unwrap(),
                    true,
                    systems_modeler_core::AggregationKind::None,
                ),
                Project::association_end(
                    interface,
                    "receiver",
                    Multiplicity::new(0, None).unwrap(),
                    true,
                    systems_modeler_core::AggregationKind::None,
                ),
            ],
        )
        .unwrap();
    let semantics = project
        .relationships
        .get_mut(&connector)
        .unwrap()
        .connector
        .as_mut()
        .unwrap();
    semantics.association_type_id = Some(association);
    semantics.end_multiplicities = [Multiplicity::ONE, Multiplicity::new(1, Some(4)).unwrap()];
    let mut database = ProjectDatabase::open_in_memory().unwrap();
    database.save_project(&project).unwrap();
    let restored = database.load_project(project.id).unwrap();
    let relationship = restored.relationship(connector).unwrap();
    let payload = relationship.connector.as_ref().unwrap();

    assert_eq!(relationship.external_id, "catia:pr44::CONN-1");
    assert_eq!(relationship.owner_id, Some(context));
    assert_eq!(relationship.source_id, port);
    assert_eq!(relationship.target_id, port);
    assert_eq!(payload.context_id, context);
    assert_eq!(payload.kind, ConnectorKind::Assembly);
    assert_eq!(payload.association_type_id, Some(association));
    assert_eq!(
        payload.end_multiplicities,
        [Multiplicity::ONE, Multiplicity::new(1, Some(4)).unwrap()]
    );
    assert_eq!(payload.source.property_path, vec![left]);
    assert_eq!(payload.source.role_id, left);
    assert_eq!(payload.source.port_id, Some(port));
    assert_eq!(payload.target.property_path, vec![right]);
    assert_eq!(payload.target.role_id, right);
    assert_eq!(payload.target.port_id, Some(port));
    restored.validate().unwrap();
}
