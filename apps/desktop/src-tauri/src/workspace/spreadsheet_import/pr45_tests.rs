use super::*;
use std::fs;
use std::path::PathBuf;

const NS: &str = "catia:pr45-fixture";

fn workspace(name: &str) -> (WorkspaceState, ElementId) {
    let state = WorkspaceState::default();
    let project = Project::new(name);
    let root = project.root_id;
    *state.project.lock().unwrap() = Some(project);
    (state, root)
}

fn temp_csv(contents: &str) -> String {
    let path = std::env::temp_dir().join(format!("pr45-{}.csv", uuid::Uuid::new_v4()));
    fs::write(&path, contents).unwrap();
    path.to_string_lossy().into_owned()
}

fn map(
    name: &str,
    source: String,
    kind: ElementKind,
    relationship_kind: Option<RelationshipKind>,
    target: ElementId,
    columns: &[(&str, SpreadsheetSemanticProperty)],
) -> SpreadsheetImportMap {
    SpreadsheetImportMap {
        name: name.into(),
        source,
        worksheet: None,
        header_row: 1,
        element_kind: kind,
        relationship_kind,
        relationship_identity: SpreadsheetRelationshipIdentityPolicy::ExternalId,
        target_scope: target,
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

fn element_map(
    name: &str,
    source: String,
    kind: ElementKind,
    target: ElementId,
    typed: bool,
) -> SpreadsheetImportMap {
    let mut columns = vec![
        ("External ID", SpreadsheetSemanticProperty::ExternalId),
        ("Name", SpreadsheetSemanticProperty::Name),
    ];
    if typed {
        columns.push(("Owner", SpreadsheetSemanticProperty::Owner));
        columns.push(("Type", SpreadsheetSemanticProperty::Type));
    }
    map(name, source, kind, None, target, &columns)
}

fn connector_map(source: String, target: ElementId) -> SpreadsheetImportMap {
    map(
        "Connectors",
        source,
        ElementKind::Block,
        Some(RelationshipKind::Connector),
        target,
        &[
            ("External ID", SpreadsheetSemanticProperty::ExternalId),
            ("Context", SpreadsheetSemanticProperty::ConnectorContext),
            ("Kind", SpreadsheetSemanticProperty::ConnectorKind),
            ("Source", SpreadsheetSemanticProperty::Source),
            ("Target", SpreadsheetSemanticProperty::Target),
            ("Name", SpreadsheetSemanticProperty::Name),
        ],
    )
}

fn item_flow_map(source: String, target: ElementId) -> SpreadsheetImportMap {
    map(
        "Interface Traffic",
        source,
        ElementKind::Block,
        Some(RelationshipKind::ItemFlow),
        target,
        &[
            ("Flow ID", SpreadsheetSemanticProperty::ExternalId),
            ("Connection ID", SpreadsheetSemanticProperty::Connector),
            ("From Endpoint", SpreadsheetSemanticProperty::Source),
            ("To Endpoint", SpreadsheetSemanticProperty::Target),
            ("Payload", SpreadsheetSemanticProperty::ConveyedItems),
            ("Name", SpreadsheetSemanticProperty::Name),
            ("Description", SpreadsheetSemanticProperty::Documentation),
            ("Visibility", SpreadsheetSemanticProperty::Visibility),
        ],
    )
}

fn plan_local_group(root: ElementId, flows: String) -> SpreadsheetImportMapGroup {
    let interface = element_map(
        "Interfaces",
        temp_csv("External ID,Name\nIF-COMMS,CommsInterface\n"),
        ElementKind::InterfaceBlock,
        root,
        false,
    );
    let blocks = element_map(
        "Blocks",
        temp_csv(
            "External ID,Name\nBLK-VEH,Vehicle\nBLK-CTRL,Controller\nBLK-DISP,Display\nBLK-SENSOR,Sensor\n",
        ),
        ElementKind::Block,
        root,
        false,
    );
    let parts = element_map(
        "Parts",
        temp_csv(
            "External ID,Name,Owner,Type\nPART-CTRL,controller,BLK-VEH,BLK-CTRL\nPART-DISP,display,BLK-VEH,BLK-DISP\nPART-SENSOR,sensor,BLK-VEH,BLK-SENSOR\n",
        ),
        ElementKind::PartProperty,
        root,
        true,
    );
    let ports = element_map(
        "Ports",
        temp_csv(
            "External ID,Name,Owner,Type\nPORT-CTRL-CMD,command,BLK-CTRL,IF-COMMS\nPORT-DISP-CMD,command,BLK-DISP,IF-COMMS\nPORT-CTRL-TEL,telemetry,BLK-CTRL,IF-COMMS\nPORT-SENSOR-TEL,telemetry,BLK-SENSOR,IF-COMMS\n",
        ),
        ElementKind::ProxyPort,
        root,
        true,
    );
    let signals = element_map(
        "Signals",
        temp_csv(
            "External ID,Name\nSIG-COMMAND,CommandSignal\nSIG-STATUS,StatusSignal\nSIG-TELEMETRY,TelemetryPacket\n",
        ),
        ElementKind::Signal,
        root,
        false,
    );
    let connectors = connector_map(
        temp_csv(
            "External ID,Context,Kind,Source,Target,Name\nCONN-CMD,BLK-VEH,Assembly,\"PART-CTRL\nPORT-CTRL-CMD\",\"PART-DISP\nPORT-DISP-CMD\",command link\nCONN-TEL,BLK-VEH,Assembly,\"PART-SENSOR\r\nPORT-SENSOR-TEL\",\"PART-CTRL\r\nPORT-CTRL-TEL\",telemetry link\n",
        ),
        root,
    );
    SpreadsheetImportMapGroup {
        mappings: vec![
            interface,
            blocks,
            parts,
            ports,
            signals,
            connectors,
            item_flow_map(flows, root),
        ],
    }
}

fn relationship_by_external<'a>(project: &'a Project, external_id: &str) -> &'a Relationship {
    project
        .relationships
        .values()
        .find(|relationship| relationship.external_id == external_key(NS, external_id))
        .unwrap()
}

