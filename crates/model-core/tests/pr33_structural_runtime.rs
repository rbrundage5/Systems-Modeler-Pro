use systems_modeler_core::*;

struct VehicleFixture {
    project: Project,
    vehicle: ElementId,
    propulsion: ElementId,
    controller: ElementId,
    guidance: ElementId,
    sensor: ElementId,
    propulsion_part: ElementId,
    controller_part: ElementId,
    guidance_part: ElementId,
    left_sensor: ElementId,
    right_sensor: ElementId,
    sensor_reference: ElementId,
    controller_mode: ElementId,
    sensor_reading: ElementId,
    sensor_port: ElementId,
    guidance_port: ElementId,
    controller_port: ElementId,
    boundary_port: ElementId,
    full_port: ElementId,
    status_signal: ElementId,
    sensor_connector: systems_modeler_core::RelationshipId,
}

fn vehicle_fixture() -> VehicleFixture {
    let mut project = Project::new("PR33 Vehicle Qualification");
    let package = project
        .create_element(ElementKind::Package, "VehicleModel", project.root_id)
        .unwrap();
    let real = project
        .create_element(ElementKind::PrimitiveType, "Real", package)
        .unwrap();
    let string = project
        .create_element(ElementKind::PrimitiveType, "String", package)
        .unwrap();
    let status_signal = project
        .create_element(ElementKind::Signal, "SensorStatus", package)
        .unwrap();
    let interface = project
        .create_element(ElementKind::InterfaceBlock, "StatusInterface", package)
        .unwrap();
    let status_flow = project
        .create_typed_feature(
            ElementKind::FlowProperty,
            "status",
            interface,
            status_signal,
            Multiplicity::ONE,
        )
        .unwrap();
    project.element_mut(status_flow).unwrap().flow_direction = Some(FlowDirection::Out);

    let base_controller = project
        .create_element(ElementKind::Block, "BaseController", package)
        .unwrap();
    let controller_mode = project
        .create_typed_feature(
            ElementKind::ValueProperty,
            "mode",
            base_controller,
            string,
            Multiplicity::ONE,
        )
        .unwrap();
    project.element_mut(controller_mode).unwrap().default_value = Some("\"Standby\"".into());
    let controller = project
        .create_element(ElementKind::Block, "Controller", package)
        .unwrap();
    project
        .create_relationship(
            RelationshipKind::Generalization,
            controller,
            base_controller,
            Some(package),
        )
        .unwrap();
    let controller_port = project
        .create_typed_feature(
            ElementKind::ProxyPort,
            "commandPort",
            controller,
            interface,
            Multiplicity::ONE,
        )
        .unwrap();

    let propulsion = project
        .create_element(ElementKind::Block, "Propulsion", package)
        .unwrap();
    let controller_part = project
        .create_typed_feature(
            ElementKind::PartProperty,
            "controller",
            propulsion,
            controller,
            Multiplicity::ONE,
        )
        .unwrap();

    let sensor = project
        .create_element(ElementKind::Block, "Sensor", package)
        .unwrap();
    let sensor_reading = project
        .create_typed_feature(
            ElementKind::ValueProperty,
            "reading",
            sensor,
            real,
            Multiplicity::ONE,
        )
        .unwrap();
    project.element_mut(sensor_reading).unwrap().default_value = Some("0.0".into());
    let sensor_port = project
        .create_typed_feature(
            ElementKind::ProxyPort,
            "statusPort",
            sensor,
            interface,
            Multiplicity::ONE,
        )
        .unwrap();
    let full_port = project
        .create_typed_feature(
            ElementKind::FullPort,
            "diagnosticPort",
            sensor,
            interface,
            Multiplicity::ONE,
        )
        .unwrap();

    let guidance = project
        .create_element(ElementKind::Block, "GuidanceComputer", package)
        .unwrap();
    let guidance_port = project
        .create_typed_feature(
            ElementKind::ProxyPort,
            "statusIn",
            guidance,
            interface,
            Multiplicity::ONE,
        )
        .unwrap();
    project.element_mut(guidance_port).unwrap().is_conjugated = true;
    let reception = project
        .create_element(ElementKind::Reception, "receiveStatus", guidance)
        .unwrap();
    project.set_element_type(reception, status_signal).unwrap();
    let sensor_reference = project
        .create_typed_feature(
            ElementKind::ReferenceProperty,
            "selectedSensor",
            guidance,
            sensor,
            Multiplicity::new(0, Some(1)).unwrap(),
        )
        .unwrap();

    let vehicle = project
        .create_element(ElementKind::Block, "Vehicle", package)
        .unwrap();
    let propulsion_part = project
        .create_typed_feature(
            ElementKind::PartProperty,
            "propulsion",
            vehicle,
            propulsion,
            Multiplicity::ONE,
        )
        .unwrap();
    let guidance_part = project
        .create_typed_feature(
            ElementKind::PartProperty,
            "guidance",
            vehicle,
            guidance,
            Multiplicity::ONE,
        )
        .unwrap();
    let left_sensor = project
        .create_typed_feature(
            ElementKind::PartProperty,
            "leftSensor",
            vehicle,
            sensor,
            Multiplicity::ONE,
        )
        .unwrap();
    let right_sensor = project
        .create_typed_feature(
            ElementKind::PartProperty,
            "rightSensor",
            vehicle,
            sensor,
            Multiplicity::ONE,
        )
        .unwrap();
    let boundary_port = project
        .create_typed_feature(
            ElementKind::ProxyPort,
            "externalStatus",
            vehicle,
            interface,
            Multiplicity::ONE,
        )
        .unwrap();

    let source = ConnectorEnd::nested_port(vec![left_sensor], sensor_port);
    let target = ConnectorEnd::nested_port(vec![guidance_part], guidance_port);
    let sensor_connector = project
        .create_connector(Connector {
            context_id: vehicle,
            kind: ConnectorKind::Assembly,
            source: source.clone(),
            target: target.clone(),
        })
        .unwrap();
    project
        .create_item_flow(ItemFlow {
            connector_id: sensor_connector,
            source,
            target,
            conveyed_item_ids: vec![status_signal],
        })
        .unwrap();

    project
        .create_connector(Connector {
            context_id: vehicle,
            kind: ConnectorKind::Assembly,
            source: ConnectorEnd::nested_port(
                vec![propulsion_part, controller_part],
                controller_port,
            ),
            target: ConnectorEnd::nested_port(vec![guidance_part], guidance_port),
        })
        .unwrap();
    project
        .create_connector(Connector {
            context_id: vehicle,
            kind: ConnectorKind::Delegation,
            source: ConnectorEnd::boundary(boundary_port),
            target: ConnectorEnd::nested_port(vec![guidance_part], guidance_port),
        })
        .unwrap();

    VehicleFixture {
        project,
        vehicle,
        propulsion,
        controller,
        guidance,
        sensor,
        propulsion_part,
        controller_part,
        guidance_part,
        left_sensor,
        right_sensor,
        sensor_reference,
        controller_mode,
        sensor_reading,
        sensor_port,
        guidance_port,
        controller_port,
        boundary_port,
        full_port,
        status_signal,
        sensor_connector,
    }
}

