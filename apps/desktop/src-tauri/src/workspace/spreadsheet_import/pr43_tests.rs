use super::*;
use std::fs;
use std::path::PathBuf;

const NS: &str = "catia:pr43-fixture";

fn workspace(name: &str) -> (WorkspaceState, ElementId) {
    let state = WorkspaceState::default();
    let project = Project::new(name);
    let root = project.root_id;
    *state.project.lock().unwrap() = Some(project);
    (state, root)
}

fn fixture_path() -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/pr43_ports.xlsx")
        .to_string_lossy()
        .into_owned()
}

fn temp_csv(contents: &str) -> String {
    let path = std::env::temp_dir().join(format!("pr43-{}.csv", uuid::Uuid::new_v4()));
    fs::write(&path, contents).unwrap();
    path.to_string_lossy().into_owned()
}

fn map(
    name: &str,
    source: String,
    worksheet: Option<&str>,
    kind: ElementKind,
    target: ElementId,
    identification_property: SpreadsheetIdentificationProperty,
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
        target_scope: target,
        identification_property,
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

fn port_columns() -> Vec<(&'static str, SpreadsheetSemanticProperty)> {
    vec![
        ("Port ID", SpreadsheetSemanticProperty::ExternalId),
        ("Owner", SpreadsheetSemanticProperty::Owner),
        ("Port Name", SpreadsheetSemanticProperty::Name),
        ("Port Type", SpreadsheetSemanticProperty::Type),
        ("Multiplicity", SpreadsheetSemanticProperty::Multiplicity),
        ("Conjugated", SpreadsheetSemanticProperty::Conjugated),
        ("Description", SpreadsheetSemanticProperty::Documentation),
        ("Visibility", SpreadsheetSemanticProperty::Visibility),
    ]
}

fn xlsx_proxy_columns() -> Vec<(&'static str, SpreadsheetSemanticProperty)> {
    vec![
        ("Port Identifier", SpreadsheetSemanticProperty::ExternalId),
        ("Owning Component", SpreadsheetSemanticProperty::Owner),
        ("Interface Name", SpreadsheetSemanticProperty::Name),
        ("Interface Type", SpreadsheetSemanticProperty::Type),
        ("Cardinality", SpreadsheetSemanticProperty::Multiplicity),
        ("Conjugated", SpreadsheetSemanticProperty::Conjugated),
        ("Description", SpreadsheetSemanticProperty::Documentation),
        ("Access", SpreadsheetSemanticProperty::Visibility),
    ]
}

fn xlsx_full_columns() -> Vec<(&'static str, SpreadsheetSemanticProperty)> {
    vec![
        ("Port ID", SpreadsheetSemanticProperty::ExternalId),
        ("Component", SpreadsheetSemanticProperty::Owner),
        ("Port Name", SpreadsheetSemanticProperty::Name),
        ("Port Type", SpreadsheetSemanticProperty::Type),
        ("Multiplicity", SpreadsheetSemanticProperty::Multiplicity),
        ("Conjugated", SpreadsheetSemanticProperty::Conjugated),
        ("Description", SpreadsheetSemanticProperty::Documentation),
        ("Visibility", SpreadsheetSemanticProperty::Visibility),
    ]
}

fn seed(
    project: &mut Project,
    owner: ElementId,
    kind: ElementKind,
    name: &str,
    external_id: &str,
) -> ElementId {
    let id = project.create_element(kind, name, owner).unwrap();
    project
        .set_external_id(id, external_key(NS, external_id))
        .unwrap();
    id
}

fn seed_port(
    project: &mut Project,
    owner: ElementId,
    type_id: ElementId,
    kind: ElementKind,
    name: &str,
    external_id: &str,
) -> ElementId {
    let id = project
        .create_typed_feature(kind, name, owner, type_id, Multiplicity::ONE)
        .unwrap();
    project
        .set_external_id(id, external_key(NS, external_id))
        .unwrap();
    id
}

fn find_ext(project: &Project, external_id: &str) -> ElementId {
    project
        .elements
        .values()
        .find(|element| element.external_id == external_key(NS, external_id))
        .unwrap()
        .id
}

fn one_port_map(
    name: &str,
    source: String,
    kind: ElementKind,
    root: ElementId,
) -> SpreadsheetImportMap {
    map(
        name,
        source,
        None,
        kind,
        root,
        SpreadsheetIdentificationProperty::ExternalId,
        &port_columns(),
    )
}

#[test]
fn pr43_xlsx_plan_local_proxy_and_full_ports_use_one_atomic_model_build_plan() {
    let (state, root) = workspace("PR43 XLSX");
    let fixture = fixture_path();
    let interfaces = map(
        "Interface Definitions",
        fixture.clone(),
        Some("Interface Definitions"),
        ElementKind::InterfaceBlock,
        root,
        SpreadsheetIdentificationProperty::ExternalId,
        &[
            ("Interface ID", SpreadsheetSemanticProperty::ExternalId),
            ("Interface Name", SpreadsheetSemanticProperty::Name),
        ],
    );
    let service_types = map(
        "Service Types",
        fixture.clone(),
        Some("Service Types"),
        ElementKind::DataType,
        root,
        SpreadsheetIdentificationProperty::ExternalId,
        &[
            ("Type ID", SpreadsheetSemanticProperty::ExternalId),
            ("Type Name", SpreadsheetSemanticProperty::Name),
        ],
    );
    let components = map(
        "Components",
        fixture.clone(),
        Some("Components"),
        ElementKind::Block,
        root,
        SpreadsheetIdentificationProperty::ExternalId,
        &[
            ("Component ID", SpreadsheetSemanticProperty::ExternalId),
            ("Component Name", SpreadsheetSemanticProperty::Name),
        ],
    );
    let proxy_ports = map(
        "Component Interfaces",
        fixture.clone(),
        Some("Component Interfaces"),
        ElementKind::ProxyPort,
        root,
        SpreadsheetIdentificationProperty::ExternalId,
        &xlsx_proxy_columns(),
    );
    let full_ports = map(
        "Service Ports",
        fixture,
        Some("Service Ports"),
        ElementKind::FullPort,
        root,
        SpreadsheetIdentificationProperty::ExternalId,
        &xlsx_full_columns(),
    );
    let group = SpreadsheetImportMapGroup {
        mappings: vec![interfaces, service_types, components, proxy_ports, full_ports],
    };

    let before_elements = state.project.lock().unwrap().as_ref().unwrap().elements.len();
    let preview = preview_spreadsheet_import_group(&group, &state);
    assert!(preview.is_valid(), "{:?}", preview.diagnostics);
    assert_eq!(preview.totals.create, 10);
    assert_eq!(
        state.project.lock().unwrap().as_ref().unwrap().elements.len(),
        before_elements,
        "preview must not mutate semantic state"
    );

    let snapshot = state.project.lock().unwrap().as_ref().unwrap().clone();
    let prepared = prepare_spreadsheet_import(&group, &snapshot);
    let creates = prepared
        .plan
        .operations
        .iter()
        .filter_map(|operation| match operation {
            ModelBuildOperation::CreateElement { kind, .. } => Some(kind.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        creates
            .iter()
            .filter(|kind| **kind == ElementKind::ProxyPort)
            .count(),
        3
    );
    assert_eq!(
        creates
            .iter()
            .filter(|kind| **kind == ElementKind::FullPort)
            .count(),
        1
    );
    assert!(prepared.plan.operations.iter().all(|operation| matches!(
        operation,
        ModelBuildOperation::CreateElement { .. } | ModelBuildOperation::UpdateElementFields { .. }
    )));

    apply_spreadsheet_import_group(&group, &state).unwrap();
    let guard = state.project.lock().unwrap();
    let project = guard.as_ref().unwrap();
    let vehicle = find_ext(project, "BLK-VEH");
    let controller = find_ext(project, "BLK-CTRL");
    let power = find_ext(project, "BLK-PWR");
    let cmd_type = find_ext(project, "IF-CMD");
    let tel_type = find_ext(project, "IF-TEL");
    let svc_type = find_ext(project, "DT-SVC");

    let command = project.element(find_ext(project, "PORT-CMD")).unwrap();
    assert_eq!(command.kind, ElementKind::ProxyPort);
    assert_eq!(command.owner_id, Some(vehicle));
    assert_eq!(command.type_id, Some(cmd_type));
    assert_eq!(command.multiplicity, Some(Multiplicity::ONE));
    assert!(!command.is_conjugated);
    assert_eq!(command.documentation, "Vehicle command port");

    let telemetry = project.element(find_ext(project, "PORT-TEL")).unwrap();
    assert_eq!(telemetry.owner_id, Some(controller));
    assert_eq!(telemetry.type_id, Some(tel_type));
    assert_eq!(telemetry.multiplicity, Some(Multiplicity::new(0, Some(1)).unwrap()));
    assert!(!telemetry.is_conjugated);

    let peer = project.element(find_ext(project, "PORT-PEER")).unwrap();
    assert_eq!(peer.owner_id, Some(controller));
    assert_eq!(peer.type_id, Some(cmd_type));
    assert!(peer.is_conjugated);
    assert_eq!(peer.visibility, VisibilityKind::Private);

    let service = project.element(find_ext(project, "PORT-SVC")).unwrap();
    assert_eq!(service.kind, ElementKind::FullPort);
    assert_eq!(service.owner_id, Some(power));
    assert_eq!(service.type_id, Some(svc_type));
    assert_eq!(service.multiplicity, Some(Multiplicity::ONE));
    assert!(!service.is_conjugated);
    assert_eq!(service.visibility, VisibilityKind::Private);
    drop(guard);

    assert!(state.diagrams.lock().unwrap().is_empty());
    assert!(state.ibd_diagrams.lock().unwrap().is_empty());

    let second = preview_spreadsheet_import_group(&group, &state);
    assert!(second.is_valid(), "{:?}", second.diagnostics);
    assert_eq!(second.totals.no_change, 10);
    assert_eq!(second.totals.create, 0);
    assert_eq!(second.totals.update, 0);
}

#[test]
fn pr43_csv_supports_external_and_exact_qualified_owner_and_type_references_for_both_ports() {
    let (state, root) = workspace("PR43 CSV");
    {
        let mut guard = state.project.lock().unwrap();
        let project = guard.as_mut().unwrap();
        let architecture = seed(project, root, ElementKind::Package, "Architecture", "PKG-ARCH");
        seed(project, architecture, ElementKind::Block, "Controller", "BLK-CTRL");
        seed(project, architecture, ElementKind::Block, "PowerUnit", "BLK-PWR");
        seed(
            project,
            architecture,
            ElementKind::InterfaceBlock,
            "CommandInterface",
            "IF-CMD",
        );
        seed(project, architecture, ElementKind::DataType, "ServiceAssembly", "DT-SVC");
    }

    let proxy_source = temp_csv(&format!(
        "Port ID,Owner,Port Name,Port Type,Multiplicity,Conjugated,Description,Visibility\nPORT-C1,BLK-CTRL,command,PR43 CSV::Architecture::CommandInterface,1,true,Command docs,Private\n"
    ));
    let full_source = temp_csv(
        "Port ID,Owner,Port Name,Port Type,Multiplicity,Conjugated,Description,Visibility\nPORT-F1,PR43 CSV::Architecture::PowerUnit,service,DT-SVC,0..*,false,Service docs,Public\n",
    );
    let group = SpreadsheetImportMapGroup {
        mappings: vec![
            one_port_map("CSV Proxy", proxy_source, ElementKind::ProxyPort, root),
            one_port_map("CSV Full", full_source, ElementKind::FullPort, root),
        ],
    };
    let preview = preview_spreadsheet_import_group(&group, &state);
    assert!(preview.is_valid(), "{:?}", preview.diagnostics);
    assert_eq!(preview.totals.create, 2);
    apply_spreadsheet_import_group(&group, &state).unwrap();

    let guard = state.project.lock().unwrap();
    let project = guard.as_ref().unwrap();
    let proxy = project.element(find_ext(project, "PORT-C1")).unwrap();
    assert_eq!(proxy.kind, ElementKind::ProxyPort);
    assert!(proxy.is_conjugated);
    assert_eq!(proxy.visibility, VisibilityKind::Private);
    let full = project.element(find_ext(project, "PORT-F1")).unwrap();
    assert_eq!(full.kind, ElementKind::FullPort);
    assert_eq!(full.multiplicity, Some(Multiplicity::new(0, None).unwrap()));
    assert!(!full.is_conjugated);
}

#[test]
fn pr43_stable_identity_updates_all_supported_port_fields_without_duplication() {
    let (state, root) = workspace("PR43 Updates");
    {
        let mut guard = state.project.lock().unwrap();
        let project = guard.as_mut().unwrap();
        seed(project, root, ElementKind::Block, "Controller", "BLK-1");
        seed(project, root, ElementKind::InterfaceBlock, "Command", "IF-1");
        seed(project, root, ElementKind::InterfaceBlock, "SecureCommand", "IF-2");
        seed(project, root, ElementKind::ValueType, "BadType", "BAD-1");
    }
    let initial_source = temp_csv(
        "Port ID,Owner,Port Name,Port Type,Multiplicity,Conjugated,Description,Visibility\nPORT-1,BLK-1,control,IF-1,1,false,Initial docs,Public\n",
    );
    let initial = one_port_map("Initial", initial_source, ElementKind::ProxyPort, root);
    let initial_group = SpreadsheetImportMapGroup {
        mappings: vec![initial],
    };
    apply_spreadsheet_import_group(&initial_group, &state).unwrap();
    let port_id = {
        let guard = state.project.lock().unwrap();
        find_ext(guard.as_ref().unwrap(), "PORT-1")
    };
    let same = preview_spreadsheet_import_group(&initial_group, &state);
    assert!(same.is_valid());
    assert_eq!(same.totals.no_change, 1);

    let updated_source = temp_csv(
        "Port ID,Owner,Port Name,Port Type,Multiplicity,Conjugated,Description,Visibility\nPORT-1,BLK-1,command,IF-2,0..*,yes,Updated docs,Private\n",
    );
    let updated_group = SpreadsheetImportMapGroup {
        mappings: vec![one_port_map(
            "Update",
            updated_source,
            ElementKind::ProxyPort,
            root,
        )],
    };
    let preview = preview_spreadsheet_import_group(&updated_group, &state);
    assert!(preview.is_valid(), "{:?}", preview.diagnostics);
    assert_eq!(preview.totals.update, 1);
    apply_spreadsheet_import_group(&updated_group, &state).unwrap();
    {
        let guard = state.project.lock().unwrap();
        let project = guard.as_ref().unwrap();
        let port = project.element(port_id).unwrap();
        assert_eq!(find_ext(project, "PORT-1"), port_id);
        assert_eq!(port.name, "command");
        assert_eq!(port.type_id, Some(find_ext(project, "IF-2")));
        assert_eq!(port.multiplicity, Some(Multiplicity::new(0, None).unwrap()));
        assert!(port.is_conjugated);
        assert_eq!(port.documentation, "Updated docs");
        assert_eq!(port.visibility, VisibilityKind::Private);
    }

    let bad_source = temp_csv(
        "Port ID,Owner,Port Name,Port Type,Multiplicity,Conjugated,Description,Visibility\nPORT-1,BLK-1,command,BAD-1,1,false,Should not apply,Public\n",
    );
    let bad_group = SpreadsheetImportMapGroup {
        mappings: vec![one_port_map(
            "Invalid Type Update",
            bad_source,
            ElementKind::ProxyPort,
            root,
        )],
    };
    let before = {
        let guard = state.project.lock().unwrap();
        serde_json::to_string(guard.as_ref().unwrap()).unwrap()
    };
    let blocked = preview_spreadsheet_import_group(&bad_group, &state);
    assert!(!blocked.is_valid());
    assert!(blocked.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "SEMANTIC_VALIDATION"
            && diagnostic.reason.contains("cannot be typed by")
    }));
    assert!(apply_spreadsheet_import_group(&bad_group, &state).is_err());
    let after = {
        let guard = state.project.lock().unwrap();
        serde_json::to_string(guard.as_ref().unwrap()).unwrap()
    };
    assert_eq!(before, after);
}

#[test]
fn pr43_conjugation_vocabulary_is_deterministic_and_full_port_rule_stays_native() {
    let (state, root) = workspace("PR43 Conjugation");
    {
        let mut guard = state.project.lock().unwrap();
        let project = guard.as_mut().unwrap();
        seed(project, root, ElementKind::Block, "Controller", "BLK-1");
        seed(project, root, ElementKind::InterfaceBlock, "Iface", "IF-1");
        seed(project, root, ElementKind::DataType, "Service", "DT-1");
    }
    let source = temp_csv(
        "Port ID,Owner,Port Name,Port Type,Multiplicity,Conjugated,Description,Visibility\nP-T,BLK-1,pTrue,IF-1,1,true,,Public\nP-F,BLK-1,pFalse,IF-1,1,false,,Public\nP-Y,BLK-1,pYes,IF-1,1,yes,,Public\nP-N,BLK-1,pNo,IF-1,1,no,,Public\nP-1,BLK-1,pOne,IF-1,1,1,,Public\nP-0,BLK-1,pZero,IF-1,1,0,,Public\n",
    );
    let group = SpreadsheetImportMapGroup {
        mappings: vec![one_port_map("Boolean Ports", source, ElementKind::ProxyPort, root)],
    };
    let preview = preview_spreadsheet_import_group(&group, &state);
    assert!(preview.is_valid(), "{:?}", preview.diagnostics);
    apply_spreadsheet_import_group(&group, &state).unwrap();
    let guard = state.project.lock().unwrap();
    let project = guard.as_ref().unwrap();
    for id in ["P-T", "P-Y", "P-1"] {
        assert!(project.element(find_ext(project, id)).unwrap().is_conjugated);
    }
    for id in ["P-F", "P-N", "P-0"] {
        assert!(!project.element(find_ext(project, id)).unwrap().is_conjugated);
    }
    drop(guard);

    let invalid = temp_csv(
        "Port ID,Owner,Port Name,Port Type,Multiplicity,Conjugated,Description,Visibility\nP-BAD,BLK-1,bad,IF-1,1,maybe,,Public\n",
    );
    let invalid_preview = preview_spreadsheet_import_group(
        &SpreadsheetImportMapGroup {
            mappings: vec![one_port_map(
                "Invalid Conjugation",
                invalid,
                ElementKind::ProxyPort,
                root,
            )],
        },
        &state,
    );
    assert!(!invalid_preview.is_valid());
    assert!(invalid_preview.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "CONJUGATED_INVALID"
            && diagnostic.semantic_property == Some(SpreadsheetSemanticProperty::Conjugated)
            && diagnostic.column.as_deref() == Some("Conjugated")
    }));

    let full_true = temp_csv(
        "Port ID,Owner,Port Name,Port Type,Multiplicity,Conjugated,Description,Visibility\nF-BAD,BLK-1,service,DT-1,1,true,,Public\n",
    );
    let full_preview = preview_spreadsheet_import_group(
        &SpreadsheetImportMapGroup {
            mappings: vec![one_port_map(
                "FullPort Native Rule",
                full_true,
                ElementKind::FullPort,
                root,
            )],
        },
        &state,
    );
    assert!(!full_preview.is_valid());
    assert!(full_preview.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "SEMANTIC_VALIDATION"
            && diagnostic.reason.contains("FullPort cannot be conjugated")
    }));
}