#[test]
fn pr45_xlsx_arbitrary_sheets_build_plan_local_connectors_and_item_flows() {
    let (state, root) = workspace("PR45 XLSX");
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/pr45_item_flows.xlsx")
        .to_string_lossy()
        .into_owned();
    let mut interface = element_map(
        "Interfaces",
        fixture.clone(),
        ElementKind::InterfaceBlock,
        root,
        false,
    );
    interface.worksheet = Some("Interface Definitions".into());
    let mut blocks = element_map("Blocks", fixture.clone(), ElementKind::Block, root, false);
    blocks.worksheet = Some("Components".into());
    let mut parts = element_map(
        "Parts",
        fixture.clone(),
        ElementKind::PartProperty,
        root,
        true,
    );
    parts.worksheet = Some("Structural Roles".into());
    let mut ports = element_map("Ports", fixture.clone(), ElementKind::ProxyPort, root, true);
    ports.worksheet = Some("Component Ports".into());
    let mut signals = element_map("Signals", fixture.clone(), ElementKind::Signal, root, false);
    signals.worksheet = Some("Traffic Types".into());
    let mut connectors = connector_map(fixture.clone(), root);
    connectors.worksheet = Some("Internal Connections".into());
    let mut flows = map(
        "Traffic",
        fixture,
        ElementKind::Block,
        Some(RelationshipKind::ItemFlow),
        root,
        &[
            ("Flow ID", SpreadsheetSemanticProperty::ExternalId),
            ("Connection ID", SpreadsheetSemanticProperty::Connector),
            ("From Endpoint", SpreadsheetSemanticProperty::Source),
            ("To Endpoint", SpreadsheetSemanticProperty::Target),
            ("Payload", SpreadsheetSemanticProperty::ConveyedItems),
            ("Description", SpreadsheetSemanticProperty::Documentation),
        ],
    );
    flows.worksheet = Some("Interface Traffic".into());
    let group = SpreadsheetImportMapGroup {
        mappings: vec![interface, blocks, parts, ports, signals, connectors, flows],
    };
    let preview = preview_spreadsheet_import_group(&group, &state);
    assert!(preview.is_valid(), "{:?}", preview.diagnostics);
    apply_spreadsheet_import_group(&group, &state).unwrap();
    let guard = state.project.lock().unwrap();
    let project = guard.as_ref().unwrap();
    assert_eq!(
        relationship_by_external(project, "FLOW-CMD").kind,
        RelationshipKind::ItemFlow
    );
    assert_eq!(
        relationship_by_external(project, "FLOW-TEL")
            .item_flow
            .as_ref()
            .unwrap()
            .conveyed_item_ids
            .len(),
        2
    );
    project.validate().unwrap();
}

