use super::*;
use std::fs;
use std::path::PathBuf;

const NS: &str = "catia:pr44-fixture";

fn workspace(name: &str) -> (WorkspaceState, ElementId) {
    let state = WorkspaceState::default();
    let project = Project::new(name);
    let root = project.root_id;
    *state.project.lock().unwrap() = Some(project);
    (state, root)
}

fn temp_csv(contents: &str) -> String {
    let path = std::env::temp_dir().join(format!("pr44-{}.csv", uuid::Uuid::new_v4()));
    fs::write(&path, contents).unwrap();
    path.to_string_lossy().into_owned()
}

fn fixture_path() -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/pr44_connectors.xlsx")
        .to_string_lossy()
        .into_owned()
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

fn connector_map(
    name: &str,
    source: String,
    target: ElementId,
) -> SpreadsheetImportMap {
    map(
        name,
        source,
        ElementKind::Block,
        Some(RelationshipKind::Connector),
        target,
        &[
            ("Connection Identifier", SpreadsheetSemanticProperty::ExternalId),
            ("Owning Block", SpreadsheetSemanticProperty::ConnectorContext),
            ("Connection Type", SpreadsheetSemanticProperty::ConnectorKind),
            ("From Endpoint", SpreadsheetSemanticProperty::Source),
            ("To Endpoint", SpreadsheetSemanticProperty::Target),
            ("Name", SpreadsheetSemanticProperty::Name),
            ("Description", SpreadsheetSemanticProperty::Documentation),
            ("Visibility", SpreadsheetSemanticProperty::Visibility),
        ],
    )
}

fn plan_local_group(root: ElementId, connector_source: String) -> SpreadsheetImportMapGroup {
    let interfaces = element_map(
        "Interface Definitions",
        temp_csv("External ID,Name\nIF-CMD,CommandInterface\n"),
        ElementKind::InterfaceBlock,
        root,
        false,
    );
    let blocks = element_map(
        "Components",
        temp_csv(
            "External ID,Name\nBLK-VEH,Vehicle\nBLK-CTRL,Controller\nBLK-PWR,PowerUnit\nBLK-SUB,Subsystem\n",
        ),
        ElementKind::Block,
        root,
        false,
    );
    let parts = element_map(
        "Structural Roles",
        temp_csv(
            "External ID,Name,Owner,Type\nPART-CTRL,controller,BLK-VEH,BLK-CTRL\nPART-PEER,controllerPeer,BLK-VEH,BLK-CTRL\nPART-PWR,powerUnit,BLK-VEH,BLK-PWR\nPART-SUB,subsystem,BLK-VEH,BLK-SUB\nPART-NEST,nestedController,BLK-SUB,BLK-CTRL\n",
        ),
        ElementKind::PartProperty,
        root,
        true,
    );
    let ports = element_map(
        "Component Ports",
        temp_csv(
            "External ID,Name,Owner,Type\nPORT-BOUNDARY,commandBoundary,BLK-VEH,IF-CMD\nPORT-CTRL,command,BLK-CTRL,IF-CMD\nPORT-PWR,command,BLK-PWR,IF-CMD\n",
        ),
        ElementKind::ProxyPort,
        root,
        true,
    );
    SpreadsheetImportMapGroup {
        mappings: vec![
            interfaces,
            blocks,
            parts,
            ports,
            connector_map("Internal Connections", connector_source, root),
        ],
    }
}

fn connector_by_external<'a>(project: &'a Project, external_id: &str) -> &'a Relationship {
    project
        .relationships
        .values()
        .find(|relationship| relationship.external_id == external_key(NS, external_id))
        .unwrap()
}

