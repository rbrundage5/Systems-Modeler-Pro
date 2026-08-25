use systems_modeler_core::{
    ElementKind, EngineStepOutcome, ExecutionConfiguration, ExecutionEngine, ExecutionError,
    ExecutionManager, ExecutionSession, ExecutionState, Multiplicity, NamespaceResolutionError,
    Project, RuntimeEvent, RuntimeEventKind, RuntimeValue, SimulationTime, VisibilityKind,
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
        project
            .resolve_qualified_name("Common_Library::Mass")
            .unwrap(),
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

    assert_eq!(
        project.resolve_name(vehicle, "CarType").unwrap(),
        vehicle_type
    );
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
fn resolution_from_nested_context_uses_nearest_package_namespace() {
    let mut project = Project::new("VehicleModel");
    let root = project.root_id;
    let library = project
        .create_element(ElementKind::ModelLibrary, "CommonLibrary", root)
        .unwrap();
    let vehicle = project
        .create_element(ElementKind::Package, "Vehicle", root)
        .unwrap();
    let controller = project
        .create_element(ElementKind::Block, "Controller", vehicle)
        .unwrap();
    let mass = project
        .create_element(ElementKind::ValueType, "Mass", library)
        .unwrap();
    project
        .create_package_import(vehicle, library, VisibilityKind::Public)
        .unwrap();

    assert_eq!(project.enclosing_namespace(controller).unwrap(), vehicle);
    assert_eq!(
        project.resolve_from_context(controller, "Mass").unwrap(),
        mass
    );
    assert!(project
        .visible_members_from_context(controller)
        .unwrap()
        .contains(&mass));
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
        project
            .element(controller)
            .unwrap()
            .default_value
            .as_deref(),
        Some("authored-default")
    );

    session.reset(&project).unwrap();
    assert_eq!(session.state, ExecutionState::Initialized);
    assert_eq!(session.value(None, controller), None);
    assert_eq!(
        project
            .element(controller)
            .unwrap()
            .default_value
            .as_deref(),
        Some("authored-default")
    );
}

#[test]
fn scheduler_orders_by_simulation_time_then_event_sequence() {
    let mut project = Project::new("VehicleModel");
    let root = project.root_id;
    let controller = project
        .create_element(ElementKind::Block, "Controller", root)
        .unwrap();
    let mut session = ExecutionSession::new(&project);
    session.initialize(&project).unwrap();

    let late = session
        .queue_event_after(
            &project,
            10,
            RuntimeEventKind::Time,
            "late",
            None,
            Some(controller),
            None,
            None,
            Vec::new(),
        )
        .unwrap();
    let first_at_five = session
        .queue_event_after(
            &project,
            5,
            RuntimeEventKind::Time,
            "first-five",
            None,
            Some(controller),
            None,
            None,
            Vec::new(),
        )
        .unwrap();
    let second_at_five = session
        .queue_event_after(
            &project,
            5,
            RuntimeEventKind::Time,
            "second-five",
            None,
            Some(controller),
            None,
            None,
            Vec::new(),
        )
        .unwrap();

    assert_eq!((late, first_at_five, second_at_five), (0, 1, 2));
    assert_eq!(session.next_event().unwrap().event.sequence, first_at_five);

    let first = session.step().unwrap().unwrap();
    assert_eq!(first.name, "first-five");
    assert_eq!(session.simulation_time, SimulationTime::from_nanos(5));
    let second = session.step().unwrap().unwrap();
    assert_eq!(second.name, "second-five");
    assert_eq!(session.simulation_time, SimulationTime::from_nanos(5));
    let third = session.step().unwrap().unwrap();
    assert_eq!(third.name, "late");
    assert_eq!(session.simulation_time, SimulationTime::from_nanos(10));
}

#[test]
fn runtime_instance_addressing_is_preserved_in_events_and_structured_trace() {
    let mut project = Project::new("VehicleModel");
    let root = project.root_id;
    let controller = project
        .create_element(ElementKind::Block, "Controller", root)
        .unwrap();
    let mut session = ExecutionSession::new(&project);
    session.initialize(&project).unwrap();
    let instance = session
        .create_instance(&project, controller, Some(controller))
        .unwrap();

    let event_sequence = session
        .queue_event_after(
            &project,
            0,
            RuntimeEventKind::Signal,
            "StartMotor",
            None,
            None,
            None,
            Some(instance),
            Vec::new(),
        )
        .unwrap();
    let event = session.step().unwrap().unwrap();

    assert_eq!(event.target_runtime_instance_id, Some(instance));
    assert_eq!(event.target_semantic_id, Some(controller));
    let trace = session
        .trace
        .iter()
        .find(|entry| {
            entry.event_sequence == Some(event_sequence)
                && entry.kind == systems_modeler_core::TraceKind::EventDispatched
        })
        .unwrap();
    assert_eq!(trace.target_runtime_instance_id, Some(instance));
    assert_eq!(trace.target_semantic_id, Some(controller));
}

