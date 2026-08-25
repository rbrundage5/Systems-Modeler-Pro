use systems_modeler_core::{
    ElementKind, ExecutionSession, ExecutionState, NamespaceResolutionError, Project, RuntimeEventKind,
    RuntimeValue, VisibilityKind,
};

#[test]
fn package_import_makes_public_library_members_resolvable_without_reparenting() {
    let mut project = Project::new("VehicleModel");
    let root = project.root_id;
    let library = project
        .create_element(ElementKind::ModelLibrary, "Common_Library", root)
        .unwrap();
    let vehicle = project
        .create_element(ElementKind::Package, "Vehicle_System", root)
        .unwrap();
    let mass = project
        .create_element(ElementKind::ValueType, "Mass", library)
        .unwrap();

    assert!(matches!(
        project.resolve_name(vehicle, "Mass"),
        Err(NamespaceResolutionError::NotFound { .. })
    ));

    project
        .create_package_import(vehicle, library, VisibilityKind::Public)
        .unwrap();

    assert_eq!(project.resolve_name(vehicle, "Mass").unwrap(), mass);
    assert_eq!(project.element(mass).unwrap().owner_id, Some(library));
    assert_eq!(
        project.qualified_name(mass).unwrap(),
        "VehicleModel::Common_Library::Mass"
    );
    assert_eq!(
        project.resolve_qualified_name("Common_Library::Mass").unwrap(),
        mass
    );
}

#[test]
fn private_package_import_is_local_while_public_import_is_reexported() {
    let mut project = Project::new("Model");
    let root = project.root_id;
    let library = project
        .create_element(ElementKind::Package, "Library", root)
        .unwrap();
    let shared_type = project
        .create_element(ElementKind::Block, "SharedType", library)
        .unwrap();
    let private_middle = project
        .create_element(ElementKind::Package, "PrivateMiddle", root)
        .unwrap();
    let public_middle = project
        .create_element(ElementKind::Package, "PublicMiddle", root)
        .unwrap();
    let private_consumer = project
        .create_element(ElementKind::Package, "PrivateConsumer", root)
        .unwrap();
    let public_consumer = project
        .create_element(ElementKind::Package, "PublicConsumer", root)
        .unwrap();

    project
        .create_package_import(private_middle, library, VisibilityKind::Private)
        .unwrap();
    project
        .create_package_import(private_consumer, private_middle, VisibilityKind::Public)
        .unwrap();
    project
        .create_package_import(public_middle, library, VisibilityKind::Public)
        .unwrap();
    project
        .create_package_import(public_consumer, public_middle, VisibilityKind::Public)
        .unwrap();

    assert_eq!(
        project.resolve_name(private_middle, "SharedType").unwrap(),
        shared_type
    );
    assert!(matches!(
        project.resolve_name(private_consumer, "SharedType"),
        Err(NamespaceResolutionError::NotFound { .. })
    ));
    assert_eq!(
        project.resolve_name(public_consumer, "SharedType").unwrap(),
        shared_type
    );
}

#[test]
fn element_import_alias_resolves_without_renaming_or_reparenting_target() {
    let mut project = Project::new("Model");
    let root = project.root_id;
    let library = project
        .create_element(ElementKind::Package, "Library", root)
        .unwrap();
    let vehicle = project
        .create_element(ElementKind::Package, "Vehicle", root)
        .unwrap();
    let vehicle_type = project
        .create_element(ElementKind::Block, "VehicleType", library)
        .unwrap();

    project
        .create_element_import(
            vehicle,
            vehicle_type,
            VisibilityKind::Public,
            Some("CarType".into()),
        )
        .unwrap();

    assert_eq!(project.resolve_name(vehicle, "CarType").unwrap(), vehicle_type);
    assert!(matches!(
        project.resolve_name(vehicle, "VehicleType"),
        Err(NamespaceResolutionError::NotFound { .. })
    ));
    let original = project.element(vehicle_type).unwrap();
    assert_eq!(original.name, "VehicleType");
    assert_eq!(original.owner_id, Some(library));
}

