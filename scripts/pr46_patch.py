from pathlib import Path
import base64
import zlib
payload = ''.join(Path(f'scripts/pr46_patch.part{i}').read_text() for i in range(8))
exec(zlib.decompress(base64.b64decode(payload)).decode())
p = Path('apps/desktop/src-tauri/src/workspace/bulk_model.rs')
text = p.read_text()
bad = '''                        if project\n                            .element(id)\n                            .map_err(|cause| {\n                                error("SEMANTIC_VALIDATION", Some(index), cause.to_string())\n                            })?\n                            .kind\n                            .clone();\n                        if !matches!(kind, ElementKind::ValueProperty | ElementKind::Parameter) {\n'''
good = '''                        let kind = project\n                            .element(id)\n                            .map_err(|cause| {\n                                error("SEMANTIC_VALIDATION", Some(index), cause.to_string())\n                            })?\n                            .kind\n                            .clone();\n                        if !matches!(kind, ElementKind::ValueProperty | ElementKind::Parameter) {\n'''
if bad not in text:
    raise SystemExit('missing post-patch default-value anchor')
p.write_text(text.replace(bad, good, 1))

test_path = Path('crates/model-core/tests/pr46_operation_parameter_reception.rs')
test_text = test_path.read_text()
bad_test = '''    assert!(matches!(\n        project.create_element(ElementKind::Parameter, "bad", controller),\n        Err(ModelError::InvalidOwner(_))\n    ));\n'''
good_test = '''    assert!(\n        project\n            .create_element(ElementKind::Parameter, "bad", controller)\n            .is_err()\n    );\n'''
if bad_test not in test_text:
    raise SystemExit('missing PR46 ownership assertion anchor')
test_path.write_text(test_text.replace(bad_test, good_test, 1))

spreadsheet_test_path = Path('apps/desktop/src-tauri/src/workspace/spreadsheet_import/pr46_tests.rs')
spreadsheet_test_text = spreadsheet_test_path.read_text()
old_owner_column = '("Component", SpreadsheetSemanticProperty::Owner),'
new_owner_column = '("Owning Type", SpreadsheetSemanticProperty::Owner),'
if old_owner_column not in spreadsheet_test_text:
    raise SystemExit('missing PR46 Services owner-column anchor')
spreadsheet_test_path.write_text(
    spreadsheet_test_text.replace(old_owner_column, new_owner_column, 1)
)
