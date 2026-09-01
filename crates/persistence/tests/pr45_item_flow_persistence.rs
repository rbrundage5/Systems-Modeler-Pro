use systems_modeler_core::{
    Connector, ConnectorEnd, ConnectorKind, ElementKind, ItemFlow, Multiplicity, Project,
    RelationshipKind,
};
use systems_modeler_persistence::ProjectDatabase;

#[test]
fn pr45_database_round_trip_preserves_complete_item_flow_payload() {
    let mut project = Project::new("PR45 ItemFlow Persistence");
    let context = project
        .create_element(ElementKind::Block, "Vehicle", project.root_id)
        .unwrap();
    let component = project
        .create_element(ElementKind::Block, "Component", project.root_id)
        .unwrap();
    let interface = project
        .create_element(ElementKind::InterfaceBlock, "Traffic", project.root_id)
        .unwrap();
    let signal = project
        .create_element(ElementKind::Signal, "StatusSignal", project.root_id)
        .unwrap();
    let left = project
        .create_typed_feature(
            ElementKind::PartProperty,
            "left",
            context,
            component,
            Multiplicity::ONE,
        )
        .unwrap();
    let right = project
        .create_typed_feature(
            ElementKind::PartProperty,
            "right",
            context,
            component,
            Multiplicity::ONE,
        )
        .unwrap();
    let port = project
        .create_typed_feature(
            ElementKind::ProxyPort,
            "traffic",
            component,
            interface,
            Multiplicity::ONE,
        )
        .unwrap();
    let source = ConnectorEnd::nested_port(vec![left], port);
    let target = ConnectorEnd::nested_port(vec![right], port);
    let connector = project
        .create_connector(Connector {
            context_id: context,
            kind: ConnectorKind::Assembly,
            source: source.clone(),
            target: target.clone(),
        })
        .unwrap();
    project
        .relationships
        .get_mut(&connector)
        .unwrap()
        .external_id = "catia:pr45::CONN-1".into();
    let flow = project
        .create_item_flow(ItemFlow {
            connector_id: connector,
            source,
            target,
            conveyed_item_ids: vec![signal],
        })
        .unwrap();
    {
        let relationship = project.relationships.get_mut(&flow).unwrap();
        relationship.external_id = "catia:pr45::FLOW-1".into();
        relationship.name = "status traffic".into();
        relationship.documentation = "Imported ItemFlow".into();
    }
    project.validate().unwrap();

    let mut database = ProjectDatabase::open_in_memory().unwrap();
    database.save_project(&project).unwrap();
    let restored = database.load_project(project.id).unwrap();
    let relationship = restored.relationship(flow).unwrap();
    let payload = relationship.item_flow.as_ref().unwrap();
    assert_eq!(relationship.kind, RelationshipKind::ItemFlow);
    assert_eq!(relationship.external_id, "catia:pr45::FLOW-1");
    assert_eq!(relationship.owner_id, Some(context));
    assert_eq!(payload.connector_id, connector);
    assert_eq!(payload.conveyed_item_ids, vec![signal]);
    assert_eq!(payload.source.property_path, vec![left]);
    assert_eq!(payload.target.property_path, vec![right]);
    restored.validate().unwrap();
}
