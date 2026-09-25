use super::*;
use systems_modeler_core::behavior::{BehaviorSemanticId, PseudostateKind, TransitionKind};
use systems_modeler_core::{ActivityEdgeKind, ActivitySemanticId, ElementKind};

fn states() -> (
    WorkspaceState,
    activity_workspace::ActivityWorkspaceState,
    ElementId,
) {
    let workspace = WorkspaceState::default();
    let project = Project::new("PR48 Unified");
    let root = project.root_id;
    *workspace.project.lock().unwrap() = Some(project);
    (
        workspace,
        activity_workspace::ActivityWorkspaceState::default(),
        root,
    )
}

#[test]
fn pr48_unified_candidate_commits_project_activity_and_behavior_once() {
    let (workspace, activity_state, root) = states();
    let plan = ModelBuildPlan {
        source_namespace: "catia:pr48".into(),
        operations: vec![
            ModelBuildOperation::CreateElement {
                external_id: "SYS".into(),
                kind: ElementKind::Block,
                name: "Controller".into(),
                owner: BuildReference::Existing(root),
                type_ref: None,
            },
            ModelBuildOperation::Activity {
                operation: ActivityBuildOperation::CreateActivity {
                    external_id: "ACT".into(),
                    name: "Flow".into(),
                    owner: BuildReference::External("SYS".into()),
                    context: Some(BuildReference::External("SYS".into())),
                },
            },
            ModelBuildOperation::Activity {
                operation: ActivityBuildOperation::CreateNode {
                    external_id: "AI".into(),
                    activity: BuildReference::External("ACT".into()),
                    name: "Start".into(),
                    kind: ActivityNodeBuildKind::Initial,
                    partition: None,
                    structured_node: None,
                },
            },
            ModelBuildOperation::Activity {
                operation: ActivityBuildOperation::CreateNode {
                    external_id: "AF".into(),
                    activity: BuildReference::External("ACT".into()),
                    name: "Done".into(),
                    kind: ActivityNodeBuildKind::ActivityFinal,
                    partition: None,
                    structured_node: None,
                },
            },
            ModelBuildOperation::Activity {
                operation: ActivityBuildOperation::CreateEdge {
                    external_id: "AE".into(),
                    activity: BuildReference::External("ACT".into()),
                    name: "flow".into(),
                    kind: ActivityEdgeKind::ControlFlow,
                    source: ActivityEndpointReference::Node(BuildReference::External("AI".into())),
                    target: ActivityEndpointReference::Node(BuildReference::External("AF".into())),
                    guard: None,
                    weight: None,
                    selection: None,
                    transformation: None,
                    interrupting_region: None,
                },
            },
            ModelBuildOperation::StateMachine {
                operation: StateMachineBuildOperation::CreateStateMachine {
                    external_id: "SM".into(),
                    name: "Lifecycle".into(),
                    context: BuildReference::External("SYS".into()),
                },
            },
            ModelBuildOperation::StateMachine {
                operation: StateMachineBuildOperation::CreateRegion {
                    external_id: "REG".into(),
                    parent: RegionParentReference::StateMachine(BuildReference::External(
                        "SM".into(),
                    )),
                    name: "main".into(),
                },
            },
            ModelBuildOperation::StateMachine {
                operation: StateMachineBuildOperation::CreateVertex {
                    external_id: "SI".into(),
                    region: BuildReference::External("REG".into()),
                    name: "Initial".into(),
                    kind: VertexBuildKind::Pseudostate(PseudostateKind::Initial),
                },
            },
            ModelBuildOperation::StateMachine {
                operation: StateMachineBuildOperation::CreateVertex {
                    external_id: "SF".into(),
                    region: BuildReference::External("REG".into()),
                    name: "Final".into(),
                    kind: VertexBuildKind::FinalState,
                },
            },
            ModelBuildOperation::StateMachine {
                operation: StateMachineBuildOperation::CreateTransition {
                    external_id: "ST".into(),
                    region: BuildReference::External("REG".into()),
                    source: BuildReference::External("SI".into()),
                    target: BuildReference::External("SF".into()),
                    kind: TransitionKind::External,
                    trigger: None,
                    guard: None,
                    effect: None,
                },
            },
        ],
    };
    let preview = preview_unified_model_build(&plan, &workspace, &activity_state);
    assert!(preview.is_valid(), "{:?}", preview.diagnostics);
    assert_eq!(
        workspace
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .elements
            .len(),
        1
    );
    assert!(
        activity_state
            .repository
            .lock()
            .unwrap()
            .activities
            .is_empty()
    );
    assert!(workspace.behavior.lock().unwrap().state_machines.is_empty());
    apply_unified_model_build(&plan, &workspace, &activity_state).unwrap();
    let project = workspace.project.lock().unwrap();
    assert_eq!(project.as_ref().unwrap().elements.len(), 2);
    let activities = activity_state.repository.lock().unwrap();
    assert!(matches!(
        activity_identity_for_external(&activities, "catia:pr48::AI"),
        Some(ActivitySemanticId::Node(_))
    ));
    let behavior = workspace.behavior.lock().unwrap();
    assert!(matches!(
        behavior_identity_for_external(&behavior, "catia:pr48::ST"),
        Some(BehaviorSemanticId::Transition(_))
    ));
}

#[test]
fn pr48_late_state_machine_error_rolls_back_project_and_activity_candidate() {
    let (workspace, activity_state, root) = states();
    let plan = ModelBuildPlan {
        source_namespace: "catia:pr48".into(),
        operations: vec![
            ModelBuildOperation::CreateElement {
                external_id: "SYS".into(),
                kind: ElementKind::Block,
                name: "Controller".into(),
                owner: BuildReference::Existing(root),
                type_ref: None,
            },
            ModelBuildOperation::Activity {
                operation: ActivityBuildOperation::CreateActivity {
                    external_id: "ACT".into(),
                    name: "Flow".into(),
                    owner: BuildReference::External("SYS".into()),
                    context: Some(BuildReference::External("SYS".into())),
                },
            },
            ModelBuildOperation::StateMachine {
                operation: StateMachineBuildOperation::CreateStateMachine {
                    external_id: "SM".into(),
                    name: "Lifecycle".into(),
                    context: BuildReference::External("SYS".into()),
                },
            },
            ModelBuildOperation::StateMachine {
                operation: StateMachineBuildOperation::CreateRegion {
                    external_id: "REG".into(),
                    parent: RegionParentReference::StateMachine(BuildReference::External(
                        "SM".into(),
                    )),
                    name: "main".into(),
                },
            },
            ModelBuildOperation::StateMachine {
                operation: StateMachineBuildOperation::CreateVertex {
                    external_id: "SI".into(),
                    region: BuildReference::External("REG".into()),
                    name: "Initial".into(),
                    kind: VertexBuildKind::Pseudostate(PseudostateKind::Initial),
                },
            },
        ],
    };
    let preview = preview_unified_model_build(&plan, &workspace, &activity_state);
    assert!(!preview.is_valid());
    assert!(
        preview
            .diagnostics
            .iter()
            .any(|d| d.code == "STATE_MACHINE_SEMANTIC_VALIDATION")
    );
    assert!(apply_unified_model_build(&plan, &workspace, &activity_state).is_err());
    assert_eq!(
        workspace
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .elements
            .len(),
        1
    );
    assert!(
        activity_state
            .repository
            .lock()
            .unwrap()
            .activities
            .is_empty()
    );
    assert!(workspace.behavior.lock().unwrap().state_machines.is_empty());
}
