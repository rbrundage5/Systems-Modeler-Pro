use systems_modeler_core::*;

fn vertex(name: &str, kind: VertexKind) -> Vertex {
    Vertex {
        id: VertexId::new(),
        name: name.into(),
        kind,
    }
}

fn transition(source: &Vertex, target: &Vertex) -> Transition {
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

fn waiting_time_activity(
    project: &Project,
    repository: &mut ActivityRepository,
    owner: ElementId,
    context: ElementId,
) -> ActivityId {
    let id = repository
        .create_activity(project, owner, Some(context), "Wait for Time")
        .unwrap();
    let activity = repository.activities.get_mut(&id).unwrap();
    let initial = activity_node("Initial", ActivityNodeKind::Initial);
    let timer = activity_node(
        "Timer",
        ActivityNodeKind::Action(Action {
            kind: ActionKind::AcceptTimeEvent {
                expression: "after 10s".into(),
            },
            pins: Vec::new(),
        }),
    );
    let final_node = activity_node("Final", ActivityNodeKind::ActivityFinal);
    activity
        .nodes
        .extend([initial.clone(), timer.clone(), final_node.clone()]);
    activity.edges.extend([
        activity_control(&initial, &timer),
        activity_control(&timer, &final_node),
    ]);
    id
}

fn waiting_signal_activity(
    project: &Project,
    repository: &mut ActivityRepository,
    owner: ElementId,
    context: ElementId,
    signal_id: ElementId,
) -> ActivityId {
    let id = repository
        .create_activity(project, owner, Some(context), "Wait for Signal")
        .unwrap();
    let activity = repository.activities.get_mut(&id).unwrap();
    let initial = activity_node("Initial", ActivityNodeKind::Initial);
    let accept = activity_node(
        "Accept",
        ActivityNodeKind::Action(Action {
            kind: ActionKind::AcceptEvent {
                signal_id: Some(signal_id),
            },
            pins: Vec::new(),
        }),
    );
    let final_node = activity_node("Final", ActivityNodeKind::ActivityFinal);
    activity
        .nodes
        .extend([initial.clone(), accept.clone(), final_node.clone()]);
    activity.edges.extend([
        activity_control(&initial, &accept),
        activity_control(&accept, &final_node),
    ]);
    id
}

fn fixture(
    name: &str,
) -> (
    Project,
    ElementId,
    ElementId,
    BehaviorRepository,
    StateMachineId,
    ActivityRepository,
) {
    let mut project = Project::new("PR32 State Activity reset");
    let behavior = project
        .create_element(ElementKind::Package, "Behavior", project.root_id)
        .unwrap();
    let context = project
        .create_element(ElementKind::Block, "Controller", behavior)
        .unwrap();
    let mut behaviors = BehaviorRepository::default();
    let machine_id = behaviors
        .create_state_machine(&project, context, name)
        .unwrap();
    (
        project,
        behavior,
        context,
        behaviors,
        machine_id,
        ActivityRepository::default(),
    )
}

fn execution_session(project: &Project, context: ElementId) -> ExecutionSession {
    ExecutionSession::with_configuration(
        project,
        ExecutionConfiguration {
            root_semantic_id: context,
            random_seed: 0,
            max_steps: 10_000,
            max_queued_events: 1_000,
        },
    )
    .unwrap()
}

fn active_names(engine: &StateMachineExecutionEngine, session: &ExecutionSession) -> Vec<String> {
    engine
        .snapshot(session)
        .active_states
        .into_iter()
        .map(|state| state.state_name)
        .collect()
}

fn advance_until_event(
    engine: &mut StateMachineExecutionEngine,
    project: &Project,
    session: &mut ExecutionSession,
) {
    for _ in 0..4 {
        engine.advance(project, session).unwrap();
        if engine.snapshot(session).pending_event_count > 0 {
            return;
        }
    }
    panic!("State doActivity did not schedule its deterministic TimeEvent");
}

#[test]
fn reset_replays_state_activity_runtime_without_leaking_pending_events() {
    let (project, behavior, context, mut behaviors, machine_id, mut activities) =
        fixture("DoActivityReset");
    let do_activity = waiting_time_activity(&project, &mut activities, behavior, context);
    let initial = vertex("Initial", VertexKind::Pseudostate(PseudostateKind::Initial));
    let working = vertex(
        "Working",
        VertexKind::State(State {
            do_activity: Some(do_activity.to_string()),
            ..State::default()
        }),
    );
    let machine = behaviors.state_machines.get_mut(&machine_id).unwrap();
    machine.regions[0]
        .vertices
        .extend([initial.clone(), working.clone()]);
    machine.regions[0]
        .transitions
        .push(transition(&initial, &working));
    activities.validate(&project).unwrap();
    behaviors.validate(&project).unwrap();

    let fresh_behaviors = behaviors.clone();
    let fresh_activities = activities.clone();
    let authored_project = serde_json::to_string(&project).unwrap();
    let authored_behaviors = serde_json::to_string(&behaviors).unwrap();

    let mut engine = StateMachineExecutionEngine::new(behaviors, machine_id)
        .with_activity_repository(activities);
    let mut session = execution_session(&project, context);
    engine.initialize(&project, &mut session).unwrap();
    advance_until_event(&mut engine, &project, &mut session);
    assert!(engine.snapshot(&session).pending_event_count > 0);

    engine.reset(&project, &mut session).unwrap();
    assert_eq!(active_names(&engine, &session), ["Working"]);
    assert_eq!(session.simulation_time, SimulationTime::ZERO);
    assert_eq!(engine.snapshot(&session).pending_event_count, 0);
    advance_until_event(&mut engine, &project, &mut session);

    let mut fresh_engine = StateMachineExecutionEngine::new(fresh_behaviors, machine_id)
        .with_activity_repository(fresh_activities);
    let mut fresh_session = execution_session(&project, context);
    fresh_engine
        .initialize(&project, &mut fresh_session)
        .unwrap();
    advance_until_event(&mut fresh_engine, &project, &mut fresh_session);

    assert_eq!(
        active_names(&engine, &session),
        active_names(&fresh_engine, &fresh_session)
    );
    assert_eq!(
        engine.snapshot(&session).pending_event_count,
        fresh_engine.snapshot(&fresh_session).pending_event_count
    );
    assert_eq!(session.simulation_time, fresh_session.simulation_time);
    assert_eq!(
        serde_json::to_string(&session.trace).unwrap(),
        serde_json::to_string(&fresh_session.trace).unwrap()
    );
    assert_eq!(serde_json::to_string(&project).unwrap(), authored_project);
    assert_eq!(
        serde_json::to_string(engine.authored_repository()).unwrap(),
        authored_behaviors
    );
}

#[test]
fn state_activity_execution_is_repeatable_for_identical_inputs() {
    let (mut project, behavior, context, mut behaviors, machine_id, mut activities) =
        fixture("DoActivityRepeatable");
    let proceed = project
        .create_element(ElementKind::Signal, "Proceed", behavior)
        .unwrap();
    let do_activity =
        waiting_signal_activity(&project, &mut activities, behavior, context, proceed);
    let initial = vertex("Initial", VertexKind::Pseudostate(PseudostateKind::Initial));
    let working = vertex(
        "Working",
        VertexKind::State(State {
            do_activity: Some(do_activity.to_string()),
            ..State::default()
        }),
    );
    let done = vertex("Done", VertexKind::State(State::default()));
    let machine = behaviors.state_machines.get_mut(&machine_id).unwrap();
    machine.regions[0]
        .vertices
        .extend([initial.clone(), working.clone(), done.clone()]);
    machine.regions[0]
        .transitions
        .extend([transition(&initial, &working), transition(&working, &done)]);
    activities.validate(&project).unwrap();
    behaviors.validate(&project).unwrap();

    let run = |behaviors: BehaviorRepository, activities: ActivityRepository| {
        let mut engine = StateMachineExecutionEngine::new(behaviors, machine_id)
            .with_activity_repository(activities);
        let mut session = execution_session(&project, context);
        engine.initialize(&project, &mut session).unwrap();
        for _ in 0..4 {
            if engine.advance(&project, &mut session).unwrap() == EngineStepOutcome::Idle {
                break;
            }
        }
        engine
            .queue_signal(&project, &mut session, proceed, "Proceed", Vec::new())
            .unwrap();
        for _ in 0..8 {
            engine.advance(&project, &mut session).unwrap();
            if active_names(&engine, &session) == ["Done"] {
                break;
            }
        }
        assert_eq!(active_names(&engine, &session), ["Done"]);
        serde_json::to_string(&session.trace).unwrap()
    };

    assert_eq!(
        run(behaviors.clone(), activities.clone()),
        run(behaviors, activities)
    );
}
