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

fn signal_transition(
    source: &Vertex,
    target: &Vertex,
    signal_id: ElementId,
    kind: TransitionKind,
) -> Transition {
    Transition {
        kind,
        trigger: Some(Trigger {
            event: Event::Signal { signal_id },
        }),
        ..transition(source, target)
    }
}

fn fixture(name: &str) -> (Project, BehaviorRepository, StateMachineId, ElementId) {
    let mut project = Project::new("PR32 semantic closure");
    let context = project
        .create_element(ElementKind::Block, "Controller", project.root_id)
        .unwrap();
    let mut repository = BehaviorRepository::default();
    let machine_id = repository
        .create_state_machine(&project, context, name)
        .unwrap();
    (project, repository, machine_id, context)
}

fn session(project: &Project, context: ElementId) -> ExecutionSession {
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
    let mut names = engine
        .snapshot(session)
        .active_states
        .into_iter()
        .map(|state| state.state_name)
        .collect::<Vec<_>>();
    names.sort();
    names
}

#[test]
fn one_signal_fires_non_conflicting_transitions_in_two_orthogonal_regions() {
    let (mut project, mut repository, machine_id, context) = fixture("OrthogonalSignal");
    let start = project
        .create_element(ElementKind::Signal, "Start", project.root_id)
        .unwrap();

    let root_initial = vertex("Initial", VertexKind::Pseudostate(PseudostateKind::Initial));
    let left_initial = vertex(
        "Left Initial",
        VertexKind::Pseudostate(PseudostateKind::Initial),
    );
    let left_waiting = vertex("Left Waiting", VertexKind::State(State::default()));
    let left_running = vertex("Left Running", VertexKind::State(State::default()));
    let right_initial = vertex(
        "Right Initial",
        VertexKind::Pseudostate(PseudostateKind::Initial),
    );
    let right_waiting = vertex("Right Waiting", VertexKind::State(State::default()));
    let right_running = vertex("Right Running", VertexKind::State(State::default()));

    let parent = vertex(
        "Parent",
        VertexKind::State(State {
            regions: vec![
                Region {
                    id: RegionId::new(),
                    name: "Left".into(),
                    vertices: vec![
                        left_initial.clone(),
                        left_waiting.clone(),
                        left_running.clone(),
                    ],
                    transitions: vec![
                        transition(&left_initial, &left_waiting),
                        signal_transition(
                            &left_waiting,
                            &left_running,
                            start,
                            TransitionKind::External,
                        ),
                    ],
                },
                Region {
                    id: RegionId::new(),
                    name: "Right".into(),
                    vertices: vec![
                        right_initial.clone(),
                        right_waiting.clone(),
                        right_running.clone(),
                    ],
                    transitions: vec![
                        transition(&right_initial, &right_waiting),
                        signal_transition(
                            &right_waiting,
                            &right_running,
                            start,
                            TransitionKind::External,
                        ),
                    ],
                },
            ],
            ..State::default()
        }),
    );

    let machine = repository.state_machines.get_mut(&machine_id).unwrap();
    machine.regions[0]
        .vertices
        .extend([root_initial.clone(), parent.clone()]);
    machine.regions[0]
        .transitions
        .push(transition(&root_initial, &parent));

    let mut engine = StateMachineExecutionEngine::new(repository, machine_id);
    let mut execution = session(&project, context);
    engine.initialize(&project, &mut execution).unwrap();
    assert_eq!(
        active_names(&engine, &execution),
        ["Left Waiting", "Parent", "Right Waiting"]
    );

    engine
        .queue_signal(&project, &mut execution, start, "Start", Vec::new())
        .unwrap();
    assert_eq!(
        engine.advance(&project, &mut execution).unwrap(),
        EngineStepOutcome::Progressed
    );
    assert_eq!(
        active_names(&engine, &execution),
        ["Left Running", "Parent", "Right Running"]
    );
}

