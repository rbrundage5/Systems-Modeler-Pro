use systems_modeler_core::*;

fn base_project() -> (Project, ElementId, ElementId, ElementId) {
    let mut project = Project::new("Activity qualification");
    let package = project
        .create_element(ElementKind::Package, "Behavior", project.root_id)
        .unwrap();
    let context = project
        .create_element(ElementKind::Block, "Controller", package)
        .unwrap();
    let data_type = project
        .create_element(ElementKind::DataType, "Command", package)
        .unwrap();
    (project, package, context, data_type)
}

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
fn activity_has_stable_identity_owner_and_classifier_context() {
    let (project, package, context, _) = base_project();
    let mut repository = ActivityRepository::default();
    let id = repository
        .create_activity(&project, package, Some(context), "Control")
        .unwrap();
    let activity = repository.activities.get(&id).unwrap();
    assert_eq!(activity.owner_id, package);
    assert_eq!(activity.context_id, Some(context));
    assert!(activity.external_id.starts_with("ACT-"));
}

#[test]
fn validates_initial_action_and_final_control_topology() {
    let (project, package, context, _) = base_project();
    let mut repository = ActivityRepository::default();
    let id = repository
        .create_activity(&project, package, Some(context), "Control")
        .unwrap();
    let activity = repository.activities.get_mut(&id).unwrap();
    let initial = node("", ActivityNodeKind::Initial);
    let action = node(
        "Compute",
        ActivityNodeKind::Action(Action {
            kind: ActionKind::Opaque {
                body: "x = 1".into(),
            },
            pins: vec![],
        }),
    );
    let final_node = node("", ActivityNodeKind::ActivityFinal);
    activity.edges.push(control(initial.id, action.id));
    activity.edges.push(control(action.id, final_node.id));
    activity.nodes.extend([initial, action, final_node]);
    repository.validate(&project).unwrap();
}

#[test]
fn rejects_invalid_fork_topology() {
    let (project, package, context, _) = base_project();
    let mut repository = ActivityRepository::default();
    let id = repository
        .create_activity(&project, package, Some(context), "Control")
        .unwrap();
    let activity = repository.activities.get_mut(&id).unwrap();
    let initial = node("", ActivityNodeKind::Initial);
    let fork = node("", ActivityNodeKind::Fork);
    let action = node(
        "A",
        ActivityNodeKind::Action(Action {
            kind: ActionKind::Opaque {
                body: String::new(),
            },
            pins: vec![],
        }),
    );
    activity.edges.push(control(initial.id, fork.id));
    activity.edges.push(control(fork.id, action.id));
    activity.nodes.extend([initial, fork, action]);
    assert_eq!(
        repository.validate(&project),
        Err(ActivityError::InvalidForkTopology)
    );
}

#[test]
fn object_flow_connects_output_to_input_with_matching_types() {
    let (project, package, context, data_type) = base_project();
    let mut repository = ActivityRepository::default();
    let id = repository
        .create_activity(&project, package, Some(context), "Data Flow")
        .unwrap();
    let activity = repository.activities.get_mut(&id).unwrap();
    let output = Pin::output("command", Some(data_type));
    let output_id = output.id;
    let input = Pin::input("command", Some(data_type));
    let input_id = input.id;
    activity.nodes.push(node(
        "Produce",
        ActivityNodeKind::Action(Action {
            kind: ActionKind::Opaque {
                body: String::new(),
            },
            pins: vec![output],
        }),
    ));
    activity.nodes.push(node(
        "Consume",
        ActivityNodeKind::Action(Action {
            kind: ActionKind::Opaque {
                body: String::new(),
            },
            pins: vec![input],
        }),
    ));
    activity.edges.push(ActivityEdge {
        id: ActivityEdgeId::new(),
        name: "command".into(),
        kind: ActivityEdgeKind::ObjectFlow,
        source: ActivityEndpoint::Pin(output_id),
        target: ActivityEndpoint::Pin(input_id),
        guard: None,
        weight: None,
        selection: None,
        transformation: None,
        interrupting_region_id: None,
    });
    repository.validate(&project).unwrap();
}

#[test]
fn call_operation_pins_reference_real_operation_parameters() {
    let (mut project, package, context, data_type) = base_project();
    let operation = project
        .create_element(ElementKind::Operation, "process", context)
        .unwrap();
    let parameter = project
        .create_typed_feature(
            ElementKind::Parameter,
            "command",
            operation,
            data_type,
            Multiplicity::ONE,
        )
        .unwrap();
    project.element_mut(parameter).unwrap().parameter_direction = Some(ParameterDirection::In);

    let mut repository = ActivityRepository::default();
    let id = repository
        .create_activity(&project, package, Some(context), "Call Operation")
        .unwrap();
    let mut pin = Pin::input("command", Some(data_type));
    pin.parameter_id = Some(parameter);
    repository.activities.get_mut(&id).unwrap().nodes.push(node(
        "process",
        ActivityNodeKind::Action(Action {
            kind: ActionKind::CallOperation {
                operation_id: operation,
            },
            pins: vec![pin],
        }),
    ));
    repository.validate(&project).unwrap();
}

#[test]
fn interrupting_edge_must_reference_interruptible_region() {
    let (project, package, context, _) = base_project();
    let mut repository = ActivityRepository::default();
    let id = repository
        .create_activity(&project, package, Some(context), "Interrupt")
        .unwrap();
    let activity = repository.activities.get_mut(&id).unwrap();
    let region = StructuredActivityNode {
        id: StructuredNodeId::new(),
        name: "interruptible".into(),
        kind: StructuredActivityNodeKind::InterruptibleRegion,
        parent_id: None,
    };
    let a = node(
        "A",
        ActivityNodeKind::Object(ObjectNode {
            kind: ObjectNodeKind::Object,
            type_id: None,
            multiplicity: Multiplicity::ONE,
            ordering: ObjectNodeOrdering::Unordered,
            selection: None,
        }),
    );
    let b = node(
        "B",
        ActivityNodeKind::Object(ObjectNode {
            kind: ObjectNodeKind::Object,
            type_id: None,
            multiplicity: Multiplicity::ONE,
            ordering: ObjectNodeOrdering::Unordered,
            selection: None,
        }),
    );
    activity.edges.push(ActivityEdge {
        id: ActivityEdgeId::new(),
        name: String::new(),
        kind: ActivityEdgeKind::ObjectFlow,
        source: ActivityEndpoint::Node(a.id),
        target: ActivityEndpoint::Node(b.id),
        guard: None,
        weight: None,
        selection: None,
        transformation: None,
        interrupting_region_id: Some(region.id),
    });
    activity.structured_nodes.push(region);
    activity.nodes.extend([a, b]);
    repository.validate(&project).unwrap();
}
