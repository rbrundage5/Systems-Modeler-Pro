from pathlib import Path
import zipfile
from xml.sax.saxutils import escape

ROOT = Path(__file__).resolve().parents[1]
SPREADSHEET = ROOT / "apps/desktop/src-tauri/src/workspace/spreadsheet_import.rs"


def make_xlsx(path: Path) -> None:
    sheets = [
        ("Component Parts", [
            ["Feature Identifier", "Owning Component", "Property Name", "Classifier", "Cardinality", "Description"],
            ["PART-ENGINE-001", "Vehicle", "engine", "Engine", "1", "Primary propulsion unit"],
            ["PART-CTRL-001", "Vehicle", "controller", "Controller", "1", "Primary controller"],
            ["PART-BACKUP-001", "Vehicle", "backupController", "Controller", "0..1", "Optional backup controller"],
        ]),
        ("Component Values", [
            ["Feature Identifier", "Owning Component", "Property Name", "Classifier", "Cardinality", "Initial Value", "Description"],
            ["VAL-MASS-001", "Vehicle", "mass", "Mass", "1", "1500", "Vehicle mass"],
        ]),
        ("Interface Flows", [
            ["Feature Identifier", "Owning Component", "Property Name", "Classifier", "Cardinality", "Flow", "Description"],
            ["FLOW-CMD-001", "VehicleInterface", "command", "Command", "1", "in", "Inbound command"],
        ]),
    ]

    def col_name(n: int) -> str:
        result = ""
        while n:
            n, rem = divmod(n - 1, 26)
            result = chr(65 + rem) + result
        return result

    def sheet_xml(rows):
        parts = [
            '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>',
            '<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>',
        ]
        for row_index, row in enumerate(rows, 1):
            parts.append(f'<row r="{row_index}">')
            for column_index, value in enumerate(row, 1):
                ref = f"{col_name(column_index)}{row_index}"
                parts.append(f'<c r="{ref}" t="inlineStr"><is><t>{escape(str(value))}</t></is></c>')
            parts.append('</row>')
        parts.append('</sheetData></worksheet>')
        return ''.join(parts)

    content_types = [
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>',
        '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">',
        '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>',
        '<Default Extension="xml" ContentType="application/xml"/>',
        '<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>',
    ]
    workbook_sheets = []
    workbook_rels = []
    for index, (name, _) in enumerate(sheets, 1):
        content_types.append(f'<Override PartName="/xl/worksheets/sheet{index}.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>')
        workbook_sheets.append(f'<sheet name="{escape(name)}" sheetId="{index}" r:id="rId{index}"/>')
        workbook_rels.append(f'<Relationship Id="rId{index}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet{index}.xml"/>')
    content_types.append('</Types>')
    workbook = ('<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets>'
        + ''.join(workbook_sheets) + '</sheets></workbook>')
    rels = ('<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
        + ''.join(workbook_rels) + '</Relationships>')
    root_rels = ('<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
        '<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>'
        '</Relationships>')
    path.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as archive:
        archive.writestr("[Content_Types].xml", ''.join(content_types))
        archive.writestr("_rels/.rels", root_rels)
        archive.writestr("xl/workbook.xml", workbook)
        archive.writestr("xl/_rels/workbook.xml.rels", rels)
        for index, (_, rows) in enumerate(sheets, 1):
            archive.writestr(f"xl/worksheets/sheet{index}.xml", sheet_xml(rows))


make_xlsx(ROOT / "apps/desktop/src-tauri/tests/fixtures/pr39_owned_features.xlsx")

text = SPREADSHEET.read_text(encoding="utf-8")
marker = "\n}\n"
insert_at = text.rfind(marker)
if insert_at < 0:
    raise SystemExit("spreadsheet test module closing brace not found")

