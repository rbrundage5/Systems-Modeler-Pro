use systems_modeler_core::*;

struct RuntimeFixture {
    project: Project,
    system: ElementId,
    controller: ElementId,
    left: ElementId,
    right: ElementId,
    operation: ElementId,
    input: ElementId,
    returned: ElementId,
    signal: ElementId,
    reception: ElementId,
    outbound: ElementId,
    inbound: ElementId,
}

fn fixture() -> RuntimeFixture {
    let mut project = Project::new("PR34 runtime qualification");
    let package = project
        .create_element(ElementKind::Package, "VehicleModel", project.root_id)
        .unwrap();
    let integer = project
        .create_element(ElementKind::PrimitiveType, "Integer", package)
        .unwrap();
    let string = project
        .create_element(ElementKind::PrimitiveType, "String", package)
        .unwrap();
    let signal = project
        .create_element(ElementKind::Signal, "Start", package)
        .unwrap();
    let interface = project
        .create_element(ElementKind::InterfaceBlock, "StartInterface", package)
        .unwrap();
    let flow = project
        .create_typed_feature(
            ElementKind::FlowProperty,
            "start",
            interface,
            signal,
            Multiplicity::ONE,
        )
        .unwrap();
    project.element_mut(flow).unwrap().flow_direction = Some(FlowDirection::Out);

    let controller = project
        .create_element(ElementKind::Block, "Controller", package)
        .unwrap();
    let outbound = project
        .create_typed_feature(
            ElementKind::ProxyPort,
            "startOut",
            controller,
            interface,
            Multiplicity::ONE,
        )
        .unwrap();
    let inbound = project
        .create_typed_feature(
            ElementKind::ProxyPort,
            "startIn",
            controller,
            interface,
            Multiplicity::ONE,
        )
        .unwrap();
    project.element_mut(inbound).unwrap().is_conjugated = true;
    let reception = project
        .create_element(ElementKind::Reception, "receiveStart", controller)
        .unwrap();
    project.set_element_type(reception, signal).unwrap();
    let operation = project
        .create_element(ElementKind::Operation, "start", controller)
        .unwrap();
    let input = project
        .create_typed_feature(
            ElementKind::Parameter,
            "level",
            operation,
            integer,
            Multiplicity::ONE,
        )
        .unwrap();
    project.element_mut(input).unwrap().parameter_direction = Some(ParameterDirection::In);
    let returned = project
        .create_typed_feature(
            ElementKind::Parameter,
            "status",
            operation,
            string,
            Multiplicity::ONE,
        )
        .unwrap();
    {
        let returned = project.element_mut(returned).unwrap();
        returned.parameter_direction = Some(ParameterDirection::Return);
        returned.default_value = Some("\"started\"".into());
    }

    let system = project
        .create_element(ElementKind::Block, "Vehicle", package)
        .unwrap();
    let left = project
        .create_typed_feature(
            ElementKind::PartProperty,
            "leftController",
            system,
            controller,
            Multiplicity::ONE,
        )
        .unwrap();
    let right = project
        .create_typed_feature(
            ElementKind::PartProperty,
            "rightController",
            system,
            controller,
            Multiplicity::ONE,
        )
        .unwrap();
    let source = ConnectorEnd::nested_port(vec![left], outbound);
    let target = ConnectorEnd::nested_port(vec![right], inbound);
    let connector = project
        .create_connector(Connector {
            context_id: system,
            kind: ConnectorKind::Assembly,
            source: source.clone(),
            target: target.clone(),
        })
        .unwrap();
    project
        .create_item_flow(ItemFlow {
            connector_id: connector,
            source,
            target,
            conveyed_item_ids: vec![signal],
        })
        .unwrap();
    RuntimeFixture {
        project,
        system,
        controller,
        left,
        right,
        operation,
        input,
        returned,
        signal,
        reception,
        outbound,
        inbound,
    }
}

fn session(fixture: &RuntimeFixture) -> ExecutionSession {
    let mut session = ExecutionSession::with_configuration(
        &fixture.project,
        ExecutionConfiguration {
            root_semantic_id: fixture.system,
            random_seed: 0,
            max_steps: 100,
            max_queued_events: 100,
        },
    )
    .unwrap();
    session.initialize(&fixture.project).unwrap();
    session
}

