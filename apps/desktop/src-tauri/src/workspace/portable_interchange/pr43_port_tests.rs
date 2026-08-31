use super::*;
use systems_modeler_core::{ElementKind, Multiplicity, VisibilityKind};

#[test]
fn pr43_portable_json_round_trip_preserves_proxy_and_full_port_state() {
    let source = WorkspaceState::default();
    let activity = ActivityWorkspaceState::default();
    let mut project = Project::new("PR43 Portable Source");
    let package = project
        .create_element(ElementKind::Package, "Architecture", project.root_id)
        .unwrap();
    let owner = project
        .create_element(ElementKind::Block, "Controller", package)
        .unwrap();
    let iface = project
        .create_element(ElementKind::InterfaceBlock, "CommandInterface", package)
        .unwrap();
    let service_type = project
        .create_element(ElementKind::DataType, "ServiceAssembly", package)
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
    {
        let port = project.element_mut(proxy).unwrap();
        port.external_id = "catia:pr43::PORT-PROXY".into();
        port.is_conjugated = true;
        port.documentation = "Portable proxy port".into();
        port.visibility = VisibilityKind::Private;
    }
    project.validate_element(proxy).unwrap();

    let full = project
        .create_typed_feature(
            ElementKind::FullPort,
            "service",
            owner,
            service_type,
            Multiplicity::new(1, Some(2)).unwrap(),
        )
        .unwrap();
    {
        let port = project.element_mut(full).unwrap();
        port.external_id = "catia:pr43::PORT-FULL".into();
        port.is_conjugated = false;
        port.documentation = "Portable full port".into();
        port.visibility = VisibilityKind::Public;
    }
    project.validate_element(full).unwrap();
    *source.project.lock().unwrap() = Some(project);

    let json = export_from_states(&source, &activity).unwrap();
    assert!(json.contains("ProxyPort"));
    assert!(json.contains("FullPort"));
    assert!(json.contains("is_conjugated"));

    let target = WorkspaceState::default();
    let target_activity = ActivityWorkspaceState::default();
    import_into_states(&json, &target, &target_activity).unwrap();
    let guard = target.project.lock().unwrap();
    let restored = guard.as_ref().unwrap();

    let proxy_port = restored.element(proxy).unwrap();
    assert_eq!(proxy_port.kind, ElementKind::ProxyPort);
    assert_eq!(proxy_port.external_id, "catia:pr43::PORT-PROXY");
    assert_eq!(proxy_port.owner_id, Some(owner));
    assert_eq!(proxy_port.type_id, Some(iface));
    assert_eq!(
        proxy_port.multiplicity,
        Some(Multiplicity::new(0, None).unwrap())
    );
    assert!(proxy_port.is_conjugated);
    assert_eq!(proxy_port.documentation, "Portable proxy port");
    assert_eq!(proxy_port.visibility, VisibilityKind::Private);

    let full_port = restored.element(full).unwrap();
    assert_eq!(full_port.kind, ElementKind::FullPort);
    assert_eq!(full_port.external_id, "catia:pr43::PORT-FULL");
    assert_eq!(full_port.owner_id, Some(owner));
    assert_eq!(full_port.type_id, Some(service_type));
    assert_eq!(
        full_port.multiplicity,
        Some(Multiplicity::new(1, Some(2)).unwrap())
    );
    assert!(!full_port.is_conjugated);
    assert_eq!(full_port.documentation, "Portable full port");
    assert_eq!(full_port.visibility, VisibilityKind::Public);
    restored.validate().unwrap();
}
