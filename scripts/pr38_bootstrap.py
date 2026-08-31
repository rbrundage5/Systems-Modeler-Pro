from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def replace_once(path: Path, old: str, new: str) -> None:
    text = path.read_text(encoding="utf-8")
    if old not in text:
        raise SystemExit(f"expected PR38 patch anchor not found in {path}: {old[:120]!r}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


bulk = ROOT / "apps/desktop/src-tauri/src/workspace/bulk_model.rs"
replace_once(
    bulk,
    """    UpdateElement {\n        element: ElementReference,\n        name: String,\n    },\n    CreateRelationship {\n""",
    """    UpdateElement {\n        element: ElementReference,\n        name: String,\n    },\n    /// Narrow PR38 compatibility operation for spreadsheet-authored scalar fields.\n    /// It remains inside PR36 so spreadsheet import never gains a mutation path of its own.\n    UpdateElementFields {\n        element: ElementReference,\n        name: Option<String>,\n        owner: Option<ElementReference>,\n        external_id: Option<String>,\n        documentation: Option<String>,\n        visibility: Option<VisibilityKind>,\n        requirement_id: Option<String>,\n        requirement_text: Option<String>,\n    },\n    CreateRelationship {\n""",
)
replace_once(
    bulk,
    "fn external_key(namespace: &str, external_id: &str) -> String {",
    "pub(super) fn external_key(namespace: &str, external_id: &str) -> String {",
)
replace_once(
    bulk,
    """        ModelBuildOperation::UpdateElement { name, .. } => {\n            format!(\"UPDATE element name to {name}\")\n        }\n        ModelBuildOperation::CreateRelationship { external_id, .. } => {\n""",
    """        ModelBuildOperation::UpdateElement { name, .. } => {\n            format!(\"UPDATE element name to {name}\")\n        }\n        ModelBuildOperation::UpdateElementFields { .. } => \"UPDATE mapped element fields\".into(),\n        ModelBuildOperation::CreateRelationship { external_id, .. } => {\n""",
)
replace_once(
    bulk,
    """                ModelBuildOperation::UpdateElement { element, name } => {\n                    let id = resolve_element(&project, &element_ids, namespace, element, index)?;\n                    project.rename_element(id, name).map_err(|cause| {\n                        error(\"SEMANTIC_VALIDATION\", Some(index), cause.to_string())\n                    })?;\n                }\n                ModelBuildOperation::CreateRelationship {\n""",
    """                ModelBuildOperation::UpdateElement { element, name } => {\n                    let id = resolve_element(&project, &element_ids, namespace, element, index)?;\n                    project.rename_element(id, name).map_err(|cause| {\n                        error(\"SEMANTIC_VALIDATION\", Some(index), cause.to_string())\n                    })?;\n                }\n                ModelBuildOperation::UpdateElementFields {\n                    element,\n                    name,\n                    owner,\n                    external_id,\n                    documentation,\n                    visibility,\n                    requirement_id,\n                    requirement_text,\n                } => {\n                    let id = resolve_element(&project, &element_ids, namespace, element, index)?;\n                    if let Some(owner) = owner {\n                        let owner_id =\n                            resolve_element(&project, &element_ids, namespace, owner, index)?;\n                        project.move_element(id, owner_id).map_err(|cause| {\n                            error(\"SEMANTIC_VALIDATION\", Some(index), cause.to_string())\n                        })?;\n                    }\n                    if let Some(name) = name {\n                        project.rename_element(id, name.clone()).map_err(|cause| {\n                            error(\"SEMANTIC_VALIDATION\", Some(index), cause.to_string())\n                        })?;\n                    }\n                    if let Some(external_id) = external_id {\n                        project\n                            .set_external_id(id, external_key(namespace, external_id))\n                            .map_err(|cause| {\n                                error(\"DUPLICATE_EXTERNAL_ID\", Some(index), cause.to_string())\n                            })?;\n                    }\n                    if let Some(documentation) = documentation {\n                        project.element_mut(id).map_err(|cause| {\n                            error(\"SEMANTIC_VALIDATION\", Some(index), cause.to_string())\n                        })?.documentation = documentation.clone();\n                    }\n                    if let Some(visibility) = visibility {\n                        project.element_mut(id).map_err(|cause| {\n                            error(\"SEMANTIC_VALIDATION\", Some(index), cause.to_string())\n                        })?.visibility = *visibility;\n                    }\n                    if requirement_id.is_some() || requirement_text.is_some() {\n                        let current = project.element(id).map_err(|cause| {\n                            error(\"SEMANTIC_VALIDATION\", Some(index), cause.to_string())\n                        })?;\n                        if current.kind != ElementKind::Requirement {\n                            return Err(error(\n                                \"SEMANTIC_VALIDATION\",\n                                Some(index),\n                                \"Requirement ID/Text mappings are valid only for Requirement elements\",\n                            ));\n                        }\n                        let next_requirement_id = requirement_id\n                            .clone()\n                            .or_else(|| current.requirement_id.clone())\n                            .ok_or_else(|| {\n                                error(\n                                    \"SEMANTIC_VALIDATION\",\n                                    Some(index),\n                                    \"Requirement ID is required when applying mapped Requirement fields\",\n                                )\n                            })?;\n                        let next_requirement_text = requirement_text\n                            .clone()\n                            .or_else(|| current.requirement_text.clone())\n                            .unwrap_or_default();\n                        project\n                            .update_requirement(id, next_requirement_id, next_requirement_text)\n                            .map_err(|cause| {\n                                error(\"SEMANTIC_VALIDATION\", Some(index), cause.to_string())\n                            })?;\n                    }\n                }\n                ModelBuildOperation::CreateRelationship {\n""",
)

main = ROOT / "apps/desktop/src-tauri/src/main.rs"
replace_once(
    main,
    "    mod portable_interchange;\n    mod presentation_interaction;\n",
    "    mod portable_interchange;\n    mod spreadsheet_import;\n    mod presentation_interaction;\n",
)
replace_once(
    main,
    "    pub use portable_interchange::{export_portable_project_json, import_portable_project_json};\n",
    "    pub use portable_interchange::{export_portable_project_json, import_portable_project_json};\n    pub use spreadsheet_import::{apply_spreadsheet_import, preview_spreadsheet_import};\n",
)
replace_once(
    main,
    "    export_portable_project_json, fit_diagram_viewport, get_diagram_frame_preference,\n",
    "    apply_spreadsheet_import, export_portable_project_json, fit_diagram_viewport,\n    get_diagram_frame_preference,\n",
)
replace_once(
    main,
    "    history_reset, history_undo, ibd_item_flow_notation, import_portable_project_json,\n",
    "    history_reset, history_undo, ibd_item_flow_notation, import_portable_project_json,\n    preview_spreadsheet_import,\n",
)
replace_once(
    main,
    "            export_portable_project_json,\n            import_portable_project_json,\n",
    "            export_portable_project_json,\n            import_portable_project_json,\n            preview_spreadsheet_import,\n            apply_spreadsheet_import,\n",
)

cargo = ROOT / "apps/desktop/src-tauri/Cargo.toml"
replace_once(
    cargo,
    "systems-modeler-persistence = { path = \"../../../crates/persistence\" }\n",
    "systems-modeler-persistence = { path = \"../../../crates/persistence\" }\ncalamine = \"0.36.1\"\ncsv = \"1.4.0\"\n",
)

spreadsheet = ROOT / "apps/desktop/src-tauri/src/workspace/spreadsheet_import.rs"
replace_once(
    spreadsheet,
    """struct PlannedElement {\n    external_id: String,\n    kind: ElementKind,\n    name: String,\n    qualified_name: String,\n    owner: ElementReference,\n    depth_from_target: usize,\n}\n""",
    """struct PlannedElement {\n    external_id: String,\n    kind: ElementKind,\n    qualified_name: String,\n    depth_from_target: usize,\n}\n""",
)
replace_once(
    spreadsheet,
    """        .elements\n        .values()\n        .filter(|element| element.is_namespace())\n        .filter(|element| existing_owner_in_scope(project, element.id, map.target_scope, map.search_scope))\n""",
    """        .elements\n        .values()\n        .filter(|element| existing_owner_in_scope(project, element.id, map.target_scope, map.search_scope))\n""",
)
replace_once(
    spreadsheet,
    """        .iter()\n        .filter(|element| is_namespace_kind(&element.kind))\n        .filter(|element| match map.search_scope {\n""",
    """        .iter()\n        .filter(|element| match map.search_scope {\n""",
)
replace_once(
    spreadsheet,
    """            planned.push(PlannedElement {\n                external_id: external_id.to_string(),\n                kind: map.element_kind.clone(),\n                name: name.to_string(),\n                qualified_name,\n                owner: owner.reference,\n                depth_from_target: owner.depth_from_target + 1,\n            });\n""",
    """            planned.push(PlannedElement {\n                external_id: external_id.to_string(),\n                kind: map.element_kind.clone(),\n                qualified_name,\n                depth_from_target: owner.depth_from_target + 1,\n            });\n""",
)
replace_once(
    spreadsheet,
    """    if !supported_kind(&map.element_kind) {\n        return Err(diagnostic(\n            Some(map),\n            None,\n            None,\n            None,\n            None,\n            \"ELEMENT_KIND_UNSUPPORTED\",\n            format!(\"{:?} is outside the PR38 basic-element scope\", map.element_kind),\n        ));\n    }\n""",
    """    if !supported_kind(&map.element_kind) {\n        return Err(diagnostic(\n            Some(map),\n            None,\n            None,\n            None,\n            None,\n            \"ELEMENT_KIND_UNSUPPORTED\",\n            format!(\"{:?} is outside the PR38 basic-element scope\", map.element_kind),\n        ));\n    }\n    if map.element_kind != ElementKind::Requirement\n        && map.column_mappings.iter().any(|mapping| {\n            matches!(\n                mapping.property,\n                SpreadsheetSemanticProperty::RequirementId\n                    | SpreadsheetSemanticProperty::RequirementText\n            )\n        })\n    {\n        return Err(diagnostic(\n            Some(map),\n            None,\n            None,\n            None,\n            None,\n            \"SEMANTIC_PROPERTY_INVALID\",\n            \"Requirement ID/Text columns can be mapped only for Requirement elements\",\n        ));\n    }\n""",
)
closure = """            let id_value = identification_value(map, &values);\n            let mut block_row = |mut error: SpreadsheetImportDiagnostic| {\n                error.row = Some(row.row_number);\n                if error.identification_value.is_none() {\n                    error.identification_value = id_value.clone();\n                }\n                preview.diagnostics.push(error);\n                preview.rows.push(row_preview(\n                    map,\n                    row.row_number,\n                    &values,\n                    SpreadsheetRowAction::Blocked,\n                ));\n            };\n\n"""
replace_once(
    spreadsheet,
    closure,
    "            let id_value = identification_value(map, &values);\n\n",
)
text = spreadsheet.read_text(encoding="utf-8")
text = text.replace(
    "block_row(",
    "push_blocked_row(&mut preview, map, row.row_number, &values, ",
)
anchor = "fn prepare_spreadsheet_import(\n"
helper = """fn push_blocked_row(\n    preview: &mut SpreadsheetImportPreview,\n    map: &SpreadsheetImportMap,\n    row_number: usize,\n    values: &BTreeMap<SpreadsheetSemanticProperty, String>,\n    mut error: SpreadsheetImportDiagnostic,\n) {\n    error.row = Some(row_number);\n    if error.identification_value.is_none() {\n        error.identification_value = identification_value(map, values);\n    }\n    preview.diagnostics.push(error);\n    preview.rows.push(row_preview(\n        map,\n        row_number,\n        values,\n        SpreadsheetRowAction::Blocked,\n    ));\n}\n\n"""
if anchor not in text:
    raise SystemExit("prepare_spreadsheet_import anchor missing")
text = text.replace(anchor, helper + anchor, 1)
spreadsheet.write_text(text, encoding="utf-8")

print("PR38 focused integration patch applied")
