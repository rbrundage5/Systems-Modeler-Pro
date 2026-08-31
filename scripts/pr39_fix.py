from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]
path = ROOT / "apps/desktop/src-tauri/src/workspace/spreadsheet_import.rs"
text = path.read_text(encoding="utf-8")

# This importer intentionally carries rich contextual diagnostics (source, worksheet,
# row, column, semantic property, identity, and reason). Keep the structured value
# rather than boxing every internal Result boundary merely to satisfy size heuristics.
if not text.startswith("#![allow(clippy::result_large_err)]"):
    text = "#![allow(clippy::result_large_err)]\n\n" + text

# PR39's supported-kind replacement spans the old namespace helper. Restore the
# PR38 namespace predicate before applying the compatibility ownership guard.
if "fn is_namespace_kind(kind: &ElementKind) -> bool {" not in text:
    anchor = "fn is_feature_kind(kind: &ElementKind) -> bool {"
    if anchor not in text:
        raise SystemExit("feature-kind anchor missing")
    helper = '''fn is_namespace_kind(kind: &ElementKind) -> bool {\n    matches!(\n        kind,\n        ElementKind::Model | ElementKind::Package | ElementKind::ModelLibrary\n    )\n}\n\n'''
    text = text.replace(anchor, helper + anchor, 1)

# PR38 package/basic-element mappings only permit namespace owners. PR39 feature
# mappings intentionally resolve structural owners and defer legality to model validation.
needle = '''            let type_resolution = if is_feature_kind(&map.element_kind) {\n'''
insert = '''            if !is_feature_kind(&map.element_kind) && !is_namespace_kind(&owner.kind) {\n                block_row(diagnostic(\n                    Some(map),\n                    Some(row.row_number),\n                    mapped_column_name(map, SpreadsheetSemanticProperty::Owner),\n                    Some(SpreadsheetSemanticProperty::Owner),\n                    id_value.clone(),\n                    "INVALID_OWNERSHIP",\n                    format!(\n                        "{:?} cannot be owned by {:?} in the PR38 packageable-element scope",\n                        map.element_kind, owner.kind\n                    ),\n                ));\n                continue;\n            }\n\n            let type_resolution = if is_feature_kind(&map.element_kind) {\n'''
if needle not in text:
    raise SystemExit("type-resolution anchor missing")
text = text.replace(needle, insert, 1)

# These PR38 planning fields became redundant after PR39 generalized exact reference
# resolution. Keep only the identity/path data actually used by pending-plan resolution.
text = text.replace('''struct PlannedElement {\n    external_id: String,\n    kind: ElementKind,\n    name: String,\n    qualified_name: String,\n    owner: ElementReference,\n    depth_from_target: usize,\n}\n''', '''struct PlannedElement {\n    external_id: String,\n    kind: ElementKind,\n    qualified_name: String,\n    depth_from_target: usize,\n}\n''', 1)
text = re.sub(r'\n\s*name: name\.to_string\(\),\n\s*qualified_name,\n\s*owner: owner\.reference,\n', '\n                qualified_name,\n', text)

# Keep the field-diff helper explicit; the arguments correspond directly to resolved
# semantic inputs and avoiding a wrapper struct keeps this focused PR smaller.
needle = "fn mapped_field_changes(\n"
if needle in text and "#[allow(clippy::too_many_arguments)]\nfn mapped_field_changes(" not in text:
    text = text.replace(needle, "#[allow(clippy::too_many_arguments)]\n" + needle, 1)

# Preserve the compact PR38/PR39 test fixture constructor without introducing a test-only
# options struct solely for Clippy's argument-count heuristic.
needle = "    fn map(\n"
if needle in text and "    #[allow(clippy::too_many_arguments)]\n    fn map(" not in text:
    text = text.replace(needle, "    #[allow(clippy::too_many_arguments)]\n" + needle, 1)

# Apply Clippy's equivalent flattening for the two inherited nested conditionals so the
# module remains clean under -D warnings without suppressing collapsible_if globally.
old = '''            if map.element_kind == ElementKind::Requirement {\n                if non_empty_value(&values, SpreadsheetSemanticProperty::RequirementId).is_none()\n                    || non_empty_value(&values, SpreadsheetSemanticProperty::RequirementText).is_none()\n                {\n                    block_row(diagnostic(\n                        Some(map),\n                        Some(row.row_number),\n                        None,\n                        None,\n                        id_value.clone(),\n                        "REQUIREMENT_FIELDS_REQUIRED",\n                        "new Requirement rows require mapped, non-empty Requirement ID and Requirement Text",\n                    ));\n                    continue;\n                }\n            }\n'''
new = '''            if map.element_kind == ElementKind::Requirement\n                && (non_empty_value(&values, SpreadsheetSemanticProperty::RequirementId).is_none()\n                    || non_empty_value(&values, SpreadsheetSemanticProperty::RequirementText).is_none())\n            {\n                block_row(diagnostic(\n                    Some(map),\n                    Some(row.row_number),\n                    None,\n                    None,\n                    id_value.clone(),\n                    "REQUIREMENT_FIELDS_REQUIRED",\n                    "new Requirement rows require mapped, non-empty Requirement ID and Requirement Text",\n                ));\n                continue;\n            }\n'''
if old not in text:
    raise SystemExit("requirement-collapse anchor missing")
text = text.replace(old, new, 1)

old = '''        if let Some(row) = spreadsheet_diagnostic.row {\n            if let Some(row_preview) = preview.rows.iter_mut().find(|candidate| {\n                candidate.row == row\n                    && spreadsheet_diagnostic\n                        .import_map\n                        .as_deref()\n                        .is_some_and(|map| candidate.import_map == map)\n            }) {\n                row_preview.action = match spreadsheet_diagnostic.severity {\n                    SpreadsheetDiagnosticSeverity::Error => SpreadsheetRowAction::Blocked,\n                    SpreadsheetDiagnosticSeverity::Warning => SpreadsheetRowAction::Warning,\n                };\n            }\n        }\n'''
new = '''        if let Some(row) = spreadsheet_diagnostic.row\n            && let Some(row_preview) = preview.rows.iter_mut().find(|candidate| {\n                candidate.row == row\n                    && spreadsheet_diagnostic\n                        .import_map\n                        .as_deref()\n                        .is_some_and(|map| candidate.import_map == map)\n            })\n        {\n            row_preview.action = match spreadsheet_diagnostic.severity {\n                SpreadsheetDiagnosticSeverity::Error => SpreadsheetRowAction::Blocked,\n                SpreadsheetDiagnosticSeverity::Warning => SpreadsheetRowAction::Warning,\n            };\n        }\n'''
if old not in text:
    raise SystemExit("diagnostic-collapse anchor missing")
text = text.replace(old, new, 1)

# Group apply is a focused test helper; production Tauri commands use the command entry point.
needle = '''pub fn apply_spreadsheet_import_group(\n'''
if needle not in text:
    raise SystemExit("group-apply anchor missing")
text = text.replace(needle, '''#[cfg(test)]\npub fn apply_spreadsheet_import_group(\n''', 1)

path.write_text(text, encoding="utf-8")
print("PR39 compatibility/lint fix applied")