#[test]
fn cross_hierarchy_external_transition_exits_child_then_parent() {
    let (mut project, mut repository, machine_id, context) = fixture("CrossHierarchy");
    let leave = project
        .create_element(ElementKind::Signal, "Leave", project.root_id)
        .unwrap();

    let root_initial = vertex("Initial", VertexKind::Pseudostate(PseudostateKind::Initial));
    let child_initial = vertex(
        "Child Initial",
        VertexKind::Pseudostate(PseudostateKind::Initial),
    );
    let child = vertex("Child", VertexKind::State(State::default()));
    let outside = vertex("Outside", VertexKind::State(State::default()));
    let child_region = Region {
        id: RegionId::new(),
        name: "Child Region".into(),
        vertices: vec![child_initial.clone(), child.clone()],
        transitions: vec![
            transition(&child_initial, &child),
            signal_transition(&child, &outside, leave, TransitionKind::External),
        ],
    };
    let parent = vertex(
        "Parent",
        VertexKind::State(State {
            regions: vec![child_region],
            ..State::default()
        }),
    );

    let machine = repository.state_machines.get_mut(&machine_id).unwrap();
    machine.regions[0]
        .vertices
        .extend([root_initial.clone(), parent.clone(), outside.clone()]);
    machine.regions[0]
        .transitions
        .push(transition(&root_initial, &parent));

    let mut engine = StateMachineExecutionEngine::new(repository, machine_id);
    let mut execution = session(&project, context);
    engine.initialize(&project, &mut execution).unwrap();
    engine
        .queue_signal(&project, &mut execution, leave, "Leave", Vec::new())
        .unwrap();
    engine.advance(&project, &mut execution).unwrap();

    assert_eq!(active_names(&engine, &execution), ["Outside"]);
    let exits = execution
        .trace
        .iter()
        .filter(|entry| entry.message.starts_with("Exited State"))
        .map(|entry| entry.message.clone())
        .collect::<Vec<_>>();
    assert!(exits.ends_with(&[
        "Exited State 'Child'".to_string(),
        "Exited State 'Parent'".to_string(),
    ]));
}

#[test]
fn local_transition_from_composite_to_descendant_retains_composite() {
    let (mut project, mut repository, machine_id, context) = fixture("LocalDescendant");
    let switch = project
        .create_element(ElementKind::Signal, "Switch", project.root_id)
        .unwrap();

    let root_initial = vertex("Initial", VertexKind::Pseudostate(PseudostateKind::Initial));
    let child_initial = vertex(
        "Child Initial",
        VertexKind::Pseudostate(PseudostateKind::Initial),
    );
    let first = vertex("First", VertexKind::State(State::default()));
    let second = vertex("Second", VertexKind::State(State::default()));
    let region = Region {
        id: RegionId::new(),
        name: "Nested".into(),
        vertices: vec![child_initial.clone(), first.clone(), second.clone()],
        transitions: vec![transition(&child_initial, &first)],
    };
    let parent = vertex(
        "Parent",
        VertexKind::State(State {
            regions: vec![region],
            ..State::default()
        }),
    );

    let machine = repository.state_machines.get_mut(&machine_id).unwrap();
    machine.regions[0]
        .vertices
        .extend([root_initial.clone(), parent.clone()]);
    machine.regions[0].transitions.extend([
        transition(&root_initial, &parent),
        signal_transition(&parent, &second, switch, TransitionKind::Local),
    ]);

    let mut engine = StateMachineExecutionEngine::new(repository, machine_id);
    let mut execution = session(&project, context);
    engine.initialize(&project, &mut execution).unwrap();
    engine
        .queue_signal(&project, &mut execution, switch, "Switch", Vec::new())
        .unwrap();
    engine.advance(&project, &mut execution).unwrap();

    assert_eq!(active_names(&engine, &execution), ["Parent", "Second"]);
    assert!(
        !execution
            .trace
            .iter()
            .any(|entry| entry.message == "Exited State 'Parent'")
    );
}

