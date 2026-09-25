use super::*;
use std::fs;
use std::path::PathBuf;
use systems_modeler_core::{
    ElementKind, ExecutionConfiguration, ExecutionSession, ModeledOperationRequest, Multiplicity,
    ParameterDirection, RuntimeValue,
};

const NS: &str = "catia:pr46";

fn workspace(name: &str) -> (WorkspaceState, ElementId) {
    let state = WorkspaceState::default();
    let project = Project::new(name);
    let root = project.root_id;
    *state.project.lock().unwrap() = Some(project);
    (state, root)
}

fn fixture_path() -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/pr46_operation_parameter_reception.xlsx")
        .to_string_lossy()
        .into_owned()
}

fn temp_csv(prefix: &str, contents: &str) -> String {
    let path = std::env::temp_dir().join(format!("pr46-{prefix}-{}.csv", uuid::Uuid::new_v4()));
    fs::write(&path, contents).unwrap();
    path.to_string_lossy().into_owned()
}

fn map(
    name: &str,
    source: String,
    worksheet: Option<&str>,
    kind: ElementKind,
    root: ElementId,
    columns: &[(&str, SpreadsheetSemanticProperty)],
) -> SpreadsheetImportMap {
    SpreadsheetImportMap {
        name: name.into(),
        source,
        worksheet: worksheet.map(ToOwned::to_owned),
        header_row: 1,
        element_kind: kind,
        relationship_kind: None,
        relationship_identity: SpreadsheetRelationshipIdentityPolicy::ExternalId,
        target_scope: root,
        identification_property: SpreadsheetIdentificationProperty::ExternalId,
        search_scope: SpreadsheetSearchScope::TargetRecursive,
        source_namespace: NS.into(),
        mapping_version: "1".into(),
        column_mappings: columns
            .iter()
            .map(|(source_column, property)| SpreadsheetColumnMapping {
                source_column: (*source_column).into(),
                property: *property,
            })
            .collect(),
    }
}

fn basic_map(
    name: &str,
    source: String,
    worksheet: Option<&str>,
    kind: ElementKind,
    root: ElementId,
    id: &str,
    name_col: &str,
) -> SpreadsheetImportMap {
    map(
        name,
        source,
        worksheet,
        kind,
        root,
        &[
            (id, SpreadsheetSemanticProperty::ExternalId),
            (name_col, SpreadsheetSemanticProperty::Name),
        ],
    )
}

fn operation_map(source: String, worksheet: Option<&str>, root: ElementId) -> SpreadsheetImportMap {
    map(
        "Operations",
        source,
        worksheet,
        ElementKind::Operation,
        root,
        &[
            ("Service ID", SpreadsheetSemanticProperty::ExternalId),
            ("Component", SpreadsheetSemanticProperty::Owner),
            ("Service Name", SpreadsheetSemanticProperty::Name),
            ("Description", SpreadsheetSemanticProperty::Documentation),
            ("Visibility", SpreadsheetSemanticProperty::Visibility),
        ],
    )
}

fn parameter_map(source: String, worksheet: Option<&str>, root: ElementId) -> SpreadsheetImportMap {
    map(
        "Parameters",
        source,
        worksheet,
        ElementKind::Parameter,
        root,
        &[
            ("Argument ID", SpreadsheetSemanticProperty::ExternalId),
            ("Service", SpreadsheetSemanticProperty::Owner),
            ("Argument Name", SpreadsheetSemanticProperty::Name),
            ("Data Type", SpreadsheetSemanticProperty::Type),
            ("Direction", SpreadsheetSemanticProperty::ParameterDirection),
            ("Multiplicity", SpreadsheetSemanticProperty::Multiplicity),
            ("Default Value", SpreadsheetSemanticProperty::DefaultValue),
            ("Description", SpreadsheetSemanticProperty::Documentation),
            ("Visibility", SpreadsheetSemanticProperty::Visibility),
        ],
    )
}