fn occurrences(
    fixture: &RuntimeFixture,
    session: &ExecutionSession,
) -> (RuntimeInstanceId, RuntimeInstanceId) {
    let runtime = session.structural_runtime.as_ref().unwrap();
    (
        runtime.instances_for_usage(fixture.left)[0].id,
        runtime.instances_for_usage(fixture.right)[0].id,
    )
}

fn activity_node(name: &str, kind: ActivityNodeKind) -> ActivityNode {
    ActivityNode {
        id: ActivityNodeId::new(),
        name: name.into(),
        kind,
        partition_id: None,
        structured_node_id: None,
    }
}

fn activity_control(source: &ActivityNode, target: &ActivityNode) -> ActivityEdge {
    ActivityEdge {
        id: ActivityEdgeId::new(),
        name: String::new(),
        kind: ActivityEdgeKind::ControlFlow,
        source: ActivityEndpoint::Node(source.id),
        target: ActivityEndpoint::Node(target.id),
        guard: None,
        weight: None,
        selection: None,
        transformation: None,
        interrupting_region_id: None,
    }
}

#[test]
fn modeled_operation_targets_one_occurrence_and_returns_authored_value() {
    let fixture = fixture();
    let mut session = session(&fixture);
    let (left, right) = occurrences(&fixture, &session);
    let result = invoke_modeled_operation(
        &fixture.project,
        &mut session,
        &ModeledOperationRequest {
            operation_id: fixture.operation,
            target_runtime_instance_id: left,
            arguments: vec![("level".into(), RuntimeValue::Integer(2))],
        },
    )
    .unwrap();
    assert_eq!(
        result.outputs,
        vec![("status".into(), RuntimeValue::Text("started".into()))]
    );
    assert_eq!(
        session.value(Some(left), fixture.input),
        Some(&RuntimeValue::Integer(2))
    );
    assert_ne!(
        session.value(Some(right), fixture.input),
        Some(&RuntimeValue::Integer(2))
    );
    assert_eq!(
        session.value(Some(left), fixture.returned),
        Some(&RuntimeValue::Text("started".into()))
    );
}

#[test]
fn two_same_typed_occurrences_execute_independently() {
    let fixture = fixture();
    let mut session = session(&fixture);
    let (left, right) = occurrences(&fixture, &session);
    for (target, value) in [(left, 3), (right, 8)] {
        invoke_modeled_operation(
            &fixture.project,
            &mut session,
            &ModeledOperationRequest {
                operation_id: fixture.operation,
                target_runtime_instance_id: target,
                arguments: vec![("level".into(), RuntimeValue::Integer(value))],
            },
        )
        .unwrap();
    }
    assert_eq!(
        session.value(Some(left), fixture.input),
        Some(&RuntimeValue::Integer(3))
    );
    assert_eq!(
        session.value(Some(right), fixture.input),
        Some(&RuntimeValue::Integer(8))
    );
}

#[test]
fn operation_parameter_presence_and_type_are_enforced() {
    let fixture = fixture();
    let mut session = session(&fixture);
    let (left, _) = occurrences(&fixture, &session);
    let missing = invoke_modeled_operation(
        &fixture.project,
        &mut session,
        &ModeledOperationRequest {
            operation_id: fixture.operation,
            target_runtime_instance_id: left,
            arguments: Vec::new(),
        },
    )
    .unwrap_err()
    .to_string();
    assert!(missing.contains("required input parameter 'level'"));
    let wrong_type = invoke_modeled_operation(
        &fixture.project,
        &mut session,
        &ModeledOperationRequest {
            operation_id: fixture.operation,
            target_runtime_instance_id: left,
            arguments: vec![("level".into(), RuntimeValue::Text("fast".into()))],
        },
    )
    .unwrap_err()
    .to_string();
    assert!(wrong_type.contains("input parameter 'level' is invalid"));
    assert!(wrong_type.contains("Integer"));
}

#[test]
fn operation_rejects_incompatible_runtime_target_with_names() {
    let fixture = fixture();
    let mut project = fixture.project.clone();
    let package = project.element(fixture.system).unwrap().owner_id.unwrap();
    let sensor = project
        .create_element(ElementKind::Block, "Sensor", package)
        .unwrap();
    let sensor_part = project
        .create_typed_feature(
            ElementKind::PartProperty,
            "sensor",
            fixture.system,
            sensor,
            Multiplicity::ONE,
        )
        .unwrap();
    let expanded = RuntimeFixture { project, ..fixture };
    let mut session = session(&expanded);
    let sensor_occurrence = session
        .structural_runtime
        .as_ref()
        .unwrap()
        .instances_for_usage(sensor_part)[0]
        .id;
    let error = invoke_modeled_operation(
        &expanded.project,
        &mut session,
        &ModeledOperationRequest {
            operation_id: expanded.operation,
            target_runtime_instance_id: sensor_occurrence,
            arguments: vec![("level".into(), RuntimeValue::Integer(1))],
        },
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("Sensor"));
    assert!(error.contains("Controller"));
    assert!(error.contains("start"));
}