#[test]
fn pr44_xlsx_arbitrary_business_sheets_and_headers_build_connector_topology() {
    let (state, root) = workspace("PR44 XLSX");
    let fixture = fixture_path();
    let mut interfaces = map(
        "Interface map",
        fixture.clone(),
        ElementKind::InterfaceBlock,
        None,
        root,
        &[
            ("Interface ID", SpreadsheetSemanticProperty::ExternalId),
            ("Interface Name", SpreadsheetSemanticProperty::Name),
        ],
    );
    interfaces.worksheet = Some("Interface Definitions".into());
    let mut blocks = map(
        "Component map",
        fixture.clone(),
        ElementKind::Block,
        None,
        root,
        &[
            ("Component ID", SpreadsheetSemanticProperty::ExternalId),
            ("Component Name", SpreadsheetSemanticProperty::Name),
        ],
    );
    blocks.worksheet = Some("Components".into());
    let mut parts = map(
        "Role map",
        fixture.clone(),
        ElementKind::PartProperty,
        None,
        root,
        &[
            ("Role ID", SpreadsheetSemanticProperty::ExternalId),
            ("Role Name", SpreadsheetSemanticProperty::Name),
            ("Owning Component", SpreadsheetSemanticProperty::Owner),
            ("Role Type", SpreadsheetSemanticProperty::Type),
        ],
    );
    parts.worksheet = Some("Structural Roles".into());
    let mut ports = map(
        "Port map",
        fixture.clone(),
        ElementKind::ProxyPort,
        None,
        root,
        &[
            ("Port ID", SpreadsheetSemanticProperty::ExternalId),
            ("Port Name", SpreadsheetSemanticProperty::Name),
            ("Owning Component", SpreadsheetSemanticProperty::Owner),
            ("Interface Type", SpreadsheetSemanticProperty::Type),
        ],
    );
    ports.worksheet = Some("Component Ports".into());
    let mut connectors = map(
        "Connector map",
        fixture,
        ElementKind::Block,
        Some(RelationshipKind::Connector),
        root,
        &[
            ("Connection Identifier", SpreadsheetSemanticProperty::ExternalId),
            ("Owning Block", SpreadsheetSemanticProperty::ConnectorContext),
            ("Connection Type", SpreadsheetSemanticProperty::ConnectorKind),
            ("From Endpoint", SpreadsheetSemanticProperty::Source),
            ("To Endpoint", SpreadsheetSemanticProperty::Target),
            ("Description", SpreadsheetSemanticProperty::Documentation),
        ],
    );
    connectors.worksheet = Some("Internal Connections".into());
    let group = SpreadsheetImportMapGroup {
        mappings: vec![interfaces, blocks, parts, ports, connectors],
    };

    let preview = preview_spreadsheet_import_group(&group, &state);
    assert!(preview.is_valid(), "{:?}", preview.diagnostics);
    apply_spreadsheet_import_group(&group, &state).unwrap();
    let guard = state.project.lock().unwrap();
    let project = guard.as_ref().unwrap();
    assert_eq!(
        project
            .relationships
            .values()
            .filter(|relationship| relationship.kind == RelationshipKind::Connector)
            .count(),
        3
    );
    assert_eq!(
        connector_by_external(project, "CONN-XLSX-N")
            .connector
            .as_ref()
            .unwrap()
            .source
            .property_path
            .len(),
        2
    );
    project.validate().unwrap();
}