fn reception_map(source: String, worksheet: Option<&str>, root: ElementId) -> SpreadsheetImportMap {
    map(
        "Receptions",
        source,
        worksheet,
        ElementKind::Reception,
        root,
        &[
            ("Reception ID", SpreadsheetSemanticProperty::ExternalId),
            ("Component", SpreadsheetSemanticProperty::Owner),
            ("Reception Name", SpreadsheetSemanticProperty::Name),
            (
                "Accepted Event",
                SpreadsheetSemanticProperty::AcceptedSignal,
            ),
            ("Description", SpreadsheetSemanticProperty::Documentation),
            ("Visibility", SpreadsheetSemanticProperty::Visibility),
        ],
    )
}

fn xlsx_group(root: ElementId) -> SpreadsheetImportMapGroup {
    let fixture = fixture_path();
    let mut components = basic_map(
        "Components",
        fixture.clone(),
        Some("Components"),
        ElementKind::Block,
        root,
        "Component ID",
        "Component Name",
    );
    components.column_mappings.push(SpreadsheetColumnMapping {
        source_column: "Description".into(),
        property: SpreadsheetSemanticProperty::Documentation,
    });
    let enumerations = basic_map(
        "Enumerations",
        fixture.clone(),
        Some("Enumerations"),
        ElementKind::Enumeration,
        root,
        "Type ID",
        "Type Name",
    );
    let value_types = basic_map(
        "Value Types",
        fixture.clone(),
        Some("Value Types"),
        ElementKind::ValueType,
        root,
        "Type ID",
        "Type Name",
    );
    let primitives = basic_map(
        "Primitive Types",
        fixture.clone(),
        Some("Primitive Types"),
        ElementKind::PrimitiveType,
        root,
        "Type ID",
        "Type Name",
    );
    let signals = basic_map(
        "Signals",
        fixture.clone(),
        Some("Signals"),
        ElementKind::Signal,
        root,
        "Signal ID",
        "Signal Name",
    );
    let mut operations = operation_map(fixture.clone(), Some("Services"), root);
    operations
        .column_mappings
        .iter_mut()
        .find(|mapping| mapping.property == SpreadsheetSemanticProperty::Owner)
        .expect("Operation mapping includes owner")
        .source_column = "Owning Type".into();
    SpreadsheetImportMapGroup {
        mappings: vec![
            components,
            enumerations,
            value_types,
            primitives,
            signals,
            operations,
            parameter_map(fixture.clone(), Some("Arguments"), root),
            reception_map(fixture, Some("Accepted Signals"), root),
        ],
    }
}

fn by_external<'a>(project: &'a Project, id: &str) -> &'a systems_modeler_core::Element {
    let key = external_key(NS, id);
    project
        .elements
        .values()
        .find(|element| element.external_id == key)
        .unwrap_or_else(|| panic!("missing imported element with external ID {key}"))
}

#[test]
fn pr46_xlsx_constructs_plan_local_operations_parameters_and_receptions() {
    let (state, root) = workspace("PR46 XLSX");
    let group = xlsx_group(root);
    let before = state
        .project
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .elements
        .len();
    let preview = preview_spreadsheet_import_group(&group, &state);
    assert!(preview.is_valid(), "{:?}", preview.diagnostics);
    assert_eq!(
        state
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .elements
            .len(),
        before
    );
    apply_spreadsheet_import_group(&group, &state).unwrap();

    let guard = state.project.lock().unwrap();
    let project = guard.as_ref().unwrap();
    let controller = by_external(project, "BLK-CONTROLLER");
    let start = by_external(project, "OP-START");
    assert_eq!(start.kind, ElementKind::Operation);
    assert_eq!(start.owner_id, Some(controller.id));
    let mode = by_external(project, "PARAM-MODE");
    assert_eq!(mode.owner_id, Some(start.id));
    assert_eq!(mode.parameter_direction, Some(ParameterDirection::In));
    assert_eq!(mode.multiplicity, Some(Multiplicity::ONE));
    assert_eq!(
        project.element(mode.type_id.unwrap()).unwrap().external_id,
        external_key(NS, "TYPE-MODE")
    );
    let result = by_external(project, "PARAM-RESULT");
    assert_eq!(result.parameter_direction, Some(ParameterDirection::Return));
    assert_eq!(result.default_value.as_deref(), Some("0"));
    let reception = by_external(project, "RECP-START");
    assert_eq!(reception.kind, ElementKind::Reception);
    assert_eq!(reception.owner_id, Some(controller.id));
    assert_eq!(
        project
            .element(reception.type_id.unwrap())
            .unwrap()
            .external_id,
        external_key(NS, "SIG-START")
    );
    project.validate().unwrap();
}