#[test]
fn structural_signal_targets_only_the_connected_occurrence() {
    let fixture = fixture();
    let mut session = session(&fixture);
    let (left, right) = occurrences(&fixture, &session);
    session
        .queue_structural_signal_to_instance(
            &fixture.project,
            left,
            right,
            fixture.signal,
            "Start",
            Vec::new(),
        )
        .unwrap();
    let event = &session.next_event().unwrap().event;
    assert_eq!(event.semantic_event_id, Some(fixture.signal));
    assert_eq!(event.source_runtime_instance_id, Some(left));
    assert_eq!(event.target_runtime_instance_id, Some(right));
    assert_eq!(event.source_port_id, Some(fixture.outbound));
    assert_eq!(event.target_port_id, Some(fixture.inbound));
}

#[test]
fn signal_reception_compatibility_and_unrelated_occurrences_are_enforced() {
    let fixture = fixture();
    let mut project = fixture.project.clone();
    let unrelated = project
        .create_typed_feature(
            ElementKind::PartProperty,
            "unrelatedController",
            fixture.system,
            fixture.controller,
            Multiplicity::ONE,
        )
        .unwrap();
    let expanded = RuntimeFixture { project, ..fixture };
    let mut expanded_session = session(&expanded);
    let (left, _) = occurrences(&expanded, &expanded_session);
    let unrelated = expanded_session
        .structural_runtime
        .as_ref()
        .unwrap()
        .instances_for_usage(unrelated)[0]
        .id;
    let error = expanded_session
        .queue_structural_signal_to_instance(
            &expanded.project,
            left,
            unrelated,
            expanded.signal,
            "Start",
            Vec::new(),
        )
        .unwrap_err()
        .to_string();
    assert!(error.contains("0 compatible structural routes"));
    assert!(expanded_session.next_event().is_none());

    let mut incompatible_project = expanded.project.clone();
    let package = incompatible_project
        .element(expanded.system)
        .unwrap()
        .owner_id
        .unwrap();
    let other_signal = incompatible_project
        .create_element(ElementKind::Signal, "Stop", package)
        .unwrap();
    incompatible_project
        .set_element_type(expanded.reception, other_signal)
        .unwrap();
    let incompatible = RuntimeFixture {
        project: incompatible_project,
        ..expanded
    };
    let mut session = session(&incompatible);
    let (left, right) = occurrences(&incompatible, &session);
    let error = session
        .queue_structural_signal_to_instance(
            &incompatible.project,
            left,
            right,
            incompatible.signal,
            "Start",
            Vec::new(),
        )
        .unwrap_err()
        .to_string();
    assert!(error.contains("0 compatible structural routes"));
    assert!(session.next_event().is_none());
}