#[test]
fn pr45_csv_plan_local_item_flows_preserve_direction_items_and_atomic_preview() {
    let (state, root) = workspace("PR45 CSV");
    let flows = temp_csv(
        "Flow ID,Connection ID,From Endpoint,To Endpoint,Payload,Name,Description,Visibility\n\
FLOW-CMD,CONN-CMD,\"PART-CTRL\nPORT-CTRL-CMD\",\"PART-DISP\nPORT-DISP-CMD\",SIG-COMMAND,command traffic,Command docs,Private\n\
FLOW-TEL,CONN-TEL,\"PART-CTRL\r\nPORT-CTRL-TEL\",\"PART-SENSOR\r\nPORT-SENSOR-TEL\",\"SIG-STATUS\r\nSIG-TELEMETRY\",reverse telemetry,Telemetry docs,Public\n",
    );
    let group = plan_local_group(root, flows);
    let before = {
        let guard = state.project.lock().unwrap();
        let project = guard.as_ref().unwrap();
        (project.elements.len(), project.relationships.len())
    };
    let preview = preview_spreadsheet_import_group(&group, &state);
    assert!(preview.is_valid(), "{:?}", preview.diagnostics);
    {
        let guard = state.project.lock().unwrap();
        let project = guard.as_ref().unwrap();
        assert_eq!(
            (project.elements.len(), project.relationships.len()),
            before
        );
    }

    apply_spreadsheet_import_group(&group, &state).unwrap();
    let guard = state.project.lock().unwrap();
    let project = guard.as_ref().unwrap();
    let command = relationship_by_external(project, "FLOW-CMD");
    assert_eq!(command.kind, RelationshipKind::ItemFlow);
    assert_eq!(command.visibility, VisibilityKind::Private);
    assert_eq!(
        command.item_flow.as_ref().unwrap().conveyed_item_ids.len(),
        1
    );
    let telemetry = relationship_by_external(project, "FLOW-TEL");
    let payload = telemetry.item_flow.as_ref().unwrap();
    assert_eq!(payload.conveyed_item_ids.len(), 2);
    let connector = project
        .relationship(payload.connector_id)
        .unwrap()
        .connector
        .as_ref()
        .unwrap();
    assert_eq!(payload.source, connector.target);
    assert_eq!(payload.target, connector.source);
    assert!(state.ibd_diagrams.lock().unwrap().is_empty());
    project.validate().unwrap();
}

#[test]
fn pr45_reimport_is_no_change_and_conveyed_items_update_same_identity() {
    let (state, root) = workspace("PR45 reimport");
    let initial = temp_csv(
        "Flow ID,Connection ID,From Endpoint,To Endpoint,Payload,Name,Description,Visibility\n\
FLOW-CMD,CONN-CMD,\"PART-CTRL\nPORT-CTRL-CMD\",\"PART-DISP\nPORT-DISP-CMD\",SIG-COMMAND,command traffic,Initial,Public\n",
    );
    apply_spreadsheet_import_group(&plan_local_group(root, initial), &state).unwrap();
    let id =
        relationship_by_external(state.project.lock().unwrap().as_ref().unwrap(), "FLOW-CMD").id;

    let identical = item_flow_map(
        temp_csv(
            "Flow ID,Connection ID,From Endpoint,To Endpoint,Payload,Name,Description,Visibility\n\
FLOW-CMD,CONN-CMD,\"PART-CTRL\nPORT-CTRL-CMD\",\"PART-DISP\nPORT-DISP-CMD\",SIG-COMMAND,command traffic,Initial,Public\n",
        ),
        root,
    );
    let preview = preview_spreadsheet_import_group(
        &SpreadsheetImportMapGroup {
            mappings: vec![identical],
        },
        &state,
    );
    assert!(preview.is_valid(), "{:?}", preview.diagnostics);
    assert_eq!(preview.totals.no_change, 1);

    let updated = item_flow_map(
        temp_csv(
            "Flow ID,Connection ID,From Endpoint,To Endpoint,Payload,Name,Description,Visibility\n\
FLOW-CMD,CONN-CMD,\"PART-CTRL\nPORT-CTRL-CMD\",\"PART-DISP\nPORT-DISP-CMD\",\"SIG-COMMAND\nSIG-STATUS\",command traffic,Updated,Private\n",
        ),
        root,
    );
    let update_group = SpreadsheetImportMapGroup {
        mappings: vec![updated],
    };
    let preview = preview_spreadsheet_import_group(&update_group, &state);
    assert!(preview.is_valid(), "{:?}", preview.diagnostics);
    assert_eq!(preview.totals.update, 1);
    apply_spreadsheet_import_group(&update_group, &state).unwrap();
    let guard = state.project.lock().unwrap();
    let relationship = relationship_by_external(guard.as_ref().unwrap(), "FLOW-CMD");
    assert_eq!(relationship.id, id);
    assert_eq!(relationship.documentation, "Updated");
    assert_eq!(relationship.visibility, VisibilityKind::Private);
    assert_eq!(
        relationship
            .item_flow
            .as_ref()
            .unwrap()
            .conveyed_item_ids
            .len(),
        2
    );
}

