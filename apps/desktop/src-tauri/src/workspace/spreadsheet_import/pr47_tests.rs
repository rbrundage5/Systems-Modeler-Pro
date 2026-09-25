use super::*;
use std::fs;
use std::path::PathBuf;

const NS: &str = "catia:pr47";

fn workspace(name: &str) -> (WorkspaceState, ElementId) {
    let state = WorkspaceState::default();
    let project = Project::new(name);
    let root = project.root_id;
    *state.project.lock().unwrap() = Some(project);
    (state, root)
}

fn fixture_path() -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/pr47_core_namespace_relationships.xlsx")
        .to_string_lossy()
        .into_owned()
}

fn temp_csv(prefix: &str, body: &str) -> String {
    let path = std::env::temp_dir().join(format!("pr47-{prefix}-{}.csv", uuid::Uuid::new_v4()));
    fs::write(&path, body).unwrap();
    path.to_string_lossy().into_owned()
}

fn element_map(
    name: &str,
    source: String,
    sheet: Option<&str>,
    kind: ElementKind,
    root: ElementId,
    columns: &[(&str, SpreadsheetSemanticProperty)],
) -> SpreadsheetImportMap {
    SpreadsheetImportMap {
        name: name.into(),
        source,
        worksheet: sheet.map(ToOwned::to_owned),
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
            .map(|(c, p)| SpreadsheetColumnMapping {
                source_column: (*c).into(),
                property: *p,
            })
            .collect(),
    }
}

fn relationship_map(
    name: &str,
    source: String,
    sheet: Option<&str>,
    root: ElementId,
    kind: Option<RelationshipKind>,
    columns: &[(&str, SpreadsheetSemanticProperty)],
) -> SpreadsheetImportMap {
    SpreadsheetImportMap {
        name: name.into(),
        source,
        worksheet: sheet.map(ToOwned::to_owned),
        header_row: 1,
        element_kind: ElementKind::Package,
        relationship_kind: kind,
        relationship_identity: SpreadsheetRelationshipIdentityPolicy::ExternalId,
        target_scope: root,
        identification_property: SpreadsheetIdentificationProperty::ExternalId,
        search_scope: SpreadsheetSearchScope::TargetRecursive,
        source_namespace: NS.into(),
        mapping_version: "1".into(),
        column_mappings: columns
            .iter()
            .map(|(c, p)| SpreadsheetColumnMapping {
                source_column: (*c).into(),
                property: *p,
            })
            .collect(),
    }
}

fn xlsx_group(root: ElementId) -> SpreadsheetImportMapGroup {
    let file = fixture_path();
    let packages = element_map(
        "Packages",
        file.clone(),
        Some("Packages"),
        ElementKind::Package,
        root,
        &[
            ("Package ID", SpreadsheetSemanticProperty::ExternalId),
            ("Package Name", SpreadsheetSemanticProperty::Name),
            ("Parent", SpreadsheetSemanticProperty::Owner),
        ],
    );
    let use_cases = element_map(
        "Use Cases",
        file.clone(),
        Some("Use Cases"),
        ElementKind::UseCase,
        root,
        &[
            ("Use Case ID", SpreadsheetSemanticProperty::ExternalId),
            ("Use Case Name", SpreadsheetSemanticProperty::Name),
            ("Package", SpreadsheetSemanticProperty::Owner),
            (
                "Extension Points",
                SpreadsheetSemanticProperty::ExtensionPoints,
            ),
        ],
    );
    let signals = element_map(
        "Signals",
        file.clone(),
        Some("Imported Elements"),
        ElementKind::Signal,
        root,
        &[
            ("Element ID", SpreadsheetSemanticProperty::ExternalId),
            ("Element Name", SpreadsheetSemanticProperty::Name),
            ("Package", SpreadsheetSemanticProperty::Owner),
        ],
    );
    let relations = relationship_map(
        "Reuse and Namespace Rules",
        file,
        Some("Relationships"),
        root,
        None,
        &[
            ("Relationship ID", SpreadsheetSemanticProperty::ExternalId),
            ("Rule", SpreadsheetSemanticProperty::RelationshipKind),
            ("From", SpreadsheetSemanticProperty::Source),
            ("To", SpreadsheetSemanticProperty::Target),
            ("Owner", SpreadsheetSemanticProperty::Owner),
            ("Alias", SpreadsheetSemanticProperty::Alias),
            ("Visibility", SpreadsheetSemanticProperty::Visibility),
            (
                "Extension Point",
                SpreadsheetSemanticProperty::ExtensionLocation,
            ),
            ("Condition", SpreadsheetSemanticProperty::ExtensionCondition),
            ("Name", SpreadsheetSemanticProperty::Name),
            ("Description", SpreadsheetSemanticProperty::Documentation),
        ],
    );
    SpreadsheetImportMapGroup {
        mappings: vec![packages, use_cases, signals, relations],
    }
}

