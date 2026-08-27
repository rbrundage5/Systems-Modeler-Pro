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

fn time_transition(source: &Vertex, target: &Vertex, expression: &str) -> Transition {
    Transition {
        trigger: Some(Trigger {
            event: Event::Time {
                expression: expression.into(),
                is_relative: true,
            },
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
    activity.nodes.extend([initial.clone(), action.clone(), final_node.clone()]);
    activity.edges.extend([
        activity_control(&initial, &action),
        activity_control(&action, &final_node),
    ]);
    id
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

#[test]
fn submachine_state_inherits_shared_activity_repository_for_child_state_behaviors() {
    let mut project = Project::new("PR32 Submachine Activity composition");
    let behavior = project
        .create_element(ElementKind::Package, "Behavior", project.root_id)
        .unwrap();
    let context = project
        .create_element(ElementKind::Block, "Controller", behavior)
        .unwrap();
    let mut activities = ActivityRepository::default();
    let child_entry = synchronous_activity(
        &project,
        &mut activities,
        behavior,
        context,
        "Child Entry",
    );

    let mut behaviors = BehaviorRepository::default();
    let child_id = behaviors
        .create_state_machine(&project, context, "Child Machine")
        .unwrap();
    let child_initial = vertex("Child Initial", VertexKind::Pseudostate(PseudostateKind::Initial));
    let child_active = vertex(
        "Child Active",
        VertexKind::State(State {
            entry: Some(child_entry.to_string()),
            ..State::default()
        }),
    );
    let child_final = vertex("Child Final", VertexKind::FinalState);
    let child = behaviors.state_machines.get_mut(&child_id).unwrap();
    child.regions[0].vertices.extend([
        child_initial.clone(),
        child_active.clone(),
        child_final.clone(),
    ]);
    child.regions[0].transitions.extend([
        transition(&child_initial, &child_active),
        transition(&child_active, &child_final),
    ]);

    let parent_id = behaviors
        .create_state_machine(&project, context, "Parent Machine")
        .unwrap();
    let parent_initial = vertex("Parent Initial", VertexKind::Pseudostate(PseudostateKind::Initial));
    let child_state = vertex(
        "Run Child",
        VertexKind::State(State {
            submachine: Some(child_id),
            ..State::default()
        }),
    );
    let parent_done = vertex("Parent Done", VertexKind::State(State::default()));
    let parent = behaviors.state_machines.get_mut(&parent_id).unwrap();
    parent.regions[0].vertices.extend([
        parent_initial.clone(),
        child_state.clone(),
        parent_done.clone(),
    ]);
    parent.regions[0].transitions.extend([
        transition(&parent_initial, &child_state),
        transition(&child_state, &parent_done),
    ]);
    activities.validate(&project).unwrap();
    behaviors.validate(&project).unwrap();

    let mut engine = StateMachineExecutionEngine::new(behaviors, parent_id)
        .with_activity_repository(activities);
    let mut session = execution_session(&project, context);
    engine.initialize(&project, &mut session).unwrap();

    assert!(session.trace.iter().any(|entry| {
        entry
            .message
            .contains("State 'Child Active' completed entry Activity 'Child Entry'")
    }));
    for _ in 0..8 {
        engine.advance(&project, &mut session).unwrap();
        if active_names(&engine, &session) == ["Parent Done"] {
            break;
        }
    }
    assert_eq!(active_names(&engine, &session), ["Parent Done"]);
}

#[test]
fn future_state_time_event_does_not_preempt_zero_time_do_activity_progress() {
    let mut project = Project::new("PR32 doActivity time ordering");
    let behavior = project
        .create_element(ElementKind::Package, "Behavior", project.root_id)
        .unwrap();
    let context = project
        .create_element(ElementKind::Block, "Controller", behavior)
        .unwrap();
    let mut activities = ActivityRepository::default();
    let do_activity = synchronous_activity(
        &project,
        &mut activities,
        behavior,
        context,
        "Background Work",
    );

    let mut behaviors = BehaviorRepository::default();
    let machine_id = behaviors
        .create_state_machine(&project, context, "Timed Machine")
        .unwrap();
    let initial = vertex("Initial", VertexKind::Pseudostate(PseudostateKind::Initial));
    let working = vertex(
        "Working",
        VertexKind::State(State {
            do_activity: Some(do_activity.to_string()),
            ..State::default()
        }),
    );
    let elapsed = vertex("Elapsed", VertexKind::State(State::default()));
    let machine = behaviors.state_machines.get_mut(&machine_id).unwrap();
    machine.regions[0]
        .vertices
        .extend([initial.clone(), working.clone(), elapsed.clone()]);
    machine.regions[0].transitions.extend([
        transition(&initial, &working),
        time_transition(&working, &elapsed, "after 5s"),
    ]);
    activities.validate(&project).unwrap();
    behaviors.validate(&project).unwrap();

    let mut engine = StateMachineExecutionEngine::new(behaviors, machine_id)
        .with_activity_repository(activities);
    let mut session = execution_session(&project, context);
    engine.initialize(&project, &mut session).unwrap();
    assert_eq!(session.simulation_time, SimulationTime::ZERO);

    let mut activity_completed_at_zero = false;
    for _ in 0..8 {
        engine.advance(&project, &mut session).unwrap();
        activity_completed_at_zero = session.trace.iter().any(|entry| {
            entry.message.contains("State 'Working' doActivity completed")
                && entry.simulation_time == SimulationTime::ZERO
        });
        if activity_completed_at_zero {
            break;
        }
        assert_eq!(session.simulation_time, SimulationTime::ZERO);
    }
    assert!(activity_completed_at_zero);
    assert_eq!(active_names(&engine, &session), ["Working"]);

    for _ in 0..4 {
        engine.advance(&project, &mut session).unwrap();
        if active_names(&engine, &session) == ["Elapsed"] {
            break;
        }
    }
    assert_eq!(active_names(&engine, &session), ["Elapsed"]);
    assert_eq!(session.simulation_time, SimulationTime::from_nanos(5_000_000_000));
}