#[test]
fn pr46_csv_supports_directions_qualified_type_and_signal_resolution() {
    let (state, root) = workspace("PR46 CSV");
    let blocks = temp_csv("blocks", "ID,Name\nBLK-C,Controller\n");
    let types = temp_csv("types", "ID,Name\nTYPE-I,Integer\nTYPE-B,Boolean\n");
    let signals = temp_csv("signals", "ID,Name\nSIG-S,StartSignal\n");
    let operations = temp_csv(
        "operations",
        "Service ID,Component,Service Name,Description,Visibility\nOP-X,BLK-C,run,,Public\n",
    );
    let parameters = temp_csv(
        "parameters",
        "Argument ID,Service,Argument Name,Data Type,Direction,Multiplicity,Default Value,Description,Visibility\nP-IN,OP-X,a,PR46 CSV::Integer,in,1,,,Public\nP-OUT,OP-X,b,TYPE-I,out,0..1,2,,Public\nP-INOUT,OP-X,c,TYPE-I,inout,1..*,3,,Public\nP-RETURN,OP-X,d,TYPE-B,return,2..4,true,,Private\n",
    );
    let receptions = temp_csv(
        "receptions",
        "Reception ID,Component,Reception Name,Accepted Event,Description,Visibility\nR-X,BLK-C,onStart,PR46 CSV::StartSignal,,Public\n",
    );
    let group = SpreadsheetImportMapGroup {
        mappings: vec![
            basic_map(
                "Blocks",
                blocks,
                None,
                ElementKind::Block,
                root,
                "ID",
                "Name",
            ),
            basic_map(
                "Types",
                types,
                None,
                ElementKind::PrimitiveType,
                root,
                "ID",
                "Name",
            ),
            basic_map(
                "Signals",
                signals,
                None,
                ElementKind::Signal,
                root,
                "ID",
                "Name",
            ),
            operation_map(operations, None, root),
            parameter_map(parameters, None, root),
            reception_map(receptions, None, root),
        ],
    };
    apply_spreadsheet_import_group(&group, &state).unwrap();
    let guard = state.project.lock().unwrap();
    let project = guard.as_ref().unwrap();
    assert_eq!(
        by_external(project, "P-IN").parameter_direction,
        Some(ParameterDirection::In)
    );
    assert_eq!(
        by_external(project, "P-OUT").parameter_direction,
        Some(ParameterDirection::Out)
    );
    assert_eq!(
        by_external(project, "P-INOUT").parameter_direction,
        Some(ParameterDirection::InOut)
    );
    assert_eq!(
        by_external(project, "P-RETURN").parameter_direction,
        Some(ParameterDirection::Return)
    );
    assert_eq!(by_external(project, "P-OUT").multiplicity.unwrap().lower, 0);
    assert_eq!(
        by_external(project, "P-INOUT").multiplicity.unwrap().upper,
        None
    );
    assert_eq!(
        by_external(project, "P-RETURN").multiplicity.unwrap().upper,
        Some(4)
    );
    assert_eq!(
        project
            .element(by_external(project, "R-X").type_id.unwrap())
            .unwrap()
            .name,
        "StartSignal"
    );
}

