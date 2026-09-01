use super::*;
use systems_modeler_core::{
    Connector, ConnectorEnd, ConnectorKind, ElementKind, Multiplicity, RelationshipKind,
};

#[test]
fn pr44_portable_json_round_trip_preserves_full_connector_topology() {
    let source = WorkspaceState::default();
    let activity = ActivityWorkspaceState::default();
    let mut project = Project::new("PR44 Portable Connector");
    let context = project
        .create_element(ElementKind::Block, "Vehicle", project.root_id)
        .unwrap();
    let subsystem_type = project
        .create_element(ElementKind::Block, "Subsystem", project.root_id)
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
    let subsystem = project
        .create_typed_feature(
            ElementKind::PartProperty,
            "subsystem",
            context,
            subsystem_type,
            Multiplicity::ONE,
        )
        .unwrap();
    let controller = project
        .create_typed_feature(
            ElementKind::ReferenceProperty,
            "controller",
            subsystem_type,
            controller_type,
            Multiplicity::ONE,
        )
        .unwrap();
    let peer = project
        .create_typed_feature(
            ElementKind::PartProperty,
            "peer",
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
    let id = project
        .create_connector(Connector {
            context_id: context,
            kind: ConnectorKind::Assembly,
            source: ConnectorEnd::nested_port(vec![subsystem, controller], port),
            target: ConnectorEnd::nested_port(vec![peer], port),
        })
        .unwrap();
    project.relationships.get_mut(&id).unwrap().external_id = "catia:pr44::CONN-NESTED".into();
    project.validate().unwrap();
    *source.project.lock().unwrap() = Some(project);

    let json = export_from_states(&source, &activity).unwrap();
    assert!(json.contains("CONN-NESTED"));
    assert!(json.contains("property_path"));

    let target = WorkspaceState::default();
    let target_activity = ActivityWorkspaceState::default();
    import_into_states(&json, &target, &target_activity).unwrap();
    let guard = target.project.lock().unwrap();
    let restored = guard.as_ref().unwrap();
    let relationship = restored.relationship(id).unwrap();
    let connector_payload = relationship.connector.as_ref().unwrap();

    assert_eq!(relationship.kind, RelationshipKind::Connector);
    assert_eq!(relationship.external_id, "catia:pr44::CONN-NESTED");
    assert_eq!(connector_payload.context_id, context);
    assert_eq!(connector_payload.kind, ConnectorKind::Assembly);
    assert_eq!(
        connector_payload.source.property_path,
        vec![subsystem, controller]
    );
    assert_eq!(connector_payload.source.role_id, controller);
    assert_eq!(connector_payload.source.port_id, Some(port));
    assert_eq!(connector_payload.target.property_path, vec![peer]);
    assert_eq!(connector_payload.target.role_id, peer);
    assert_eq!(connector_payload.target.port_id, Some(port));
    restored.validate().unwrap();
}