#[test]
fn call_operation_action_uses_the_modeled_operation_runtime() {
    let fixture = fixture();
    let mut repository = ActivityRepository::default();
    let activity_id = repository
        .create_activity(
            &fixture.project,
            fixture
                .project
                .element(fixture.system)
                .unwrap()
                .owner_id
                .unwrap(),
            Some(fixture.controller),
            "Start controller",
        )
        .unwrap();
    let initial = activity_node("Initial", ActivityNodeKind::Initial);
    let mut input = Pin::input(
        "level",
        fixture.project.element(fixture.input).unwrap().type_id,
    );
    input.direction = PinDirection::Value;
    input.value = Some("4".into());
    input.parameter_id = Some(fixture.input);
    let mut output = Pin::output(
        "status",
        fixture.project.element(fixture.returned).unwrap().type_id,
    );
    output.parameter_id = Some(fixture.returned);
    let call = activity_node(
        "Invoke start",
        ActivityNodeKind::Action(Action {
            kind: ActionKind::CallOperation {
                operation_id: fixture.operation,
            },
            pins: vec![input, output],
        }),
    );
    let final_node = activity_node("Final", ActivityNodeKind::ActivityFinal);
    let activity = repository.activities.get_mut(&activity_id).unwrap();
    activity.edges.extend([
        activity_control(&initial, &call),
        activity_control(&call, &final_node),
    ]);
    activity.nodes.extend([initial, call, final_node]);
    repository.validate(&fixture.project).unwrap();

    let mut session = ExecutionSession::with_configuration(
        &fixture.project,
        ExecutionConfiguration {
            root_semantic_id: fixture.system,
            random_seed: 0,
            max_steps: 100,
            max_queued_events: 100,
        },
    )
    .unwrap();
    session.initialize(&fixture.project).unwrap();
    let (left, _) = occurrences(&fixture, &session);
    let mut engine =
        ActivityExecutionEngine::new(repository, activity_id).with_runtime_instance(left);
    engine
        .initialize_embedded(&fixture.project, &mut session)
        .unwrap();
    for _ in 0..10 {
        if engine.advance(&fixture.project, &mut session).unwrap()
            == ActivityAdvanceOutcome::Completed
        {
            break;
        }
    }
    assert_eq!(
        session.value(Some(left), fixture.input),
        Some(&RuntimeValue::Integer(4))
    );
}

#[test]
fn send_and_accept_actions_share_typed_structural_signal_delivery() {
    let fixture = fixture();
    let package = fixture
        .project
        .element(fixture.system)
        .unwrap()
        .owner_id
        .unwrap();
    let mut repository = ActivityRepository::default();
    let sender_id = repository
        .create_activity(
            &fixture.project,
            package,
            Some(fixture.controller),
            "Sender",
        )
        .unwrap();
    let receiver_id = repository
        .create_activity(
            &fixture.project,
            package,
            Some(fixture.controller),
            "Receiver",
        )
        .unwrap();

    let sender_initial = activity_node("Initial", ActivityNodeKind::Initial);
    let send = activity_node(
        "Send Start",
        ActivityNodeKind::Action(Action {
            kind: ActionKind::SendSignal {
                signal_id: fixture.signal,
            },
            pins: Vec::new(),
        }),
    );
    let sender_final = activity_node("Final", ActivityNodeKind::ActivityFinal);
    let sender = repository.activities.get_mut(&sender_id).unwrap();
    sender.edges.extend([
        activity_control(&sender_initial, &send),
        activity_control(&send, &sender_final),
    ]);
    sender.nodes.extend([sender_initial, send, sender_final]);

    let receiver_initial = activity_node("Initial", ActivityNodeKind::Initial);
    let accept = activity_node(
        "Accept Start",
        ActivityNodeKind::Action(Action {
            kind: ActionKind::AcceptEvent {
                signal_id: Some(fixture.signal),
            },
            pins: Vec::new(),
        }),
    );
    let accept_id = accept.id;
    let receiver_final = activity_node("Final", ActivityNodeKind::ActivityFinal);
    let receiver = repository.activities.get_mut(&receiver_id).unwrap();
    receiver.edges.extend([
        activity_control(&receiver_initial, &accept),
        activity_control(&accept, &receiver_final),
    ]);
    receiver
        .nodes
        .extend([receiver_initial, accept, receiver_final]);
    repository.validate(&fixture.project).unwrap();

    let mut session = ExecutionSession::with_configuration(
        &fixture.project,
        ExecutionConfiguration {
            root_semantic_id: fixture.system,
            random_seed: 0,
            max_steps: 100,
            max_queued_events: 100,
        },
    )
    .unwrap();
    session.initialize(&fixture.project).unwrap();
    let (left, right) = occurrences(&fixture, &session);
    let mut receiver =
        ActivityExecutionEngine::new(repository.clone(), receiver_id).with_runtime_instance(right);
    receiver
        .initialize_embedded(&fixture.project, &mut session)
        .unwrap();
    assert_eq!(
        receiver.step(&fixture.project, &mut session).unwrap(),
        EngineStepOutcome::Progressed
    );
    assert_eq!(
        receiver.step(&fixture.project, &mut session).unwrap(),
        EngineStepOutcome::Progressed
    );
    let waiting = receiver.snapshot(&session);
    assert!(waiting.nodes.iter().any(|node| {
        node.node_id == accept_id && node.state == ActivityNodeExecutionState::Waiting
    }));

    let mut sender =
        ActivityExecutionEngine::new(repository, sender_id).with_runtime_instance(left);
    sender
        .initialize_embedded(&fixture.project, &mut session)
        .unwrap();
    sender.step(&fixture.project, &mut session).unwrap();
    sender.step(&fixture.project, &mut session).unwrap();
    let event = session.step().unwrap().unwrap();
    assert_eq!(event.semantic_event_id, Some(fixture.signal));
    assert_eq!(event.target_runtime_instance_id, Some(right));
    assert_eq!(
        receiver
            .handle_event(&fixture.project, &mut session, &event)
            .unwrap(),
        EngineStepOutcome::Progressed
    );
    assert!(receiver.snapshot(&session).nodes.iter().any(|node| {
        node.node_id == accept_id && node.state == ActivityNodeExecutionState::Completed
    }));
}