fn vehicle_configuration() -> StructuralRuntimeConfiguration {
    StructuralRuntimeConfiguration {
        root_instance_name: Some("vehicle".into()),
        populations: Vec::new(),
        reference_bindings: vec![RuntimeReferenceBindingDecision {
            owner_runtime_path: "vehicle.guidance".into(),
            reference_property_id: ElementId::default(),
            target_runtime_paths: vec!["vehicle.leftSensor".into()],
        }],
        configured_instance_specification_ids: Vec::new(),
    }
}

fn build_vehicle(fixture: &VehicleFixture) -> StructuralRuntime {
    let mut configuration = vehicle_configuration();
    configuration.reference_bindings[0].reference_property_id = fixture.sensor_reference;
    StructuralRuntime::build(&fixture.project, fixture.vehicle, &configuration).unwrap()
}

#[test]
fn definitions_usages_and_occurrences_remain_distinct() {
    let fixture = vehicle_fixture();
    let runtime = build_vehicle(&fixture);

    assert_eq!(runtime.instances_for_classifier(fixture.vehicle).len(), 1);
    assert_eq!(runtime.instances_for_classifier(fixture.sensor).len(), 2);
    let left = runtime.instance_by_path("vehicle.leftSensor").unwrap();
    let right = runtime.instance_by_path("vehicle.rightSensor").unwrap();
    assert_ne!(left.id, right.id);
    assert_eq!(left.semantic_usage_id, Some(fixture.left_sensor));
    assert_eq!(right.semantic_usage_id, Some(fixture.right_sensor));
    assert_eq!(left.classifier_id, Some(fixture.sensor));
    assert_eq!(right.classifier_id, Some(fixture.sensor));
    assert_eq!(runtime.instances_for_usage(fixture.left_sensor).len(), 1);
    assert_eq!(runtime.instances_for_usage(fixture.right_sensor).len(), 1);
}