#[test]
fn pr45_invalid_late_flow_rolls_back_and_reports_connector_and_item_errors() {
    let (state, root) = workspace("PR45 rollback");
    let flows = temp_csv(
        "Flow ID,Connection ID,From Endpoint,To Endpoint,Payload,Name,Description,Visibility\n\
FLOW-GOOD,CONN-CMD,\"PART-CTRL\nPORT-CTRL-CMD\",\"PART-DISP\nPORT-DISP-CMD\",SIG-COMMAND,good,good,Public\n\
FLOW-BAD,CONN-CMD,\"PART-SENSOR\nPORT-SENSOR-TEL\",\"PART-CTRL\nPORT-CTRL-TEL\",SIG-STATUS,bad,bad,Public\n",
    );
    let group = plan_local_group(root, flows);
    let preview = preview_spreadsheet_import_group(&group, &state);
    assert!(!preview.is_valid());
    assert!(
        preview
            .diagnostics
            .iter()
            .any(|item| item.code == "SEMANTIC_VALIDATION")
    );
    assert!(apply_spreadsheet_import_group(&group, &state).is_err());
    let guard = state.project.lock().unwrap();
    let project = guard.as_ref().unwrap();
    assert!(
        project
            .elements
            .values()
            .all(|element| !element.external_id.starts_with(NS))
    );
    assert!(project.relationships.is_empty());

    drop(guard);
    let unresolved = item_flow_map(
        temp_csv(
            "Flow ID,Connection ID,From Endpoint,To Endpoint,Payload,Name,Description,Visibility\n\
FLOW-X,MISSING,A,B,SIG-COMMAND,x,x,Public\n",
        ),
        root,
    );
    let preview = preview_spreadsheet_import_group(
        &SpreadsheetImportMapGroup {
            mappings: vec![unresolved],
        },
        &state,
    );
    assert!(
        preview
            .diagnostics
            .iter()
            .any(|item| item.code == "CONNECTOR_UNRESOLVED")
    );
}

#[test]
fn pr45_duplicate_or_blank_conveyed_items_are_blocked() {
    let (state, root) = workspace("PR45 conveyed diagnostics");
    let initial = temp_csv(
        "Flow ID,Connection ID,From Endpoint,To Endpoint,Payload,Name,Description,Visibility\n\
FLOW-CMD,CONN-CMD,\"PART-CTRL\nPORT-CTRL-CMD\",\"PART-DISP\nPORT-DISP-CMD\",SIG-COMMAND,command,command,Public\n",
    );
    apply_spreadsheet_import_group(&plan_local_group(root, initial), &state).unwrap();
    for (payload, code) in [
        ("SIG-COMMAND\nSIG-COMMAND", "DUPLICATE_CONVEYED_ITEM"),
        ("   ", "CONVEYED_ITEMS_REQUIRED"),
        ("MISSING-SIGNAL", "CONVEYED_ITEM_UNRESOLVED"),
    ] {
        let source = temp_csv(&format!(
            "Flow ID,Connection ID,From Endpoint,To Endpoint,Payload,Name,Description,Visibility\nFLOW-{code},CONN-CMD,\"PART-CTRL\nPORT-CTRL-CMD\",\"PART-DISP\nPORT-DISP-CMD\",\"{payload}\",x,x,Public\n"
        ));
        let preview = preview_spreadsheet_import_group(
            &SpreadsheetImportMapGroup {
                mappings: vec![item_flow_map(source, root)],
            },
            &state,
        );
        assert!(
            preview.diagnostics.iter().any(|item| item.code == code),
            "{:?}",
            preview.diagnostics
        );
    }
}