fn sequence_repository(fixture: &RuntimeFixture) -> (BehaviorRepository, InteractionId, VertexId) {
    let mut repository = BehaviorRepository::default();
    let machine_id = repository
        .create_state_machine(&fixture.project, fixture.controller, "Controller lifecycle")
        .unwrap();
    let machine = repository.state_machines.get_mut(&machine_id).unwrap();
    let initial = Vertex {
        id: VertexId::new(),
        name: "Initial".into(),
        kind: VertexKind::Pseudostate(PseudostateKind::Initial),
    };
    let idle = Vertex {
        id: VertexId::new(),
        name: "Idle".into(),
        kind: VertexKind::State(State::default()),
    };
    let active = Vertex {
        id: VertexId::new(),
        name: "Active".into(),
        kind: VertexKind::State(State::default()),
    };
    let active_id = active.id;
    machine.regions[0].transitions.extend([
        Transition {
            id: TransitionId::new(),
            source_id: initial.id,
            target_id: idle.id,
            kind: TransitionKind::External,
            trigger: None,
            guard: None,
            effect: None,
        },
        Transition {
            id: TransitionId::new(),
            source_id: idle.id,
            target_id: active.id,
            kind: TransitionKind::External,
            trigger: Some(Trigger {
                event: Event::Signal {
                    signal_id: fixture.signal,
                },
            }),
            guard: None,
            effect: None,
        },
    ]);
    machine.regions[0].vertices.extend([initial, idle, active]);
    let interaction_id = repository
        .create_interaction(&fixture.project, fixture.system, "Controller startup")
        .unwrap();
    let interaction = repository.interactions.get_mut(&interaction_id).unwrap();
    let left = Lifeline {
        id: LifelineId::new(),
        name: "leftController".into(),
        represented_path: vec![fixture.left],
    };
    let right = Lifeline {
        id: LifelineId::new(),
        name: "rightController".into(),
        represented_path: vec![fixture.right],
    };
    interaction.lifelines.extend([left.clone(), right.clone()]);
    interaction.messages.extend([
        Message {
            id: MessageId::new(),
            name: "start".into(),
            sort: MessageSort::SynchCall,
            send_event: Some(Occurrence {
                id: OccurrenceId::new(),
                lifeline_id: left.id,
                order: 10,
            }),
            receive_event: Some(Occurrence {
                id: OccurrenceId::new(),
                lifeline_id: right.id,
                order: 20,
            }),
            signature: Some(MessageSignature::Operation(fixture.operation)),
            arguments: vec!["9".into()],
        },
        Message {
            id: MessageId::new(),
            name: "Start".into(),
            sort: MessageSort::AsynchSignal,
            send_event: Some(Occurrence {
                id: OccurrenceId::new(),
                lifeline_id: left.id,
                order: 30,
            }),
            receive_event: Some(Occurrence {
                id: OccurrenceId::new(),
                lifeline_id: right.id,
                order: 40,
            }),
            signature: Some(MessageSignature::Signal(fixture.signal)),
            arguments: Vec::new(),
        },
    ]);
    repository.validate(&fixture.project).unwrap();
    (repository, interaction_id, active_id)
}