#[test]
fn pr44_plan_local_csv_builds_native_assembly_delegation_role_and_nested_ends_atomically() {
    let (state, root) = workspace("PR44 plan local");
    let connectors = temp_csv(
        "Connection Identifier,Owning Block,Connection Type,From Endpoint,To Endpoint,Name,Description,Visibility\n\
CONN-A,BLK-VEH,Assembly,\"PART-CTRL\nPORT-CTRL\",\"PART-PWR\nPORT-PWR\",controlPower,Assembly docs,Private\n\
CONN-D,BLK-VEH,Delegation,PORT-BOUNDARY,\"PART-CTRL\r\nPORT-CTRL\",commandDelegation,Delegation docs,Public\n\
CONN-N,BLK-VEH,Assembly,\"PART-SUB\nPART-NEST\nPORT-CTRL\",\"PART-PWR\nPORT-PWR\",nestedCommand,Nested docs,Public\n\
CONN-R,BLK-VEH,Assembly,PART-CTRL,PART-PEER,roleLink,Role docs,Public\n",
    );
    let group = plan_local_group(root, connectors);

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
        assert_eq!((project.elements.len(), project.relationships.len()), before);
    }

    apply_spreadsheet_import_group(&group, &state).unwrap();
    let guard = state.project.lock().unwrap();
    let project = guard.as_ref().unwrap();
    project.validate().unwrap();

    let assembly = connector_by_external(project, "CONN-A").connector.as_ref().unwrap();
    assert_eq!(assembly.kind, ConnectorKind::Assembly);
    assert_eq!(assembly.source.property_path.len(), 1);
    assert!(assembly.source.port_id.is_some());

    let delegation = connector_by_external(project, "CONN-D").connector.as_ref().unwrap();
    assert_eq!(delegation.kind, ConnectorKind::Delegation);
    assert!(delegation.source.property_path.is_empty());
    assert_eq!(delegation.target.property_path.len(), 1);

    let nested = connector_by_external(project, "CONN-N").connector.as_ref().unwrap();
    assert_eq!(nested.source.property_path.len(), 2);
    assert!(nested.source.port_id.is_some());

    let role = connector_by_external(project, "CONN-R").connector.as_ref().unwrap();
    assert!(role.source.port_id.is_none());
    assert!(role.target.port_id.is_none());
    assert!(state.ibd_diagrams.lock().unwrap().is_empty());
}

