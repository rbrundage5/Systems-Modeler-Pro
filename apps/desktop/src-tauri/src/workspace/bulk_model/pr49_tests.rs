use super::*;
use systems_modeler_core::behavior::{BehaviorSemanticId, InteractionOperator, MessageSort};

fn fixture() -> (
    WorkspaceState,
    activity_workspace::ActivityWorkspaceState,
    ElementId,
    ElementId,
    ElementId,
    ElementId,
    ElementId,
    ElementId,
    ElementId,
) {
    let workspace = WorkspaceState::default();
    let mut project = Project::new("PR49 native semantics");
    let root = project.root_id;
    let package = project
        .create_element(ElementKind::Package, "Model", root)
        .unwrap();
    let real = project
        .create_element(ElementKind::PrimitiveType, "Real", package)
        .unwrap();
    let system = project
        .create_element(ElementKind::Block, "System", package)
        .unwrap();
    let controller = project
        .create_element(ElementKind::Block, "Controller", package)
        .unwrap();
    let part = project
        .create_typed_feature(
            ElementKind::PartProperty,
            "controller",
            system,
            controller,
            Multiplicity::ONE,
        )
        .unwrap();
    let operation = project
        .create_element(ElementKind::Operation, "initialize", controller)
        .unwrap();
    let value = project
        .create_typed_feature(
            ElementKind::ValueProperty,
            "mass",
            system,
            real,
            Multiplicity::ONE,
        )
        .unwrap();
    let constraint = project
        .create_element(ElementKind::ConstraintBlock, "MassConstraint", package)
        .unwrap();
    let parameter = project
        .create_typed_feature(
            ElementKind::ConstraintParameter,
            "m",
            constraint,
            real,
            Multiplicity::ONE,
        )
        .unwrap();
    let constraint_property = project
        .create_typed_feature(
            ElementKind::ConstraintProperty,
            "massConstraint",
            system,
            constraint,
            Multiplicity::ONE,
        )
        .unwrap();
    *workspace.project.lock().unwrap() = Some(project);
    (
        workspace,
        activity_workspace::ActivityWorkspaceState::default(),
        system,
        part,
        operation,
        value,
        constraint,
        parameter,
        constraint_property,
    )
}

