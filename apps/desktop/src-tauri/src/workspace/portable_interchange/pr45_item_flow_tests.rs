use super::*;
use systems_modeler_core::{
    Connector, ConnectorEnd, ConnectorKind, ElementKind, ItemFlow, Multiplicity, RelationshipKind,
};

#[test]
fn pr45_portable_json_round_trip_preserves_item_flow_semantics() {
    let source_state = WorkspaceState::default();
    let source_activity = ActivityWorkspaceState::default();
    let mut project = Project::new("PR45 Portable ItemFlow");
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
    let packet = project
        .create_element(ElementKind::DataType, "TelemetryPacket", project.root_id)
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
            ElementKind::ReferenceProperty,
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
        .external_id = "catia:pr45::CONN-TRAFFIC".into();
    let flow = project
        .create_item_flow(ItemFlow {
            connector_id: connector,
            source: target,
            target: source,
            conveyed_item_ids: vec![signal, packet],
        })
        .unwrap();
    {
        let relationship = project.relationships.get_mut(&flow).unwrap();
        relationship.external_id = "catia:pr45::FLOW-TRAFFIC".into();
        relationship.name = "reverse traffic".into();
        relationship.documentation = "Portable ItemFlow".into();
    }
    project.validate().unwrap();
    *source_state.project.lock().unwrap() = Some(project);

    let json = export_from_states(&source_state, &source_activity).unwrap();
    let target_state = WorkspaceState::default();
    let target_activity = ActivityWorkspaceState::default();
    import_into_states(&json, &target_state, &target_activity).unwrap();
    let guard = target_state.project.lock().unwrap();
    let restored = guard.as_ref().unwrap();
    let relationship = restored.relationship(flow).unwrap();
    let payload = relationship.item_flow.as_ref().unwrap();
    assert_eq!(relationship.kind, RelationshipKind::ItemFlow);
    assert_eq!(relationship.external_id, "catia:pr45::FLOW-TRAFFIC");
    assert_eq!(relationship.name, "reverse traffic");
    assert_eq!(payload.connector_id, connector);
    assert_eq!(payload.conveyed_item_ids, vec![signal, packet]);
    assert_eq!(payload.source.property_path, vec![right]);
    assert_eq!(payload.target.property_path, vec![left]);
    restored.validate().unwrap();
}