#[test]
fn nested_parts_inheritance_values_and_reference_identity_are_qualified() {
    let fixture = vehicle_fixture();
    let runtime = build_vehicle(&fixture);
    let controller = runtime
        .instance_by_path("vehicle.propulsion.controller")
        .unwrap();
    assert_eq!(controller.classifier_id, Some(fixture.controller));
    assert_eq!(controller.semantic_usage_id, Some(fixture.controller_part));
    assert_eq!(
        runtime
            .initial_values
            .get(&systems_modeler_core::RuntimeValueKey {
                instance_id: Some(controller.id),
                semantic_element_id: fixture.controller_mode,
            }),
        Some(&RuntimeValue::Text("Standby".into()))
    );

    let guidance = runtime.instance_by_path("vehicle.guidance").unwrap();
    let left = runtime.instance_by_path("vehicle.leftSensor").unwrap();
    let binding = runtime
        .references
        .iter()
        .find(|binding| binding.owner_instance_id == guidance.id)
        .unwrap();
    assert_eq!(binding.reference_property_id, fixture.sensor_reference);
    assert_eq!(binding.target_instance_ids, vec![left.id]);
    assert_eq!(runtime.instances_for_classifier(fixture.sensor).len(), 2);
}

#[test]
fn ports_nested_connectors_delegation_item_flow_and_signal_route_are_semantic() {
    let fixture = vehicle_fixture();
    let runtime = build_vehicle(&fixture);
    let left = runtime.instance_by_path("vehicle.leftSensor").unwrap();
    let guidance = runtime.instance_by_path("vehicle.guidance").unwrap();

    let sensor_proxy = runtime.port(left.id, fixture.sensor_port).unwrap();
    let sensor_full = runtime.port(left.id, fixture.full_port).unwrap();
    assert_eq!(
        sensor_proxy.kind,
        systems_modeler_core::RuntimePortKind::Proxy
    );
    assert_eq!(
        sensor_full.kind,
        systems_modeler_core::RuntimePortKind::Full
    );
    assert_ne!(sensor_proxy.key, sensor_full.key);

    let nested = runtime
        .connector_links
        .iter()
        .find(|link| link.source.qualified_path.contains("propulsion.controller"))
        .unwrap();
    assert!(nested.source.property_path.len() > 1);
    assert_eq!(
        nested.source.semantic_port_id,
        Some(fixture.controller_port)
    );
    assert_eq!(nested.target.instance_id, guidance.id);
    assert!(runtime.connector_links.iter().any(|link| {
        link.kind == ConnectorKind::Delegation
            && link.source.semantic_port_id == Some(fixture.boundary_port)
    }));

    let destinations = runtime
        .signal_destinations(
            &fixture.project,
            left.id,
            fixture.sensor_port,
            fixture.status_signal,
        )
        .unwrap();
    assert_eq!(destinations.len(), 1);
    assert_eq!(destinations[0].instance_id, guidance.id);
    assert_eq!(
        destinations[0].semantic_port_id,
        Some(fixture.guidance_port)
    );
    assert!(runtime.connector_links.iter().any(|link| {
        link.semantic_connector_id == fixture.sensor_connector && !link.item_flows.is_empty()
    }));
}

#[test]
fn execution_session_scopes_values_routes_events_and_resets_deterministically() {
    let fixture = vehicle_fixture();
    let before = serde_json::to_string(&fixture.project).unwrap();
    let configuration = ExecutionConfiguration {
        root_semantic_id: fixture.vehicle,
        random_seed: 0,
        max_steps: 1000,
        max_queued_events: 100,
    };
    let mut session =
        ExecutionSession::with_configuration(&fixture.project, configuration).unwrap();
    let mut structural = vehicle_configuration();
    structural.reference_bindings[0].reference_property_id = fixture.sensor_reference;
    session.set_structural_configuration(structural).unwrap();
    session.initialize(&fixture.project).unwrap();

    let left = session
        .structural_runtime
        .as_ref()
        .unwrap()
        .instance_by_path("vehicle.leftSensor")
        .unwrap()
        .id;
    let right = session
        .structural_runtime
        .as_ref()
        .unwrap()
        .instance_by_path("vehicle.rightSensor")
        .unwrap()
        .id;
    let original_ids: Vec<_> = session
        .snapshot()
        .runtime_instances
        .iter()
        .map(|instance| (instance.qualified_path.clone(), instance.id))
        .collect();
    session
        .set_value(
            &fixture.project,
            Some(left),
            fixture.sensor_reading,
            RuntimeValue::Real(42.5),
        )
        .unwrap();
    assert_eq!(
        session.value(Some(left), fixture.sensor_reading),
        Some(&RuntimeValue::Real(42.5))
    );
    assert_eq!(
        session.value(Some(right), fixture.sensor_reading),
        Some(&RuntimeValue::Real(0.0))
    );
    let queued = session
        .queue_structural_signal(
            &fixture.project,
            left,
            fixture.sensor_port,
            fixture.status_signal,
            "SensorStatus",
            Vec::new(),
        )
        .unwrap();
    assert_eq!(queued.len(), 1);
    let event = &session.next_event().unwrap().event;
    assert_eq!(event.source_runtime_instance_id, Some(left));
    assert_ne!(event.target_runtime_instance_id, Some(right));
    assert_eq!(event.source_port_id, Some(fixture.sensor_port));
    assert_eq!(event.target_port_id, Some(fixture.guidance_port));

    session.reset(&fixture.project).unwrap();
    let reset_ids: Vec<_> = session
        .snapshot()
        .runtime_instances
        .iter()
        .map(|instance| (instance.qualified_path.clone(), instance.id))
        .collect();
    assert_eq!(original_ids, reset_ids);
    assert_eq!(
        session.value(Some(left), fixture.sensor_reading),
        Some(&RuntimeValue::Real(0.0))
    );
    assert_eq!(before, serde_json::to_string(&fixture.project).unwrap());
}