#[test]
fn entering_nested_target_initializes_other_orthogonal_regions() {
    let (mut project, mut repository, machine_id, context) = fixture("NestedEntry");
    let enter = project
        .create_element(ElementKind::Signal, "Enter", project.root_id)
        .unwrap();

    let root_initial = vertex("Initial", VertexKind::Pseudostate(PseudostateKind::Initial));
    let outside = vertex("Outside", VertexKind::State(State::default()));
    let left_target = vertex("Left Target", VertexKind::State(State::default()));
    let right_initial = vertex(
        "Right Initial",
        VertexKind::Pseudostate(PseudostateKind::Initial),
    );
    let right_default = vertex("Right Default", VertexKind::State(State::default()));
    let parent = vertex(
        "Parent",
        VertexKind::State(State {
            regions: vec![
                Region {
                    id: RegionId::new(),
                    name: "Left".into(),
                    vertices: vec![left_target.clone()],
                    transitions: Vec::new(),
                },
                Region {
                    id: RegionId::new(),
                    name: "Right".into(),
                    vertices: vec![right_initial.clone(), right_default.clone()],
                    transitions: vec![transition(&right_initial, &right_default)],
                },
            ],
            ..State::default()
        }),
    );

    let machine = repository.state_machines.get_mut(&machine_id).unwrap();
    machine.regions[0]
        .vertices
        .extend([root_initial.clone(), outside.clone(), parent.clone()]);
    machine.regions[0].transitions.extend([
        transition(&root_initial, &outside),
        signal_transition(&outside, &left_target, enter, TransitionKind::External),
    ]);

    let mut engine = StateMachineExecutionEngine::new(repository, machine_id);
    let mut execution = session(&project, context);
    engine.initialize(&project, &mut execution).unwrap();
    engine
        .queue_signal(&project, &mut execution, enter, "Enter", Vec::new())
        .unwrap();
    engine.advance(&project, &mut execution).unwrap();

    assert_eq!(
        active_names(&engine, &execution),
        ["Left Target", "Parent", "Right Default"]
    );
}

#[test]
fn same_priority_conflicting_transitions_fail_with_ambiguity_diagnostic() {
    let (mut project, mut repository, machine_id, context) = fixture("Ambiguous");
    let choose = project
        .create_element(ElementKind::Signal, "Choose", project.root_id)
        .unwrap();

    let initial = vertex("Initial", VertexKind::Pseudostate(PseudostateKind::Initial));
    let idle = vertex("Idle", VertexKind::State(State::default()));
    let left = vertex("Left", VertexKind::State(State::default()));
    let right = vertex("Right", VertexKind::State(State::default()));

    let machine = repository.state_machines.get_mut(&machine_id).unwrap();
    machine.regions[0].vertices.extend([
        initial.clone(),
        idle.clone(),
        left.clone(),
        right.clone(),
    ]);
    machine.regions[0].transitions.extend([
        transition(&initial, &idle),
        signal_transition(&idle, &left, choose, TransitionKind::External),
        signal_transition(&idle, &right, choose, TransitionKind::External),
    ]);

    let mut engine = StateMachineExecutionEngine::new(repository, machine_id);
    let mut execution = session(&project, context);
    engine.initialize(&project, &mut execution).unwrap();
    engine
        .queue_signal(&project, &mut execution, choose, "Choose", Vec::new())
        .unwrap();
    let error = engine.advance(&project, &mut execution).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("Ambiguous State Machine transition")
    );
}

#[test]
fn entry_point_execution_is_explicitly_rejected_until_connection_point_semantics_exist() {
    let (project, mut repository, machine_id, context) = fixture("EntryPointUnsupported");
    let initial = vertex("Initial", VertexKind::Pseudostate(PseudostateKind::Initial));
    let entry = vertex(
        "Entry",
        VertexKind::Pseudostate(PseudostateKind::EntryPoint),
    );
    let state = vertex("State", VertexKind::State(State::default()));
    let machine = repository.state_machines.get_mut(&machine_id).unwrap();
    machine.regions[0]
        .vertices
        .extend([initial.clone(), entry.clone(), state.clone()]);
    machine.regions[0]
        .transitions
        .extend([transition(&initial, &entry), transition(&entry, &state)]);

    let mut engine = StateMachineExecutionEngine::new(repository, machine_id);
    let mut execution = session(&project, context);
    let error = engine.initialize(&project, &mut execution).unwrap_err();
    assert!(error.to_string().contains("connection-point"));
}
