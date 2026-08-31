from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]
path = ROOT / "apps/desktop/src-tauri/src/workspace/spreadsheet_import.rs"
text = path.read_text(encoding="utf-8")

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

# Group apply is a focused test helper; production Tauri commands use the single-map
# apply entry point. Avoid a production dead-code warning under Clippy -D warnings.
needle = '''pub fn apply_spreadsheet_import_group(\n'''
if needle not in text:
    raise SystemExit("group-apply anchor missing")
text = text.replace(needle, '''#[cfg(test)]\npub fn apply_spreadsheet_import_group(\n''', 1)

path.write_text(text, encoding="utf-8")
print("PR39 compatibility/lint fix applied")