#[test]
fn explicit_and_unbounded_population_never_invent_an_arbitrary_count() {
    let mut project = Project::new("Population");
    let package = project
        .create_element(ElementKind::Package, "Model", project.root_id)
        .unwrap();
    let component = project
        .create_element(ElementKind::Block, "Component", package)
        .unwrap();
    let system = project
        .create_element(ElementKind::Block, "System", package)
        .unwrap();
    let optional = project
        .create_typed_feature(
            ElementKind::PartProperty,
            "optional",
            system,
            component,
            Multiplicity::new(0, None).unwrap(),
        )
        .unwrap();
    let default_runtime =
        StructuralRuntime::build(&project, system, &StructuralRuntimeConfiguration::default())
            .unwrap();
    assert!(default_runtime.instances_for_usage(optional).is_empty());

    let configured = StructuralRuntime::build(
        &project,
        system,
        &StructuralRuntimeConfiguration {
            populations: vec![RuntimePopulationDecision {
                owner_runtime_path: None,
                part_property_id: optional,
                count: 3,
            }],
            ..StructuralRuntimeConfiguration::default()
        },
    )
    .unwrap();
    let paths: Vec<_> = configured
        .instances_for_usage(optional)
        .iter()
        .map(|instance| instance.qualified_path.as_str())
        .collect();
    assert_eq!(
        paths,
        vec![
            "System.optional[0]",
            "System.optional[1]",
            "System.optional[2]"
        ]
    );
}

#[test]
fn recursive_composition_is_rejected_with_the_semantic_path() {
    let mut project = Project::new("Cycle");
    let package = project
        .create_element(ElementKind::Package, "Model", project.root_id)
        .unwrap();
    let a = project
        .create_element(ElementKind::Block, "A", package)
        .unwrap();
    let b = project
        .create_element(ElementKind::Block, "B", package)
        .unwrap();
    project
        .create_typed_feature(ElementKind::PartProperty, "b", a, b, Multiplicity::ONE)
        .unwrap();
    project
        .create_typed_feature(ElementKind::PartProperty, "a", b, a, Multiplicity::ONE)
        .unwrap();
    let error = StructuralRuntime::build(&project, a, &StructuralRuntimeConfiguration::default())
        .unwrap_err();
    assert!(matches!(
        error,
        StructuralRuntimeError::RecursiveComposition { .. }
    ));
    assert!(error.to_string().contains("A.b.a"));
}

#[test]
fn required_reference_and_invalid_proxy_port_fail_with_engineer_readable_remedies() {
    let mut project = Project::new("Negative structure");
    let package = project
        .create_element(ElementKind::Package, "Model", project.root_id)
        .unwrap();
    let external = project
        .create_element(ElementKind::Block, "External", package)
        .unwrap();
    let system = project
        .create_element(ElementKind::Block, "System", package)
        .unwrap();
    project
        .create_typed_feature(
            ElementKind::ReferenceProperty,
            "requiredExternal",
            system,
            external,
            Multiplicity::ONE,
        )
        .unwrap();
    let error =
        StructuralRuntime::build(&project, system, &StructuralRuntimeConfiguration::default())
            .unwrap_err();
    assert!(matches!(
        error,
        StructuralRuntimeError::RequiredReferenceUnresolved { .. }
    ));
    assert!(
        error
            .to_string()
            .contains("does not create owned structure")
    );

    let mut project = Project::new("Invalid proxy");
    let package = project
        .create_element(ElementKind::Package, "Model", project.root_id)
        .unwrap();
    let wrong_type = project
        .create_element(ElementKind::Block, "Controller", package)
        .unwrap();
    let system = project
        .create_element(ElementKind::Block, "System", package)
        .unwrap();
    project
        .create_typed_feature(
            ElementKind::ProxyPort,
            "commandPort",
            system,
            wrong_type,
            Multiplicity::ONE,
        )
        .unwrap();
    let error =
        StructuralRuntime::build(&project, system, &StructuralRuntimeConfiguration::default())
            .unwrap_err();
    assert!(matches!(
        error,
        StructuralRuntimeError::ProxyPortRequiresInterfaceBlock { .. }
    ));
    assert!(
        error
            .to_string()
            .contains("Select or create an InterfaceBlock")
    );
}