fn rel<'a>(project: &'a Project, external: &str) -> &'a Relationship {
    let key = external_key(NS, external);
    project
        .relationships
        .values()
        .find(|r| r.external_id == key)
        .unwrap()
}

#[test]
fn pr47_xlsx_constructs_all_five_kinds_plan_locally_and_preview_is_nonmutating() {
    let (state, root) = workspace("PR47 XLSX");
    let group = xlsx_group(root);
    let before = state.project.lock().unwrap().as_ref().unwrap().clone();
    let preview = preview_spreadsheet_import_group(&group, &state);
    assert!(preview.is_valid(), "{:?}", preview.diagnostics);
    assert_eq!(
        state
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .relationships
            .len(),
        0
    );
    assert_eq!(
        state
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .elements
            .len(),
        before.elements.len()
    );
    apply_spreadsheet_import_group(&group, &state).unwrap();
    let guard = state.project.lock().unwrap();
    let project = guard.as_ref().unwrap();
    for (id, kind) in [
        ("INC-1", RelationshipKind::Include),
        ("EXT-1", RelationshipKind::Extend),
        ("PI-1", RelationshipKind::PackageImport),
        ("EI-1", RelationshipKind::ElementImport),
        ("PM-1", RelationshipKind::PackageMerge),
    ] {
        assert_eq!(rel(project, id).kind, kind);
    }
    let extend = rel(project, "EXT-1");
    assert_eq!(extend.extension_condition.as_deref(), Some("emergency"));
    assert_eq!(
        extend.extension_location.as_deref(),
        Some("EmergencyHandling")
    );
    let element_import = rel(project, "EI-1");
    assert_eq!(element_import.alias.as_deref(), Some("Command"));
    assert_eq!(element_import.visibility, VisibilityKind::Public);
    assert_eq!(element_import.owner_id, Some(element_import.source_id));
    assert_eq!(
        rel(project, "PI-1").owner_id,
        Some(rel(project, "PI-1").source_id)
    );
    assert_eq!(
        rel(project, "PM-1").owner_id,
        Some(rel(project, "PM-1").source_id)
    );
    project.validate().unwrap();
}