#[test]
fn sequence_resolves_lifelines_and_executes_operation_then_signal() {
    let fixture = fixture();
    let (repository, interaction_id, active_state_id) = sequence_repository(&fixture);
    let mut session = ExecutionSession::with_configuration(
        &fixture.project,
        ExecutionConfiguration {
            root_semantic_id: fixture.system,
            random_seed: 0,
            max_steps: 100,
            max_queued_events: 100,
        },
    )
    .unwrap();
    let mut engine = SequenceExecutionEngine::new(repository, interaction_id);
    engine.initialize(&fixture.project, &mut session).unwrap();
    let initialized = engine.snapshot(&session);
    assert_eq!(initialized.lifeline_bindings.len(), 2);
    let left = initialized
        .lifeline_bindings
        .iter()
        .find(|binding| binding.lifeline_name == "leftController")
        .unwrap()
        .runtime_instance_id;
    let right = initialized
        .lifeline_bindings
        .iter()
        .find(|binding| binding.lifeline_name == "rightController")
        .unwrap()
        .runtime_instance_id;
    assert_eq!(
        engine.step(&fixture.project, &mut session).unwrap(),
        EngineStepOutcome::Progressed
    );
    assert_eq!(
        session.value(Some(right), fixture.input),
        Some(&RuntimeValue::Integer(9))
    );
    assert_ne!(
        session.value(Some(left), fixture.input),
        Some(&RuntimeValue::Integer(9))
    );
    assert_eq!(
        engine.step(&fixture.project, &mut session).unwrap(),
        EngineStepOutcome::Progressed
    );
    assert!(session.next_event().is_none());
    assert!(session.trace.iter().any(|entry| {
        entry.kind == TraceKind::EventDispatched
            && entry.semantic_element_id == Some(fixture.right)
            && entry.source_runtime_instance_id == Some(left)
            && entry.target_runtime_instance_id == Some(right)
    }));
    let machine_snapshots = engine.state_machine_snapshots(&session);
    assert_eq!(machine_snapshots.len(), 2);
    let right_machine = machine_snapshots
        .iter()
        .find(|snapshot| snapshot.runtime_instance_id == Some(right))
        .unwrap();
    assert!(
        right_machine
            .active_states
            .iter()
            .any(|state| state.state_id == active_state_id)
    );
    let left_machine = machine_snapshots
        .iter()
        .find(|snapshot| snapshot.runtime_instance_id == Some(left))
        .unwrap();
    assert!(
        left_machine
            .active_states
            .iter()
            .all(|state| state.state_id != active_state_id),
        "an addressed Sequence Signal must not drive another same-typed runtime occurrence"
    );
}

#[test]
fn sequence_order_reset_and_authored_model_are_deterministic() {
    let fixture = fixture();
    let before = serde_json::to_string(&fixture.project).unwrap();
    let (repository, interaction_id, _) = sequence_repository(&fixture);
    let authored = serde_json::to_string(&repository).unwrap();
    let execute = || {
        let mut session = ExecutionSession::with_configuration(
            &fixture.project,
            ExecutionConfiguration {
                root_semantic_id: fixture.system,
                random_seed: 0,
                max_steps: 100,
                max_queued_events: 100,
            },
        )
        .unwrap();
        let mut engine = SequenceExecutionEngine::new(repository.clone(), interaction_id);
        engine.initialize(&fixture.project, &mut session).unwrap();
        engine.step(&fixture.project, &mut session).unwrap();
        engine.step(&fixture.project, &mut session).unwrap();
        let completed = engine.snapshot(&session).completed_message_ids;
        let trace = session.trace.clone();
        engine.reset(&fixture.project, &mut session).unwrap();
        assert!(engine.snapshot(&session).completed_message_ids.is_empty());
        (completed, trace)
    };
    let first = execute();
    let second = execute();
    assert_eq!(first, second);
    assert_eq!(before, serde_json::to_string(&fixture.project).unwrap());
    assert_eq!(authored, serde_json::to_string(&repository).unwrap());
}

#[test]
fn reception_type_is_restricted_to_modeled_signal() {
    let mut project = Project::new("Reception typing");
    let package = project
        .create_element(ElementKind::Package, "Pkg", project.root_id)
        .unwrap();
    let controller = project
        .create_element(ElementKind::Block, "Controller", package)
        .unwrap();
    let wrong_type = project
        .create_element(ElementKind::Block, "WrongType", package)
        .unwrap();
    let start = project
        .create_element(ElementKind::Signal, "Start", package)
        .unwrap();
    let reception = project
        .create_element(ElementKind::Reception, "receiveStart", controller)
        .unwrap();

    assert!(project.set_element_type(reception, wrong_type).is_err());
    project.set_element_type(reception, start).unwrap();
    assert_eq!(project.element(reception).unwrap().type_id, Some(start));
}