tests = r'''
    fn pr39_fixture_path() -> String {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/pr39_owned_features.xlsx")
            .to_string_lossy()
            .into_owned()
    }

    fn set_pr39_external(project: &mut Project, id: ElementId, external_id: &str) {
        project
            .set_external_id(id, external_key("catia:pr38-fixture", external_id))
            .unwrap();
    }

    #[test]
    fn pr39_xlsx_business_columns_create_typed_owned_features() {
        let (state, root) = workspace("PR39 XLSX");
        {
            let mut guard = state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            project.create_element(ElementKind::Block, "Vehicle", root).unwrap();
            project.create_element(ElementKind::InterfaceBlock, "VehicleInterface", root).unwrap();
            project.create_element(ElementKind::Block, "Engine", root).unwrap();
            project.create_element(ElementKind::Block, "Controller", root).unwrap();
            project.create_element(ElementKind::ValueType, "Mass", root).unwrap();
            project.create_element(ElementKind::DataType, "Command", root).unwrap();
        }
        let parts = map(
            "Part Properties", pr39_fixture_path(), Some("Component Parts"), 1,
            ElementKind::PartProperty, root,
            SpreadsheetIdentificationProperty::ExternalId,
            SpreadsheetSearchScope::TargetRecursive,
            &[
                ("Feature Identifier", SpreadsheetSemanticProperty::ExternalId),
                ("Owning Component", SpreadsheetSemanticProperty::Owner),
                ("Property Name", SpreadsheetSemanticProperty::Name),
                ("Classifier", SpreadsheetSemanticProperty::Type),
                ("Cardinality", SpreadsheetSemanticProperty::Multiplicity),
                ("Description", SpreadsheetSemanticProperty::Documentation),
            ],
        );
        let values = map(
            "Value Properties", pr39_fixture_path(), Some("Component Values"), 1,
            ElementKind::ValueProperty, root,
            SpreadsheetIdentificationProperty::ExternalId,
            SpreadsheetSearchScope::TargetRecursive,
            &[
                ("Feature Identifier", SpreadsheetSemanticProperty::ExternalId),
                ("Owning Component", SpreadsheetSemanticProperty::Owner),
                ("Property Name", SpreadsheetSemanticProperty::Name),
                ("Classifier", SpreadsheetSemanticProperty::Type),
                ("Cardinality", SpreadsheetSemanticProperty::Multiplicity),
                ("Initial Value", SpreadsheetSemanticProperty::DefaultValue),
                ("Description", SpreadsheetSemanticProperty::Documentation),
            ],
        );
        let flows = map(
            "Flow Properties", pr39_fixture_path(), Some("Interface Flows"), 1,
            ElementKind::FlowProperty, root,
            SpreadsheetIdentificationProperty::ExternalId,
            SpreadsheetSearchScope::TargetRecursive,
            &[
                ("Feature Identifier", SpreadsheetSemanticProperty::ExternalId),
                ("Owning Component", SpreadsheetSemanticProperty::Owner),
                ("Property Name", SpreadsheetSemanticProperty::Name),
                ("Classifier", SpreadsheetSemanticProperty::Type),
                ("Cardinality", SpreadsheetSemanticProperty::Multiplicity),
                ("Flow", SpreadsheetSemanticProperty::FlowDirection),
                ("Description", SpreadsheetSemanticProperty::Documentation),
            ],
        );
        let group = SpreadsheetImportMapGroup { mappings: vec![parts, values, flows] };
        let preview = preview_spreadsheet_import_group(&group, &state);
        assert!(preview.is_valid(), "{:?}", preview.diagnostics);
        assert_eq!(preview.totals.create, 5);
        apply_spreadsheet_import_group(&group, &state).unwrap();
        let guard = state.project.lock().unwrap();
        let project = guard.as_ref().unwrap();
        let engine = project.elements.values().find(|element| element.name == "engine").unwrap();
        assert_eq!(engine.kind, ElementKind::PartProperty);
        assert_eq!(engine.multiplicity.unwrap().notation(), "1");
        assert_eq!(project.element(engine.type_id.unwrap()).unwrap().name, "Engine");
        let backup = project.elements.values().find(|element| element.name == "backupController").unwrap();
        assert_eq!(backup.multiplicity.unwrap().notation(), "0..1");
        let mass = project.elements.values().find(|element| element.name == "mass").unwrap();
        assert_eq!(mass.default_value.as_deref(), Some("1500"));
        let command = project.elements.values().find(|element| element.name == "command").unwrap();
        assert_eq!(command.flow_direction, Some(FlowDirection::In));
    }

    #[test]
    fn pr39_supports_all_six_feature_kinds() {
        let (state, root) = workspace("PR39 Six Kinds");
        {
            let mut guard = state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            project.create_element(ElementKind::Block, "Vehicle", root).unwrap();
            project.create_element(ElementKind::InterfaceBlock, "Iface", root).unwrap();
            project.create_element(ElementKind::ConstraintBlock, "Equation", root).unwrap();
            project.create_element(ElementKind::Block, "Engine", root).unwrap();
            project.create_element(ElementKind::ValueType, "Scalar", root).unwrap();
        }
        let cases = [
            (ElementKind::PartProperty, "Vehicle", "Engine", "part", None),
            (ElementKind::ReferenceProperty, "Vehicle", "Engine", "reference", None),
            (ElementKind::ValueProperty, "Vehicle", "Scalar", "value", None),
            (ElementKind::FlowProperty, "Iface", "Scalar", "flow", Some("out")),
            (ElementKind::ConstraintProperty, "Vehicle", "Equation", "constraint", None),
            (ElementKind::ConstraintParameter, "Equation", "Scalar", "parameter", None),
        ];
        let mut mappings = Vec::new();
        for (index, (kind, owner, type_name, name, direction)) in cases.into_iter().enumerate() {
            let id = format!("F-{index}");
            let source = if let Some(direction) = direction {
                temp_csv(&format!("ID,Owner,Name,Type,Multiplicity,Flow\n{id},{owner},{name},{type_name},1,{direction}\n"))
            } else {
                temp_csv(&format!("ID,Owner,Name,Type,Multiplicity\n{id},{owner},{name},{type_name},1\n"))
            };
            let mut columns = vec![
                ("ID", SpreadsheetSemanticProperty::ExternalId),
                ("Owner", SpreadsheetSemanticProperty::Owner),
                ("Name", SpreadsheetSemanticProperty::Name),
                ("Type", SpreadsheetSemanticProperty::Type),
                ("Multiplicity", SpreadsheetSemanticProperty::Multiplicity),
            ];
            if direction.is_some() {
                columns.push(("Flow", SpreadsheetSemanticProperty::FlowDirection));
            }
            mappings.push(map(
                &format!("Feature {index}"), source, None, 1, kind, root,
                SpreadsheetIdentificationProperty::ExternalId,
                SpreadsheetSearchScope::TargetRecursive,
                &columns,
            ));
        }
        let group = SpreadsheetImportMapGroup { mappings };
        let preview = preview_spreadsheet_import_group(&group, &state);
        assert!(preview.is_valid(), "{:?}", preview.diagnostics);
        apply_spreadsheet_import_group(&group, &state).unwrap();
        let guard = state.project.lock().unwrap();
        let project = guard.as_ref().unwrap();
        for expected in [
            ElementKind::PartProperty,
            ElementKind::ReferenceProperty,
            ElementKind::ValueProperty,
            ElementKind::FlowProperty,
            ElementKind::ConstraintProperty,
            ElementKind::ConstraintParameter,
        ] {
            assert!(project.elements.values().any(|element| element.kind == expected));
        }
    }

    #[test]
    fn pr39_resolves_owner_and_type_by_namespaced_external_identity() {
        let (state, root) = workspace("PR39 External References");
        {
            let mut guard = state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            let owner = project.create_element(ElementKind::Block, "Renamed Vehicle", root).unwrap();
            let ty = project.create_element(ElementKind::Block, "Renamed Engine", root).unwrap();
            set_pr39_external(project, owner, "BLK-VEHICLE");
            set_pr39_external(project, ty, "BLK-ENGINE");
        }
        let source = temp_csv("ID,Owner,Name,Type,Multiplicity\nPART-1,BLK-VEHICLE,engine,BLK-ENGINE,1\n");
        let mapping = map(
            "External owner/type", source, None, 1, ElementKind::PartProperty, root,
            SpreadsheetIdentificationProperty::ExternalId,
            SpreadsheetSearchScope::TargetRecursive,
            &[
                ("ID", SpreadsheetSemanticProperty::ExternalId),
                ("Owner", SpreadsheetSemanticProperty::Owner),
                ("Name", SpreadsheetSemanticProperty::Name),
                ("Type", SpreadsheetSemanticProperty::Type),
                ("Multiplicity", SpreadsheetSemanticProperty::Multiplicity),
            ],
        );
        let group = SpreadsheetImportMapGroup { mappings: vec![mapping] };
        let preview = preview_spreadsheet_import_group(&group, &state);
        assert!(preview.is_valid(), "{:?}", preview.diagnostics);
        apply_spreadsheet_import_group(&group, &state).unwrap();
        let guard = state.project.lock().unwrap();
        let project = guard.as_ref().unwrap();
        let feature = project.elements.values().find(|element| element.name == "engine").unwrap();
        assert_eq!(project.element(feature.owner_id.unwrap()).unwrap().name, "Renamed Vehicle");
        assert_eq!(project.element(feature.type_id.unwrap()).unwrap().name, "Renamed Engine");
    }

    #[test]
    fn pr39_ordered_maps_resolve_plan_local_owner_and_type_without_early_commit() {
        let (state, root) = workspace("PR39 Ordered");
        let blocks = map(
            "Blocks",
            temp_csv("ID,Name\nBLK-VEHICLE,Vehicle\nBLK-ENGINE,Engine\n"),
            None, 1, ElementKind::Block, root,
            SpreadsheetIdentificationProperty::ExternalId,
            SpreadsheetSearchScope::TargetOnly,
            &[("ID", SpreadsheetSemanticProperty::ExternalId), ("Name", SpreadsheetSemanticProperty::Name)],
        );
        let features = map(
            "Features",
            temp_csv("ID,Owner,Name,Type,Multiplicity\nPART-1,BLK-VEHICLE,engine,BLK-ENGINE,1\n"),
            None, 1, ElementKind::PartProperty, root,
            SpreadsheetIdentificationProperty::ExternalId,
            SpreadsheetSearchScope::TargetOnly,
            &[
                ("ID", SpreadsheetSemanticProperty::ExternalId),
                ("Owner", SpreadsheetSemanticProperty::Owner),
                ("Name", SpreadsheetSemanticProperty::Name),
                ("Type", SpreadsheetSemanticProperty::Type),
                ("Multiplicity", SpreadsheetSemanticProperty::Multiplicity),
            ],
        );
        let group = SpreadsheetImportMapGroup { mappings: vec![blocks, features] };
        let before = state.project.lock().unwrap().as_ref().unwrap().elements.len();
        let preview = preview_spreadsheet_import_group(&group, &state);
        assert!(preview.is_valid(), "{:?}", preview.diagnostics);
        assert_eq!(state.project.lock().unwrap().as_ref().unwrap().elements.len(), before);
        apply_spreadsheet_import_group(&group, &state).unwrap();
        let guard = state.project.lock().unwrap();
        let project = guard.as_ref().unwrap();
        let feature = project.elements.values().find(|element| element.name == "engine").unwrap();
        assert_eq!(project.element(feature.owner_id.unwrap()).unwrap().name, "Vehicle");
        assert_eq!(project.element(feature.type_id.unwrap()).unwrap().name, "Engine");
    }

    #[test]
    fn pr39_reimport_updates_same_feature_and_then_noops() {
        let (state, root) = workspace("PR39 Reimport");
        {
            let mut guard = state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            project.create_element(ElementKind::Block, "Vehicle", root).unwrap();
            project.create_element(ElementKind::Block, "Engine", root).unwrap();
        }
        let mapping_for = |source: String| map(
            "Part", source, None, 1, ElementKind::PartProperty, root,
            SpreadsheetIdentificationProperty::ExternalId,
            SpreadsheetSearchScope::TargetRecursive,
            &[
                ("ID", SpreadsheetSemanticProperty::ExternalId),
                ("Owner", SpreadsheetSemanticProperty::Owner),
                ("Name", SpreadsheetSemanticProperty::Name),
                ("Type", SpreadsheetSemanticProperty::Type),
                ("Multiplicity", SpreadsheetSemanticProperty::Multiplicity),
            ],
        );
        let first = SpreadsheetImportMapGroup { mappings: vec![mapping_for(temp_csv("ID,Owner,Name,Type,Multiplicity\nPART-1,Vehicle,engine,Engine,1\n"))] };
        apply_spreadsheet_import_group(&first, &state).unwrap();
        let update = SpreadsheetImportMapGroup { mappings: vec![mapping_for(temp_csv("ID,Owner,Name,Type,Multiplicity\nPART-1,Vehicle,propulsionUnit,Engine,0..1\n"))] };
        assert_eq!(preview_spreadsheet_import_group(&update, &state).totals.update, 1);
        apply_spreadsheet_import_group(&update, &state).unwrap();
        let noop = SpreadsheetImportMapGroup { mappings: vec![mapping_for(temp_csv("ID,Owner,Name,Type,Multiplicity\nPART-1,Vehicle,propulsionUnit,Engine,0..1\n"))] };
        assert_eq!(preview_spreadsheet_import_group(&noop, &state).totals.no_change, 1);
        let guard = state.project.lock().unwrap();
        let project = guard.as_ref().unwrap();
        assert_eq!(project.elements.values().filter(|element| element.kind == ElementKind::PartProperty).count(), 1);
    }

    #[test]
    fn pr39_blocks_ambiguous_unresolved_illegal_multiplicity_and_flow_errors() {
        let (state, root) = workspace("PR39 Blocking");
        {
            let mut guard = state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            project.create_element(ElementKind::Block, "Vehicle", root).unwrap();
            project.create_element(ElementKind::Block, "Vehicle", root).unwrap();
            project.create_element(ElementKind::ValueType, "Scalar", root).unwrap();
            project.create_element(ElementKind::ValueType, "Scalar", root).unwrap();
            project.create_element(ElementKind::Package, "WrongOwner", root).unwrap();
            project.create_element(ElementKind::Block, "Engine", root).unwrap();
            project.create_element(ElementKind::InterfaceBlock, "Iface", root).unwrap();
        }
        let cases = [
            ("A,Vehicle,a,Engine,1\n", ElementKind::PartProperty, "OWNER_AMBIGUOUS"),
            ("B,Missing,b,Engine,1\n", ElementKind::PartProperty, "OWNER_UNRESOLVED"),
            ("C,WrongOwner,c,Engine,1\n", ElementKind::PartProperty, "SEMANTIC_VALIDATION"),
            ("D,WrongOwner,d,Missing,1\n", ElementKind::PartProperty, "TYPE_UNRESOLVED"),
            ("E,WrongOwner,e,Scalar,1\n", ElementKind::ValueProperty, "TYPE_AMBIGUOUS"),
            ("F,WrongOwner,f,Engine,2..1\n", ElementKind::PartProperty, "MULTIPLICITY_INVALID"),
        ];
        for (row, kind, code) in cases {
            let mapping = map(
                "Blocked feature",
                temp_csv(&format!("ID,Owner,Name,Type,Multiplicity\n{row}")),
                None, 1, kind, root,
                SpreadsheetIdentificationProperty::ExternalId,
                SpreadsheetSearchScope::TargetRecursive,
                &[
                    ("ID", SpreadsheetSemanticProperty::ExternalId),
                    ("Owner", SpreadsheetSemanticProperty::Owner),
                    ("Name", SpreadsheetSemanticProperty::Name),
                    ("Type", SpreadsheetSemanticProperty::Type),
                    ("Multiplicity", SpreadsheetSemanticProperty::Multiplicity),
                ],
            );
            let preview = preview_spreadsheet_import_group(&SpreadsheetImportMapGroup { mappings: vec![mapping] }, &state);
            assert!(!preview.is_valid(), "expected {code}");
            assert!(preview.diagnostics.iter().any(|diagnostic| diagnostic.code == code), "{:?}", preview.diagnostics);
        }
        let flow = map(
            "Bad flow",
            temp_csv("ID,Owner,Name,Type,Multiplicity,Flow\nG,Iface,command,Engine,1,input/output\n"),
            None, 1, ElementKind::FlowProperty, root,
            SpreadsheetIdentificationProperty::ExternalId,
            SpreadsheetSearchScope::TargetRecursive,
            &[
                ("ID", SpreadsheetSemanticProperty::ExternalId),
                ("Owner", SpreadsheetSemanticProperty::Owner),
                ("Name", SpreadsheetSemanticProperty::Name),
                ("Type", SpreadsheetSemanticProperty::Type),
                ("Multiplicity", SpreadsheetSemanticProperty::Multiplicity),
                ("Flow", SpreadsheetSemanticProperty::FlowDirection),
            ],
        );
        let preview = preview_spreadsheet_import_group(&SpreadsheetImportMapGroup { mappings: vec![flow] }, &state);
        assert!(preview.diagnostics.iter().any(|diagnostic| diagnostic.code == "FLOW_DIRECTION_INVALID"));
    }

    #[test]
    fn pr39_feature_apply_is_atomic_when_one_feature_is_invalid() {
        let (state, root) = workspace("PR39 Atomic");
        {
            let mut guard = state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            project.create_element(ElementKind::Block, "Vehicle", root).unwrap();
            project.create_element(ElementKind::Block, "Engine", root).unwrap();
            project.create_element(ElementKind::Package, "WrongOwner", root).unwrap();
        }
        let before = state.project.lock().unwrap().as_ref().unwrap().elements.len();
        let mapping = map(
            "Atomic features",
            temp_csv("ID,Owner,Name,Type,Multiplicity\nGOOD,Vehicle,engine,Engine,1\nBAD,WrongOwner,bad,Engine,1\n"),
            None, 1, ElementKind::PartProperty, root,
            SpreadsheetIdentificationProperty::ExternalId,
            SpreadsheetSearchScope::TargetRecursive,
            &[
                ("ID", SpreadsheetSemanticProperty::ExternalId),
                ("Owner", SpreadsheetSemanticProperty::Owner),
                ("Name", SpreadsheetSemanticProperty::Name),
                ("Type", SpreadsheetSemanticProperty::Type),
                ("Multiplicity", SpreadsheetSemanticProperty::Multiplicity),
            ],
        );
        let group = SpreadsheetImportMapGroup { mappings: vec![mapping] };
        let preview = preview_spreadsheet_import_group(&group, &state);
        assert!(!preview.is_valid());
        assert!(apply_spreadsheet_import_group(&group, &state).is_err());
        assert_eq!(state.project.lock().unwrap().as_ref().unwrap().elements.len(), before);
    }
'''

SPREADSHEET.write_text(text[:insert_at] + tests + text[insert_at:], encoding="utf-8")
print("PR39 fixture and tests added")
