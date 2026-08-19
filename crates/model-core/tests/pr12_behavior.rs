use systems_modeler_core::{
    BehaviorError, BehaviorRepository, CombinedFragment, ElementKind, Event,
    ExecutionSpecification, InteractionOperand, InteractionOperator, Lifeline, LifelineId, Message,
    MessageSignature, MessageSort, Multiplicity, Occurrence, OccurrenceId, OperandId, Project,
    PseudostateKind, State, StateInvariant, Transition, TransitionId, TransitionKind, Trigger,
    Vertex, VertexId, VertexKind,
};

struct Fixture {
    project: Project,
    system: systems_modeler_core::ElementId,
    left_part: systems_modeler_core::ElementId,
    right_part: systems_modeler_core::ElementId,
    operation: systems_modeler_core::ElementId,
    signal: systems_modeler_core::ElementId,
}

fn fixture() -> Fixture {
    let mut project = Project::new("Behavior");
    let package = project
        .create_element(ElementKind::Package, "Behavior", project.root_id)
        .unwrap();
    let component = project
        .create_element(ElementKind::Block, "Component", package)
        .unwrap();
    let system = project
        .create_element(ElementKind::Block, "System", package)
        .unwrap();
    let operation = project
        .create_element(ElementKind::Operation, "start", component)
        .unwrap();
    let signal = project
        .create_element(ElementKind::Signal, "Ready", package)
        .unwrap();
    let left_part = project
        .create_typed_feature(
            ElementKind::PartProperty,
            "left",
            system,
            component,
            Multiplicity::ONE,
        )
        .unwrap();
    let right_part = project
        .create_typed_feature(
            ElementKind::PartProperty,
            "right",
            system,
            component,
            Multiplicity::ONE,
        )
        .unwrap();
    Fixture {
        project,
        system,
        left_part,
        right_part,
        operation,
        signal,
    }
}

#[test]
fn state_machine_supports_valid_initial_state_final_flow() {
    let f = fixture();
    let mut repository = BehaviorRepository::default();
    let machine_id = repository
        .create_state_machine(&f.project, f.system, "System States")
        .unwrap();
    let machine = repository.state_machines.get_mut(&machine_id).unwrap();
    let region = machine.regions.first_mut().unwrap();

    let initial = VertexId::new();
    let ready = VertexId::new();
    let final_state = VertexId::new();
    region.vertices.extend([
        Vertex {
            id: initial,
            name: String::new(),
            kind: VertexKind::Pseudostate(PseudostateKind::Initial),
        },
        Vertex {
            id: ready,
            name: "Ready".into(),
            kind: VertexKind::State(State::default()),
        },
        Vertex {
            id: final_state,
            name: String::new(),
            kind: VertexKind::FinalState,
        },
    ]);
    region.transitions.extend([
        Transition {
            id: TransitionId::new(),
            source_id: initial,
            target_id: ready,
            kind: TransitionKind::External,
            trigger: None,
            guard: None,
            effect: None,
        },
        Transition {
            id: TransitionId::new(),
            source_id: ready,
            target_id: final_state,
            kind: TransitionKind::External,
            trigger: Some(Trigger {
                event: Event::Signal {
                    signal_id: f.signal,
                },
            }),
            guard: Some("enabled".into()),
            effect: Some("stop".into()),
        },
    ]);

    repository.validate(&f.project).unwrap();
}

#[test]
fn initial_transition_rejects_trigger_or_guard() {
    let f = fixture();
    let mut repository = BehaviorRepository::default();
    let machine_id = repository
        .create_state_machine(&f.project, f.system, "System States")
        .unwrap();
    let machine = repository.state_machines.get_mut(&machine_id).unwrap();
    let region = machine.regions.first_mut().unwrap();
    let initial = VertexId::new();
    let state = VertexId::new();
    region.vertices.extend([
        Vertex {
            id: initial,
            name: String::new(),
            kind: VertexKind::Pseudostate(PseudostateKind::Initial),
        },
        Vertex {
            id: state,
            name: "Ready".into(),
            kind: VertexKind::State(State::default()),
        },
    ]);
    region.transitions.push(Transition {
        id: TransitionId::new(),
        source_id: initial,
        target_id: state,
        kind: TransitionKind::External,
        trigger: Some(Trigger {
            event: Event::Signal {
                signal_id: f.signal,
            },
        }),
        guard: None,
        effect: None,
    });

    assert_eq!(
        repository.validate(&f.project).unwrap_err(),
        BehaviorError::InitialTransitionHasTriggerOrGuard
    );
}

#[test]
fn sequence_supports_property_lifelines_call_execution_fragment_and_invariant() {
    let f = fixture();
    let mut repository = BehaviorRepository::default();
    let interaction_id = repository
        .create_interaction(&f.project, f.system, "System Sequence")
        .unwrap();
    let interaction = repository.interactions.get_mut(&interaction_id).unwrap();
    let left = LifelineId::new();
    let right = LifelineId::new();
    interaction.lifelines.extend([
        Lifeline {
            id: left,
            name: "left".into(),
            represented_path: vec![f.left_part],
        },
        Lifeline {
            id: right,
            name: "right".into(),
            represented_path: vec![f.right_part],
        },
    ]);

    interaction.messages.push(Message {
        id: systems_modeler_core::MessageId::new(),
        name: "start".into(),
        sort: MessageSort::SynchCall,
        send_event: Some(Occurrence {
            id: OccurrenceId::new(),
            lifeline_id: left,
            order: 10,
        }),
        receive_event: Some(Occurrence {
            id: OccurrenceId::new(),
            lifeline_id: right,
            order: 15,
        }),
        signature: Some(MessageSignature::Operation(f.operation)),
        arguments: vec!["mode".into()],
    });

    interaction.executions.push(ExecutionSpecification {
        id: systems_modeler_core::ExecutionId::new(),
        lifeline_id: right,
        start: Occurrence {
            id: OccurrenceId::new(),
            lifeline_id: right,
            order: 20,
        },
        finish: Occurrence {
            id: OccurrenceId::new(),
            lifeline_id: right,
            order: 30,
        },
        behavior_id: None,
    });

    interaction.fragments.push(CombinedFragment {
        id: systems_modeler_core::FragmentId::new(),
        operator: InteractionOperator::Alt,
        covered_lifelines: vec![left, right],
        operands: vec![
            InteractionOperand {
                id: OperandId::new(),
                guard: Some("ok".into()),
                start_order: 40,
                end_order: 50,
            },
            InteractionOperand {
                id: OperandId::new(),
                guard: Some("else".into()),
                start_order: 50,
                end_order: 60,
            },
        ],
    });

    interaction.state_invariants.push(StateInvariant {
        id: systems_modeler_core::InvariantId::new(),
        lifeline_id: right,
        order: 35,
        constraint: "state = Ready".into(),
    });

    repository.validate(&f.project).unwrap();
}