#[test]
fn pr43_owner_type_and_kind_failures_are_blocked_without_guessing() {
    let (state, root) = workspace("PR43 Failures");
    let (req, value_type, full_collision);
    {
        let mut guard = state.project.lock().unwrap();
        let project = guard.as_mut().unwrap();
        let owner = seed(project, root, ElementKind::Block, "Owner", "BLK-1");
        let iface = seed(project, root, ElementKind::InterfaceBlock, "Iface", "IF-1");
        value_type = seed(project, root, ElementKind::ValueType, "Value", "VT-1");
        req = seed(project, root, ElementKind::Requirement, "RequirementOwner", "REQ-1");
        project.element_mut(req).unwrap().requirement_id = Some("R-1".into());
        project.element_mut(req).unwrap().requirement_text = Some("text".into());
        let wrong = seed(project, root, ElementKind::Block, "WrongKind", "PORT-WRONG");
        assert_eq!(project.element(wrong).unwrap().kind, ElementKind::Block);
        full_collision = seed_port(
            project,
            owner,
            iface,
            ElementKind::FullPort,
            "existingFull",
            "PORT-COLLIDE",
        );
    }

    let preview_for = |csv: String, kind: ElementKind| {
        preview_spreadsheet_import_group(
            &SpreadsheetImportMapGroup {
                mappings: vec![one_port_map("Failure", csv, kind, root)],
            },
            &state,
        )
    };

    let unresolved_owner = preview_for(
        temp_csv("Port ID,Owner,Port Name,Port Type,Multiplicity,Conjugated,Description,Visibility\nP-UO,MISSING,p,IF-1,1,false,,Public\n"),
        ElementKind::ProxyPort,
    );
    assert!(unresolved_owner.diagnostics.iter().any(|d| d.code == "OWNER_UNRESOLVED"));

    let illegal_owner = preview_for(
        temp_csv("Port ID,Owner,Port Name,Port Type,Multiplicity,Conjugated,Description,Visibility\nP-IO,REQ-1,p,IF-1,1,false,,Public\n"),
        ElementKind::ProxyPort,
    );
    assert!(!illegal_owner.is_valid());
    assert!(illegal_owner.diagnostics.iter().any(|d| d.code == "SEMANTIC_VALIDATION"));

    let unresolved_type = preview_for(
        temp_csv("Port ID,Owner,Port Name,Port Type,Multiplicity,Conjugated,Description,Visibility\nP-UT,BLK-1,p,MISSING,1,false,,Public\n"),
        ElementKind::ProxyPort,
    );
    assert!(unresolved_type.diagnostics.iter().any(|d| d.code == "TYPE_UNRESOLVED"));

    let invalid_type = preview_for(
        temp_csv("Port ID,Owner,Port Name,Port Type,Multiplicity,Conjugated,Description,Visibility\nP-IT,BLK-1,p,VT-1,1,false,,Public\n"),
        ElementKind::ProxyPort,
    );
    assert!(!invalid_type.is_valid());
    assert!(invalid_type.diagnostics.iter().any(|d| {
        d.code == "SEMANTIC_VALIDATION" && d.reason.contains("cannot be typed by")
    }));

    let wrong_kind = preview_for(
        temp_csv("Port ID,Owner,Port Name,Port Type,Multiplicity,Conjugated,Description,Visibility\nPORT-WRONG,BLK-1,p,IF-1,1,false,,Public\n"),
        ElementKind::ProxyPort,
    );
    assert!(wrong_kind
        .diagnostics
        .iter()
        .any(|d| d.code == "IDENTIFICATION_KIND_MISMATCH"));

    let port_kind_collision = preview_for(
        temp_csv("Port ID,Owner,Port Name,Port Type,Multiplicity,Conjugated,Description,Visibility\nPORT-COLLIDE,BLK-1,p,IF-1,1,false,,Public\n"),
        ElementKind::ProxyPort,
    );
    assert!(port_kind_collision
        .diagnostics
        .iter()
        .any(|d| d.code == "IDENTIFICATION_KIND_MISMATCH"));
    assert_eq!(
        state.project.lock().unwrap().as_ref().unwrap().element(full_collision).unwrap().kind,
        ElementKind::FullPort
    );
    let _ = value_type;
}