#[test]
fn snapshots_are_read_only_deterministic_views_with_monotonic_revision() {
    let mut project = Project::new("VehicleModel");
    let root = project.root_id;
    let controller = project
        .create_element(ElementKind::Block, "Controller", root)
        .unwrap();
    let motor = project
        .create_element(ElementKind::Block, "Motor", root)
        .unwrap();
    let mut session = ExecutionSession::new(&project);
    assert_eq!(session.snapshot().revision, 0);

    session.initialize(&project).unwrap();
    let initialized_revision = session.snapshot().revision;
    session
        .set_active_semantic_elements(&project, [motor, controller])
        .unwrap();
    let snapshot = session.snapshot();

    assert!(snapshot.revision > initialized_revision);
    assert_eq!(snapshot.active_semantic_element_ids.len(), 2);
    assert!(snapshot.active_semantic_element_ids.contains(&controller));
    assert!(snapshot.active_semantic_element_ids.contains(&motor));
    assert_eq!(snapshot.session_id, session.id);
}

#[test]
fn runtime_assignment_boundary_rejects_obvious_typed_value_mismatch() {
    let mut project = Project::new("TypedRuntime");
    let root = project.root_id;
    let boolean = project
        .create_element(ElementKind::PrimitiveType, "Boolean", root)
        .unwrap();
    let controller = project
        .create_element(ElementKind::Block, "Controller", root)
        .unwrap();
    let enabled = project
        .create_typed_feature(
            ElementKind::ValueProperty,
            "enabled",
            controller,
            boolean,
            Multiplicity::ONE,
        )
        .unwrap();
    let mut session = ExecutionSession::new(&project);
    session.initialize(&project).unwrap();

    session
        .set_value(&project, None, enabled, RuntimeValue::Boolean(true))
        .unwrap();
    assert!(matches!(
        session.set_value(&project, None, enabled, RuntimeValue::Integer(1)),
        Err(ExecutionError::RuntimeValueTypeMismatch { .. })
    ));
}

#[test]
fn configured_limits_and_cancellation_stop_runaway_execution_cleanly() {
    let mut project = Project::new("GuardedRuntime");
    let root = project.root_id;
    let controller = project
        .create_element(ElementKind::Block, "Controller", root)
        .unwrap();
    let configuration = ExecutionConfiguration {
        root_semantic_id: root,
        random_seed: 42,
        max_steps: 1,
        max_queued_events: 2,
    };
    let mut session = ExecutionSession::with_configuration(&project, configuration).unwrap();
    session.initialize(&project).unwrap();
    session
        .queue_event(
            &project,
            RuntimeEventKind::Internal,
            "one",
            None,
            Some(controller),
            Vec::new(),
        )
        .unwrap();
    session
        .queue_event(
            &project,
            RuntimeEventKind::Internal,
            "two",
            None,
            Some(controller),
            Vec::new(),
        )
        .unwrap();
    assert!(matches!(
        session.queue_event(
            &project,
            RuntimeEventKind::Internal,
            "three",
            None,
            Some(controller),
            Vec::new(),
        ),
        Err(ExecutionError::EventQueueLimitExceeded { limit: 2 })
    ));

    assert!(session.step().unwrap().is_some());
    assert!(matches!(
        session.step(),
        Err(ExecutionError::StepLimitExceeded { limit: 1 })
    ));
    assert_eq!(session.state, ExecutionState::Failed);

    session.reset(&project).unwrap();
    session.request_cancellation();
    assert!(matches!(
        session.step(),
        Err(ExecutionError::CancellationRequested)
    ));
    assert_eq!(session.state, ExecutionState::Terminated);
}

#[test]
fn execution_manager_owns_multiple_independent_sessions() {
    let project = Project::new("MultiSession");
    let mut manager = ExecutionManager::default();
    let first = manager.create_default_session(&project);
    let second = manager.create_default_session(&project);

    assert_ne!(first, second);
    assert_eq!(manager.len(), 2);
    manager.session_mut(first).unwrap().initialize(&project).unwrap();
    assert_eq!(
        manager.session(first).unwrap().state,
        ExecutionState::Initialized
    );
    assert_eq!(manager.session(second).unwrap().state, ExecutionState::Created);
    manager.terminate_session(first).unwrap();
    assert_eq!(
        manager.session(first).unwrap().state,
        ExecutionState::Terminated
    );
    manager.remove_session(first).unwrap();
    assert_eq!(manager.len(), 1);
}

struct FoundationEngine;

impl ExecutionEngine for FoundationEngine {
    fn initialize(
        &mut self,
        project: &Project,
        session: &mut ExecutionSession,
    ) -> Result<(), ExecutionError> {
        session.initialize(project)
    }

    fn step(
        &mut self,
        _project: &Project,
        session: &mut ExecutionSession,
    ) -> Result<EngineStepOutcome, ExecutionError> {
        session.consume_step_budget()?;
        Ok(EngineStepOutcome::Progressed)
    }

    fn handle_event(
        &mut self,
        _project: &Project,
        _session: &mut ExecutionSession,
        _event: &RuntimeEvent,
    ) -> Result<EngineStepOutcome, ExecutionError> {
        Ok(EngineStepOutcome::Idle)
    }
}

#[test]
fn execution_engine_boundary_supports_semantic_steps_without_activity_logic() {
    let project = Project::new("EngineBoundary");
    let mut session = ExecutionSession::new(&project);
    let mut engine = FoundationEngine;

    engine.initialize(&project, &mut session).unwrap();
    assert_eq!(
        engine.step(&project, &mut session).unwrap(),
        EngineStepOutcome::Progressed
    );
    assert_eq!(session.steps_executed, 1);
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
    assert!(
        session
            .trace
            .windows(2)
            .all(|entries| entries[0].sequence < entries[1].sequence)
    );
    assert!(session
        .trace
        .iter()
        .any(|entry| entry.message.contains("StartMotor")));
}