#[test]
fn pr46_reimport_no_change_and_stable_updates() {
    let (state, root) = workspace("PR46 Reimport");
    apply_spreadsheet_import_group(&xlsx_group(root), &state).unwrap();
    let (param_id, reception_id) = {
        let guard = state.project.lock().unwrap();
        let project = guard.as_ref().unwrap();
        (
            by_external(project, "PARAM-RESULT").id,
            by_external(project, "RECP-START").id,
        )
    };
    let second = preview_spreadsheet_import_group(&xlsx_group(root), &state);
    assert!(second.is_valid(), "{:?}", second.diagnostics);
    assert_eq!(second.totals.create, 0);
    assert_eq!(second.totals.update, 0);

    let new_type = temp_csv("newtype", "ID,Name\nTYPE-NEW,NewPower\n");
    let new_signal = temp_csv("newsignal", "ID,Name\nSIG-RESTART,RestartSignal\n");
    let param = temp_csv(
        "param-update",
        "Argument ID,Service,Argument Name,Data Type,Direction,Multiplicity,Default Value,Description,Visibility\nPARAM-RESULT,OP-CALC,result,TYPE-NEW,out,0..1,7,Updated,Private\n",
    );
    let reception = temp_csv(
        "rec-update",
        "Reception ID,Component,Reception Name,Accepted Event,Description,Visibility\nRECP-START,BLK-CONTROLLER,startRequest,SIG-RESTART,Updated reception,Private\n",
    );
    let update = SpreadsheetImportMapGroup {
        mappings: vec![
            basic_map(
                "New Type",
                new_type,
                None,
                ElementKind::ValueType,
                root,
                "ID",
                "Name",
            ),
            basic_map(
                "New Signal",
                new_signal,
                None,
                ElementKind::Signal,
                root,
                "ID",
                "Name",
            ),
            parameter_map(param, None, root),
            reception_map(reception, None, root),
        ],
    };
    let preview = preview_spreadsheet_import_group(&update, &state);
    assert!(preview.is_valid(), "{:?}", preview.diagnostics);
    assert_eq!(preview.totals.update, 2);
    apply_spreadsheet_import_group(&update, &state).unwrap();
    let guard = state.project.lock().unwrap();
    let project = guard.as_ref().unwrap();
    let p = by_external(project, "PARAM-RESULT");
    assert_eq!(p.id, param_id);
    assert_eq!(p.parameter_direction, Some(ParameterDirection::Out));
    assert_eq!(p.multiplicity.unwrap().lower, 0);
    assert_eq!(p.default_value.as_deref(), Some("7"));
    assert_eq!(
        project.element(p.type_id.unwrap()).unwrap().name,
        "NewPower"
    );
    assert_eq!(p.visibility, VisibilityKind::Private);
    let r = by_external(project, "RECP-START");
    assert_eq!(r.id, reception_id);
    assert_eq!(
        project.element(r.type_id.unwrap()).unwrap().name,
        "RestartSignal"
    );
}

#[test]
fn pr46_invalid_rows_are_diagnostic_and_map_group_atomic() {
    let (state, root) = workspace("PR46 Invalid");
    let blocks = temp_csv("blocks", "ID,Name\nBLK-C,Controller\nBLK-B,BadOwner\n");
    let types = temp_csv("types", "ID,Name\nTYPE-I,Integer\n");
    let operations = temp_csv(
        "ops",
        "Service ID,Component,Service Name,Description,Visibility\nOP-GOOD,BLK-C,good,,Public\n",
    );
    let parameters = temp_csv(
        "bad-param",
        "Argument ID,Service,Argument Name,Data Type,Direction,Multiplicity,Default Value,Description,Visibility\nP-BAD,BLK-B,x,TYPE-I,sideways,1,,,Public\n",
    );
    let receptions = temp_csv(
        "bad-rec",
        "Reception ID,Component,Reception Name,Accepted Event,Description,Visibility\nR-BAD,BLK-C,bad,TYPE-I,,Public\n",
    );
    let group = SpreadsheetImportMapGroup {
        mappings: vec![
            basic_map(
                "Blocks",
                blocks,
                None,
                ElementKind::Block,
                root,
                "ID",
                "Name",
            ),
            basic_map(
                "Types",
                types,
                None,
                ElementKind::PrimitiveType,
                root,
                "ID",
                "Name",
            ),
            operation_map(operations, None, root),
            parameter_map(parameters, None, root),
            reception_map(receptions, None, root),
        ],
    };
    let preview = preview_spreadsheet_import_group(&group, &state);
    assert!(!preview.is_valid());
    assert!(
        preview
            .diagnostics
            .iter()
            .any(|d| d.code == "INVALID_OWNERSHIP")
    );
    assert!(
        preview
            .diagnostics
            .iter()
            .any(|d| d.code == "ACCEPTED_SIGNAL_KIND_INVALID")
    );
    assert!(apply_spreadsheet_import_group(&group, &state).is_err());
    assert_eq!(
        state
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .elements
            .len(),
        1
    );
}