#[test]
fn pr43_ambiguity_duplicate_and_owner_qualified_name_fallback_are_deterministic() {
    let (state, root) = workspace("PR43 Identity");
    let owner_a;
    let owner_b;
    let iface;
    {
        let mut guard = state.project.lock().unwrap();
        let project = guard.as_mut().unwrap();
        owner_a = seed(project, root, ElementKind::Block, "OwnerA", "BLK-A");
        owner_b = seed(project, root, ElementKind::Block, "OwnerB", "BLK-B");
        iface = seed(project, root, ElementKind::InterfaceBlock, "Iface", "IF-1");
        seed_port(project, owner_a, iface, ElementKind::ProxyPort, "command", "OLD-A");
        seed_port(project, owner_b, iface, ElementKind::ProxyPort, "command", "OLD-B");
    }

    let owner_qualified = temp_csv(
        "Port ID,Owner,Port Name,Port Type,Multiplicity,Conjugated,Description,Visibility\nNEW-A,BLK-A,command,IF-1,1,false,Updated by owner-qualified fallback,Public\n",
    );
    let owner_map = map(
        "Owner-qualified fallback",
        owner_qualified,
        None,
        ElementKind::ProxyPort,
        root,
        SpreadsheetIdentificationProperty::Name,
        &port_columns(),
    );
    let preview = preview_spreadsheet_import_group(
        &SpreadsheetImportMapGroup {
            mappings: vec![owner_map.clone()],
        },
        &state,
    );
    assert!(preview.is_valid(), "{:?}", preview.diagnostics);
    assert_eq!(preview.totals.update, 1);
    apply_spreadsheet_import_group(
        &SpreadsheetImportMapGroup {
            mappings: vec![owner_map],
        },
        &state,
    )
    .unwrap();
    {
        let guard = state.project.lock().unwrap();
        let project = guard.as_ref().unwrap();
        let selected = project.element(find_ext(project, "NEW-A")).unwrap();
        assert_eq!(selected.owner_id, Some(owner_a));
        assert_eq!(selected.documentation, "Updated by owner-qualified fallback");
        assert_eq!(
            project
                .elements
                .values()
                .filter(|e| e.kind == ElementKind::ProxyPort && e.name == "command")
                .count(),
            2
        );
    }

    {
        let mut guard = state.project.lock().unwrap();
        let project = guard.as_mut().unwrap();
        seed_port(project, owner_a, iface, ElementKind::ProxyPort, "ambiguous", "AMB-1");
        seed_port(project, owner_a, iface, ElementKind::ProxyPort, "ambiguous", "AMB-2");
    }
    let ambiguous = temp_csv(
        "Port ID,Owner,Port Name,Port Type,Multiplicity,Conjugated,Description,Visibility\nAMB-NEW,BLK-A,ambiguous,IF-1,1,false,,Public\n",
    );
    let ambiguous_map = map(
        "Ambiguous fallback",
        ambiguous,
        None,
        ElementKind::ProxyPort,
        root,
        SpreadsheetIdentificationProperty::Name,
        &port_columns(),
    );
    let blocked = preview_spreadsheet_import_group(
        &SpreadsheetImportMapGroup {
            mappings: vec![ambiguous_map],
        },
        &state,
    );
    assert!(!blocked.is_valid());
    assert!(blocked
        .diagnostics
        .iter()
        .any(|d| d.code == "AMBIGUOUS_IDENTIFICATION"));

    let duplicate = temp_csv(
        "Port ID,Owner,Port Name,Port Type,Multiplicity,Conjugated,Description,Visibility\nDUP-1,BLK-A,a,IF-1,1,false,,Public\nDUP-1,BLK-B,b,IF-1,1,false,,Public\n",
    );
    let duplicate_preview = preview_spreadsheet_import_group(
        &SpreadsheetImportMapGroup {
            mappings: vec![one_port_map(
                "Duplicate IDs",
                duplicate,
                ElementKind::ProxyPort,
                root,
            )],
        },
        &state,
    );
    assert!(!duplicate_preview.is_valid());
    assert!(duplicate_preview
        .diagnostics
        .iter()
        .any(|d| d.code == "DUPLICATE_SOURCE_EXTERNAL_ID"));
}

