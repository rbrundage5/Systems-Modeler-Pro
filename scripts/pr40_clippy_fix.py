from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]

bulk_path = ROOT / "apps/desktop/src-tauri/src/workspace/bulk_model.rs"
bulk = bulk_path.read_text(encoding="utf-8")
old = '''                    if let Some(key) = &next_external_id {
                        if project
                            .elements
                            .values()
                            .any(|element| element.external_id == *key)
                            || project.relationships.values().any(|candidate| {
                                candidate.id != id && candidate.external_id == *key
                            })
                        {
                            return Err(error(
                                "DUPLICATE_EXTERNAL_ID",
                                Some(index),
                                format!("external ID already exists: {key}"),
                            ));
                        }
                    }
'''
new = '''                    if let Some(key) = &next_external_id
                        && (project
                            .elements
                            .values()
                            .any(|element| element.external_id == *key)
                            || project.relationships.values().any(|candidate| {
                                candidate.id != id && candidate.external_id == *key
                            }))
                    {
                        return Err(error(
                            "DUPLICATE_EXTERNAL_ID",
                            Some(index),
                            format!("external ID already exists: {key}"),
                        ));
                    }
'''
if old not in bulk:
    raise SystemExit("PR40 collapsible-if lint anchor missing")
bulk = bulk.replace(old, new, 1)
bulk_path.write_text(bulk, encoding="utf-8")

spreadsheet_path = ROOT / "apps/desktop/src-tauri/src/workspace/spreadsheet_import.rs"
text = spreadsheet_path.read_text(encoding="utf-8")
text, count = re.subn(r"\.association_ends\s*\.get\(0\)", ".association_ends.first()", text, count=1)
if count != 1:
    raise SystemExit("PR40 association first-element lint anchor missing")
helper = "    fn relationship_map(\n"
if helper not in text:
    raise SystemExit("PR40 relationship-map test helper lint anchor missing")
text = text.replace(
    helper,
    "    #[allow(clippy::too_many_arguments)]\n    fn relationship_map(\n",
    1,
)
spreadsheet_path.write_text(text, encoding="utf-8")
print("PR40 Clippy fixes applied")