#[test]
fn fixture_keeps_expected_semantic_ids_reachable() {
    let fixture = vehicle_fixture();
    assert_eq!(
        fixture.project.element(fixture.propulsion).unwrap().name,
        "Propulsion"
    );
    assert_eq!(
        fixture.project.element(fixture.guidance).unwrap().name,
        "GuidanceComputer"
    );
    assert_eq!(
        fixture
            .project
            .element(fixture.propulsion_part)
            .unwrap()
            .name,
        "propulsion"
    );
    assert_eq!(
        fixture.project.element(fixture.guidance_part).unwrap().name,
        "guidance"
    );
}

fn state_vertex(name: &str, kind: VertexKind) -> Vertex {
    Vertex {
        id: VertexId::new(),
        name: name.into(),
        kind,
    }
}

fn state_transition(source: &Vertex, target: &Vertex) -> Transition {
    Transition {
        id: TransitionId::new(),
        source_id: source.id,
        target_id: target.id,
        kind: TransitionKind::External,
        trigger: None,
        guard: None,
        effect: None,
    }
}

#[test]
fn repeated_classifier_state_machines_use_independent_instance_values_and_addresses() {
    let mut project = Project::new("Instance behavior context");
    let package = project
        .create_element(ElementKind::Package, "Model", project.root_id)
        .unwrap();
    let real = project
        .create_element(ElementKind::PrimitiveType, "Real", package)
        .unwrap();
    let activate = project
        .create_element(ElementKind::Signal, "Activate", package)
        .unwrap();
    let controller = project
        .create_element(ElementKind::Block, "Controller", package)
        .unwrap();
    let enabled = project
        .create_typed_feature(
            ElementKind::ValueProperty,
            "enabled",
            controller,
            real,
            Multiplicity::ONE,
        )
        .unwrap();
    project.element_mut(enabled).unwrap().default_value = Some("0.0".into());
    let vehicle = project
        .create_element(ElementKind::Block, "Vehicle", package)
        .unwrap();
    project
        .create_typed_feature(
            ElementKind::PartProperty,
            "leftController",
            vehicle,
            controller,
            Multiplicity::ONE,
        )
        .unwrap();
    project
        .create_typed_feature(
            ElementKind::PartProperty,
            "rightController",
            vehicle,
            controller,
            Multiplicity::ONE,
        )
        .unwrap();

    let structural_configuration = StructuralRuntimeConfiguration {
        root_instance_name: Some("vehicle".into()),
        ..StructuralRuntimeConfiguration::default()
    };
    let structure = StructuralRuntime::build(&project, vehicle, &structural_configuration).unwrap();
    let left = structure
        .instance_by_path("vehicle.leftController")
        .unwrap()
        .id;
    let right = structure
        .instance_by_path("vehicle.rightController")
        .unwrap()
        .id;

    let mut repository = BehaviorRepository::default();
    let machine_id = repository
        .create_state_machine(&project, controller, "ControllerLifecycle")
        .unwrap();
    let initial = state_vertex("Initial", VertexKind::Pseudostate(PseudostateKind::Initial));
    let idle = state_vertex("Idle", VertexKind::State(State::default()));
    let running = state_vertex("Running", VertexKind::State(State::default()));
    let mut activate_transition = state_transition(&idle, &running);
    activate_transition.trigger = Some(Trigger {
        event: Event::Signal {
            signal_id: activate,
        },
    });
    activate_transition.guard = Some("enabled > 0".into());
    let machine = repository.state_machines.get_mut(&machine_id).unwrap();
    machine.regions[0]
        .vertices
        .extend([initial.clone(), idle.clone(), running.clone()]);
    machine.regions[0]
        .transitions
        .extend([state_transition(&initial, &idle), activate_transition]);

    let execution_configuration = ExecutionConfiguration {
        root_semantic_id: vehicle,
        random_seed: 0,
        max_steps: 1_000,
        max_queued_events: 100,
    };
    let mut left_session =
        ExecutionSession::with_configuration(&project, execution_configuration.clone()).unwrap();
    left_session
        .set_structural_configuration(structural_configuration.clone())
        .unwrap();
    let mut right_session =
        ExecutionSession::with_configuration(&project, execution_configuration).unwrap();
    right_session
        .set_structural_configuration(structural_configuration)
        .unwrap();
    let mut left_engine = StateMachineExecutionEngine::new(repository.clone(), machine_id)
        .with_runtime_instance(left);
    let mut right_engine =
        StateMachineExecutionEngine::new(repository, machine_id).with_runtime_instance(right);
    left_engine.initialize(&project, &mut left_session).unwrap();
    right_engine
        .initialize(&project, &mut right_session)
        .unwrap();

    left_session
        .set_value(&project, Some(left), enabled, RuntimeValue::Real(1.0))
        .unwrap();
    assert_eq!(
        right_session.value(Some(right), enabled),
        Some(&RuntimeValue::Real(0.0))
    );
    left_engine
        .queue_signal(
            &project,
            &mut left_session,
            activate,
            "Activate",
            Vec::new(),
        )
        .unwrap();
    assert_eq!(
        left_engine.advance(&project, &mut left_session).unwrap(),
        EngineStepOutcome::Progressed
    );
    assert_eq!(
        left_engine.snapshot(&left_session).active_states[0].state_name,
        "Running"
    );
    assert_eq!(
        right_engine.snapshot(&right_session).active_states[0].state_name,
        "Idle"
    );

    right_session
        .queue_typed_event_at(
            &project,
            RuntimeEventRequest {
                due_time: right_session.simulation_time,
                kind: RuntimeEventKind::Signal,
                name: "Activate".into(),
                semantic_event_id: Some(activate),
                address: RuntimeEventAddress {
                    target_semantic_id: Some(controller),
                    target_runtime_instance_id: Some(left),
                    ..RuntimeEventAddress::default()
                },
                payload: Vec::new(),
            },
        )
        .unwrap();
    assert_eq!(
        right_engine.advance(&project, &mut right_session).unwrap(),
        EngineStepOutcome::Idle
    );
    assert_eq!(
        right_engine.snapshot(&right_session).active_states[0].state_name,
        "Idle"
    );
}