#[test]
fn pr46_invalid_direction_unresolved_ambiguous_signal_and_wrong_kind_block() {
    let (state, root) = workspace("PR46 Diagnostics");
    {
        let mut guard = state.project.lock().unwrap();
        let project = guard.as_mut().unwrap();
        let controller = project
            .create_element(ElementKind::Block, "Controller", root)
            .unwrap();
        project
            .set_external_id(controller, external_key(NS, "BLK-C"))
            .unwrap();
        let op = project
            .create_element(ElementKind::Operation, "run", controller)
            .unwrap();
        project
            .set_external_id(op, external_key(NS, "OP-X"))
            .unwrap();
        let integer = project
            .create_element(ElementKind::PrimitiveType, "Integer", root)
            .unwrap();
        project
            .set_external_id(integer, external_key(NS, "TYPE-I"))
            .unwrap();
        for _ in 0..2 {
            project
                .create_element(ElementKind::Signal, "DuplicateSignal", root)
                .unwrap();
        }
    }
    let bad_direction = temp_csv(
        "direction",
        "Argument ID,Service,Argument Name,Data Type,Direction,Multiplicity,Default Value,Description,Visibility\nP-X,OP-X,x,TYPE-I,sideways,1,,,Public\n",
    );
    let preview = preview_spreadsheet_import_group(
        &SpreadsheetImportMapGroup {
            mappings: vec![parameter_map(bad_direction, None, root)],
        },
        &state,
    );
    assert!(
        preview
            .diagnostics
            .iter()
            .any(|d| d.code == "PARAMETER_DIRECTION_INVALID")
    );

    for (label, signal, expected) in [
        ("unresolved", "MissingSignal", "ACCEPTED_SIGNAL_UNRESOLVED"),
        ("ambiguous", "DuplicateSignal", "ACCEPTED_SIGNAL_AMBIGUOUS"),
    ] {
        let source = temp_csv(
            label,
            &format!(
                "Reception ID,Component,Reception Name,Accepted Event,Description,Visibility\nR-{label},BLK-C,r,{signal},,Public\n"
            ),
        );
        let preview = preview_spreadsheet_import_group(
            &SpreadsheetImportMapGroup {
                mappings: vec![reception_map(source, None, root)],
            },
            &state,
        );
        assert!(
            preview.diagnostics.iter().any(|d| d.code == expected),
            "{:?}",
            preview.diagnostics
        );
    }

    {
        let mut guard = state.project.lock().unwrap();
        let project = guard.as_mut().unwrap();
        let collision = project
            .create_element(ElementKind::Block, "Collision", root)
            .unwrap();
        project
            .set_external_id(collision, external_key(NS, "OP-COLLIDE"))
            .unwrap();
    }
    let source = temp_csv(
        "collision",
        "Service ID,Component,Service Name,Description,Visibility\nOP-COLLIDE,BLK-C,start,,Public\n",
    );
    let preview = preview_spreadsheet_import_group(
        &SpreadsheetImportMapGroup {
            mappings: vec![operation_map(source, None, root)],
        },
        &state,
    );
    assert!(
        preview
            .diagnostics
            .iter()
            .any(|d| d.code == "IDENTIFICATION_KIND_MISMATCH")
    );
}

#[test]
fn pr46_name_fallback_is_owner_aware() {
    let (state, root) = workspace("PR46 Fallback");
    let (left_op, right_op) = {
        let mut guard = state.project.lock().unwrap();
        let project = guard.as_mut().unwrap();
        let left = project
            .create_element(ElementKind::Block, "Left", root)
            .unwrap();
        let right = project
            .create_element(ElementKind::Block, "Right", root)
            .unwrap();
        (
            project
                .create_element(ElementKind::Operation, "run", left)
                .unwrap(),
            project
                .create_element(ElementKind::Operation, "run", right)
                .unwrap(),
        )
    };
    let source = temp_csv(
        "fallback",
        "Component,Service Name,Description,Visibility\nLeft,run,Updated,Private\n",
    );
    let mut mapping = map(
        "Operation fallback",
        source,
        None,
        ElementKind::Operation,
        root,
        &[
            ("Component", SpreadsheetSemanticProperty::Owner),
            ("Service Name", SpreadsheetSemanticProperty::Name),
            ("Description", SpreadsheetSemanticProperty::Documentation),
            ("Visibility", SpreadsheetSemanticProperty::Visibility),
        ],
    );
    mapping.identification_property = SpreadsheetIdentificationProperty::Name;
    let group = SpreadsheetImportMapGroup {
        mappings: vec![mapping],
    };
    let preview = preview_spreadsheet_import_group(&group, &state);
    assert!(preview.is_valid(), "{:?}", preview.diagnostics);
    assert_eq!(preview.totals.update, 1);
    apply_spreadsheet_import_group(&group, &state).unwrap();
    let guard = state.project.lock().unwrap();
    let project = guard.as_ref().unwrap();
    assert_eq!(project.element(left_op).unwrap().documentation, "Updated");
    assert_eq!(project.element(right_op).unwrap().documentation, "");
}

