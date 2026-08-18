use systems_modeler_core::ElementId;
use systems_modeler_core::behavior::{
    BehaviorRepository, Interaction, InteractionId, Lifeline, LifelineId, Message, MessageId,
    MessageSort, Occurrence, OccurrenceId, Region, RegionId, State, StateMachine, StateMachineId,
    Vertex, VertexId, VertexKind,
};
use uuid::Uuid;

fn element_id() -> ElementId {
    ElementId(Uuid::new_v4())
}

#[test]
fn behavior_repository_round_trip_preserves_submachine_and_occurrence_identity() {
    let context_id = element_id();
    let child_id = StateMachineId::new();
    let parent_id = StateMachineId::new();
    let submachine_vertex_id = VertexId::new();

    let child = StateMachine {
        id: child_id,
        external_id: "SM-CHILD".into(),
        name: "Child".into(),
        context_id,
        regions: vec![Region {
            id: RegionId::new(),
            name: "Region".into(),
            vertices: Vec::new(),
            transitions: Vec::new(),
        }],
    };
    let parent = StateMachine {
        id: parent_id,
        external_id: "SM-PARENT".into(),
        name: "Parent".into(),
        context_id,
        regions: vec![Region {
            id: RegionId::new(),
            name: "Region".into(),
            vertices: vec![Vertex {
                id: submachine_vertex_id,
                name: "Run Child".into(),
                kind: VertexKind::State(State {
                    submachine: Some(child_id),
                    ..State::default()
                }),
            }],
            transitions: Vec::new(),
        }],
    };

    let interaction_id = InteractionId::new();
    let source_id = LifelineId::new();
    let target_id = LifelineId::new();
    let send_id = OccurrenceId::new();
    let receive_id = OccurrenceId::new();
    let message_id = MessageId::new();
    let interaction = Interaction {
        id: interaction_id,
        external_id: "INT-1".into(),
        name: "Sequence".into(),
        context_id,
        lifelines: vec![
            Lifeline {
                id: source_id,
                name: "source".into(),
                represented_path: Vec::new(),
            },
            Lifeline {
                id: target_id,
                name: "target".into(),
                represented_path: Vec::new(),
            },
        ],
        messages: vec![Message {
            id: message_id,
            name: "call".into(),
            sort: MessageSort::SynchCall,
            send_event: Some(Occurrence {
                id: send_id,
                lifeline_id: source_id,
                order: 10,
            }),
            receive_event: Some(Occurrence {
                id: receive_id,
                lifeline_id: target_id,
                order: 15,
            }),
            signature: None,
            arguments: vec!["arg".into()],
        }],
        executions: Vec::new(),
        fragments: Vec::new(),
        state_invariants: Vec::new(),
    };

    let mut repository = BehaviorRepository::default();
    repository.state_machines.insert(child_id, child);
    repository.state_machines.insert(parent_id, parent);
    repository.interactions.insert(interaction_id, interaction);

    let payload = serde_json::to_string(&repository).expect("serialize behavior repository");
    let restored: BehaviorRepository =
        serde_json::from_str(&payload).expect("deserialize behavior repository");

    let restored_parent = restored
        .state_machines
        .get(&parent_id)
        .expect("parent state machine");
    let restored_state = match &restored_parent.regions[0].vertices[0].kind {
        VertexKind::State(state) => state,
        _ => panic!("expected State"),
    };
    assert_eq!(restored_state.submachine, Some(child_id));

    let restored_message = &restored
        .interactions
        .get(&interaction_id)
        .expect("interaction")
        .messages[0];
    assert_eq!(restored_message.id, message_id);
    assert_eq!(restored_message.send_event.as_ref().expect("send").id, send_id);
    assert_eq!(
        restored_message.receive_event.as_ref().expect("receive").id,
        receive_id
    );
    assert_eq!(restored_message.send_event.as_ref().expect("send").order, 10);
    assert_eq!(
        restored_message.receive_event.as_ref().expect("receive").order,
        15
    );
}
