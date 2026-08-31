from pathlib import Path
from xml.sax.saxutils import escape
from zipfile import ZIP_DEFLATED, ZipFile

ROOT = Path(__file__).resolve().parents[1]
source_path = ROOT / "apps/desktop/src-tauri/src/workspace/spreadsheet_import.rs"
fixture_path = ROOT / "apps/desktop/src-tauri/tests/fixtures/pr41_requirements_traceability.xlsx"


def col_name(index: int) -> str:
    value = index + 1
    letters = ""
    while value:
        value, remainder = divmod(value - 1, 26)
        letters = chr(65 + remainder) + letters
    return letters


def worksheet_xml(rows):
    rendered_rows = []
    for row_index, row in enumerate(rows, start=1):
        cells = []
        for col_index, value in enumerate(row):
            ref = f"{col_name(col_index)}{row_index}"
            text = escape(str(value))
            cells.append(f'<c r="{ref}" t="inlineStr"><is><t>{text}</t></is></c>')
        rendered_rows.append(f'<row r="{row_index}">{"".join(cells)}</row>')
    return (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">'
        f'<sheetData>{"".join(rendered_rows)}</sheetData></worksheet>'
    )


sheets = [
    (
        "Package Catalog",
        [
            ["Package Key", "Package Label", "Parent Package"],
            ["PKG-REQ", "SystemRequirements", ""],
            ["PKG-DES", "Design", ""],
            ["PKG-VER", "Verification", ""],
            ["PKG-TRC", "Traceability", ""],
        ],
    ),
    (
        "Requirement Catalog",
        [
            ["Requirement External Key", "Requirement Number", "Requirement Title", "Requirement Statement", "Parent Package"],
            ["REQEXT-SYS-001", "REQ-SYS-001", "System Safety", "Master safety text", "SystemRequirements"],
            ["REQEXT-SYS-002", "REQ-SYS-002", "Behavior Requirement", "Behavior text", "SystemRequirements"],
            ["REQEXT-SUB-001", "REQ-SUB-001", "Brake Requirement", "Brake text", "SystemRequirements"],
            ["REQEXT-COPY-001", "REQ-COPY-001", "Copied Safety", "Master safety text", "SystemRequirements"],
        ],
    ),
    (
        "Design Catalog",
        [
            ["Design Key", "Design Name", "Parent Package"],
            ["BLK-VEH", "Vehicle", "Design"],
            ["BLK-BRAKE", "BrakeSystem", "Design"],
            ["BLK-BEHAV", "VehicleBehavior", "Design"],
        ],
    ),
    (
        "Verification Catalog",
        [
            ["Test Key", "Test Name", "Parent Package"],
            ["TC-BRAKE", "BrakeTest", "Verification"],
        ],
    ),
    (
        "Requirement Links",
        [
            ["Link Identifier", "Link Type", "Design Object", "Requirement Number", "Description", "Relationship Package"],
            ["DER-1", "deriveReqt", "REQ-SUB-001", "REQ-SYS-001", "Derived from system requirement", "Traceability"],
            ["SAT-1", "Satisfy", "BLK-BRAKE", "REQ-SUB-001", "Brake design satisfies requirement", "Traceability"],
            ["VER-1", "Verify", "TC-BRAKE", "REQ-SUB-001", "Planned verification coverage", "Traceability"],
            ["REF-1", "Refine", "Design::VehicleBehavior", "REQ-SYS-002", "Behavior refines requirement", "Traceability"],
            ["COPY-1", "Copy", "REQ-COPY-001", "REQ-SYS-001", "Copy relationship", "Traceability"],
            ["TRACE-1", "Trace", "BLK-VEH", "REQ-SYS-002", "General trace", "Traceability"],
        ],
    ),
]

fixture_path.parent.mkdir(parents=True, exist_ok=True)
content_types = [
    '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>',
    '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">',
    '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>',
    '<Default Extension="xml" ContentType="application/xml"/>',
    '<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>',
]
for index in range(1, len(sheets) + 1):
    content_types.append(
        f'<Override PartName="/xl/worksheets/sheet{index}.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>'
    )
content_types.append('</Types>')