#[test]
fn package_import_reports_ambiguous_imported_names_with_qualified_candidates() {
    let mut project = Project::new("Model");
    let root = project.root_id;
    let library_a = project
        .create_element(ElementKind::Package, "LibraryA", root)
        .unwrap();
    let library_b = project
        .create_element(ElementKind::Package, "LibraryB", root)
        .unwrap();
    let consumer = project
        .create_element(ElementKind::Package, "Consumer", root)
        .unwrap();
    project
        .create_element(ElementKind::Block, "Shared", library_a)
        .unwrap();
    project
        .create_element(ElementKind::Block, "Shared", library_b)
        .unwrap();
    project
        .create_package_import(consumer, library_a, VisibilityKind::Public)
        .unwrap();
    project
        .create_package_import(consumer, library_b, VisibilityKind::Public)
        .unwrap();

    let error = project.resolve_name(consumer, "Shared").unwrap_err();
    match error {
        NamespaceResolutionError::Ambiguous {
            context,
            name,
            candidates,
        } => {
            assert_eq!(context, "Model::Consumer");
            assert_eq!(name, "Shared");
            assert_eq!(
                candidates,
                vec![
                    "Model::LibraryA::Shared".to_string(),
                    "Model::LibraryB::Shared".to_string(),
                ]
            );
        }
        other => panic!("unexpected resolution result: {other:?}"),
    }
}

#[test]
fn runtime_values_are_session_local_and_reset_does_not_mutate_authored_model() {
    let mut project = Project::new("VehicleModel");
    let root = project.root_id;
    let controller = project
        .create_element(ElementKind::Block, "Controller", root)
        .unwrap();
    project.element_mut(controller).unwrap().default_value = Some("authored-default".into());

    let mut session = ExecutionSession::new(&project);
    session.initialize(&project).unwrap();
    session
        .set_value(&project, None, controller, RuntimeValue::Real(27.5))
        .unwrap();

    assert_eq!(
        session.value(None, controller),
        Some(&RuntimeValue::Real(27.5))
    );
    assert_eq!(
        project.element(controller).unwrap().default_value.as_deref(),
        Some("authored-default")
    );

    session.reset(&project).unwrap();
    assert_eq!(session.state, ExecutionState::Initialized);
    assert_eq!(session.value(None, controller), None);
    assert_eq!(
        project.element(controller).unwrap().default_value.as_deref(),
        Some("authored-default")
    );
}

#[test]
fn event_queue_and_trace_are_deterministic_and_support_run_pause_resume() {
    let mut project = Project::new("VehicleModel");
    let root = project.root_id;
    let controller = project
        .create_element(ElementKind::Block, "Controller", root)
        .unwrap();

    let mut session = ExecutionSession::new(&project);
    session.initialize(&project).unwrap();
    let first = session
        .queue_event(
            &project,
            RuntimeEventKind::Signal,
            "StartMotor",
            Some(controller),
            Some(controller),
            Vec::new(),
        )
        .unwrap();
    let second = session
        .queue_event(
            &project,
            RuntimeEventKind::Time,
            "Tick",
            None,
            Some(controller),
            Vec::new(),
        )
        .unwrap();
    assert_eq!((first, second), (0, 1));

    session.run().unwrap();
    assert_eq!(session.state, ExecutionState::Running);
    session.pause().unwrap();
    assert_eq!(session.state, ExecutionState::Paused);
    session.resume().unwrap();
    assert_eq!(session.state, ExecutionState::Running);

    let start = session.step().unwrap().unwrap();
    let tick = session.step().unwrap().unwrap();
    assert_eq!(start.name, "StartMotor");
    assert_eq!(tick.name, "Tick");
    assert!(session.step().unwrap().is_none());
    assert!(session
        .trace
        .windows(2)
        .all(|entries| entries[0].sequence < entries[1].sequence));
    assert!(session
        .trace
        .iter()
        .any(|entry| entry.message.contains("Controller") || entry.message.contains("StartMotor")));
}