#[test]
fn pr33_instance_context_prefers_occurrence_value_over_legacy_global_fallback() {
    let fixture = vehicle_fixture();
    let mut session = ExecutionSession::with_configuration(
        &fixture.project,
        ExecutionConfiguration {
            root_semantic_id: fixture.vehicle,
            random_seed: 0,
            max_steps: 100,
            max_queued_events: 100,
        },
    )
    .unwrap();
    let mut structural = vehicle_configuration();
    structural.reference_bindings[0].reference_property_id = fixture.sensor_reference;
    session.set_structural_configuration(structural).unwrap();
    session.initialize(&fixture.project).unwrap();
    let left = session
        .structural_runtime
        .as_ref()
        .unwrap()
        .instance_by_path("vehicle.leftSensor")
        .unwrap()
        .id;
    session
        .set_value(
            &fixture.project,
            None,
            fixture.sensor_reading,
            RuntimeValue::Real(99.0),
        )
        .unwrap();
    session
        .set_value(
            &fixture.project,
            Some(left),
            fixture.sensor_reading,
            RuntimeValue::Real(12.5),
        )
        .unwrap();
    assert_eq!(
        session.value_in_instance_context(Some(left), fixture.sensor_reading),
        Some(&RuntimeValue::Real(12.5))
    );
}

#[test]
fn duplicate_runtime_configuration_decisions_are_rejected_instead_of_first_match_wins() {
    let mut project = Project::new("PR33 duplicate decisions");
    let package = project
        .create_element(ElementKind::Package, "Model", project.root_id)
        .unwrap();
    let component = project
        .create_element(ElementKind::Block, "Component", package)
        .unwrap();
    let root = project
        .create_element(ElementKind::Block, "Root", package)
        .unwrap();
    let parts = project
        .create_typed_feature(
            ElementKind::PartProperty,
            "parts",
            root,
            component,
            Multiplicity::new(0, Some(4)).unwrap(),
        )
        .unwrap();
    let error = StructuralRuntime::build(
        &project,
        root,
        &StructuralRuntimeConfiguration {
            populations: vec![
                RuntimePopulationDecision {
                    owner_runtime_path: None,
                    part_property_id: parts,
                    count: 1,
                },
                RuntimePopulationDecision {
                    owner_runtime_path: None,
                    part_property_id: parts,
                    count: 2,
                },
            ],
            ..StructuralRuntimeConfiguration::default()
        },
    )
    .unwrap_err();
    assert!(matches!(
        error,
        StructuralRuntimeError::DuplicatePopulationDecision { .. }
    ));
}

