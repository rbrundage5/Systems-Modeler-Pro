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

fn control(source: &ActivityNode, target: &ActivityNode) -> ActivityEdge {
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
fn embedded_state_activity_consumes_shared_budget_once_per_activity_node() {
    let mut project = Project::new("PR32 embedded Activity budget");
    let behavior = project
        .create_element(ElementKind::Package, "Behavior", project.root_id)
        .unwrap();
    let context = project
        .create_element(ElementKind::Block, "Controller", behavior)
        .unwrap();

    let mut activities = ActivityRepository::default();
    let activity_id = activities
        .create_activity(&project, behavior, Some(context), "Background Work")
        .unwrap();
    let activity = activities.activities.get_mut(&activity_id).unwrap();
    let initial = activity_node("Initial", ActivityNodeKind::Initial);
    let work = activity_node(
        "Work",
        ActivityNodeKind::Action(Action {
            kind: ActionKind::Opaque {
                body: String::new(),
            },
            pins: Vec::new(),
        }),
    );
    let final_node = activity_node("Final", ActivityNodeKind::ActivityFinal);
    activity
        .nodes
        .extend([initial.clone(), work.clone(), final_node.clone()]);
    activity
        .edges
        .extend([control(&initial, &work), control(&work, &final_node)]);

    let mut behaviors = BehaviorRepository::default();
    let machine_id = behaviors
        .create_state_machine(&project, context, "Budget Machine")
        .unwrap();
    let machine_initial = vertex("Initial", VertexKind::Pseudostate(PseudostateKind::Initial));
    let working = vertex(
        "Working",
        VertexKind::State(State {
            do_activity: Some(activity_id.to_string()),
            ..State::default()
        }),
    );
    let machine = behaviors.state_machines.get_mut(&machine_id).unwrap();
    machine.regions[0]
        .vertices
        .extend([machine_initial.clone(), working.clone()]);
    machine.regions[0]
        .transitions
        .push(transition(&machine_initial, &working));

    activities.validate(&project).unwrap();
    behaviors.validate(&project).unwrap();

    let mut engine = StateMachineExecutionEngine::new(behaviors, machine_id)
        .with_activity_repository(activities);
    let mut session = ExecutionSession::with_configuration(
        &project,
        ExecutionConfiguration {
            root_semantic_id: context,
            random_seed: 0,
            max_steps: 100,
            max_queued_events: 100,
        },
    )
    .unwrap();
    engine.initialize(&project, &mut session).unwrap();
    let before_activity = session.steps_executed;

    for _ in 0..8 {
        engine.advance(&project, &mut session).unwrap();
        if session.trace.iter().any(|entry| {
            entry
                .message
                .contains("State 'Working' doActivity completed")
        }) {
            break;
        }
    }

    assert!(session.trace.iter().any(|entry| {
        entry
            .message
            .contains("State 'Working' doActivity completed")
    }));
    assert_eq!(
        session.steps_executed - before_activity,
        3,
        "embedded State Activities must use the PR31 Activity engine's semantic-step accounting without an extra State Machine charge"
    );
}
