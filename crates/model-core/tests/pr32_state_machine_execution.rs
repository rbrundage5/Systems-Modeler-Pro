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

fn project_and_machine(name: &str) -> (Project, BehaviorRepository, StateMachineId, ElementId) {
    let mut project = Project::new("PR32 State Machine execution");
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

fn active_names(snapshot: &StateMachineExecutionSnapshot) -> Vec<String> {
    snapshot
        .active_states
        .iter()
        .map(|state| state.state_name.clone())
        .collect()
}

#[test]
fn basic_signal_lifecycle_is_deterministic_and_completes() {
    let (mut project, mut repository, machine_id, context) = project_and_machine("Lifecycle");
    let start = project
        .create_element(ElementKind::Signal, "Start", project.root_id)
        .unwrap();
    let stop = project
        .create_element(ElementKind::Signal, "Stop", project.root_id)
        .unwrap();
    let machine = repository.state_machines.get_mut(&machine_id).unwrap();
    let region = &mut machine.regions[0];
    let initial = vertex("Initial", VertexKind::Pseudostate(PseudostateKind::Initial));
    let idle = vertex("Idle", VertexKind::State(State::default()));
    let running = vertex("Running", VertexKind::State(State::default()));
    let final_state = vertex("Final", VertexKind::FinalState);
    region.transitions.extend([
        transition(&initial, &idle),
        signal_transition(&idle, &running, start),
        signal_transition(&running, &final_state, stop),
    ]);
    region
        .vertices
        .extend([initial, idle, running, final_state]);

    let mut engine = StateMachineExecutionEngine::new(repository, machine_id);
    let mut execution = session(&project, context);
    engine.initialize(&project, &mut execution).unwrap();
    assert_eq!(active_names(&engine.snapshot(&execution)), ["Idle"]);
    engine
        .queue_signal(&project, &mut execution, start, "Start", Vec::new())
        .unwrap();
    assert_eq!(
        engine.advance(&project, &mut execution).unwrap(),
        EngineStepOutcome::Progressed
    );
    assert_eq!(active_names(&engine.snapshot(&execution)), ["Running"]);
    engine
        .queue_signal(&project, &mut execution, stop, "Stop", Vec::new())
        .unwrap();
    assert_eq!(
        engine.advance(&project, &mut execution).unwrap(),
        EngineStepOutcome::Completed
    );
    assert_eq!(execution.state, ExecutionState::Completed);
    assert!(
        execution
            .trace
            .iter()
            .any(|entry| entry.message == "Entered State 'Running'")
    );
}

#[test]
fn guarded_choice_uses_shared_expression_evaluator_and_event_payload() {
    let (mut project, mut repository, machine_id, context) = project_and_machine("Choice");
    let classify = project
        .create_element(ElementKind::Signal, "Classify", project.root_id)
        .unwrap();
    let machine = repository.state_machines.get_mut(&machine_id).unwrap();
    let region = &mut machine.regions[0];
    let initial = vertex("Initial", VertexKind::Pseudostate(PseudostateKind::Initial));
    let idle = vertex("Idle", VertexKind::State(State::default()));
    let choice = vertex("Classify", VertexKind::Pseudostate(PseudostateKind::Choice));
    let high = vertex("High", VertexKind::State(State::default()));
    let low = vertex("Low", VertexKind::State(State::default()));
    let mut to_choice = signal_transition(&idle, &choice, classify);
    to_choice.effect = Some("value + threshold".into());
    let mut to_high = transition(&choice, &high);
    to_high.guard = Some("value >= threshold".into());
    let mut to_low = transition(&choice, &low);
    to_low.guard = Some("else".into());
    region
        .transitions
        .extend([transition(&initial, &idle), to_choice, to_high, to_low]);
    region.vertices.extend([initial, idle, choice, high, low]);

    let mut engine = StateMachineExecutionEngine::new(repository, machine_id);
    let mut execution = session(&project, context);
    engine.initialize(&project, &mut execution).unwrap();
    engine
        .queue_signal(
            &project,
            &mut execution,
            classify,
            "Classify",
            vec![
                ("value".into(), RuntimeValue::Integer(12)),
                ("threshold".into(), RuntimeValue::Integer(10)),
            ],
        )
        .unwrap();
    engine.advance(&project, &mut execution).unwrap();
    assert_eq!(active_names(&engine.snapshot(&execution)), ["High"]);
}

#[test]
fn composite_and_orthogonal_regions_maintain_active_configuration() {
    let (project, mut repository, machine_id, context) = project_and_machine("Orthogonal");
    let machine = repository.state_machines.get_mut(&machine_id).unwrap();
    let root = &mut machine.regions[0];
    let initial = vertex("Initial", VertexKind::Pseudostate(PseudostateKind::Initial));
    let left_initial = vertex(
        "Left Initial",
        VertexKind::Pseudostate(PseudostateKind::Initial),
    );
    let left = vertex("Left Active", VertexKind::State(State::default()));
    let right_initial = vertex(
        "Right Initial",
        VertexKind::Pseudostate(PseudostateKind::Initial),
    );
    let right = vertex("Right Active", VertexKind::State(State::default()));
    let parent = vertex(
        "Parent",
        VertexKind::State(State {
            regions: vec![
                Region {
                    id: RegionId::new(),
                    name: "Left".into(),
                    vertices: vec![left_initial.clone(), left.clone()],
                    transitions: vec![transition(&left_initial, &left)],
                },
                Region {
                    id: RegionId::new(),
                    name: "Right".into(),
                    vertices: vec![right_initial.clone(), right.clone()],
                    transitions: vec![transition(&right_initial, &right)],
                },
            ],
            ..State::default()
        }),
    );
    root.transitions.push(transition(&initial, &parent));
    root.vertices.extend([initial, parent]);

    let mut engine = StateMachineExecutionEngine::new(repository, machine_id);
    let mut execution = session(&project, context);
    engine.initialize(&project, &mut execution).unwrap();
    let snapshot = engine.snapshot(&execution);
    let mut names = active_names(&snapshot);
    names.sort();
    assert_eq!(
        names,
        ["Left Active", "Parent", "Right Active"]
    );
    assert_eq!(snapshot.active_region_ids.len(), 3);
}

#[test]
fn time_event_uses_simulation_time_not_wall_clock() {
    let (project, mut repository, machine_id, context) = project_and_machine("Timer");
    let machine = repository.state_machines.get_mut(&machine_id).unwrap();
    let region = &mut machine.regions[0];
    let initial = vertex("Initial", VertexKind::Pseudostate(PseudostateKind::Initial));
    let waiting = vertex("Waiting", VertexKind::State(State::default()));
    let elapsed = vertex("Elapsed", VertexKind::State(State::default()));
    let mut timed = transition(&waiting, &elapsed);
    timed.trigger = Some(Trigger {
        event: Event::Time {
            expression: "after 5s".into(),
            is_relative: true,
        },
    });
    region
        .transitions
        .extend([transition(&initial, &waiting), timed]);
    region.vertices.extend([initial, waiting, elapsed]);

    let mut engine = StateMachineExecutionEngine::new(repository, machine_id);
    let mut execution = session(&project, context);
    engine.initialize(&project, &mut execution).unwrap();
    engine.advance(&project, &mut execution).unwrap();
    assert_eq!(
        execution.simulation_time,
        SimulationTime::from_nanos(5_000_000_000)
    );
    assert_eq!(active_names(&engine.snapshot(&execution)), ["Elapsed"]);
}

#[test]
fn fork_and_join_synchronize_concurrent_paths() {
    let (project, mut repository, machine_id, context) = project_and_machine("Parallel");
    let machine = repository.state_machines.get_mut(&machine_id).unwrap();
    let region = &mut machine.regions[0];
    let initial = vertex("Initial", VertexKind::Pseudostate(PseudostateKind::Initial));
    let fork = vertex("Fork", VertexKind::Pseudostate(PseudostateKind::Fork));
    let left = vertex("Left", VertexKind::State(State::default()));
    let right = vertex("Right", VertexKind::State(State::default()));
    let join = vertex("Join", VertexKind::Pseudostate(PseudostateKind::Join));
    let final_state = vertex("Final", VertexKind::FinalState);
    region.transitions.extend([
        transition(&initial, &fork),
        transition(&fork, &left),
        transition(&fork, &right),
        transition(&left, &join),
        transition(&right, &join),
        transition(&join, &final_state),
    ]);
    region
        .vertices
        .extend([initial, fork, left, right, join, final_state]);

    let mut engine = StateMachineExecutionEngine::new(repository, machine_id);
    let mut execution = session(&project, context);
    engine.initialize(&project, &mut execution).unwrap();
    assert_eq!(
        active_names(&engine.snapshot(&execution)),
        ["Left", "Right"]
    );
    assert_eq!(
        engine.advance(&project, &mut execution).unwrap(),
        EngineStepOutcome::Progressed
    );
    assert_ne!(execution.state, ExecutionState::Completed);
    assert_eq!(
        engine.advance(&project, &mut execution).unwrap(),
        EngineStepOutcome::Completed
    );
    assert_eq!(execution.state, ExecutionState::Completed);
}

#[test]
fn identical_inputs_produce_identical_semantic_trace() {
    let (mut project, mut repository, machine_id, context) = project_and_machine("Repeatable");
    let start = project
        .create_element(ElementKind::Signal, "Start", project.root_id)
        .unwrap();
    let machine = repository.state_machines.get_mut(&machine_id).unwrap();
    let region = &mut machine.regions[0];
    let initial = vertex("Initial", VertexKind::Pseudostate(PseudostateKind::Initial));
    let idle = vertex("Idle", VertexKind::State(State::default()));
    let final_state = vertex("Final", VertexKind::FinalState);
    region.transitions.extend([
        transition(&initial, &idle),
        signal_transition(&idle, &final_state, start),
    ]);
    region.vertices.extend([initial, idle, final_state]);

    let run = |repository: BehaviorRepository| {
        let mut engine = StateMachineExecutionEngine::new(repository, machine_id);
        let mut execution = session(&project, context);
        engine.initialize(&project, &mut execution).unwrap();
        engine
            .queue_signal(&project, &mut execution, start, "Start", Vec::new())
            .unwrap();
        engine.advance(&project, &mut execution).unwrap();
        execution
            .trace
            .iter()
            .map(|entry| (entry.simulation_time, entry.kind, entry.message.clone()))
            .collect::<Vec<_>>()
    };
    assert_eq!(run(repository.clone()), run(repository));
}

#[test]
fn reset_matches_fresh_initialization_and_does_not_mutate_authored_model() {
    let (mut project, mut repository, machine_id, context) = project_and_machine("Reset");
    let start = project
        .create_element(ElementKind::Signal, "Start", project.root_id)
        .unwrap();
    let machine = repository.state_machines.get_mut(&machine_id).unwrap();
    let region = &mut machine.regions[0];
    let initial = vertex("Initial", VertexKind::Pseudostate(PseudostateKind::Initial));
    let idle = vertex("Idle", VertexKind::State(State::default()));
    let running = vertex("Running", VertexKind::State(State::default()));
    region.transitions.extend([
        transition(&initial, &idle),
        signal_transition(&idle, &running, start),
    ]);
    region.vertices.extend([initial, idle, running]);
    let authored_before = serde_json::to_string(&repository).unwrap();

    let mut engine = StateMachineExecutionEngine::new(repository, machine_id);
    let mut execution = session(&project, context);
    engine.initialize(&project, &mut execution).unwrap();
    engine
        .queue_signal(&project, &mut execution, start, "Start", Vec::new())
        .unwrap();
    engine.advance(&project, &mut execution).unwrap();
    engine.reset(&project, &mut execution).unwrap();
    assert_eq!(active_names(&engine.snapshot(&execution)), ["Idle"]);
    assert_eq!(execution.simulation_time, SimulationTime::ZERO);
    assert_eq!(
        serde_json::to_string(engine.authored_repository()).unwrap(),
        authored_before
    );
}

#[test]
fn pseudostate_cycle_fails_at_bounded_run_to_completion_limit() {
    let (project, mut repository, machine_id, context) = project_and_machine("Cycle");
    let machine = repository.state_machines.get_mut(&machine_id).unwrap();
    let region = &mut machine.regions[0];
    let initial = vertex("Initial", VertexKind::Pseudostate(PseudostateKind::Initial));
    let first = vertex("A", VertexKind::Pseudostate(PseudostateKind::Junction));
    let second = vertex("B", VertexKind::Pseudostate(PseudostateKind::Junction));
    region.transitions.extend([
        transition(&initial, &first),
        transition(&first, &second),
        transition(&second, &first),
    ]);
    region.vertices.extend([initial, first, second]);

    let mut engine = StateMachineExecutionEngine::new(repository, machine_id);
    let mut execution = session(&project, context);
    let error = engine.initialize(&project, &mut execution).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("run-to-completion step limit exceeded")
    );
    assert_eq!(execution.state, ExecutionState::Failed);
}
