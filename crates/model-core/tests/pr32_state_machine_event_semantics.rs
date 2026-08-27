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

fn fixture(name: &str) -> (Project, BehaviorRepository, StateMachineId, ElementId) {
    let mut project = Project::new("PR32 event semantics");
    let context = project
        .create_element(ElementKind::Block, "Controller", project.root_id)
        .unwrap();
    let mut repository = BehaviorRepository::default();
    let machine_id = repository
        .create_state_machine(&project, context, name)
        .unwrap();
    (project, repository, machine_id, context)
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
    let mut names = engine
        .snapshot(session)
        .active_states
        .into_iter()
        .map(|state| state.state_name)
        .collect::<Vec<_>>();
    names.sort();
    names
}

fn real_value_property(
    project: &mut Project,
    context: ElementId,
    name: &str,
    default: &str,
) -> ElementId {
    let real = project
        .elements
        .values()
        .find(|element| element.kind == ElementKind::PrimitiveType && element.name == "Real")
        .map(|element| element.id)
        .unwrap_or_else(|| {
            project
                .create_element(ElementKind::PrimitiveType, "Real", project.root_id)
                .unwrap()
        });
    let id = project
        .create_typed_feature(
            ElementKind::ValueProperty,
            name,
            context,
            real,
            Multiplicity::ONE,
        )
        .unwrap();
    project.element_mut(id).unwrap().default_value = Some(default.into());
    id
}

#[test]
fn authored_defaults_are_available_to_state_machine_guards() {
    let (mut project, mut repository, machine_id, context) = fixture("DefaultGuard");
    real_value_property(&mut project, context, "temperature", "90.0");
    real_value_property(&mut project, context, "limit", "80.0");

    let initial = vertex("Initial", VertexKind::Pseudostate(PseudostateKind::Initial));
    let choice = vertex("Choice", VertexKind::Pseudostate(PseudostateKind::Choice));
    let high = vertex("High", VertexKind::State(State::default()));
    let low = vertex("Low", VertexKind::State(State::default()));
    let mut high_transition = transition(&choice, &high);
    high_transition.guard = Some("temperature > limit".into());
    let mut low_transition = transition(&choice, &low);
    low_transition.guard = Some("else".into());

    let machine = repository.state_machines.get_mut(&machine_id).unwrap();
    machine.regions[0]
        .vertices
        .extend([initial.clone(), choice.clone(), high.clone(), low.clone()]);
    machine.regions[0].transitions.extend([
        transition(&initial, &choice),
        high_transition,
        low_transition,
    ]);

    let mut engine = StateMachineExecutionEngine::new(repository, machine_id);
    let mut session = execution_session(&project, context);
    engine.initialize(&project, &mut session).unwrap();
    assert_eq!(active_names(&engine, &session), ["High"]);
}

#[test]
fn change_event_fires_only_on_false_to_true_edge() {
    let (mut project, mut repository, machine_id, context) = fixture("ChangeEdge");
    let flag = real_value_property(&mut project, context, "flag", "1.0");

    let initial = vertex("Initial", VertexKind::Pseudostate(PseudostateKind::Initial));
    let watching = vertex("Watching", VertexKind::State(State::default()));
    let changed = vertex("Changed", VertexKind::State(State::default()));
    let mut change = transition(&watching, &changed);
    change.trigger = Some(Trigger {
        event: Event::Change {
            expression: "flag > 0".into(),
        },
    });

    let machine = repository.state_machines.get_mut(&machine_id).unwrap();
    machine.regions[0]
        .vertices
        .extend([initial.clone(), watching.clone(), changed.clone()]);
    machine.regions[0]
        .transitions
        .extend([transition(&initial, &watching), change]);

    let mut engine = StateMachineExecutionEngine::new(repository, machine_id);
    let mut session = execution_session(&project, context);
    engine.initialize(&project, &mut session).unwrap();

    assert_eq!(engine.advance(&project, &mut session).unwrap(), EngineStepOutcome::Idle);
    assert_eq!(active_names(&engine, &session), ["Watching"]);

    session
        .set_value(&project, None, flag, RuntimeValue::Real(0.0))
        .unwrap();
    assert_eq!(engine.advance(&project, &mut session).unwrap(), EngineStepOutcome::Idle);
    assert_eq!(active_names(&engine, &session), ["Watching"]);

    session
        .set_value(&project, None, flag, RuntimeValue::Real(1.0))
        .unwrap();
    assert_eq!(
        engine.advance(&project, &mut session).unwrap(),
        EngineStepOutcome::Progressed
    );
    assert_eq!(active_names(&engine, &session), ["Changed"]);
}