#[test]
fn pr44_reimport_is_no_change_and_valid_topology_metadata_update_keeps_identity() {
    let (state, root) = workspace("PR44 reimport");
    let initial_source = temp_csv(
        "Connection Identifier,Owning Block,Connection Type,From Endpoint,To Endpoint,Name,Description,Visibility\n\
CONN-1,BLK-VEH,Assembly,\"PART-CTRL\nPORT-CTRL\",\"PART-PWR\nPORT-PWR\",command,Initial docs,Public\n",
    );
    let initial_group = plan_local_group(root, initial_source);
    apply_spreadsheet_import_group(&initial_group, &state).unwrap();
    let id = connector_by_external(state.project.lock().unwrap().as_ref().unwrap(), "CONN-1").id;

    let identical = connector_map(
        "Identical",
        temp_csv(
            "Connection Identifier,Owning Block,Connection Type,From Endpoint,To Endpoint,Name,Description,Visibility\n\
CONN-1,BLK-VEH,Assembly,\"PART-CTRL\nPORT-CTRL\",\"PART-PWR\nPORT-PWR\",command,Initial docs,Public\n",
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

    let updated = connector_map(
        "Updated",
        temp_csv(
            "Connection Identifier,Owning Block,Connection Type,From Endpoint,To Endpoint,Name,Description,Visibility\n\
CONN-1,BLK-VEH,Assembly,\"PART-SUB\nPART-NEST\nPORT-CTRL\",\"PART-PWR\nPORT-PWR\",nested,Updated docs,Private\n",
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
    let project = guard.as_ref().unwrap();
    let relationship = connector_by_external(project, "CONN-1");
    assert_eq!(relationship.id, id);
    assert_eq!(relationship.name, "nested");
    assert_eq!(relationship.documentation, "Updated docs");
    assert_eq!(relationship.visibility, VisibilityKind::Private);
    assert_eq!(relationship.connector.as_ref().unwrap().source.property_path.len(), 2);
}

#[test]
fn pr44_native_semantic_failures_block_the_whole_map_group_without_mutation() {
    let (state, root) = workspace("PR44 rollback");
    let invalid = temp_csv(
        "Connection Identifier,Owning Block,Connection Type,From Endpoint,To Endpoint,Name,Description,Visibility\n\
CONN-GOOD,BLK-VEH,Delegation,PORT-BOUNDARY,\"PART-CTRL\nPORT-CTRL\",good,good,Public\n\
CONN-BAD,BLK-VEH,Assembly,PORT-BOUNDARY,\"PART-PWR\nPORT-PWR\",bad,bad,Public\n",
    );
    let group = plan_local_group(root, invalid);
    let before = {
        let guard = state.project.lock().unwrap();
        let project = guard.as_ref().unwrap();
        (project.elements.len(), project.relationships.len())
    };

    let preview = preview_spreadsheet_import_group(&group, &state);
    assert!(!preview.is_valid());
    assert!(preview.diagnostics.iter().any(|item| {
        item.code == "SEMANTIC_VALIDATION" && item.reason.contains("Assembly")
    }));
    assert!(apply_spreadsheet_import_group(&group, &state).is_err());
    {
        let guard = state.project.lock().unwrap();
        let project = guard.as_ref().unwrap();
        assert_eq!((project.elements.len(), project.relationships.len()), before);
    }
}

#[test]
fn pr44_reports_path_context_and_rejects_wrong_relationship_kind_collision() {
    let (state, root) = workspace("PR44 diagnostics");
    let setup = plan_local_group(
        root,
        temp_csv(
            "Connection Identifier,Owning Block,Connection Type,From Endpoint,To Endpoint,Name,Description,Visibility\n\
CONN-SETUP,BLK-VEH,Assembly,\"PART-CTRL\nPORT-CTRL\",\"PART-PWR\nPORT-PWR\",setup,setup,Public\n",
        ),
    );
    apply_spreadsheet_import_group(&setup, &state).unwrap();

    let unresolved = connector_map(
        "Unresolved path",
        temp_csv(
            "Connection Identifier,Owning Block,Connection Type,From Endpoint,To Endpoint,Name,Description,Visibility\n\
CONN-MISSING,BLK-VEH,Assembly,\"PART-CTRL\nmissingPort\",\"PART-PWR\nPORT-PWR\",missing,missing,Public\n",
        ),
        root,
    );
    let preview = preview_spreadsheet_import_group(
        &SpreadsheetImportMapGroup {
            mappings: vec![unresolved],
        },
        &state,
    );
    assert!(!preview.is_valid());
    let diagnostic = preview
        .diagnostics
        .iter()
        .find(|item| item.code == "CONNECTOR_PATH_UNRESOLVED")
        .unwrap();
    assert_eq!(diagnostic.row, Some(2));
    assert_eq!(diagnostic.source_endpoint.as_deref(), Some("PART-CTRL\nmissingPort"));

    {
        let mut guard = state.project.lock().unwrap();
        let project = guard.as_mut().unwrap();
        let connector = connector_by_external(project, "CONN-SETUP").clone();
        let id = project
            .create_relationship(
                RelationshipKind::Dependency,
                connector.source_id,
                connector.target_id,
                Some(root),
            )
            .unwrap();
        project.relationships.get_mut(&id).unwrap().external_id = external_key(NS, "COLLISION");
    }
    let collision = connector_map(
        "Wrong kind",
        temp_csv(
            "Connection Identifier,Owning Block,Connection Type,From Endpoint,To Endpoint,Name,Description,Visibility\n\
COLLISION,BLK-VEH,Assembly,\"PART-CTRL\nPORT-CTRL\",\"PART-PWR\nPORT-PWR\",collision,collision,Public\n",
        ),
        root,
    );
    let preview = preview_spreadsheet_import_group(
        &SpreadsheetImportMapGroup {
            mappings: vec![collision],
        },
        &state,
    );
    assert!(preview.diagnostics.iter().any(|item| {
        item.code == "RELATIONSHIP_IDENTITY_KIND_MISMATCH"
    }));
}
