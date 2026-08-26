use systems_modeler_core::*;

fn node(name: &str, kind: ActivityNodeKind) -> ActivityNode {
    ActivityNode {
        id: ActivityNodeId::new(),
        name: name.into(),
        kind,
        partition_id: None,
        structured_node_id: None,
    }
}

fn action(name: &str) -> ActivityNode {
    node(
        name,
        ActivityNodeKind::Action(Action {
            kind: ActionKind::Opaque {
                body: String::new(),
            },
            pins: Vec::new(),
        }),
    )
}

fn edge(
    kind: ActivityEdgeKind,
    source: ActivityEndpoint,
    target: ActivityEndpoint,
    guard: Option<&str>,
) -> ActivityEdge {
    ActivityEdge {
        id: ActivityEdgeId::new(),
        name: String::new(),
        kind,
        source,
        target,
        guard: guard.map(str::to_owned),
        weight: None,
        selection: None,
        transformation: None,
        interrupting_region_id: None,
    }
}

fn control(source: &ActivityNode, target: &ActivityNode) -> ActivityEdge {
    edge(
        ActivityEdgeKind::ControlFlow,
        ActivityEndpoint::Node(source.id),
        ActivityEndpoint::Node(target.id),
        None,
    )
}

fn execution_project() -> (Project, ElementId, ElementId) {
    let mut project = Project::new("PR31 Activity execution");
    let behavior = project
        .create_element(ElementKind::Package, "Behavior", project.root_id)
        .unwrap();
    let controller = project
        .create_element(ElementKind::Block, "Controller", behavior)
        .unwrap();
    (project, behavior, controller)
}

fn run_to_completion(
    project: &Project,
    engine: &mut ActivityExecutionEngine,
    session: &mut ExecutionSession,
) {
    for _ in 0..1_000 {
        match engine.advance(project, session).unwrap() {
            ActivityAdvanceOutcome::Completed => return,
            ActivityAdvanceOutcome::Progressed | ActivityAdvanceOutcome::Waiting => {}
        }
    }
    panic!("Activity did not complete within the test bound");
}

fn control_fixture() -> (Project, ActivityRepository, ActivityId, ElementId) {
    let (mut project, behavior, controller) = execution_project();
    let real = project
        .create_element(ElementKind::PrimitiveType, "Real", behavior)
        .unwrap();
    let speed = project
        .create_typed_feature(
            ElementKind::ValueProperty,
            "speed",
            controller,
            real,
            Multiplicity::ONE,
        )
        .unwrap();
    project.element_mut(speed).unwrap().default_value = Some("0".into());

    let mut repository = ActivityRepository::default();
    let activity_id = repository
        .create_activity(
            &project,
            behavior,
            Some(controller),
            "Deterministic Control",
        )
        .unwrap();
    let activity = repository.activities.get_mut(&activity_id).unwrap();
    let initial = node("Initial", ActivityNodeKind::Initial);
    let initialize = action("Initialize");
    let decision = node(
        "Select path",
        ActivityNodeKind::Decision {
            decision_input: None,
        },
    );
    let high = action("A");
    let low = action("B");
    let merge = node("Merge", ActivityNodeKind::Merge);
    let fork = node("Fork", ActivityNodeKind::Fork);
    let task_one = action("Task1");
    let task_two = action("Task2");
    let join = node(
        "Join",
        ActivityNodeKind::Join {
            join_specification: None,
        },
    );
    let final_node = node("Final", ActivityNodeKind::ActivityFinal);
    activity.edges.extend([
        control(&initial, &initialize),
        control(&initialize, &decision),
        edge(
            ActivityEdgeKind::ControlFlow,
            ActivityEndpoint::Node(decision.id),
            ActivityEndpoint::Node(high.id),
            Some("speed >= 20"),
        ),
        edge(
            ActivityEdgeKind::ControlFlow,
            ActivityEndpoint::Node(decision.id),
            ActivityEndpoint::Node(low.id),
            Some("else"),
        ),
        control(&high, &merge),
        control(&low, &merge),
        control(&merge, &fork),
        control(&fork, &task_one),
        control(&fork, &task_two),
        control(&task_one, &join),
        control(&task_two, &join),
        control(&join, &final_node),
    ]);
    activity.nodes.extend([
        initial, initialize, decision, high, low, merge, fork, task_one, task_two, join, final_node,
    ]);
    repository.validate(&project).unwrap();
    (project, repository, activity_id, speed)
}