#[test]
fn pr49_unified_candidate_builds_native_sequence_and_parametric_semantics_atomically() {
    let (
        workspace,
        activity_state,
        system,
        part,
        operation,
        value,
        constraint,
        parameter,
        constraint_property,
    ) = fixture();
    let plan = ModelBuildPlan {
        source_namespace: "catia:pr49".into(),
        operations: vec![
            ModelBuildOperation::Sequence {
                operation: SequenceBuildOperation::CreateInteraction {
                    external_id: "INT".into(),
                    name: "ControlExchange".into(),
                    context: BuildReference::Existing(system),
                },
            },
            ModelBuildOperation::Sequence {
                operation: SequenceBuildOperation::CreateLifeline {
                    external_id: "LL".into(),
                    interaction: BuildReference::External("INT".into()),
                    name: "controller".into(),
                    represented_path: vec![BuildReference::Existing(part)],
                },
            },
            ModelBuildOperation::Sequence {
                operation: SequenceBuildOperation::CreateOccurrence {
                    external_id: "SEND".into(),
                    interaction: BuildReference::External("INT".into()),
                    lifeline: BuildReference::External("LL".into()),
                    order: 1,
                },
            },
            ModelBuildOperation::Sequence {
                operation: SequenceBuildOperation::CreateOccurrence {
                    external_id: "RECV".into(),
                    interaction: BuildReference::External("INT".into()),
                    lifeline: BuildReference::External("LL".into()),
                    order: 2,
                },
            },
            ModelBuildOperation::Sequence {
                operation: SequenceBuildOperation::CreateMessage {
                    external_id: "MSG".into(),
                    interaction: BuildReference::External("INT".into()),
                    name: "initialize".into(),
                    sort: MessageSort::SynchCall,
                    send: Some(BuildReference::External("SEND".into())),
                    receive: Some(BuildReference::External("RECV".into())),
                    signature: Some(MessageSignatureBuild::Operation(BuildReference::Existing(
                        operation,
                    ))),
                    arguments: vec!["mode=normal".into()],
                },
            },
            ModelBuildOperation::Sequence {
                operation: SequenceBuildOperation::CreateExecution {
                    external_id: "EXEC".into(),
                    interaction: BuildReference::External("INT".into()),
                    lifeline: BuildReference::External("LL".into()),
                    start: BuildReference::External("SEND".into()),
                    finish: BuildReference::External("RECV".into()),
                    behavior: Some(BuildReference::Existing(operation)),
                },
            },
            ModelBuildOperation::Sequence {
                operation: SequenceBuildOperation::CreateFragment {
                    external_id: "FRAG".into(),
                    interaction: BuildReference::External("INT".into()),
                    operator: InteractionOperator::Opt,
                    covered_lifelines: vec![BuildReference::External("LL".into())],
                },
            },
            ModelBuildOperation::Sequence {
                operation: SequenceBuildOperation::CreateOperand {
                    external_id: "OPERAND".into(),
                    fragment: BuildReference::External("FRAG".into()),
                    guard: Some("enabled".into()),
                    start_order: 1,
                    end_order: 2,
                },
            },
            ModelBuildOperation::Sequence {
                operation: SequenceBuildOperation::CreateInvariant {
                    external_id: "INV".into(),
                    interaction: BuildReference::External("INT".into()),
                    lifeline: BuildReference::External("LL".into()),
                    order: 2,
                    constraint: "ready".into(),
                },
            },
            ModelBuildOperation::Parametric {
                operation: ParametricBuildOperation::UpdateElementSemantics {
                    element: BuildReference::Existing(constraint),
                    constraint_expression: Some("m > 0".into()),
                    quantity_kind_external_id: None,
                    unit_external_id: None,
                    quantity_dimension: Some(Some("M".into())),
                    unit_symbol: Some(Some("kg".into())),
                    unit_scale_to_base: Some(1.0),
                },
            },
            ModelBuildOperation::Parametric {
                operation: ParametricBuildOperation::CreateBinding {
                    external_id: "BIND".into(),
                    name: "massBinding".into(),
                    owner: BuildReference::Existing(system),
                    source: BindingEndpointBuild {
                        role: BuildReference::Existing(value),
                        parameter: None,
                    },
                    target: BindingEndpointBuild {
                        role: BuildReference::Existing(constraint_property),
                        parameter: Some(BuildReference::Existing(parameter)),
                    },
                },
            },
        ],
    };

    let preview = preview_unified_model_build(&plan, &workspace, &activity_state);
    assert!(preview.is_valid(), "{:?}", preview.diagnostics);
    assert!(workspace.behavior.lock().unwrap().interactions.is_empty());
    assert!(workspace
        .project
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .relationships
        .is_empty());

    apply_unified_model_build(&plan, &workspace, &activity_state).unwrap();

    let behavior = workspace.behavior.lock().unwrap();
    let interaction = behavior
        .interactions
        .values()
        .find(|record| record.external_id == "catia:pr49::INT")
        .unwrap();
    assert_eq!(interaction.context_id, system);
    assert_eq!(interaction.lifelines[0].represented_path, vec![part]);
    assert_eq!(interaction.messages.len(), 1);
    assert_eq!(interaction.messages[0].sort, MessageSort::SynchCall);
    assert!(matches!(
        interaction.messages[0].signature,
        Some(systems_modeler_core::behavior::MessageSignature::Operation(id)) if id == operation
    ));
    assert_eq!(interaction.executions.len(), 1);
    assert_eq!(interaction.fragments.len(), 1);
    assert_eq!(interaction.fragments[0].operands.len(), 1);
    assert_eq!(interaction.state_invariants.len(), 1);
    assert!(matches!(
        behavior.external_ids.get("catia:pr49::MSG"),
        Some(BehaviorSemanticId::Message(_))
    ));
    drop(behavior);

    let project = workspace.project.lock().unwrap();
    let project = project.as_ref().unwrap();
    assert_eq!(
        project.element(constraint).unwrap().constraint_expression,
        "m > 0"
    );
    assert_eq!(
        project.element(constraint).unwrap().quantity_dimension.as_deref(),
        Some("M")
    );
    assert_eq!(project.element(constraint).unwrap().unit_symbol.as_deref(), Some("kg"));
    assert_eq!(project.element(constraint).unwrap().unit_scale_to_base, 1.0);
    let binding = project
        .relationships
        .values()
        .find(|record| record.external_id == "catia:pr49::BIND")
        .unwrap();
    assert_eq!(binding.kind, RelationshipKind::BindingConnector);
    let endpoints = binding.binding.as_ref().unwrap();
    assert_eq!(endpoints.source.role_id, value);
    assert_eq!(endpoints.source.parameter_id, None);
    assert_eq!(endpoints.target.role_id, constraint_property);
    assert_eq!(endpoints.target.parameter_id, Some(parameter));
}

#[test]
fn pr49_late_cross_interaction_occurrence_error_rolls_back_sequence_candidate() {
    let (workspace, activity_state, system, part, ..) = fixture();
    let plan = ModelBuildPlan {
        source_namespace: "catia:pr49".into(),
        operations: vec![
            ModelBuildOperation::Sequence {
                operation: SequenceBuildOperation::CreateInteraction {
                    external_id: "A".into(),
                    name: "A".into(),
                    context: BuildReference::Existing(system),
                },
            },
            ModelBuildOperation::Sequence {
                operation: SequenceBuildOperation::CreateInteraction {
                    external_id: "B".into(),
                    name: "B".into(),
                    context: BuildReference::Existing(system),
                },
            },
            ModelBuildOperation::Sequence {
                operation: SequenceBuildOperation::CreateLifeline {
                    external_id: "L".into(),
                    interaction: BuildReference::External("A".into()),
                    name: "controller".into(),
                    represented_path: vec![BuildReference::Existing(part)],
                },
            },
            ModelBuildOperation::Sequence {
                operation: SequenceBuildOperation::CreateOccurrence {
                    external_id: "BAD".into(),
                    interaction: BuildReference::External("B".into()),
                    lifeline: BuildReference::External("L".into()),
                    order: 1,
                },
            },
        ],
    };

    let preview = preview_unified_model_build(&plan, &workspace, &activity_state);
    assert!(!preview.is_valid());
    assert!(preview
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "SEQUENCE_INTERACTION_MISMATCH"));
    assert!(apply_unified_model_build(&plan, &workspace, &activity_state).is_err());
    assert!(workspace.behavior.lock().unwrap().interactions.is_empty());
}