#[test]
fn duplicate_reference_bindings_and_runtime_paths_are_rejected() {
    let fixture = vehicle_fixture();
    let duplicate_binding = RuntimeReferenceBindingDecision {
        owner_runtime_path: "vehicle.guidance".into(),
        reference_property_id: fixture.sensor_reference,
        target_runtime_paths: vec!["vehicle.leftSensor".into()],
    };
    let error = StructuralRuntime::build(
        &fixture.project,
        fixture.vehicle,
        &StructuralRuntimeConfiguration {
            root_instance_name: Some("vehicle".into()),
            reference_bindings: vec![duplicate_binding.clone(), duplicate_binding],
            ..StructuralRuntimeConfiguration::default()
        },
    )
    .unwrap_err();
    assert!(matches!(
        error,
        StructuralRuntimeError::DuplicateReferenceBindingDecision { .. }
    ));

    let mut project = Project::new("Duplicate paths");
    let package = project
        .create_element(ElementKind::Package, "Model", project.root_id)
        .unwrap();
    let block = project
        .create_element(ElementKind::Block, "Thing", package)
        .unwrap();
    let first = project
        .create_element(ElementKind::InstanceSpecification, "same", package)
        .unwrap();
    project.set_element_type(first, block).unwrap();
    let second = project
        .create_element(ElementKind::InstanceSpecification, "same", package)
        .unwrap();
    project.set_element_type(second, block).unwrap();
    let error = StructuralRuntime::build(
        &project,
        first,
        &StructuralRuntimeConfiguration {
            configured_instance_specification_ids: vec![second],
            ..StructuralRuntimeConfiguration::default()
        },
    )
    .unwrap_err();
    assert!(matches!(
        error,
        StructuralRuntimeError::DuplicateRuntimePath { .. }
    ));
}

#[test]
fn invalid_authored_default_blocks_structural_runtime_construction() {
    let mut project = Project::new("Invalid default");
    let package = project
        .create_element(ElementKind::Package, "Model", project.root_id)
        .unwrap();
    let real = project
        .create_element(ElementKind::PrimitiveType, "Real", package)
        .unwrap();
    let block = project
        .create_element(ElementKind::Block, "Controller", package)
        .unwrap();
    let value = project
        .create_typed_feature(
            ElementKind::ValueProperty,
            "gain",
            block,
            real,
            Multiplicity::ONE,
        )
        .unwrap();
    project.element_mut(value).unwrap().default_value = Some("not-a-number".into());
    let error =
        StructuralRuntime::build(&project, block, &StructuralRuntimeConfiguration::default())
            .unwrap_err();
    assert!(matches!(
        error,
        StructuralRuntimeError::InvalidDefault { .. }
    ));
    assert!(error.to_string().contains("gain"));
}

