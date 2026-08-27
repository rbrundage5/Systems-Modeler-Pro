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

fn signal_transition(source: &Vertex, target: &Vertex, signal_id: ElementId) -> Transition {
    Transition {
        trigger: Some(Trigger {
            event: Event::Signal { signal_id },
        }),
        ..transition(source, target)
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

fn synchronous_activity(
    project: &Project,
    repository: &mut ActivityRepository,
    owner: ElementId,
    context: ElementId,
    name: &str,
) -> ActivityId {
    let id = repository
        .create_activity(project, owner, Some(context), name)
        .unwrap();
    let activity = repository.activities.get_mut(&id).unwrap();
    let initial = activity_node("Initial", ActivityNodeKind::Initial);
    let action = activity_node(
        "Work",
        ActivityNodeKind::Action(Action {
            kind: ActionKind::Opaque {
                body: String::new(),
            },
            pins: Vec::new(),
        }),
    );
    let final_node = activity_node("Final", ActivityNodeKind::ActivityFinal);
    activity.edges.extend([
        activity_control(&initial, &action),
        activity_control(&action, &final_node),
    ]);
    activity.nodes.extend([initial, action, final_node]);
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
    activity.edges.extend([
        activity_control(&initial, &accept),
        activity_control(&accept, &final_node),
    ]);
    activity.nodes.extend([initial, accept, final_node]);
    id
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
    activity.edges.extend([
        activity_control(&initial, &timer),
        activity_control(&timer, &final_node),
    ]);
    activity.nodes.extend([initial, timer, final_node]);
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
    let mut project = Project::new("PR32 State Activity bridge");
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

fn trace_signature(session: &ExecutionSession) -> Vec<(SimulationTime, TraceEntryKind, String)> {
    session
        .trace
        .iter()
        .map(|entry| (entry.simulation_time, entry.kind, entry.message.clone()))
        .collect()
}

#[test]
fn entry_and_exit_activities_execute_in_parent_session_without_completing_it() {
    let (mut project, behavior, context, mut behaviors, machine_id, mut activities) =
        fixture("EntryExitBridge");
    let stop = project
        .create_element(ElementKind::Signal, "Stop", behavior)
        .unwrap();
    let entry = synchronous_activity(&project, &mut activities, behavior, context, "Entry Work");
    let exit = synchronous_activity(&project, &mut activities, behavior, context, "Exit Work");

    let initial = vertex("Initial", VertexKind::Pseudostate(PseudostateKind::Initial));
    let active = vertex(
        "Active",
        VertexKind::State(State {
            entry: Some(entry.to_string()),
            exit: Some(exit.to_string()),
            ..State::default()
        }),
    );
    let final_state = vertex("Final", VertexKind::FinalState);
    let machine = behaviors.state_machines.get_mut(&machine_id).unwrap();
    machine.regions[0]
        .vertices
        .extend([initial.clone(), active.clone(), final_state.clone()]);
    machine.regions[0].transitions.extend([
        transition(&initial, &active),
        signal_transition(&active, &final_state, stop),
    ]);
    activities.validate(&project).unwrap();
    behaviors.validate(&project).unwrap();

    let mut engine = StateMachineExecutionEngine::new(behaviors, machine_id)
        .with_activity_repository(activities);
    let mut session = execution_session(&project, context);
    engine.initialize(&project, &mut session).unwrap();

    assert_eq!(session.state, ExecutionState::Initialized);
    assert_eq!(active_names(&engine, &session), ["Active"]);
    assert!(session.trace.iter().any(|entry| {
        entry
            .message
            .contains("State 'Active' completed entry Activity 'Entry Work'")
    }));

    engine
        .queue_signal(&project, &mut session, stop, "Stop", Vec::new())
        .unwrap();
    assert_eq!(
        engine.advance(&project, &mut session).unwrap(),
        EngineStepOutcome::Completed
    );
    assert_eq!(session.state, ExecutionState::Completed);
    assert!(session.trace.iter().any(|entry| {
        entry
            .message
            .contains("State 'Active' completed exit Activity 'Exit Work'")
    }));
}

#[test]
fn do_activity_waits_for_shared_signal_then_allows_completion_transition() {
    let (mut project, behavior, context, mut behaviors, machine_id, mut activities) =
        fixture("DoActivitySignal");
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

    let mut engine = StateMachineExecutionEngine::new(behaviors, machine_id)
        .with_activity_repository(activities);
    let mut session = execution_session(&project, context);
    engine.initialize(&project, &mut session).unwrap();

    for _ in 0..4 {
        if engine.advance(&project, &mut session).unwrap() == EngineStepOutcome::Idle {
            break;
        }
    }
    assert_eq!(active_names(&engine, &session), ["Working"]);

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
    assert!(session.trace.iter().any(|entry| {
        entry.message == "State 'Working' doActivity completed"
            || entry.message.contains("doActivity completed")
    }));
}

#[test]
fn exiting_state_cancels_do_activity_time_event_from_shared_queue() {
    let (mut project, behavior, context, mut behaviors, machine_id, mut activities) =
        fixture("DoActivityCancellation");
    let stop = project
        .create_element(ElementKind::Signal, "Stop", behavior)
        .unwrap();
    let do_activity = waiting_time_activity(&project, &mut activities, behavior, context);

    let initial = vertex("Initial", VertexKind::Pseudostate(PseudostateKind::Initial));
    let working = vertex(
        "Working",
        VertexKind::State(State {
            do_activity: Some(do_activity.to_string()),
            ..State::default()
        }),
    );
    let stopped = vertex("Stopped", VertexKind::State(State::default()));
    let machine = behaviors.state_machines.get_mut(&machine_id).unwrap();
    machine.regions[0]
        .vertices
        .extend([initial.clone(), working.clone(), stopped.clone()]);
    machine.regions[0].transitions.extend([
        transition(&initial, &working),
        signal_transition(&working, &stopped, stop),
    ]);
    activities.validate(&project).unwrap();
    behaviors.validate(&project).unwrap();

    let mut engine = StateMachineExecutionEngine::new(behaviors, machine_id)
        .with_activity_repository(activities);
    let mut session = execution_session(&project, context);
    engine.initialize(&project, &mut session).unwrap();

    for _ in 0..4 {
        assert_eq!(
            engine.advance(&project, &mut session).unwrap(),
            EngineStepOutcome::Progressed
        );
        if engine.snapshot(&session).pending_event_count >= 1 {
            break;
        }
    }
    assert!(engine.snapshot(&session).pending_event_count >= 1);

    engine
        .queue_signal(&project, &mut session, stop, "Stop", Vec::new())
        .unwrap();
    for _ in 0..4 {
        engine.advance(&project, &mut session).unwrap();
        if active_names(&engine, &session) == ["Stopped"] {
            break;
        }
    }

    assert_eq!(active_names(&engine, &session), ["Stopped"]);
    assert_eq!(engine.snapshot(&session).pending_event_count, 0);
    assert!(
        session
            .trace
            .iter()
            .any(|entry| { entry.message == "State 'Working' terminated doActivity on exit" })
    );
}

#[test]
fn queued_state_transition_preempts_progressing_do_activity() {
    let (mut project, behavior, context, mut behaviors, machine_id, mut activities) =
        fixture("DoActivityPreemption");
    let stop = project
        .create_element(ElementKind::Signal, "Stop", behavior)
        .unwrap();
    let do_activity = synchronous_activity(
        &project,
        &mut activities,
        behavior,
        context,
        "Background Work",
    );

    let initial = vertex("Initial", VertexKind::Pseudostate(PseudostateKind::Initial));
    let working = vertex(
        "Working",
        VertexKind::State(State {
            do_activity: Some(do_activity.to_string()),
            ..State::default()
        }),
    );
    let stopped = vertex("Stopped", VertexKind::State(State::default()));
    let machine = behaviors.state_machines.get_mut(&machine_id).unwrap();
    machine.regions[0]
        .vertices
        .extend([initial.clone(), working.clone(), stopped.clone()]);
    machine.regions[0].transitions.extend([
        transition(&initial, &working),
        signal_transition(&working, &stopped, stop),
    ]);
    activities.validate(&project).unwrap();
    behaviors.validate(&project).unwrap();

    let mut engine = StateMachineExecutionEngine::new(behaviors, machine_id)
        .with_activity_repository(activities);
    let mut session = execution_session(&project, context);
    engine.initialize(&project, &mut session).unwrap();
    assert_eq!(active_names(&engine, &session), ["Working"]);

    engine
        .queue_signal(&project, &mut session, stop, "Stop", Vec::new())
        .unwrap();
    assert_eq!(
        engine.advance(&project, &mut session).unwrap(),
        EngineStepOutcome::Progressed
    );
    assert_eq!(active_names(&engine, &session), ["Stopped"]);
    assert!(
        session
            .trace
            .iter()
            .any(|entry| entry.message == "State 'Working' terminated doActivity on exit")
    );
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
    let project_before = serde_json::to_string(&project).unwrap();
    let behavior_before = serde_json::to_string(&behaviors).unwrap();
    let activity_before = serde_json::to_string(&activities).unwrap();

    let mut engine = StateMachineExecutionEngine::new(behaviors, machine_id)
        .with_activity_repository(activities);
    let mut session = execution_session(&project, context);
    engine.initialize(&project, &mut session).unwrap();
    for _ in 0..4 {
        engine.advance(&project, &mut session).unwrap();
        if engine.snapshot(&session).pending_event_count >= 1 {
            break;
        }
    }
    assert!(engine.snapshot(&session).pending_event_count >= 1);

    engine.reset(&project, &mut session).unwrap();
    assert_eq!(active_names(&engine, &session), ["Working"]);
    assert_eq!(session.simulation_time, SimulationTime::ZERO);
    assert_eq!(engine.snapshot(&session).pending_event_count, 0);
    for _ in 0..4 {
        engine.advance(&project, &mut session).unwrap();
        if engine.snapshot(&session).pending_event_count >= 1 {
            break;
        }
    }
    let reset_snapshot = engine.snapshot(&session);
    let reset_trace = trace_signature(&session);

    let mut fresh_engine = StateMachineExecutionEngine::new(fresh_behaviors, machine_id)
        .with_activity_repository(fresh_activities);
    let mut fresh_session = execution_session(&project, context);
    fresh_engine.initialize(&project, &mut fresh_session).unwrap();
    for _ in 0..4 {
        fresh_engine.advance(&project, &mut fresh_session).unwrap();
        if fresh_engine.snapshot(&fresh_session).pending_event_count >= 1 {
            break;
        }
    }
    let fresh_snapshot = fresh_engine.snapshot(&fresh_session);

    assert_eq!(active_names(&engine, &session), active_names(&fresh_engine, &fresh_session));
    assert_eq!(reset_snapshot.pending_event_count, fresh_snapshot.pending_event_count);
    assert_eq!(session.simulation_time, fresh_session.simulation_time);
    assert_eq!(reset_trace, trace_signature(&fresh_session));
    assert_eq!(serde_json::to_string(&project).unwrap(), project_before);
    assert_eq!(
        serde_json::to_string(engine.authored_repository()).unwrap(),
        behavior_before
    );
    assert_eq!(serde_json::to_string(&fresh_activities).unwrap(), activity_before);
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
        trace_signature(&session)
    };

    assert_eq!(
        run(behaviors.clone(), activities.clone()),
        run(behaviors, activities)
    );
}