#[test]
fn pr43_ambiguous_owner_and_type_are_blocked_with_exact_reference_diagnostics() {
    let (state, root) = workspace("PR43 Ambiguity");
    {
        let mut guard = state.project.lock().unwrap();
        let project = guard.as_mut().unwrap();
        project.create_element(ElementKind::Block, "DupOwner", root).unwrap();
        project.create_element(ElementKind::Block, "DupOwner", root).unwrap();
        project
            .create_element(ElementKind::InterfaceBlock, "DupType", root)
            .unwrap();
        project
            .create_element(ElementKind::InterfaceBlock, "DupType", root)
            .unwrap();
        seed(project, root, ElementKind::Block, "GoodOwner", "GOOD-OWNER");
        seed(project, root, ElementKind::InterfaceBlock, "GoodType", "GOOD-TYPE");
    }

    let owner_csv = temp_csv(
        "Port ID,Owner,Port Name,Port Type,Multiplicity,Conjugated,Description,Visibility\nP-AO,PR43 Ambiguity::DupOwner,p,GOOD-TYPE,1,false,,Public\n",
    );
    let owner_preview = preview_spreadsheet_import_group(
        &SpreadsheetImportMapGroup {
            mappings: vec![one_port_map(
                "Ambiguous Owner",
                owner_csv,
                ElementKind::ProxyPort,
                root,
            )],
        },
        &state,
    );
    assert!(!owner_preview.is_valid());
    assert!(owner_preview
        .diagnostics
        .iter()
        .any(|d| d.code == "OWNER_AMBIGUOUS"));

    let type_csv = temp_csv(
        "Port ID,Owner,Port Name,Port Type,Multiplicity,Conjugated,Description,Visibility\nP-AT,GOOD-OWNER,p,PR43 Ambiguity::DupType,1,false,,Public\n",
    );
    let type_preview = preview_spreadsheet_import_group(
        &SpreadsheetImportMapGroup {
            mappings: vec![one_port_map(
                "Ambiguous Type",
                type_csv,
                ElementKind::ProxyPort,
                root,
            )],
        },
        &state,
    );
    assert!(!type_preview.is_valid());
    assert!(type_preview
        .diagnostics
        .iter()
        .any(|d| d.code == "TYPE_AMBIGUOUS"));
}