workbook_sheets = "".join(
    f'<sheet name="{escape(name)}" sheetId="{index}" r:id="rId{index}"/>'
    for index, (name, _) in enumerate(sheets, start=1)
)
workbook_xml = (
    '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
    '<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" '
    'xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">'
    f'<sheets>{workbook_sheets}</sheets></workbook>'
)
workbook_rels = [
    '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>',
    '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">',
]
for index in range(1, len(sheets) + 1):
    workbook_rels.append(
        f'<Relationship Id="rId{index}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet{index}.xml"/>'
    )
workbook_rels.append('</Relationships>')
root_rels = (
    '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
    '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
    '<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>'
    '</Relationships>'
)
with ZipFile(fixture_path, "w", ZIP_DEFLATED) as archive:
    archive.writestr("[Content_Types].xml", "".join(content_types))
    archive.writestr("_rels/.rels", root_rels)
    archive.writestr("xl/workbook.xml", workbook_xml)
    archive.writestr("xl/_rels/workbook.xml.rels", "".join(workbook_rels))
    for index, (_, rows) in enumerate(sheets, start=1):
        archive.writestr(f"xl/worksheets/sheet{index}.xml", worksheet_xml(rows))

text = source_path.read_text(encoding="utf-8")
if "mod pr41_tests {" in text:
    raise SystemExit("PR41 tests already present")