#[test]
fn pr47_csv_include_and_extend_preserve_direction_condition_location_and_qname_resolution() {
    let (state, root) = workspace("PR47 CSV");
    let package = {
        let mut g = state.project.lock().unwrap();
        g.as_mut()
            .unwrap()
            .create_element(ElementKind::Package, "UC", root)
            .unwrap()
    };
    {
        let mut g = state.project.lock().unwrap();
        let p = g.as_mut().unwrap();
        let base = p
            .create_element(ElementKind::UseCase, "OperateVehicle", package)
            .unwrap();
        p.element_mut(base).unwrap().extension_points = vec!["EmergencyHandling".into()];
        p.create_element(ElementKind::UseCase, "StartVehicle", package)
            .unwrap();
        p.create_element(ElementKind::UseCase, "EmergencyShutdown", package)
            .unwrap();
    }
    let include = temp_csv(
        "include",
        "ID,From,To,Owner,Name,Description\nINC-C,PR47 CSV::UC::OperateVehicle,PR47 CSV::UC::StartVehicle,PR47 CSV::UC,reuse,doc\n",
    );
    let extend = temp_csv(
        "extend",
        "ID,From,To,Owner,Extension Point,Condition,Description\nEXT-C,PR47 CSV::UC::EmergencyShutdown,PR47 CSV::UC::OperateVehicle,PR47 CSV::UC,EmergencyHandling,critical,doc\n",
    );
    let group = SpreadsheetImportMapGroup {
        mappings: vec![
            relationship_map(
                "Include",
                include,
                None,
                root,
                Some(RelationshipKind::Include),
                &[
                    ("ID", SpreadsheetSemanticProperty::ExternalId),
                    ("From", SpreadsheetSemanticProperty::Source),
                    ("To", SpreadsheetSemanticProperty::Target),
                    ("Owner", SpreadsheetSemanticProperty::Owner),
                    ("Name", SpreadsheetSemanticProperty::Name),
                    ("Description", SpreadsheetSemanticProperty::Documentation),
                ],
            ),
            relationship_map(
                "Extend",
                extend,
                None,
                root,
                Some(RelationshipKind::Extend),
                &[
                    ("ID", SpreadsheetSemanticProperty::ExternalId),
                    ("From", SpreadsheetSemanticProperty::Source),
                    ("To", SpreadsheetSemanticProperty::Target),
                    ("Owner", SpreadsheetSemanticProperty::Owner),
                    (
                        "Extension Point",
                        SpreadsheetSemanticProperty::ExtensionLocation,
                    ),
                    ("Condition", SpreadsheetSemanticProperty::ExtensionCondition),
                    ("Description", SpreadsheetSemanticProperty::Documentation),
                ],
            ),
        ],
    };
    apply_spreadsheet_import_group(&group, &state).unwrap();
    let g = state.project.lock().unwrap();
    let p = g.as_ref().unwrap();
    let inc = rel(p, "INC-C");
    assert_eq!(p.element(inc.source_id).unwrap().name, "OperateVehicle");
    assert_eq!(p.element(inc.target_id).unwrap().name, "StartVehicle");
    let ext = rel(p, "EXT-C");
    assert_eq!(p.element(ext.source_id).unwrap().name, "EmergencyShutdown");
    assert_eq!(p.element(ext.target_id).unwrap().name, "OperateVehicle");
    assert_eq!(ext.extension_condition.as_deref(), Some("critical"));
    assert_eq!(ext.extension_location.as_deref(), Some("EmergencyHandling"));
}

#[test]
fn pr47_invalid_use_case_endpoints_self_extension_location_and_alias_block() {
    let (state, root) = workspace("PR47 Invalid");
    let (pkg, uc, uc2, block, signal) = {
        let mut g = state.project.lock().unwrap();
        let p = g.as_mut().unwrap();
        let pkg = p.create_element(ElementKind::Package, "P", root).unwrap();
        let uc = p.create_element(ElementKind::UseCase, "A", pkg).unwrap();
        let uc2 = p.create_element(ElementKind::UseCase, "B", pkg).unwrap();
        let block = p.create_element(ElementKind::Block, "Block", pkg).unwrap();
        let signal = p
            .create_element(ElementKind::Signal, "Signal", pkg)
            .unwrap();
        (pkg, uc, uc2, block, signal)
    };
    let _ = (pkg, uc, uc2, block, signal);
    let bad = temp_csv(
        "bad",
        "ID,Kind,From,To,Owner,Alias,Point,Condition\nI1,Include,P::A,P::Block,P,,,\nI2,Include,P::A,P::A,P,,,\nE1,Extend,P::B,P::A,P,,Missing,x\nEI,ElementImport,P,P::Signal,P,not-valid!,,\n",
    );
    let map = relationship_map(
        "Bad",
        bad,
        None,
        root,
        None,
        &[
            ("ID", SpreadsheetSemanticProperty::ExternalId),
            ("Kind", SpreadsheetSemanticProperty::RelationshipKind),
            ("From", SpreadsheetSemanticProperty::Source),
            ("To", SpreadsheetSemanticProperty::Target),
            ("Owner", SpreadsheetSemanticProperty::Owner),
            ("Alias", SpreadsheetSemanticProperty::Alias),
            ("Point", SpreadsheetSemanticProperty::ExtensionLocation),
            ("Condition", SpreadsheetSemanticProperty::ExtensionCondition),
        ],
    );
    let preview = preview_spreadsheet_import_group(
        &SpreadsheetImportMapGroup {
            mappings: vec![map],
        },
        &state,
    );
    assert!(!preview.is_valid());
    assert!(
        preview
            .diagnostics
            .iter()
            .any(|d| d.code == "USE_CASE_RELATIONSHIP_ENDPOINT_KIND_INVALID")
    );
    assert!(
        preview
            .diagnostics
            .iter()
            .any(|d| d.code == "USE_CASE_RELATIONSHIP_SELF_REFERENCE")
    );
    assert!(
        preview
            .diagnostics
            .iter()
            .any(|d| d.code == "ELEMENT_IMPORT_ALIAS_INVALID")
    );
    assert_eq!(
        state
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .relationships
            .len(),
        0
    );
}

