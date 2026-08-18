use systems_modeler_core::{
    Connector, ConnectorEnd, ConnectorKind, ElementKind, ItemFlow, ModelError, Multiplicity,
    Project,
};

struct Fixture {
    project: Project,
    system: systems_modeler_core::ElementId,
    left_part: systems_modeler_core::ElementId,
    right_part: systems_modeler_core::ElementId,
    boundary: systems_modeler_core::ElementId,
    left_port: systems_modeler_core::ElementId,
    right_port: systems_modeler_core::ElementId,
    signal: systems_modeler_core::ElementId,
}

fn fixture() -> Fixture {
    let mut project = Project::new("IBD");
    let package = project
        .create_element(ElementKind::Package, "Structure", project.root_id)
        .unwrap();
    let interface = project
        .create_element(ElementKind::InterfaceBlock, "PowerInterface", package)
        .unwrap();
    let component = project
        .create_element(ElementKind::Block, "Component", package)
        .unwrap();
    let system = project
        .create_element(ElementKind::Block, "System", package)
        .unwrap();
    let signal = project
        .create_element(ElementKind::Signal, "Power", package)
        .unwrap();

    let left_port = project
        .create_typed_feature(
            ElementKind::ProxyPort,
            "power",
            component,
            interface,
            Multiplicity::ONE,
        )
        .unwrap();
    let right_port = project
        .create_typed_feature(
            ElementKind::ProxyPort,
            "power",
            component,
            interface,
            Multiplicity::ONE,
        )
        .unwrap();
    let boundary = project
        .create_typed_feature(
            ElementKind::ProxyPort,
            "externalPower",
            system,
            interface,
            Multiplicity::ONE,
        )
        .unwrap();
    let left_part = project
        .create_typed_feature(
            ElementKind::PartProperty,
            "left",
            system,
            component,
            Multiplicity::ONE,
        )
        .unwrap();
    let right_part = project
        .create_typed_feature(
            ElementKind::PartProperty,
            "right",
            system,
            component,
            Multiplicity::ONE,
        )
        .unwrap();

    Fixture {
        project,
        system,
        left_part,
        right_part,
        boundary,
        left_port,
        right_port,
        signal,
    }
}

#[test]
fn assembly_connector_uses_nested_port_paths_and_validates_types() {
    let mut f = fixture();
    let connector = Connector {
        context_id: f.system,
        kind: ConnectorKind::Assembly,
        source: ConnectorEnd::nested_port(vec![f.left_part], f.left_port),
        target: ConnectorEnd::nested_port(vec![f.right_part], f.right_port),
    };
    let id = f.project.create_connector(connector).unwrap();
    assert!(f.project.relationship(id).unwrap().connector.is_some());
    f.project.validate().unwrap();
}

#[test]
fn delegation_requires_boundary_to_internal_topology() {
    let mut f = fixture();
    let valid = Connector {
        context_id: f.system,
        kind: ConnectorKind::Delegation,
        source: ConnectorEnd::boundary(f.boundary),
        target: ConnectorEnd::nested_port(vec![f.left_part], f.left_port),
    };
    f.project.create_connector(valid).unwrap();

    let invalid = Connector {
        context_id: f.system,
        kind: ConnectorKind::Delegation,
        source: ConnectorEnd::nested_port(vec![f.left_part], f.left_port),
        target: ConnectorEnd::nested_port(vec![f.right_part], f.right_port),
    };
    assert_eq!(
        f.project.validate_connector(&invalid).unwrap_err(),
        ModelError::DelegationRequiresBoundaryAndInternal
    );
}

#[test]
fn item_flow_requires_classifier_and_realizes_connector() {
    let mut f = fixture();
    let source = ConnectorEnd::nested_port(vec![f.left_part], f.left_port);
    let target = ConnectorEnd::nested_port(vec![f.right_part], f.right_port);
    let connector_id = f
        .project
        .create_connector(Connector {
            context_id: f.system,
            kind: ConnectorKind::Assembly,
            source: source.clone(),
            target: target.clone(),
        })
        .unwrap();
    let flow_id = f
        .project
        .create_item_flow(ItemFlow {
            connector_id,
            source,
            target,
            conveyed_item_ids: vec![f.signal],
        })
        .unwrap();
    assert!(f.project.relationship(flow_id).unwrap().item_flow.is_some());
    f.project.validate().unwrap();
}

#[test]
fn full_ports_cannot_be_conjugated() {
    let mut f = fixture();
    let package = f.project.element(f.system).unwrap().owner_id.unwrap();
    let interface = f.project.element(f.boundary).unwrap().type_id.unwrap();
    let full = f
        .project
        .create_typed_feature(
            ElementKind::FullPort,
            "full",
            f.system,
            interface,
            Multiplicity::ONE,
        )
        .unwrap();
    f.project.element_mut(full).unwrap().is_conjugated = true;
    assert_eq!(
        f.project.validate_element(full).unwrap_err(),
        ModelError::FullPortCannotBeConjugated(full)
    );
    assert!(f.project.element(package).is_ok());
}
