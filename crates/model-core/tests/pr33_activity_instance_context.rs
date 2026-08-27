use systems_modeler_core::*;

fn node(name: &str, kind: ActivityNodeKind) -> ActivityNode {
    ActivityNode {
        id: ActivityNodeId::new(),
        name: name.into(),
        kind,
        partition_id: None,
        structured_node_id: None,
    }
}

fn action(name: &str) -> ActivityNode {
    node(
        name,
        ActivityNodeKind::Action(Action {
            kind: ActionKind::Opaque {
                body: String::new(),
            },
            pins: Vec::new(),
        }),
    )
}

fn edge(source: &ActivityNode, target: &ActivityNode, guard: Option<&str>) -> ActivityEdge {
    ActivityEdge {
        id: ActivityEdgeId::new(),
        name: String::new(),
        kind: ActivityEdgeKind::ControlFlow,
        source: ActivityEndpoint::Node(source.id),
        target: ActivityEndpoint::Node(target.id),
        guard: guard.map(str::to_owned),
        weight: None,
        selection: None,
        transformation: None,
        interrupting_region_id: None,
    }
}

fn run_to_completion(
    project: &Project,
    engine: &mut ActivityExecutionEngine,
    session: &mut ExecutionSession,
) {
    for _ in 0..100 {
        match engine.advance(project, session).unwrap() {
            ActivityAdvanceOutcome::Completed => return,
            ActivityAdvanceOutcome::Progressed | ActivityAdvanceOutcome::Waiting => {}
        }
    }
    panic!("Activity did not complete within the PR33 qualification bound");
}

#[test]
fn repeated_classifier_activities_resolve_values_from_the_selected_runtime_occurrence() {
    let mut project = Project::new("PR33 Activity runtime occurrence context");
    let package = project
        .create_element(ElementKind::Package, "Model", project.root_id)
        .unwrap();
    let real = project
        .create_element(ElementKind::PrimitiveType, "Real", package)
        .unwrap();
    let controller = project
        .create_element(ElementKind::Block, "Controller", package)
        .unwrap();
    let speed = project
        .create_typed_feature(
            ElementKind::ValueProperty,
            "speed",
            controller,
            real,
            Multiplicity::ONE,
        )
        .unwrap();
    project.element_mut(speed).unwrap().default_value = Some("0.0".into());

    let vehicle = project
        .create_element(ElementKind::Block, "Vehicle", package)
        .unwrap();
    project
        .create_typed_feature(
            ElementKind::PartProperty,
            "leftController",
            vehicle,
            controller,
            Multiplicity::ONE,
        )
        .unwrap();
    project
        .create_typed_feature(
            ElementKind::PartProperty,
            "rightController",
            vehicle,
            controller,
            Multiplicity::ONE,
        )
        .unwrap();

    let structural_configuration = StructuralRuntimeConfiguration {
        root_instance_name: Some("vehicle".into()),
        ..StructuralRuntimeConfiguration::default()
    };
    let structure = StructuralRuntime::build(&project, vehicle, &structural_configuration).unwrap();
    let left = structure
        .instance_by_path("vehicle.leftController")
        .unwrap()
        .id;
    let right = structure
        .instance_by_path("vehicle.rightController")
        .unwrap()
        .id;
    assert_ne!(left, right);

    let mut repository = ActivityRepository::default();
    let activity_id = repository
        .create_activity(&project, package, Some(controller), "Controller Logic")
        .unwrap();
    let activity = repository.activities.get_mut(&activity_id).unwrap();
    let initial = node("Initial", ActivityNodeKind::Initial);
    let decision = node(
        "Speed decision",
        ActivityNodeKind::Decision {
            decision_input: None,
        },
    );
    let fast = action("Fast path");
    let slow = action("Slow path");
    let final_node = node("Final", ActivityNodeKind::ActivityFinal);
    activity.edges.extend([
        edge(&initial, &decision, None),
        edge(&decision, &fast, Some("speed >= 10")),
        edge(&decision, &slow, Some("else")),
        edge(&fast, &final_node, None),
        edge(&slow, &final_node, None),
    ]);
    activity
        .nodes
        .extend([initial, decision, fast, slow, final_node]);
    repository.validate(&project).unwrap();

    let execution_configuration = ExecutionConfiguration {
        root_semantic_id: vehicle,
        random_seed: 0,
        max_steps: 100,
        max_queued_events: 100,
    };

    let execute = |runtime_instance_id: RuntimeInstanceId, value: f64| {
        let mut session =
            ExecutionSession::with_configuration(&project, execution_configuration.clone())
                .unwrap();
        session
            .set_structural_configuration(structural_configuration.clone())
            .unwrap();
        let mut engine = ActivityExecutionEngine::new(repository.clone(), activity_id)
            .with_runtime_instance(runtime_instance_id);
        engine.initialize(&project, &mut session).unwrap();
        session
            .set_value(
                &project,
                Some(runtime_instance_id),
                speed,
                RuntimeValue::Real(value),
            )
            .unwrap();
        run_to_completion(&project, &mut engine, &mut session);
        (engine.snapshot(&session), session)
    };

    let (left_snapshot, left_session) = execute(left, 25.0);
    let (right_snapshot, right_session) = execute(right, 2.0);

    assert_eq!(left_snapshot.runtime_instance_id, Some(left));
    assert_eq!(right_snapshot.runtime_instance_id, Some(right));
    assert!(
        left_snapshot
            .nodes
            .iter()
            .any(|node| node.name == "Fast path" && node.activation_count == 1)
    );
    assert!(
        left_snapshot
            .nodes
            .iter()
            .any(|node| node.name == "Slow path" && node.activation_count == 0)
    );
    assert!(
        right_snapshot
            .nodes
            .iter()
            .any(|node| node.name == "Slow path" && node.activation_count == 1)
    );
    assert!(
        right_snapshot
            .nodes
            .iter()
            .any(|node| node.name == "Fast path" && node.activation_count == 0)
    );

    assert_eq!(
        left_session.value(Some(left), speed),
        Some(&RuntimeValue::Real(25.0))
    );
    assert_eq!(
        left_session.value(Some(right), speed),
        Some(&RuntimeValue::Real(0.0))
    );
    assert_eq!(
        right_session.value(Some(right), speed),
        Some(&RuntimeValue::Real(2.0))
    );
    assert_eq!(
        right_session.value(Some(left), speed),
        Some(&RuntimeValue::Real(0.0))
    );
}