#[test]
fn pr47_reimport_no_change_updates_stable_ids_and_wrong_kind_collision_blocks() {
    let (state, root) = workspace("PR47 Reimport");
    apply_spreadsheet_import_group(&xlsx_group(root), &state).unwrap();
    let (extend_id, import_id) = {
        let g = state.project.lock().unwrap();
        let p = g.as_ref().unwrap();
        (rel(p, "EXT-1").id, rel(p, "EI-1").id)
    };
    let second = preview_spreadsheet_import_group(&xlsx_group(root), &state);
    assert!(second.is_valid(), "{:?}", second.diagnostics);
    assert_eq!(second.totals.update, 0);
    assert!(second.totals.no_change >= 5);
    let update = temp_csv(
        "update",
        "ID,Kind,From,To,Owner,Alias,Visibility,Point,Condition,Description\nEXT-1,Extend,UC-EMERG,UC-OPERATE,PKG-UC,,Public,AlternateHandling,updated,changed\nEI-1,ElementImport,PKG-VEH,SIG-CMD,PKG-VEH,Cmd,Private,,,changed\n",
    );
    let map = relationship_map(
        "Updates",
        update,
        None,
        root,
        None,
        &[
            ("ID", SpreadsheetSemanticProperty::ExternalId),
            ("Kind", SpreadsheetSemanticProperty::RelationshipKind),
            ("From", SpreadsheetSemanticProperty::Source),
            ("To", SpreadsheetSemanticProperty::Target),
            ("Owner", SpreadsheetSemanticProperty::Owner),
            ("Alias", SpreadsheetSemanticProperty::Alias),
            ("Visibility", SpreadsheetSemanticProperty::Visibility),
            ("Point", SpreadsheetSemanticProperty::ExtensionLocation),
            ("Condition", SpreadsheetSemanticProperty::ExtensionCondition),
            ("Description", SpreadsheetSemanticProperty::Documentation),
        ],
    );
    apply_spreadsheet_import_group(
        &SpreadsheetImportMapGroup {
            mappings: vec![map],
        },
        &state,
    )
    .unwrap();
    {
        let g = state.project.lock().unwrap();
        let p = g.as_ref().unwrap();
        assert_eq!(rel(p, "EXT-1").id, extend_id);
        assert_eq!(
            rel(p, "EXT-1").extension_location.as_deref(),
            Some("AlternateHandling")
        );
        assert_eq!(rel(p, "EI-1").id, import_id);
        assert_eq!(rel(p, "EI-1").alias.as_deref(), Some("Cmd"));
        assert_eq!(rel(p, "EI-1").visibility, VisibilityKind::Private);
    }
    let collision = temp_csv(
        "collision",
        "ID,From,To,Owner\nINC-1,PKG-VEH,PKG-COMMON,PKG-VEH\n",
    );
    let map = relationship_map(
        "Collision",
        collision,
        None,
        root,
        Some(RelationshipKind::PackageImport),
        &[
            ("ID", SpreadsheetSemanticProperty::ExternalId),
            ("From", SpreadsheetSemanticProperty::Source),
            ("To", SpreadsheetSemanticProperty::Target),
            ("Owner", SpreadsheetSemanticProperty::Owner),
        ],
    );
    let preview = preview_spreadsheet_import_group(
        &SpreadsheetImportMapGroup {
            mappings: vec![map],
        },
        &state,
    );
    assert!(!preview.is_valid());
    assert!(
        preview
            .diagnostics
            .iter()
            .any(|d| d.code == "RELATIONSHIP_IDENTITY_KIND_MISMATCH")
    );
}