#[test]
fn executes_control_nodes_deterministically_and_reset_replays_initial_state() {
    let (project, repository, activity_id, speed) = control_fixture();

    let execute = || {
        let mut session = ExecutionSession::with_configuration(
            &project,
            ExecutionConfiguration {
                root_semantic_id: project.root_id,
                random_seed: 7,
                max_steps: 100,
                max_queued_events: 100,
            },
        )
        .unwrap();
        let mut engine = ActivityExecutionEngine::new(repository.clone(), activity_id);
        engine.initialize(&project, &mut session).unwrap();
        session
            .set_value(&project, None, speed, RuntimeValue::Real(27.5))
            .unwrap();
        let initialized = engine.snapshot(&session);
        run_to_completion(&project, &mut engine, &mut session);
        let completed = engine.snapshot(&session);
        engine.reset(&project, &mut session).unwrap();
        let reset = engine.snapshot(&session);
        assert_eq!(initialized.nodes, reset.nodes);
        assert_eq!(initialized.token_stores, reset.token_stores);
        (completed, session.trace.clone())
    };

    let (first, first_trace) = execute();
    let (second, second_trace) = execute();
    assert_eq!(first.execution.state, ExecutionState::Completed);
    assert_eq!(first.nodes, second.nodes);
    assert_eq!(first.token_stores, second.token_stores);
    assert_eq!(first_trace, second_trace);
    assert!(
        first
            .nodes
            .iter()
            .any(|node| node.name == "A" && node.activation_count == 1)
    );
    assert!(
        first
            .nodes
            .iter()
            .any(|node| node.name == "B" && node.activation_count == 0)
    );
    assert_eq!(
        project.element(speed).unwrap().default_value.as_deref(),
        Some("0")
    );
}

#[test]
fn object_tokens_flow_through_buffer_and_typed_pins() {
    let (mut project, behavior, controller) = execution_project();
    let command_type = project
        .create_element(ElementKind::PrimitiveType, "Integer", behavior)
        .unwrap();
    let mut repository = ActivityRepository::default();
    let activity_id = repository
        .create_activity(&project, behavior, Some(controller), "Object tokens")
        .unwrap();
    let activity = repository.activities.get_mut(&activity_id).unwrap();
    let initial = node("Initial", ActivityNodeKind::Initial);
    let output = Pin {
        value: Some("41 + 1".into()),
        ..Pin::output("command", Some(command_type))
    };
    let output_id = output.id;
    let producer = node(
        "Produce",
        ActivityNodeKind::Action(Action {
            kind: ActionKind::Opaque {
                body: String::new(),
            },
            pins: vec![output],
        }),
    );
    let buffer = node(
        "Commands",
        ActivityNodeKind::Object(ObjectNode {
            kind: ObjectNodeKind::CentralBuffer,
            type_id: Some(command_type),
            multiplicity: Multiplicity::new(0, None).unwrap(),
            ordering: ObjectNodeOrdering::Fifo,
            selection: None,
        }),
    );
    let input = Pin::input("command", Some(command_type));
    let input_id = input.id;
    let consumer = node(
        "Consume",
        ActivityNodeKind::Action(Action {
            kind: ActionKind::Opaque {
                body: "command == 42".into(),
            },
            pins: vec![input],
        }),
    );
    let final_node = node("Final", ActivityNodeKind::ActivityFinal);
    activity.edges.extend([
        control(&initial, &producer),
        control(&producer, &consumer),
        control(&consumer, &final_node),
        edge(
            ActivityEdgeKind::ObjectFlow,
            ActivityEndpoint::Pin(output_id),
            ActivityEndpoint::Node(buffer.id),
            None,
        ),
        edge(
            ActivityEdgeKind::ObjectFlow,
            ActivityEndpoint::Node(buffer.id),
            ActivityEndpoint::Pin(input_id),
            None,
        ),
    ]);
    activity
        .nodes
        .extend([initial, producer, buffer, consumer, final_node]);
    repository.validate(&project).unwrap();

    let mut session = ExecutionSession::new(&project);
    let mut engine = ActivityExecutionEngine::new(repository, activity_id);
    engine.initialize(&project, &mut session).unwrap();
    run_to_completion(&project, &mut engine, &mut session);
    let snapshot = engine.snapshot(&session);
    assert_eq!(snapshot.execution.state, ExecutionState::Completed);
    assert!(snapshot.nodes.iter().any(|node| {
        node.name == "Consume"
            && node.state == ActivityNodeExecutionState::Completed
            && node.activation_count == 1
    }));
}