#[test]
fn stale_time_event_cannot_fire_after_exit_and_reentry() {
    let (mut project, mut repository, machine_id, context) = fixture("TimerGeneration");
    let leave = project
        .create_element(ElementKind::Signal, "Leave", project.root_id)
        .unwrap();
    let reenter = project
        .create_element(ElementKind::Signal, "Reenter", project.root_id)
        .unwrap();

    let initial = vertex("Initial", VertexKind::Pseudostate(PseudostateKind::Initial));
    let waiting = vertex("Waiting", VertexKind::State(State::default()));
    let away = vertex("Away", VertexKind::State(State::default()));
    let expired = vertex("Expired", VertexKind::State(State::default()));
    let mut timeout = transition(&waiting, &expired);
    timeout.trigger = Some(Trigger {
        event: Event::Time {
            expression: "after 5s".into(),
            is_relative: true,
        },
    });

    let machine = repository.state_machines.get_mut(&machine_id).unwrap();
    machine.regions[0].vertices.extend([
        initial.clone(),
        waiting.clone(),
        away.clone(),
        expired.clone(),
    ]);
    machine.regions[0].transitions.extend([
        transition(&initial, &waiting),
        signal_transition(&waiting, &away, leave),
        signal_transition(&away, &waiting, reenter),
        timeout,
    ]);

    let mut engine = StateMachineExecutionEngine::new(repository, machine_id);
    let mut session = execution_session(&project, context);
    engine.initialize(&project, &mut session).unwrap();
    engine
        .queue_signal(&project, &mut session, leave, "Leave", Vec::new())
        .unwrap();
    engine.advance(&project, &mut session).unwrap();
    assert_eq!(active_names(&engine, &session), ["Away"]);

    engine
        .queue_signal(&project, &mut session, reenter, "Reenter", Vec::new())
        .unwrap();
    engine.advance(&project, &mut session).unwrap();
    assert_eq!(active_names(&engine, &session), ["Waiting"]);

    assert_eq!(engine.advance(&project, &mut session).unwrap(), EngineStepOutcome::Idle);
    assert_eq!(active_names(&engine, &session), ["Waiting"]);
    assert_eq!(session.simulation_time, SimulationTime::from_nanos(5_000_000_000));
    assert_eq!(engine.snapshot(&session).pending_event_count, 1);

    assert_eq!(
        engine.advance(&project, &mut session).unwrap(),
        EngineStepOutcome::Progressed
    );
    assert_eq!(active_names(&engine, &session), ["Expired"]);
}

#[test]
fn event_for_another_runtime_target_does_not_block_relevant_signal() {
    let (mut project, mut repository, machine_id, context) = fixture("AddressedEvents");
    let other = project
        .create_element(ElementKind::Block, "OtherController", project.root_id)
        .unwrap();
    let unrelated = project
        .create_element(ElementKind::Signal, "Unrelated", project.root_id)
        .unwrap();
    let start = project
        .create_element(ElementKind::Signal, "Start", project.root_id)
        .unwrap();

    let initial = vertex("Initial", VertexKind::Pseudostate(PseudostateKind::Initial));
    let idle = vertex("Idle", VertexKind::State(State::default()));
    let running = vertex("Running", VertexKind::State(State::default()));
    let machine = repository.state_machines.get_mut(&machine_id).unwrap();
    machine.regions[0]
        .vertices
        .extend([initial.clone(), idle.clone(), running.clone()]);
    machine.regions[0].transitions.extend([
        transition(&initial, &idle),
        signal_transition(&idle, &running, start),
    ]);

    let mut engine = StateMachineExecutionEngine::new(repository, machine_id);
    let mut session = execution_session(&project, context);
    engine.initialize(&project, &mut session).unwrap();
    session
        .queue_typed_event_at(
            &project,
            RuntimeEventRequest {
                due_time: SimulationTime::ZERO,
                kind: RuntimeEventKind::Signal,
                name: "Unrelated".into(),
                semantic_event_id: Some(unrelated),
                address: RuntimeEventAddress {
                    target_semantic_id: Some(other),
                    ..RuntimeEventAddress::default()
                },
                payload: Vec::new(),
            },
        )
        .unwrap();
    engine
        .queue_signal(&project, &mut session, start, "Start", Vec::new())
        .unwrap();

    assert_eq!(
        engine.advance(&project, &mut session).unwrap(),
        EngineStepOutcome::Progressed
    );
    assert_eq!(active_names(&engine, &session), ["Running"]);
    assert_eq!(engine.snapshot(&session).pending_event_count, 1);
    assert_eq!(session.event_queue.front().unwrap().event.name, "Unrelated");
}