#[test]
fn pr47_unresolved_ambiguous_duplicate_identity_and_owner_mismatch_block() {
    let (state, root) = workspace("PR47 Resolution");
    {
        let mut g = state.project.lock().unwrap();
        let p = g.as_mut().unwrap();
        let p1 = p.create_element(ElementKind::Package, "One", root).unwrap();
        p.create_element(ElementKind::Package, "Two", root).unwrap();
        p.create_element(ElementKind::UseCase, "Same", p1).unwrap();
        p.create_element(ElementKind::UseCase, "Same", p1).unwrap();
        p.create_element(ElementKind::UseCase, "Target", p1)
            .unwrap();
    }
    let csv = temp_csv(
        "resolution",
        "ID,Kind,From,To,Owner\nA,Include,One::Same,One::Target,One\nB,Include,One::Missing,One::Target,One\nC,PackageImport,One,Two,Two\nD,PackageImport,One,Two,One\nD,PackageImport,One,Two,One\n",
    );
    let map = relationship_map(
        "Resolution",
        csv,
        None,
        root,
        None,
        &[
            ("ID", SpreadsheetSemanticProperty::ExternalId),
            ("Kind", SpreadsheetSemanticProperty::RelationshipKind),
            ("From", SpreadsheetSemanticProperty::Source),
            ("To", SpreadsheetSemanticProperty::Target),
            ("Owner", SpreadsheetSemanticProperty::Owner),
        ],
    );
    let preview = preview_spreadsheet_import_group(
        &SpreadsheetImportMapGroup {
            mappings: vec![map],
        },
        &state,
    );
    assert!(!preview.is_valid());
    assert!(
        preview
            .diagnostics
            .iter()
            .any(|d| d.code == "SOURCE_AMBIGUOUS")
    );
    assert!(
        preview
            .diagnostics
            .iter()
            .any(|d| d.code == "SOURCE_UNRESOLVED")
    );
    assert!(
        preview
            .diagnostics
            .iter()
            .any(|d| d.code == "NAMESPACE_RELATIONSHIP_OWNER_INVALID")
    );
    assert!(
        preview
            .diagnostics
            .iter()
            .any(|d| d.code == "DUPLICATE_SOURCE_EXTERNAL_ID")
    );
}

#[test]
fn pr47_late_invalid_extend_rolls_back_entire_map_group() {
    let (state, root) = workspace("PR47 Atomic");
    let packages = temp_csv("packages", "ID,Name\nP1,One\nP2,Two\n");
    let usecases = temp_csv(
        "ucs",
        "ID,Name,Owner,Points\nU1,Base,P1,Good\nU2,Extension,P1,\n",
    );
    let relations = temp_csv(
        "rels",
        "ID,Kind,From,To,Owner,Point\nPI,PackageImport,P1,P2,P1,\nE,Extend,U2,U1,P1,Missing\n",
    );
    let group = SpreadsheetImportMapGroup {
        mappings: vec![
            element_map(
                "Packages",
                packages,
                None,
                ElementKind::Package,
                root,
                &[
                    ("ID", SpreadsheetSemanticProperty::ExternalId),
                    ("Name", SpreadsheetSemanticProperty::Name),
                ],
            ),
            element_map(
                "UseCases",
                usecases,
                None,
                ElementKind::UseCase,
                root,
                &[
                    ("ID", SpreadsheetSemanticProperty::ExternalId),
                    ("Name", SpreadsheetSemanticProperty::Name),
                    ("Owner", SpreadsheetSemanticProperty::Owner),
                    ("Points", SpreadsheetSemanticProperty::ExtensionPoints),
                ],
            ),
            relationship_map(
                "Relations",
                relations,
                None,
                root,
                None,
                &[
                    ("ID", SpreadsheetSemanticProperty::ExternalId),
                    ("Kind", SpreadsheetSemanticProperty::RelationshipKind),
                    ("From", SpreadsheetSemanticProperty::Source),
                    ("To", SpreadsheetSemanticProperty::Target),
                    ("Owner", SpreadsheetSemanticProperty::Owner),
                    ("Point", SpreadsheetSemanticProperty::ExtensionLocation),
                ],
            ),
        ],
    };
    let preview = preview_spreadsheet_import_group(&group, &state);
    assert!(!preview.is_valid());
    assert!(
        preview
            .diagnostics
            .iter()
            .any(|d| d.reason.contains("extension point") || d.reason.contains("Missing")),
        "{:?}",
        preview.diagnostics
    );
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
    assert_eq!(
        state
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .relationships
            .len(),
        0
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