#[test]
fn flow_contract_type_compatibility_is_not_bidirectional() {
    let mut project = Project::new("Transport conformance");
    let package = project
        .create_element(ElementKind::Package, "Model", project.root_id)
        .unwrap();
    let base_signal = project
        .create_element(ElementKind::Signal, "BaseSignal", package)
        .unwrap();
    let specific_signal = project
        .create_element(ElementKind::Signal, "SpecificSignal", package)
        .unwrap();
    project
        .create_relationship(
            RelationshipKind::Generalization,
            specific_signal,
            base_signal,
            Some(package),
        )
        .unwrap();
    let interface = project
        .create_element(ElementKind::InterfaceBlock, "SpecificInterface", package)
        .unwrap();
    let flow = project
        .create_typed_feature(
            ElementKind::FlowProperty,
            "specific",
            interface,
            specific_signal,
            Multiplicity::ONE,
        )
        .unwrap();
    project.element_mut(flow).unwrap().flow_direction = Some(FlowDirection::Out);
    let source_type = project
        .create_element(ElementKind::Block, "Source", package)
        .unwrap();
    let source_port = project
        .create_typed_feature(
            ElementKind::ProxyPort,
            "out",
            source_type,
            interface,
            Multiplicity::ONE,
        )
        .unwrap();
    let target_type = project
        .create_element(ElementKind::Block, "Target", package)
        .unwrap();
    let target_port = project
        .create_typed_feature(
            ElementKind::ProxyPort,
            "in",
            target_type,
            interface,
            Multiplicity::ONE,
        )
        .unwrap();
    project.element_mut(target_port).unwrap().is_conjugated = true;
    let system = project
        .create_element(ElementKind::Block, "System", package)
        .unwrap();
    let source_part = project
        .create_typed_feature(
            ElementKind::PartProperty,
            "source",
            system,
            source_type,
            Multiplicity::ONE,
        )
        .unwrap();
    let target_part = project
        .create_typed_feature(
            ElementKind::PartProperty,
            "target",
            system,
            target_type,
            Multiplicity::ONE,
        )
        .unwrap();
    let source_end = ConnectorEnd::nested_port(vec![source_part], source_port);
    let target_end = ConnectorEnd::nested_port(vec![target_part], target_port);
    let connector = project
        .create_connector(Connector {
            context_id: system,
            kind: ConnectorKind::Assembly,
            source: source_end.clone(),
            target: target_end.clone(),
        })
        .unwrap();
    project
        .create_item_flow(ItemFlow {
            connector_id: connector,
            source: source_end,
            target: target_end,
            conveyed_item_ids: vec![base_signal],
        })
        .unwrap();
    let error =
        StructuralRuntime::build(&project, system, &StructuralRuntimeConfiguration::default())
            .unwrap_err();
    assert!(matches!(
        error,
        StructuralRuntimeError::ItemFlowTypeMismatch { .. }
    ));
}

#[test]
fn reception_does_not_accept_a_more_general_signal_than_it_declares() {
    let mut project = Project::new("Reception conformance");
    let package = project
        .create_element(ElementKind::Package, "Model", project.root_id)
        .unwrap();
    let base_signal = project
        .create_element(ElementKind::Signal, "BaseSignal", package)
        .unwrap();
    let specific_signal = project
        .create_element(ElementKind::Signal, "SpecificSignal", package)
        .unwrap();
    project
        .create_relationship(
            RelationshipKind::Generalization,
            specific_signal,
            base_signal,
            Some(package),
        )
        .unwrap();
    let interface = project
        .create_element(ElementKind::InterfaceBlock, "UntypedContract", package)
        .unwrap();
    let source_type = project
        .create_element(ElementKind::Block, "Source", package)
        .unwrap();
    let source_port = project
        .create_typed_feature(
            ElementKind::ProxyPort,
            "out",
            source_type,
            interface,
            Multiplicity::ONE,
        )
        .unwrap();
    let target_type = project
        .create_element(ElementKind::Block, "Target", package)
        .unwrap();
    let target_port = project
        .create_typed_feature(
            ElementKind::ProxyPort,
            "in",
            target_type,
            interface,
            Multiplicity::ONE,
        )
        .unwrap();
    let reception = project
        .create_element(ElementKind::Reception, "specificOnly", target_type)
        .unwrap();
    project
        .set_element_type(reception, specific_signal)
        .unwrap();
    let system = project
        .create_element(ElementKind::Block, "System", package)
        .unwrap();
    let source_part = project
        .create_typed_feature(
            ElementKind::PartProperty,
            "source",
            system,
            source_type,
            Multiplicity::ONE,
        )
        .unwrap();
    let target_part = project
        .create_typed_feature(
            ElementKind::PartProperty,
            "target",
            system,
            target_type,
            Multiplicity::ONE,
        )
        .unwrap();
    project
        .create_connector(Connector {
            context_id: system,
            kind: ConnectorKind::Assembly,
            source: ConnectorEnd::nested_port(vec![source_part], source_port),
            target: ConnectorEnd::nested_port(vec![target_part], target_port),
        })
        .unwrap();
    let runtime =
        StructuralRuntime::build(&project, system, &StructuralRuntimeConfiguration::default())
            .unwrap();
    let source = runtime.instance_by_path("System.source").unwrap();
    let error = runtime
        .signal_destinations(&project, source.id, source_port, base_signal)
        .unwrap_err();
    assert!(matches!(
        error,
        StructuralRuntimeError::ReceptionMismatch { .. }
    ));
}

#[test]
fn structural_runtime_snapshot_is_json_safe_and_deterministic() {
    let fixture = vehicle_fixture();
    let first = build_vehicle(&fixture).snapshot();
    let second = build_vehicle(&fixture).snapshot();
    assert_eq!(first, second);
    let encoded = serde_json::to_string(&first).unwrap();
    let decoded: StructuralRuntimeSnapshot = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, first);
}

#[allow(dead_code)]
fn pr33_semantic_hardening_tests_marker() {}