#[test]
fn signal_and_time_actions_wait_for_deterministic_runtime_events() {
    let (mut project, behavior, controller) = execution_project();
    let start_signal = project
        .create_element(ElementKind::Signal, "Start", behavior)
        .unwrap();
    let mut repository = ActivityRepository::default();
    let activity_id = repository
        .create_activity(&project, behavior, Some(controller), "Events")
        .unwrap();
    let activity = repository.activities.get_mut(&activity_id).unwrap();
    let initial = node("Initial", ActivityNodeKind::Initial);
    let send = node(
        "Send Start",
        ActivityNodeKind::Action(Action {
            kind: ActionKind::SendSignal {
                signal_id: start_signal,
            },
            pins: Vec::new(),
        }),
    );
    let accept = node(
        "Wait for Start",
        ActivityNodeKind::Action(Action {
            kind: ActionKind::AcceptEvent {
                signal_id: Some(start_signal),
            },
            pins: Vec::new(),
        }),
    );
    let timer = node(
        "Wait five milliseconds",
        ActivityNodeKind::Action(Action {
            kind: ActionKind::AcceptTimeEvent {
                expression: "after 5ms".into(),
            },
            pins: Vec::new(),
        }),
    );
    let final_node = node("Final", ActivityNodeKind::ActivityFinal);
    activity.edges.extend([
        control(&initial, &send),
        control(&send, &accept),
        control(&accept, &timer),
        control(&timer, &final_node),
    ]);
    activity
        .nodes
        .extend([initial, send, accept, timer, final_node]);
    repository.validate(&project).unwrap();

    let mut session = ExecutionSession::new(&project);
    let mut engine = ActivityExecutionEngine::new(repository, activity_id);
    engine.initialize(&project, &mut session).unwrap();
    run_to_completion(&project, &mut engine, &mut session);
    assert_eq!(session.state, ExecutionState::Completed);
    assert_eq!(
        session.simulation_time,
        SimulationTime::from_nanos(5_000_000)
    );
    assert!(session.trace.iter().any(|entry| {
        entry.message.contains("Wait for Start") && entry.message.contains("accepted signal")
    }));
}

