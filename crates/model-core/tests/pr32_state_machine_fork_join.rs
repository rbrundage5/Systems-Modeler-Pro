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

fn fixture() -> (Project, BehaviorRepository, StateMachineId, ElementId) {
    let mut project = Project::new("PR32 orthogonal Fork Join");
    let context = project
        .create_element(ElementKind::Block, "Controller", project.root_id)
        .unwrap();
    let mut repository = BehaviorRepository::default();
    let machine_id = repository
        .create_state_machine(&project, context, "Parallel Controller")
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

fn active_names(engine: &StateMachineExecutionEngine, execution: &ExecutionSession) -> Vec<String> {
    let mut names = engine
        .snapshot(execution)
        .active_states
        .into_iter()
        .map(|state| state.state_name)
        .collect::<Vec<_>>();
    names.sort();
    names
}

#[test]
fn fork_targets_orthogonal_regions_without_entering_sibling_defaults_and_join_completes() {
    let (project, mut repository, machine_id, context) = fixture();

    let root_initial = vertex("Root Initial", VertexKind::Pseudostate(PseudostateKind::Initial));
    let fork = vertex("Fork", VertexKind::Pseudostate(PseudostateKind::Fork));
    let root_final = vertex("Root Final", VertexKind::FinalState);

    let left_initial = vertex(
        "Left Initial",
        VertexKind::Pseudostate(PseudostateKind::Initial),
    );
    let left_default = vertex("Left Default", VertexKind::State(State::default()));
    let left_target = vertex("Left Target", VertexKind::State(State::default()));
    let join = vertex("Join", VertexKind::Pseudostate(PseudostateKind::Join));

    let right_initial = vertex(
        "Right Initial",
        VertexKind::Pseudostate(PseudostateKind::Initial),
    );
    let right_default = vertex("Right Default", VertexKind::State(State::default()));
    let right_target = vertex("Right Target", VertexKind::State(State::default()));

    let left_region = Region {
        id: RegionId::new(),
        name: "Left Region".into(),
        vertices: vec![
            left_initial.clone(),
            left_default.clone(),
            left_target.clone(),
            join.clone(),
        ],
        transitions: vec![
            transition(&left_initial, &left_default),
            transition(&left_target, &join),
            transition(&right_target, &join),
            transition(&join, &root_final),
        ],
    };
    let right_region = Region {
        id: RegionId::new(),
        name: "Right Region".into(),
        vertices: vec![right_initial.clone(), right_default.clone(), right_target.clone()],
        transitions: vec![transition(&right_initial, &right_default)],
    };
    let parent = vertex(
        "Parallel Parent",
        VertexKind::State(State {
            regions: vec![left_region, right_region],
            ..State::default()
        }),
    );

    let machine = repository.state_machines.get_mut(&machine_id).unwrap();
    machine.regions[0].vertices.extend([
        root_initial.clone(),
        fork.clone(),
        parent.clone(),
        root_final.clone(),
    ]);
    machine.regions[0].transitions.extend([
        transition(&root_initial, &fork),
        transition(&fork, &left_target),
        transition(&fork, &right_target),
    ]);

    repository.validate(&project).unwrap();

    let mut engine = StateMachineExecutionEngine::new(repository, machine_id);
    let mut execution = session(&project, context);
    engine.initialize(&project, &mut execution).unwrap();

    assert_eq!(
        active_names(&engine, &execution),
        ["Left Target", "Parallel Parent", "Right Target"]
    );
    assert!(!execution.trace.iter().any(|entry| {
        entry.message == "Entered State 'Left Default'"
            || entry.message == "Entered State 'Right Default'"
    }));

    assert_eq!(
        engine.advance(&project, &mut execution).unwrap(),
        EngineStepOutcome::Progressed
    );
    assert_eq!(execution.state, ExecutionState::Initialized);

    assert_eq!(
        engine.advance(&project, &mut execution).unwrap(),
        EngineStepOutcome::Completed
    );
    assert_eq!(execution.state, ExecutionState::Completed);
    assert!(active_names(&engine, &execution).is_empty());
    assert!(
        execution
            .trace
            .iter()
            .any(|entry| entry.message == "Region reached FinalState 'Root Final'")
    );
}
