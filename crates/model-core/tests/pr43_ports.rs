use systems_modeler_core::{ElementKind, ModelError, Multiplicity, Project};

#[test]
fn pr43_proxy_and_full_ports_use_native_owner_type_multiplicity_and_conjugation_rules() {
    let mut project = Project::new("PR43 Native Ports");
    let package = project
        .create_element(ElementKind::Package, "Architecture", project.root_id)
        .unwrap();
    let owner = project
        .create_element(ElementKind::Block, "Controller", package)
        .unwrap();
    let iface = project
        .create_element(ElementKind::InterfaceBlock, "CommandInterface", package)
        .unwrap();
    let service = project
        .create_element(ElementKind::DataType, "ServiceAssembly", package)
        .unwrap();
    let invalid_type = project
        .create_element(ElementKind::ValueType, "InvalidPortType", package)
        .unwrap();
    let requirement = project
        .create_requirement("Requirement", "REQ-1", "text", package)
        .unwrap();

    let proxy = project
        .create_typed_feature(
            ElementKind::ProxyPort,
            "command",
            owner,
            iface,
            Multiplicity::new(0, None).unwrap(),
        )
        .unwrap();
    project.element_mut(proxy).unwrap().is_conjugated = true;
    project.validate_element(proxy).unwrap();
    let proxy_element = project.element(proxy).unwrap();
    assert_eq!(proxy_element.kind, ElementKind::ProxyPort);
    assert_eq!(proxy_element.owner_id, Some(owner));
    assert_eq!(proxy_element.type_id, Some(iface));
    assert_eq!(
        proxy_element.multiplicity,
        Some(Multiplicity::new(0, None).unwrap())
    );
    assert!(proxy_element.is_conjugated);

    let full = project
        .create_typed_feature(
            ElementKind::FullPort,
            "service",
            owner,
            service,
            Multiplicity::ONE,
        )
        .unwrap();
    project.validate_element(full).unwrap();
    assert_eq!(project.element(full).unwrap().kind, ElementKind::FullPort);
    assert!(!project.element(full).unwrap().is_conjugated);

    let invalid_proxy = project.create_typed_feature(
        ElementKind::ProxyPort,
        "invalidProxy",
        owner,
        invalid_type,
        Multiplicity::ONE,
    );
    assert!(matches!(
        invalid_proxy,
        Err(ModelError::InvalidTypeKind {
            kind: ElementKind::ProxyPort,
            type_kind: ElementKind::ValueType
        })
    ));

    let invalid_full = project.create_typed_feature(
        ElementKind::FullPort,
        "invalidFull",
        owner,
        invalid_type,
        Multiplicity::ONE,
    );
    assert!(matches!(
        invalid_full,
        Err(ModelError::InvalidTypeKind {
            kind: ElementKind::FullPort,
            type_kind: ElementKind::ValueType
        })
    ));

    let illegal_owner = project.create_typed_feature(
        ElementKind::ProxyPort,
        "illegalOwner",
        requirement,
        iface,
        Multiplicity::ONE,
    );
    assert!(matches!(
        illegal_owner,
        Err(ModelError::InvalidOwnerKind {
            kind: ElementKind::ProxyPort,
            owner: ElementKind::Requirement
        })
    ));

    project.element_mut(full).unwrap().is_conjugated = true;
    assert_eq!(
        project.validate_element(full),
        Err(ModelError::FullPortCannotBeConjugated(full))
    );
}