#[test]
fn call_behavior_uses_a_nested_frame_and_returns_to_the_caller() {
    let (project, behavior, controller) = execution_project();
    let mut repository = ActivityRepository::default();
    let child_id = repository
        .create_activity(&project, behavior, Some(controller), "Child")
        .unwrap();
    let child = repository.activities.get_mut(&child_id).unwrap();
    let child_initial = node("Child Initial", ActivityNodeKind::Initial);
    let child_action = action("Child Task");
    let child_final = node("Child Final", ActivityNodeKind::ActivityFinal);
    child.edges.extend([
        control(&child_initial, &child_action),
        control(&child_action, &child_final),
    ]);
    child
        .nodes
        .extend([child_initial, child_action, child_final]);

    let root_id = repository
        .create_activity(&project, behavior, Some(controller), "Parent")
        .unwrap();
    let root = repository.activities.get_mut(&root_id).unwrap();
    let initial = node("Initial", ActivityNodeKind::Initial);
    let call = node(
        "Call Child",
        ActivityNodeKind::Action(Action {
            kind: ActionKind::CallBehavior {
                activity_id: child_id,
            },
            pins: Vec::new(),
        }),
    );
    let final_node = node("Final", ActivityNodeKind::ActivityFinal);
    root.edges
        .extend([control(&initial, &call), control(&call, &final_node)]);
    root.nodes.extend([initial, call, final_node]);
    repository.validate(&project).unwrap();

    let mut session = ExecutionSession::new(&project);
    let mut engine = ActivityExecutionEngine::new(repository, root_id);
    engine.initialize(&project, &mut session).unwrap();
    run_to_completion(&project, &mut engine, &mut session);
    assert_eq!(session.state, ExecutionState::Completed);
    assert!(
        session
            .trace
            .iter()
            .any(|entry| entry.message.contains("call to Activity 'Child' completed"))
    );
}

#[test]
fn unsafe_opaque_text_and_intentional_loop_fail_with_useful_limits() {
    let (project, behavior, controller) = execution_project();
    let mut repository = ActivityRepository::default();
    let activity_id = repository
        .create_activity(&project, behavior, Some(controller), "Unsafe")
        .unwrap();
    let activity = repository.activities.get_mut(&activity_id).unwrap();
    let initial = node("Initial", ActivityNodeKind::Initial);
    let unsafe_action = node(
        "Assignment",
        ActivityNodeKind::Action(Action {
            kind: ActionKind::Opaque {
                body: "x = 1".into(),
            },
            pins: Vec::new(),
        }),
    );
    let final_node = node("Final", ActivityNodeKind::ActivityFinal);
    activity.edges.extend([
        control(&initial, &unsafe_action),
        control(&unsafe_action, &final_node),
    ]);
    activity.nodes.extend([initial, unsafe_action, final_node]);
    repository.validate(&project).unwrap();
    let mut session = ExecutionSession::new(&project);
    let mut engine = ActivityExecutionEngine::new(repository, activity_id);
    engine.initialize(&project, &mut session).unwrap();
    assert_eq!(
        engine.advance(&project, &mut session).unwrap(),
        ActivityAdvanceOutcome::Progressed
    );
    let error = engine.advance(&project, &mut session).unwrap_err();
    assert!(error.to_string().contains("only bounded pure expressions"));
    assert_eq!(session.state, ExecutionState::Failed);

    let mut repository = ActivityRepository::default();
    let loop_id = repository
        .create_activity(&project, behavior, Some(controller), "Bounded loop")
        .unwrap();
    let activity = repository.activities.get_mut(&loop_id).unwrap();
    let initial = node("Initial", ActivityNodeKind::Initial);
    let merge = node("Merge", ActivityNodeKind::Merge);
    let repeat = action("Repeat");
    activity.edges.extend([
        control(&initial, &merge),
        control(&merge, &repeat),
        control(&repeat, &merge),
    ]);
    activity.nodes.extend([initial, merge, repeat]);
    repository.validate(&project).unwrap();
    let configuration = ExecutionConfiguration {
        root_semantic_id: project.root_id,
        random_seed: 0,
        max_steps: 5,
        max_queued_events: 10,
    };
    let mut session = ExecutionSession::with_configuration(&project, configuration).unwrap();
    let mut engine = ActivityExecutionEngine::new(repository, loop_id);
    engine.initialize(&project, &mut session).unwrap();
    let error = loop {
        match engine.advance(&project, &mut session) {
            Ok(_) => {}
            Err(error) => break error,
        }
    };
    assert_eq!(error, ExecutionError::StepLimitExceeded { limit: 5 });
    assert_eq!(session.state, ExecutionState::Failed);
}
