use systems_modeler_core::{
    Action, ActionKind, ActivityEdge, ActivityEdgeId, ActivityEdgeKind, ActivityEndpoint, ActivityNode,
    ActivityNodeId, ActivityNodeKind, ActivityRepository, ElementKind, Project,
};
use systems_modeler_persistence::{
    load_activity_repository, save_activity_repository, ProjectDatabase,
};

fn node(name: &str, kind: ActivityNodeKind) -> ActivityNode {
    ActivityNode {
        id: ActivityNodeId::new(),
        name: name.into(),
        kind,
        partition_id: None,
        structured_node_id: None,
    }
}

fn control(source: ActivityNodeId, target: ActivityNodeId) -> ActivityEdge {
    ActivityEdge {
        id: ActivityEdgeId::new(),
        name: String::new(),
        kind: ActivityEdgeKind::ControlFlow,
        source: ActivityEndpoint::Node(source),
        target: ActivityEndpoint::Node(target),
        guard: None,
        weight: None,
        selection: None,
        transformation: None,
        interrupting_region_id: None,
    }
}

#[test]
fn activity_repository_round_trips_with_stable_semantic_identity() {
    let mut project = Project::new("Activity persistence");
    let behavior_package = project
        .create_element(ElementKind::Package, "Behavior", project.root_id)
        .unwrap();
    let controller = project
        .create_element(ElementKind::Block, "Controller", behavior_package)
        .unwrap();

    let mut repository = ActivityRepository::default();
    let activity_id = repository
        .create_activity(
            &project,
            behavior_package,
            Some(controller),
            "Control Activity",
        )
        .unwrap();

    let activity = repository.activities.get_mut(&activity_id).unwrap();
    let initial = node("", ActivityNodeKind::Initial);
    let action = node(
        "Compute",
        ActivityNodeKind::Action(Action {
            kind: ActionKind::Opaque {
                body: "result = input".into(),
            },
            pins: vec![],
        }),
    );
    let final_node = node("", ActivityNodeKind::ActivityFinal);
    let first_edge = control(initial.id, action.id);
    let second_edge = control(action.id, final_node.id);

    let initial_id = initial.id;
    let action_id = action.id;
    let final_id = final_node.id;
    let first_edge_id = first_edge.id;
    let second_edge_id = second_edge.id;

    activity.nodes.extend([initial, action, final_node]);
    activity.edges.extend([first_edge, second_edge]);
    repository.validate(&project).unwrap();

    let mut database = ProjectDatabase::open_in_memory().unwrap();
    database.save_project(&project).unwrap();
    save_activity_repository(&mut database, &project, &repository).unwrap();

    let loaded_project = database.load_project(project.id).unwrap();
    let loaded = load_activity_repository(&database, &loaded_project).unwrap();
    let loaded_activity = loaded.activities.get(&activity_id).unwrap();

    assert_eq!(loaded_activity.id, activity_id);
    assert_eq!(loaded_activity.owner_id, behavior_package);
    assert_eq!(loaded_activity.context_id, Some(controller));
    assert!(loaded_activity.nodes.iter().any(|node| node.id == initial_id));
    assert!(loaded_activity.nodes.iter().any(|node| node.id == action_id));
    assert!(loaded_activity.nodes.iter().any(|node| node.id == final_id));
    assert!(loaded_activity.edges.iter().any(|edge| edge.id == first_edge_id));
    assert!(loaded_activity.edges.iter().any(|edge| edge.id == second_edge_id));

    loaded.validate(&loaded_project).unwrap();
}

#[test]
fn missing_activity_metadata_loads_as_empty_repository() {
    let project = Project::new("No activity metadata");
    let mut database = ProjectDatabase::open_in_memory().unwrap();
    database.save_project(&project).unwrap();

    let loaded = load_activity_repository(&database, &project).unwrap();
    assert!(loaded.activities.is_empty());
}