text += r'''

#[cfg(test)]
mod pr41_tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use systems_modeler_core::RelationshipId;

    const NS: &str = "catia:pr41-fixture";

    fn workspace(name: &str) -> (WorkspaceState, ElementId, ElementId) {
        let state = WorkspaceState::default();
        let mut project = Project::new(name);
        let root = project.root_id;
        let program = project
            .create_element(ElementKind::Package, "VehicleProgram", root)
            .unwrap();
        project
            .set_external_id(program, external_key(NS, "PROGRAM"))
            .unwrap();
        *state.project.lock().unwrap() = Some(project);
        (state, root, program)
    }

    fn fixture_path() -> String {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/pr41_requirements_traceability.xlsx")
            .to_string_lossy()
            .into_owned()
    }

    fn temp_csv(contents: &str) -> String {
        let path = std::env::temp_dir().join(format!("pr41-{}.csv", uuid::Uuid::new_v4()));
        fs::write(&path, contents).unwrap();
        path.to_string_lossy().into_owned()
    }

    fn map(
        name: &str,
        source: String,
        worksheet: Option<&str>,
        kind: ElementKind,
        target: ElementId,
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

    fn relationship_map(
        name: &str,
        source: String,
        worksheet: Option<&str>,
        target: ElementId,
        configured_kind: Option<RelationshipKind>,
        identity: SpreadsheetRelationshipIdentityPolicy,
        columns: &[(&str, SpreadsheetSemanticProperty)],
    ) -> SpreadsheetImportMap {
        let mut result = map(name, source, worksheet, ElementKind::Block, target, columns);
        result.relationship_kind = configured_kind;
        result.relationship_identity = identity;
        result
    }

    fn seed_element(
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

    fn seed_requirement(
        project: &mut Project,
        owner: ElementId,
        name: &str,
        requirement_id: &str,
        external_id: &str,
    ) -> ElementId {
        let id = project
            .create_requirement(name, requirement_id, format!("{name} text"), owner)
            .unwrap();
        project
            .set_external_id(id, external_key(NS, external_id))
            .unwrap();
        id
    }

    fn basic_link_columns(with_owner: bool) -> Vec<(&'static str, SpreadsheetSemanticProperty)> {
        let mut columns = vec![
            ("Link Identifier", SpreadsheetSemanticProperty::ExternalId),
            ("Link Type", SpreadsheetSemanticProperty::RelationshipKind),
            ("Design Object", SpreadsheetSemanticProperty::Source),
            ("Requirement Number", SpreadsheetSemanticProperty::Target),
            ("Description", SpreadsheetSemanticProperty::Documentation),
        ];
        if with_owner {
            columns.push(("Relationship Package", SpreadsheetSemanticProperty::Owner));
        }
        columns
    }

    fn fixture_group(program: ElementId) -> SpreadsheetImportMapGroup {
        let fixture = fixture_path();
        let packages = map(
            "Packages",
            fixture.clone(),
            Some("Package Catalog"),
            ElementKind::Package,
            program,
            &[
                ("Package Key", SpreadsheetSemanticProperty::ExternalId),
                ("Package Label", SpreadsheetSemanticProperty::Name),
                ("Parent Package", SpreadsheetSemanticProperty::Owner),
            ],
        );
        let requirements = map(
            "Requirements",
            fixture.clone(),
            Some("Requirement Catalog"),
            ElementKind::Requirement,
            program,
            &[
                ("Requirement External Key", SpreadsheetSemanticProperty::ExternalId),
                ("Requirement Number", SpreadsheetSemanticProperty::RequirementId),
                ("Requirement Title", SpreadsheetSemanticProperty::Name),
                ("Requirement Statement", SpreadsheetSemanticProperty::RequirementText),
                ("Parent Package", SpreadsheetSemanticProperty::Owner),
            ],
        );
        let design = map(
            "Design",
            fixture.clone(),
            Some("Design Catalog"),
            ElementKind::Block,
            program,
            &[
                ("Design Key", SpreadsheetSemanticProperty::ExternalId),
                ("Design Name", SpreadsheetSemanticProperty::Name),
                ("Parent Package", SpreadsheetSemanticProperty::Owner),
            ],
        );
        let verification = map(
            "Verification",
            fixture.clone(),
            Some("Verification Catalog"),
            ElementKind::TestCase,
            program,
            &[
                ("Test Key", SpreadsheetSemanticProperty::ExternalId),
                ("Test Name", SpreadsheetSemanticProperty::Name),
                ("Parent Package", SpreadsheetSemanticProperty::Owner),
            ],
        );
        let links = relationship_map(
            "Requirement Links",
            fixture,
            Some("Requirement Links"),
            program,
            None,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &basic_link_columns(true),
        );
        SpreadsheetImportMapGroup {
            mappings: vec![packages, requirements, design, verification, links],
        }
    }

    fn find_req(project: &Project, requirement_id: &str) -> ElementId {
        project
            .elements
            .values()
            .find(|element| {
                element.kind == ElementKind::Requirement
                    && element.requirement_id.as_deref() == Some(requirement_id)
            })
            .unwrap()
            .id
    }

    fn find_ext(project: &Project, external_id: &str) -> ElementId {
        project
            .elements
            .values()
            .find(|element| element.external_id == external_key(NS, external_id))
            .unwrap()
            .id
    }

    #[test]
    fn pr41_xlsx_ordered_group_creates_all_six_kinds_with_correct_direction_and_plan_local_resolution() {
        let (state, _root, program) = workspace("PR41 XLSX");
        let group = fixture_group(program);
        let before_elements = state.project.lock().unwrap().as_ref().unwrap().elements.len();
        let before_relationships = state
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .relationships
            .len();

        let preview = preview_spreadsheet_import_group(&group, &state);
        assert!(preview.is_valid(), "{:?}", preview.diagnostics);
        assert_eq!(preview.totals.create, 18);
        assert_eq!(state.project.lock().unwrap().as_ref().unwrap().elements.len(), before_elements);
        assert_eq!(
            state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .relationships
                .len(),
            before_relationships
        );

        let project_snapshot = state.project.lock().unwrap().as_ref().unwrap().clone();
        let prepared = prepare_spreadsheet_import(&group, &project_snapshot);
        let relationship_kinds = prepared
            .plan
            .operations
            .iter()
            .filter_map(|operation| match operation {
                ModelBuildOperation::CreateRelationship { kind, .. } => Some(kind.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        for kind in [
            RelationshipKind::DeriveRequirement,
            RelationshipKind::Satisfy,
            RelationshipKind::Verify,
            RelationshipKind::Refine,
            RelationshipKind::Trace,
            RelationshipKind::Copy,
        ] {
            assert!(relationship_kinds.contains(&kind), "missing {kind:?} in ModelBuildPlan");
        }
        assert_eq!(state.project.lock().unwrap().as_ref().unwrap().relationships.len(), 0);

        apply_spreadsheet_import_group(&group, &state).unwrap();
        let guard = state.project.lock().unwrap();
        let project = guard.as_ref().unwrap();
        assert_eq!(project.relationships.len(), 6);
        assert_eq!(project.elements.len(), 14);

        let req_sys_1 = find_req(project, "REQ-SYS-001");
        let req_sys_2 = find_req(project, "REQ-SYS-002");
        let req_sub_1 = find_req(project, "REQ-SUB-001");
        let req_copy_1 = find_req(project, "REQ-COPY-001");
        let brake = find_ext(project, "BLK-BRAKE");
        let vehicle = find_ext(project, "BLK-VEH");
        let behavior = find_ext(project, "BLK-BEHAV");
        let test = find_ext(project, "TC-BRAKE");

        let relationship = |external_id: &str| {
            project
                .relationships
                .values()
                .find(|relationship| relationship.external_id == external_key(NS, external_id))
                .unwrap()
        };
        let derive = relationship("DER-1");
        assert_eq!(derive.kind, RelationshipKind::DeriveRequirement);
        assert_eq!((derive.source_id, derive.target_id), (req_sub_1, req_sys_1));
        let satisfy = relationship("SAT-1");
        assert_eq!(satisfy.kind, RelationshipKind::Satisfy);
        assert_eq!((satisfy.source_id, satisfy.target_id), (brake, req_sub_1));
        let verify = relationship("VER-1");
        assert_eq!(verify.kind, RelationshipKind::Verify);
        assert_eq!((verify.source_id, verify.target_id), (test, req_sub_1));
        let refine = relationship("REF-1");
        assert_eq!(refine.kind, RelationshipKind::Refine);
        assert_eq!((refine.source_id, refine.target_id), (behavior, req_sys_2));
        let copy = relationship("COPY-1");
        assert_eq!(copy.kind, RelationshipKind::Copy);
        assert_eq!((copy.source_id, copy.target_id), (req_copy_1, req_sys_1));
        let trace = relationship("TRACE-1");
        assert_eq!(trace.kind, RelationshipKind::Trace);
        assert_eq!((trace.source_id, trace.target_id), (vehicle, req_sys_2));

        let traceability = find_ext(project, "PKG-TRC");
        assert!(project.relationships.values().all(|relationship| relationship.owner_id == Some(traceability)));
        let verified_requirement = project.element(req_sub_1).unwrap();
        assert_eq!(verified_requirement.requirement_id.as_deref(), Some("REQ-SUB-001"));
        assert_eq!(verified_requirement.requirement_text.as_deref(), Some("Brake text"));
        drop(guard);

        let second = preview_spreadsheet_import_group(&group, &state);
        assert!(second.is_valid(), "{:?}", second.diagnostics);
        assert_eq!(second.totals.no_change, 18);
        assert_eq!(second.totals.create, 0);
        assert_eq!(second.totals.update, 0);
    }

    #[test]
    fn pr41_ownerless_business_columns_use_explicit_package_target_not_loose_root() {
        let (state, root, program) = workspace("PR41 Owner");
        {
            let mut guard = state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            seed_element(project, program, ElementKind::Block, "BrakeSystem", "BLK-1");
            seed_requirement(project, program, "Brake Requirement", "REQ-1", "REQEXT-1");
        }
        let source = temp_csv(
            "Link Identifier,Link Type,Design Object,Requirement Number,Description\nSAT-OWNER,Satisfy,BLK-1,REQ-1,ownerless business mapping\n",
        );
        let mapping = relationship_map(
            "Requirement Links",
            source.clone(),
            None,
            program,
            None,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &basic_link_columns(false),
        );
        let group = SpreadsheetImportMapGroup { mappings: vec![mapping] };
        let preview = preview_spreadsheet_import_group(&group, &state);
        assert!(preview.is_valid(), "{:?}", preview.diagnostics);
        apply_spreadsheet_import_group(&group, &state).unwrap();
        let guard = state.project.lock().unwrap();
        let project = guard.as_ref().unwrap();
        assert_eq!(project.relationships.len(), 1);
        assert_eq!(project.relationships.values().next().unwrap().owner_id, Some(program));
        drop(guard);

        let root_mapping = relationship_map(
            "Loose Root",
            source,
            None,
            root,
            None,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &basic_link_columns(false),
        );
        let blocked = preview_spreadsheet_import_group(
            &SpreadsheetImportMapGroup { mappings: vec![root_mapping] },
            &state,
        );
        assert!(!blocked.is_valid());
        assert!(blocked.diagnostics.iter().any(|diagnostic| diagnostic.code == "RELATIONSHIP_OWNER_REQUIRED"));
    }

    #[test]
    fn pr41_requirement_id_ambiguity_unresolved_and_unknown_kind_are_blocked() {
        let (state, _root, program) = workspace("PR41 Requirement IDs");
        {
            let mut guard = state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            seed_element(project, program, ElementKind::Block, "Design", "BLK-1");
            let first = seed_element(project, program, ElementKind::Requirement, "First", "REQEXT-A");
            let second = seed_element(project, program, ElementKind::Requirement, "Second", "REQEXT-B");
            project.element_mut(first).unwrap().requirement_id = Some("REQ-DUP".into());
            project.element_mut(second).unwrap().requirement_id = Some("REQ-DUP".into());
        }

        let ambiguous_source = temp_csv(
            "Link Identifier,Link Type,Design Object,Requirement Number,Description\nSAT-A,Satisfy,BLK-1,REQ-DUP,ambiguous\n",
        );
        let ambiguous_map = relationship_map(
            "Ambiguous Requirement",
            ambiguous_source,
            None,
            program,
            None,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &basic_link_columns(false),
        );
        let ambiguous = preview_spreadsheet_import_group(
            &SpreadsheetImportMapGroup { mappings: vec![ambiguous_map] },
            &state,
        );
        assert!(!ambiguous.is_valid());
        assert!(ambiguous.diagnostics.iter().any(|diagnostic| diagnostic.code == "AMBIGUOUS_REQUIREMENT_ID"));

        let missing_source = temp_csv(
            "Link Identifier,Link Type,Design Object,Requirement Number,Description\nSAT-M,Satisfy,BLK-1,REQ-MISSING,missing\n",
        );
        let missing_map = relationship_map(
            "Missing Requirement",
            missing_source,
            None,
            program,
            None,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &basic_link_columns(false),
        );
        let missing = preview_spreadsheet_import_group(
            &SpreadsheetImportMapGroup { mappings: vec![missing_map] },
            &state,
        );
        assert!(!missing.is_valid());
        assert!(missing.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "TARGET_UNRESOLVED" && diagnostic.reason.contains("Requirement ID")
        }));

        let unknown_source = temp_csv(
            "Link Identifier,Link Type,Design Object,Requirement Number,Description\nALLOC-1,allocate,BLK-1,REQ-DUP,unsupported\n",
        );
        let unknown_map = relationship_map(
            "Unknown Kind",
            unknown_source,
            None,
            program,
            None,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &basic_link_columns(false),
        );
        let unknown = preview_spreadsheet_import_group(
            &SpreadsheetImportMapGroup { mappings: vec![unknown_map] },
            &state,
        );
        assert!(!unknown.is_valid());
        assert!(unknown.diagnostics.iter().any(|diagnostic| diagnostic.code == "RELATIONSHIP_KIND_UNSUPPORTED"));
        assert_eq!(state.project.lock().unwrap().as_ref().unwrap().relationships.len(), 0);
    }

    #[test]
    fn pr41_invalid_satisfy_verify_derive_and_copy_endpoints_are_blocked_without_mutation() {
        let (state, _root, program) = workspace("PR41 Invalid Endpoints");
        {
            let mut guard = state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            seed_element(project, program, ElementKind::Block, "BlockOne", "BLK-1");
            seed_element(project, program, ElementKind::Block, "BlockTwo", "BLK-2");
            seed_element(project, program, ElementKind::TestCase, "TestOne", "TC-1");
            seed_requirement(project, program, "Req One", "REQ-1", "REQEXT-1");
            seed_requirement(project, program, "Req Two", "REQ-2", "REQEXT-2");
        }
        let cases = [
            ("BAD-SAT-SRC", "Satisfy", "REQEXT-1", "REQ-2"),
            ("BAD-SAT-TGT", "Satisfy", "BLK-1", "BLK-2"),
            ("BAD-VER-SRC", "Verify", "BLK-1", "REQ-1"),
            ("BAD-VER-TGT", "Verify", "TC-1", "BLK-2"),
            ("BAD-DER-SRC", "deriveReqt", "BLK-1", "REQ-1"),
            ("BAD-DER-TGT", "deriveReqt", "REQEXT-1", "BLK-2"),
            ("BAD-COPY-SRC", "Copy", "BLK-1", "REQ-1"),
            ("BAD-COPY-TGT", "Copy", "REQEXT-1", "BLK-2"),
        ];
        for (id, kind, source_value, target_value) in cases {
            let source = temp_csv(&format!(
                "Link Identifier,Link Type,Design Object,Requirement Number,Description\n{id},{kind},{source_value},{target_value},invalid\n"
            ));
            let mapping = relationship_map(
                id,
                source,
                None,
                program,
                None,
                SpreadsheetRelationshipIdentityPolicy::ExternalId,
                &basic_link_columns(false),
            );
            let preview = preview_spreadsheet_import_group(
                &SpreadsheetImportMapGroup { mappings: vec![mapping] },
                &state,
            );
            assert!(!preview.is_valid(), "{id} unexpectedly valid: {:?}", preview.diagnostics);
            assert_eq!(state.project.lock().unwrap().as_ref().unwrap().relationships.len(), 0);
        }
    }

    #[test]
    fn pr41_reimport_no_change_valid_update_invalid_update_duplicate_and_kind_mismatch() {
        let (state, _root, program) = workspace("PR41 Reimport");
        let (block_one, block_two, requirement, testcase) = {
            let mut guard = state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            let block_one = seed_element(project, program, ElementKind::Block, "BlockOne", "BLK-1");
            let block_two = seed_element(project, program, ElementKind::Block, "BlockTwo", "BLK-2");
            let requirement = seed_requirement(project, program, "Req", "REQ-1", "REQEXT-1");
            let testcase = seed_element(project, program, ElementKind::TestCase, "Test", "TC-1");
            (block_one, block_two, requirement, testcase)
        };

        let first_source = temp_csv(
            "Link Identifier,Link Type,Design Object,Requirement Number,Description\nSAT-1,Satisfy,BLK-1,REQ-1,first\n",
        );
        let first_map = relationship_map(
            "Satisfy",
            first_source,
            None,
            program,
            None,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &basic_link_columns(false),
        );
        let first_group = SpreadsheetImportMapGroup { mappings: vec![first_map] };
        apply_spreadsheet_import_group(&first_group, &state).unwrap();
        let relationship_id = state
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .relationships
            .values()
            .next()
            .unwrap()
            .id;
        let same = preview_spreadsheet_import_group(&first_group, &state);
        assert_eq!(same.totals.no_change, 1);

        let update_source = temp_csv(
            "Link Identifier,Link Type,Design Object,Requirement Number,Description\nSAT-1,Satisfy,BLK-2,REQ-1,updated\n",
        );
        let update_map = relationship_map(
            "Satisfy Update",
            update_source,
            None,
            program,
            None,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &basic_link_columns(false),
        );
        let update_group = SpreadsheetImportMapGroup { mappings: vec![update_map] };
        let update_preview = preview_spreadsheet_import_group(&update_group, &state);
        assert!(update_preview.is_valid(), "{:?}", update_preview.diagnostics);
        assert_eq!(update_preview.totals.update, 1);
        apply_spreadsheet_import_group(&update_group, &state).unwrap();
        {
            let guard = state.project.lock().unwrap();
            let relationship = guard.as_ref().unwrap().relationship(relationship_id).unwrap();
            assert_eq!(relationship.id, relationship_id);
            assert_eq!(relationship.source_id, block_two);
            assert_eq!(relationship.target_id, requirement);
            assert_ne!(relationship.source_id, block_one);
        }

        let invalid_source = temp_csv(
            "Link Identifier,Link Type,Design Object,Requirement Number,Description\nSAT-1,Satisfy,REQEXT-1,REQ-1,invalid update\n",
        );
        let invalid_map = relationship_map(
            "Invalid Update",
            invalid_source,
            None,
            program,
            None,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &basic_link_columns(false),
        );
        let invalid = preview_spreadsheet_import_group(
            &SpreadsheetImportMapGroup { mappings: vec![invalid_map] },
            &state,
        );
        assert!(!invalid.is_valid());
        assert_eq!(
            state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .relationship(relationship_id)
                .unwrap()
                .source_id,
            block_two
        );

        let mismatch_source = temp_csv(
            "Link Identifier,Link Type,Design Object,Requirement Number,Description\nSAT-1,Verify,TC-1,REQ-1,wrong kind\n",
        );
        let mismatch_map = relationship_map(
            "Kind Mismatch",
            mismatch_source,
            None,
            program,
            None,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &basic_link_columns(false),
        );
        let mismatch = preview_spreadsheet_import_group(
            &SpreadsheetImportMapGroup { mappings: vec![mismatch_map] },
            &state,
        );
        assert!(!mismatch.is_valid());
        assert!(mismatch.diagnostics.iter().any(|diagnostic| diagnostic.code == "RELATIONSHIP_IDENTITY_KIND_MISMATCH"));

        let duplicate_source = temp_csv(
            "Link Identifier,Link Type,Design Object,Requirement Number,Description\nDUP-1,Satisfy,BLK-1,REQ-1,a\nDUP-1,Satisfy,BLK-2,REQ-1,b\n",
        );
        let duplicate_map = relationship_map(
            "Duplicate",
            duplicate_source,
            None,
            program,
            None,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &basic_link_columns(false),
        );
        let duplicate = preview_spreadsheet_import_group(
            &SpreadsheetImportMapGroup { mappings: vec![duplicate_map] },
            &state,
        );
        assert!(!duplicate.is_valid());
        assert!(duplicate.diagnostics.iter().any(|diagnostic| diagnostic.code == "DUPLICATE_SOURCE_EXTERNAL_ID"));
        assert_eq!(state.project.lock().unwrap().as_ref().unwrap().relationships.len(), 1);
        assert_eq!(testcase, find_ext(state.project.lock().unwrap().as_ref().unwrap(), "TC-1"));
    }

    #[test]
    fn pr41_fallback_identity_requires_configuration_and_ambiguity_blocks() {
        let (state, _root, program) = workspace("PR41 Fallback");
        {
            let mut guard = state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            seed_element(project, program, ElementKind::Block, "Block", "BLK-1");
            seed_requirement(project, program, "Req", "REQ-1", "REQEXT-1");
        }
        let source = temp_csv("Link Type,Design Object,Requirement Number\nSatisfy,BLK-1,REQ-1\n");
        let columns = [
            ("Link Type", SpreadsheetSemanticProperty::RelationshipKind),
            ("Design Object", SpreadsheetSemanticProperty::Source),
            ("Requirement Number", SpreadsheetSemanticProperty::Target),
        ];
        let unconfigured = relationship_map(
            "No Fallback",
            source.clone(),
            None,
            program,
            None,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &columns,
        );
        let blocked = preview_spreadsheet_import_group(
            &SpreadsheetImportMapGroup { mappings: vec![unconfigured] },
            &state,
        );
        assert!(!blocked.is_valid());
        assert!(blocked.diagnostics.iter().any(|diagnostic| diagnostic.code == "RELATIONSHIP_EXTERNAL_ID_REQUIRED"));

        let configured = relationship_map(
            "Configured Fallback",
            source,
            None,
            program,
            None,
            SpreadsheetRelationshipIdentityPolicy::KindSourceTarget,
            &columns,
        );
        let group = SpreadsheetImportMapGroup { mappings: vec![configured] };
        let preview = preview_spreadsheet_import_group(&group, &state);
        assert!(preview.is_valid(), "{:?}", preview.diagnostics);
        assert_eq!(preview.totals.create, 1);
        apply_spreadsheet_import_group(&group, &state).unwrap();
        assert_eq!(preview_spreadsheet_import_group(&group, &state).totals.no_change, 1);

        {
            let mut guard = state.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            let first_id = project.relationships.values().next().unwrap().id;
            let mut duplicate = project.relationship(first_id).unwrap().clone();
            project.relationships.get_mut(&first_id).unwrap().external_id = "manual::one".into();
            duplicate.id = RelationshipId::new();
            duplicate.external_id = "manual::two".into();
            project.relationships.insert(duplicate.id, duplicate);
        }
        let ambiguous = preview_spreadsheet_import_group(&group, &state);
        assert!(!ambiguous.is_valid());
        assert!(ambiguous.diagnostics.iter().any(|diagnostic| diagnostic.code == "AMBIGUOUS_RELATIONSHIP"));
    }

    #[test]
    fn pr41_late_invalid_verify_rolls_back_whole_mapgroup() {
        let (state, _root, program) = workspace("PR41 Atomic");
        let before = state.project.lock().unwrap().as_ref().unwrap().elements.len();
        let package_source = temp_csv(
            "Package Key,Package Label\nPKG-REQ,Requirements\nPKG-DES,Design\nPKG-VER,Verification\n",
        );
        let block_source = temp_csv(
            "Design Key,Design Name,Parent Package\nBLK-BRAKE,BrakeSystem,Design\nBLK-CONTROLLER,BrakeController,Design\n",
        );
        let requirement_source = temp_csv(
            "Requirement External Key,Requirement Number,Requirement Title,Requirement Statement,Parent Package\nREQEXT-1,REQ-1,Brake Requirement,Brake text,Requirements\n",
        );
        let test_source = temp_csv(
            "Test Key,Test Name,Parent Package\nTC-1,BrakeTest,Verification\n",
        );
        let links_source = temp_csv(
            "Link Identifier,Link Type,Design Object,Requirement Number,Description\nSAT-1,Satisfy,BLK-BRAKE,REQ-1,valid\nVER-1,Verify,TC-1,REQ-1,valid\nVER-BAD,Verify,BLK-CONTROLLER,REQ-1,invalid last row\n",
        );
        let packages = map(
            "Packages",
            package_source,
            None,
            ElementKind::Package,
            program,
            &[
                ("Package Key", SpreadsheetSemanticProperty::ExternalId),
                ("Package Label", SpreadsheetSemanticProperty::Name),
            ],
        );
        let blocks = map(
            "Blocks",
            block_source,
            None,
            ElementKind::Block,
            program,
            &[
                ("Design Key", SpreadsheetSemanticProperty::ExternalId),
                ("Design Name", SpreadsheetSemanticProperty::Name),
                ("Parent Package", SpreadsheetSemanticProperty::Owner),
            ],
        );
        let requirements = map(
            "Requirements",
            requirement_source,
            None,
            ElementKind::Requirement,
            program,
            &[
                ("Requirement External Key", SpreadsheetSemanticProperty::ExternalId),
                ("Requirement Number", SpreadsheetSemanticProperty::RequirementId),
                ("Requirement Title", SpreadsheetSemanticProperty::Name),
                ("Requirement Statement", SpreadsheetSemanticProperty::RequirementText),
                ("Parent Package", SpreadsheetSemanticProperty::Owner),
            ],
        );
        let tests = map(
            "Tests",
            test_source,
            None,
            ElementKind::TestCase,
            program,
            &[
                ("Test Key", SpreadsheetSemanticProperty::ExternalId),
                ("Test Name", SpreadsheetSemanticProperty::Name),
                ("Parent Package", SpreadsheetSemanticProperty::Owner),
            ],
        );
        let links = relationship_map(
            "Links",
            links_source,
            None,
            program,
            None,
            SpreadsheetRelationshipIdentityPolicy::ExternalId,
            &basic_link_columns(false),
        );
        let group = SpreadsheetImportMapGroup {
            mappings: vec![packages, blocks, requirements, tests, links],
        };
        let preview = preview_spreadsheet_import_group(&group, &state);
        assert!(!preview.is_valid());
        assert_eq!(state.project.lock().unwrap().as_ref().unwrap().elements.len(), before);
        assert_eq!(state.project.lock().unwrap().as_ref().unwrap().relationships.len(), 0);
        assert!(apply_spreadsheet_import_group(&group, &state).is_err());
        assert_eq!(state.project.lock().unwrap().as_ref().unwrap().elements.len(), before);
        assert_eq!(state.project.lock().unwrap().as_ref().unwrap().relationships.len(), 0);
    }
}
'''

source_path.write_text(text, encoding="utf-8")
