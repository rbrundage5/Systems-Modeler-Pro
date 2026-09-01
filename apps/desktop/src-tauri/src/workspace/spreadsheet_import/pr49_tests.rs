use super::*;
use std::fs;
use systems_modeler_core::behavior::{BehaviorSemanticId, MessageSort};

const NS: &str = "catia:pr49";

fn temp_csv(body: &str) -> String {
    let path = std::env::temp_dir().join(format!("pr49-{}.csv", uuid::Uuid::new_v4()));
    fs::write(&path, body).unwrap();
    path.to_string_lossy().into_owned()
}

fn fixture() -> (
    WorkspaceState,
    super::super::activity_workspace::ActivityWorkspaceState,
    ElementId,
    ElementId,
    ElementId,
    ElementId,
    ElementId,
    ElementId,
    ElementId,
) {
    let state = WorkspaceState::default();
    let mut project = Project::new("PR49 spreadsheet");
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
    *state.project.lock().unwrap() = Some(project);
    (
        state,
        super::super::activity_workspace::ActivityWorkspaceState::default(),
        system,
        part,
        operation,
        value,
        constraint,
        parameter,
        constraint_property,
    )
}

fn semantic_map(source: String, root: ElementId) -> SpreadsheetImportMap {
    let columns = [
        ("Kind", SpreadsheetSemanticProperty::BehaviorKind),
        ("ID", SpreadsheetSemanticProperty::ExternalId),
        ("Name", SpreadsheetSemanticProperty::Name),
        ("Context", SpreadsheetSemanticProperty::Context),
        ("Interaction", SpreadsheetSemanticProperty::Interaction),
        ("Lifeline", SpreadsheetSemanticProperty::Lifeline),
        (
            "Represented Path",
            SpreadsheetSemanticProperty::RepresentedPath,
        ),
        ("Order", SpreadsheetSemanticProperty::Order),
        ("Message Sort", SpreadsheetSemanticProperty::MessageSort),
        ("Send", SpreadsheetSemanticProperty::SendOccurrence),
        ("Receive", SpreadsheetSemanticProperty::ReceiveOccurrence),
        ("Signature", SpreadsheetSemanticProperty::Signature),
        ("Arguments", SpreadsheetSemanticProperty::Arguments),
        ("Start", SpreadsheetSemanticProperty::StartOccurrence),
        ("Finish", SpreadsheetSemanticProperty::FinishOccurrence),
        ("Behavior", SpreadsheetSemanticProperty::Behavior),
        (
            "Combined Fragment",
            SpreadsheetSemanticProperty::CombinedFragment,
        ),
        ("Operator", SpreadsheetSemanticProperty::Operator),
        (
            "Covered Lifelines",
            SpreadsheetSemanticProperty::CoveredLifelines,
        ),
        ("Guard", SpreadsheetSemanticProperty::Guard),
        ("Start Order", SpreadsheetSemanticProperty::StartOrder),
        ("End Order", SpreadsheetSemanticProperty::EndOrder),
        ("Constraint", SpreadsheetSemanticProperty::Constraint),
        ("Target", SpreadsheetSemanticProperty::Target),
        (
            "Constraint Expression",
            SpreadsheetSemanticProperty::ConstraintExpression,
        ),
        ("Unit Symbol", SpreadsheetSemanticProperty::UnitSymbol),
        (
            "Unit Scale To Base",
            SpreadsheetSemanticProperty::UnitScaleToBase,
        ),
        (
            "Binding Source Role",
            SpreadsheetSemanticProperty::BindingSourceRole,
        ),
        (
            "Binding Source Parameter",
            SpreadsheetSemanticProperty::BindingSourceParameter,
        ),
        (
            "Binding Target Role",
            SpreadsheetSemanticProperty::BindingTargetRole,
        ),
        (
            "Binding Target Parameter",
            SpreadsheetSemanticProperty::BindingTargetParameter,
        ),
    ];
    SpreadsheetImportMap {
        name: "PR49 semantics".into(),
        source,
        worksheet: None,
        header_row: 1,
        element_kind: ElementKind::Package,
        relationship_kind: None,
        relationship_identity: SpreadsheetRelationshipIdentityPolicy::ExternalId,
        target_scope: root,
        identification_property: SpreadsheetIdentificationProperty::ExternalId,
        search_scope: SpreadsheetSearchScope::TargetRecursive,
        source_namespace: NS.into(),
        mapping_version: "1".into(),
        column_mappings: columns
            .into_iter()
            .map(|(source_column, property)| SpreadsheetColumnMapping {
                source_column: source_column.into(),
                property,
            })
            .collect(),
    }
}