#[test]
fn pr43_late_invalid_port_blocks_the_entire_plan_and_preview_never_mutates() {
    let (state, root) = workspace("PR43 Atomic");
    let before = {
        let guard = state.project.lock().unwrap();
        serde_json::to_string(guard.as_ref().unwrap()).unwrap()
    };
    let types_csv = temp_csv("ID,Name\nIF-PLAN,PlannedInterface\n");
    let blocks_csv = temp_csv("ID,Name\nBLK-PLAN,PlannedController\n");
    let ports_csv = temp_csv(
        "Port ID,Owner,Port Name,Port Type,Multiplicity,Conjugated,Description,Visibility\nP-GOOD,BLK-PLAN,good,IF-PLAN,1,true,good,Public\nP-BAD,BLK-PLAN,bad,MISSING-TYPE,1,false,bad,Public\n",
    );
    let group = SpreadsheetImportMapGroup {
        mappings: vec![
            map(
                "Plan Type",
                types_csv,
                None,
                ElementKind::InterfaceBlock,
                root,
                SpreadsheetIdentificationProperty::ExternalId,
                &[
                    ("ID", SpreadsheetSemanticProperty::ExternalId),
                    ("Name", SpreadsheetSemanticProperty::Name),
                ],
            ),
            map(
                "Plan Block",
                blocks_csv,
                None,
                ElementKind::Block,
                root,
                SpreadsheetIdentificationProperty::ExternalId,
                &[
                    ("ID", SpreadsheetSemanticProperty::ExternalId),
                    ("Name", SpreadsheetSemanticProperty::Name),
                ],
            ),
            one_port_map("Plan Ports", ports_csv, ElementKind::ProxyPort, root),
        ],
    };

    let snapshot = state.project.lock().unwrap().as_ref().unwrap().clone();
    let prepared = prepare_spreadsheet_import(&group, &snapshot);
    assert!(prepared.plan.operations.iter().any(|operation| matches!(
        operation,
        ModelBuildOperation::CreateElement {
            kind: ElementKind::ProxyPort,
            ..
        }
    )));
    assert!(prepared.plan.operations.iter().all(|operation| !matches!(
        operation,
        ModelBuildOperation::CreateDiagram { .. }
            | ModelBuildOperation::PresentElement { .. }
            | ModelBuildOperation::PresentRelationship { .. }
    )));

    let preview = preview_spreadsheet_import_group(&group, &state);
    assert!(!preview.is_valid());
    assert!(preview.diagnostics.iter().any(|d| d.code == "TYPE_UNRESOLVED"));
    let after_preview = {
        let guard = state.project.lock().unwrap();
        serde_json::to_string(guard.as_ref().unwrap()).unwrap()
    };
    assert_eq!(before, after_preview);
    assert!(apply_spreadsheet_import_group(&group, &state).is_err());
    let after_apply = {
        let guard = state.project.lock().unwrap();
        serde_json::to_string(guard.as_ref().unwrap()).unwrap()
    };
    assert_eq!(before, after_apply);
}
