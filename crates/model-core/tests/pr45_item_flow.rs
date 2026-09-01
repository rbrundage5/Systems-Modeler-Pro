use systems_modeler_core::{
    Connector, ConnectorEnd, ConnectorKind, ElementKind, ItemFlow, ModelError, Multiplicity,
    Project,
};

fn fixture() -> (
    Project,
    systems_modeler_core::RelationshipId,
    ConnectorEnd,
    ConnectorEnd,
    systems_modeler_core::ElementId,
) {
    let mut project = Project::new("PR45 ItemFlow");
    let context = project
        .create_element(ElementKind::Block, "Vehicle", project.root_id)
        .unwrap();
    let component = project
        .create_element(ElementKind::Block, "Component", project.root_id)
        .unwrap();
    let interface = project
        .create_element(ElementKind::InterfaceBlock, "Command", project.root_id)
        .unwrap();
    let signal = project
        .create_element(ElementKind::Signal, "CommandSignal", project.root_id)
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
            "command",
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
    (project, connector, source, target, signal)
}

#[test]
fn pr45_native_item_flow_accepts_both_connector_orientations() {
    let (mut project, connector, source, target, signal) = fixture();
    project
        .create_item_flow(ItemFlow {
            connector_id: connector,
            source: source.clone(),
            target: target.clone(),
            conveyed_item_ids: vec![signal],
        })
        .unwrap();
    project
        .create_item_flow(ItemFlow {
            connector_id: connector,
            source: target,
            target: source,
            conveyed_item_ids: vec![signal],
        })
        .unwrap();
    project.validate().unwrap();
}

#[test]
fn pr45_native_item_flow_rejects_unrelated_ends_and_duplicate_items() {
    let (mut project, connector, source, target, signal) = fixture();
    let context = project.relationship(connector).unwrap().owner_id.unwrap();
    let interface = project.element(source.port_id.unwrap()).unwrap().type_id.unwrap();
    let unrelated = ConnectorEnd::boundary(
        project
            .create_typed_feature(
                ElementKind::ProxyPort,
                "boundary",
                context,
                interface,
                Multiplicity::ONE,
            )
            .unwrap(),
    );
    assert_eq!(
        project
            .validate_item_flow(&ItemFlow {
                connector_id: connector,
                source: unrelated,
                target: target.clone(),
                conveyed_item_ids: vec![signal],
            })
            .unwrap_err(),
        ModelError::ItemFlowEndpointsDoNotMatchConnector(connector)
    );
    assert_eq!(
        project
            .validate_item_flow(&ItemFlow {
                connector_id: connector,
                source,
                target,
                conveyed_item_ids: vec![signal, signal],
            })
            .unwrap_err(),
        ModelError::DuplicateConveyedItem(signal)
    );
}