#[test]
fn pr49_csv_constructs_sequence_and_parametric_native_semantics_and_reimports_idempotently() {
    let (
        state,
        activity,
        system,
        _part,
        operation,
        value,
        constraint,
        parameter,
        constraint_property,
    ) = fixture();
    let root = state.project.lock().unwrap().as_ref().unwrap().root_id;
    let source = temp_csv(concat!(
        "Kind,ID,Name,Context,Interaction,Lifeline,Represented Path,Order,Message Sort,Send,Receive,Signature,Arguments,Start,Finish,Behavior,Combined Fragment,Operator,Covered Lifelines,Guard,Start Order,End Order,Constraint,Target,Constraint Expression,Unit Symbol,Unit Scale To Base,Binding Source Role,Binding Source Parameter,Binding Target Role,Binding Target Parameter\n",
        "Interaction,INT,ControlExchange,System,,,,,,,,,,,,,,,,,,,,,,,,,,,,\n",
        "Lifeline,LL,controller,,INT,,controller,,,,,,,,,,,,,,,,,,,,,,,,,\n",
        "Occurrence,SEND,,,INT,LL,,1,,,,,,,,,,,,,,,,,,,,,,,,\n",
        "Occurrence,RECV,,,INT,LL,,2,,,,,,,,,,,,,,,,,,,,,,,,\n",
        "Message,MSG,initialize,,INT,,, ,SynchCall,SEND,RECV,initialize,mode=normal,,,,,,,,,,,,,,,,,,,\n",
        "ExecutionSpecification,EXEC,,,INT,LL,,,,,,,,SEND,RECV,initialize,,,,,,,,,,,,,,,\n",
        "CombinedFragment,FRAG,,,INT,,,,,,,,,,,,,opt,LL,,,,,,,,,,,,\n",
        "InteractionOperand,OPERAND,,,,,,,,,,,,,,,FRAG,,,enabled,1,2,,,,,,,,,\n",
        "StateInvariant,INV,,,INT,LL,,2,,,,,,,,,,,,,,,ready,,,,,,,,\n",
        "ParametricElement,META,,,,,,,,,,,,,,,,,,,,,,MassConstraint,m = 1,kg,1,,,,\n",
        "BindingConnector,BIND,massBinding,System,,,,,,,,,,,,,,,,,,,,,,,,mass,,massConstraint,m\n"
    ));
    let group = SpreadsheetImportMapGroup {
        mappings: vec![semantic_map(source, root)],
    };

    let preview = preview_spreadsheet_import_group_with_activity(&group, &state, &activity);
    assert!(preview.is_valid(), "{:?}", preview.diagnostics);
    assert!(state.behavior.lock().unwrap().interactions.is_empty());
    assert!(
        state
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .relationships
            .is_empty()
    );

    apply_spreadsheet_import_group_with_activity(&group, &state, &activity).unwrap();

    let behavior = state.behavior.lock().unwrap();
    let interaction = behavior
        .interactions
        .values()
        .find(|record| record.external_id == "catia:pr49::INT")
        .unwrap();
    assert_eq!(interaction.context_id, system);
    assert_eq!(interaction.messages[0].sort, MessageSort::SynchCall);
    assert!(matches!(
        interaction.messages[0].signature,
        Some(systems_modeler_core::behavior::MessageSignature::Operation(id)) if id == operation
    ));
    assert!(matches!(
        behavior.external_ids.get("catia:pr49::MSG"),
        Some(BehaviorSemanticId::Message(_))
    ));
    drop(behavior);

    {
        let project_guard = state.project.lock().unwrap();
        let project = project_guard.as_ref().unwrap();
        assert_eq!(
            project.element(constraint).unwrap().constraint_expression,
            "m = 1"
        );
        assert_eq!(
            project.element(constraint).unwrap().unit_symbol.as_deref(),
            Some("kg")
        );
        assert_eq!(project.element(constraint).unwrap().unit_scale_to_base, 1.0);
        let binding = project
            .relationships
            .values()
            .find(|record| record.external_id == "catia:pr49::BIND")
            .unwrap();
        let endpoints = binding.binding.as_ref().unwrap();
        assert_eq!(endpoints.source.role_id, value);
        assert_eq!(endpoints.source.parameter_id, None);
        assert_eq!(endpoints.target.role_id, constraint_property);
        assert_eq!(endpoints.target.parameter_id, Some(parameter));
    }

    let second = preview_spreadsheet_import_group_with_activity(&group, &state, &activity);
    assert!(second.is_valid(), "{:?}", second.diagnostics);
    assert!(
        second
            .rows
            .iter()
            .all(|row| row.action == SpreadsheetRowAction::NoChange)
    );
}

#[test]
fn pr49_rejects_non_finite_unit_scale_without_mutating_project() {
    let (state, activity, _system, _part, _operation, _value, constraint, ..) = fixture();
    let root = state.project.lock().unwrap().as_ref().unwrap().root_id;
    let source = temp_csv(concat!(
        "Kind,ID,Name,Context,Interaction,Lifeline,Represented Path,Order,Message Sort,Send,Receive,Signature,Arguments,Start,Finish,Behavior,Combined Fragment,Operator,Covered Lifelines,Guard,Start Order,End Order,Constraint,Target,Constraint Expression,Unit Symbol,Unit Scale To Base,Binding Source Role,Binding Source Parameter,Binding Target Role,Binding Target Parameter\n",
        "ParametricElement,META,,,,,,,,,,,,,,,,,,,,,,MassConstraint,,kg,NaN,,,,\n"
    ));
    let group = SpreadsheetImportMapGroup {
        mappings: vec![semantic_map(source, root)],
    };

    let preview = preview_spreadsheet_import_group_with_activity(&group, &state, &activity);
    assert!(!preview.is_valid());
    assert!(
        preview
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "PR49_NUMBER_INVALID")
    );
    assert!(apply_spreadsheet_import_group_with_activity(&group, &state, &activity).is_err());
    let project = state.project.lock().unwrap();
    assert_eq!(
        project
            .as_ref()
            .unwrap()
            .element(constraint)
            .unwrap()
            .unit_symbol,
        None
    );
}