#[test]
fn pr46_imported_semantics_are_usable_by_existing_pr34_operation_runtime() {
    let (state, root) = workspace("PR46 Runtime");
    let blocks = temp_csv(
        "runtime-blocks",
        "ID,Name\nBLK-C,Controller\nBLK-S,System\n",
    );
    let types = temp_csv("runtime-types", "ID,Name\nTYPE-I,Integer\n");
    let signals = temp_csv("runtime-signals", "ID,Name\nSIG-S,StartSignal\n");
    let operations = temp_csv(
        "runtime-ops",
        "Service ID,Component,Service Name,Description,Visibility\nOP-RUN,BLK-C,run,,Public\n",
    );
    let parameters = temp_csv(
        "runtime-params",
        "Argument ID,Service,Argument Name,Data Type,Direction,Multiplicity,Default Value,Description,Visibility\nP-IN,OP-RUN,input,TYPE-I,in,1,,,Public\nP-RET,OP-RUN,result,TYPE-I,return,1,7,,Public\n",
    );
    let receptions = temp_csv(
        "runtime-recs",
        "Reception ID,Component,Reception Name,Accepted Event,Description,Visibility\nR-S,BLK-C,onStart,SIG-S,,Public\n",
    );
    let group = SpreadsheetImportMapGroup {
        mappings: vec![
            basic_map(
                "Blocks",
                blocks,
                None,
                ElementKind::Block,
                root,
                "ID",
                "Name",
            ),
            basic_map(
                "Types",
                types,
                None,
                ElementKind::PrimitiveType,
                root,
                "ID",
                "Name",
            ),
            basic_map(
                "Signals",
                signals,
                None,
                ElementKind::Signal,
                root,
                "ID",
                "Name",
            ),
            operation_map(operations, None, root),
            parameter_map(parameters, None, root),
            reception_map(receptions, None, root),
        ],
    };
    apply_spreadsheet_import_group(&group, &state).unwrap();

    let mut guard = state.project.lock().unwrap();
    let project = guard.as_mut().unwrap();
    let controller = by_external(project, "BLK-C").id;
    let system = by_external(project, "BLK-S").id;
    let operation = by_external(project, "OP-RUN").id;
    let reception = by_external(project, "R-S").id;
    assert_eq!(
        project
            .element(project.element(reception).unwrap().type_id.unwrap())
            .unwrap()
            .kind,
        ElementKind::Signal
    );
    let part = project
        .create_typed_feature(
            ElementKind::PartProperty,
            "controller",
            system,
            controller,
            Multiplicity::ONE,
        )
        .unwrap();
    let mut session = ExecutionSession::with_configuration(
        project,
        ExecutionConfiguration {
            root_semantic_id: system,
            random_seed: 0,
            max_steps: 100,
            max_queued_events: 100,
        },
    )
    .unwrap();
    session.initialize(project).unwrap();
    let instance = session
        .structural_runtime
        .as_ref()
        .unwrap()
        .instances_for_usage(part)[0]
        .id;
    let result = systems_modeler_core::invoke_modeled_operation(
        project,
        &mut session,
        &ModeledOperationRequest {
            operation_id: operation,
            target_runtime_instance_id: instance,
            arguments: vec![("input".into(), RuntimeValue::Integer(2))],
        },
    )
    .unwrap();
    assert_eq!(
        result.outputs,
        vec![("result".into(), RuntimeValue::Integer(7))]
    );
}
