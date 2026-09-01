use systems_modeler_core::behavior::{
    BehaviorRepository, BehaviorSemanticId, PseudostateKind, Transition, TransitionId,
    TransitionKind, Vertex, VertexId, VertexKind,
};
use systems_modeler_core::{
    ActivityEdge, ActivityEdgeId, ActivityEdgeKind, ActivityEndpoint, ActivityNode, ActivityNodeId,
    ActivityNodeKind, ActivityRepository, ActivitySemanticId, ElementKind, Project,
};

#[test]
fn pr48_specialized_external_identity_is_native_validated_authored_state() {
    let mut project = Project::new("PR48 Identity");
    let block = project
        .create_element(ElementKind::Block, "Controller", project.root_id)
        .unwrap();

    let mut activities = ActivityRepository::default();
    let activity_id = activities
        .create_activity(&project, block, Some(block), "Operate")
        .unwrap();
    let activity = activities.activities.get_mut(&activity_id).unwrap();
    activity.external_id = "catia:pr48::ACT".into();
    let initial = ActivityNode {
        id: ActivityNodeId::new(),
        name: "Start".into(),
        kind: ActivityNodeKind::Initial,
        partition_id: None,
        structured_node_id: None,
    };
    let final_node = ActivityNode {
        id: ActivityNodeId::new(),
        name: "Done".into(),
        kind: ActivityNodeKind::ActivityFinal,
        partition_id: None,
        structured_node_id: None,
    };
    let edge = ActivityEdge {
        id: ActivityEdgeId::new(),
        name: "flow".into(),
        kind: ActivityEdgeKind::ControlFlow,
        source: ActivityEndpoint::Node(initial.id),
        target: ActivityEndpoint::Node(final_node.id),
        guard: None,
        weight: None,
        selection: None,
        transformation: None,
        interrupting_region_id: None,
    };
    activity.nodes.extend([initial.clone(), final_node]);
    activity.edges.push(edge.clone());
    activities.external_ids.insert(
        "catia:pr48::NODE-START".into(),
        ActivitySemanticId::Node(initial.id),
    );
    activities.external_ids.insert(
        "catia:pr48::EDGE-1".into(),
        ActivitySemanticId::Edge(edge.id),
    );
    activities.validate(&project).unwrap();

    let mut behavior = BehaviorRepository::default();
    let sm = behavior
        .create_state_machine(&project, block, "Lifecycle")
        .unwrap();
    let machine = behavior.state_machines.get_mut(&sm).unwrap();
    machine.external_id = "catia:pr48::SM".into();
    let region = &mut machine.regions[0];
    let initial = Vertex {
        id: VertexId::new(),
        name: "Initial".into(),
        kind: VertexKind::Pseudostate(PseudostateKind::Initial),
    };
    let final_state = Vertex {
        id: VertexId::new(),
        name: "Final".into(),
        kind: VertexKind::FinalState,
    };
    let transition = Transition {
        id: TransitionId::new(),
        source_id: initial.id,
        target_id: final_state.id,
        kind: TransitionKind::External,
        trigger: None,
        guard: None,
        effect: None,
    };
    region.vertices.extend([initial.clone(), final_state]);
    region.transitions.push(transition.clone());
    behavior.external_ids.insert(
        "catia:pr48::REGION".into(),
        BehaviorSemanticId::Region(region.id),
    );
    behavior.external_ids.insert(
        "catia:pr48::VERTEX-INIT".into(),
        BehaviorSemanticId::Vertex(initial.id),
    );
    behavior.external_ids.insert(
        "catia:pr48::TRANS-1".into(),
        BehaviorSemanticId::Transition(transition.id),
    );
    behavior.validate(&project).unwrap();

    let missing = ActivitySemanticId::Node(ActivityNodeId::new());
    activities
        .external_ids
        .insert("catia:pr48::MISSING".into(), missing);
    assert!(activities.validate(&project).is_err());
}
