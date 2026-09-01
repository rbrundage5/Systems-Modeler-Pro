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

signals_anchor = '''    let signals = basic_map(\n        "Signals",\n        fixture.clone(),\n        Some("Signals"),\n        ElementKind::Signal,\n        root,\n        "Signal ID",\n        "Signal Name",\n    );\n    SpreadsheetImportMapGroup {\n        mappings: vec![\n            components,\n            enumerations,\n            value_types,\n            primitives,\n            signals,\n            operation_map(fixture.clone(), Some("Services"), root),\n'''
signals_replacement = '''    let signals = basic_map(\n        "Signals",\n        fixture.clone(),\n        Some("Signals"),\n        ElementKind::Signal,\n        root,\n        "Signal ID",\n        "Signal Name",\n    );\n    let mut operations = operation_map(fixture.clone(), Some("Services"), root);\n    operations\n        .column_mappings\n        .iter_mut()\n        .find(|mapping| mapping.property == SpreadsheetSemanticProperty::Owner)\n        .expect("Operation mapping includes owner")\n        .source_column = "Owning Type".into();\n    SpreadsheetImportMapGroup {\n        mappings: vec![\n            components,\n            enumerations,\n            value_types,\n            primitives,\n            signals,\n            operations,\n'''
if signals_anchor not in spreadsheet_test_text:
    raise SystemExit('missing PR46 XLSX mapping anchor')
spreadsheet_test_text = spreadsheet_test_text.replace(signals_anchor, signals_replacement, 1)

by_external_anchor = '''        .find(|element| element.external_id == key)\n        .unwrap()\n'''
by_external_replacement = '''        .find(|element| element.external_id == key)\n        .unwrap_or_else(|| panic!("missing imported element with external ID {key}"))\n'''
if by_external_anchor not in spreadsheet_test_text:
    raise SystemExit('missing PR46 by_external assertion anchor')
spreadsheet_test_text = spreadsheet_test_text.replace(by_external_anchor, by_external_replacement, 1)

fixture_identity_replacements = {
    'PARAM-START-MODE': 'PARAM-MODE',
    'PARAM-CALC-RESULT': 'PARAM-RESULT',
    'OP-CALCULATE': 'OP-CALC',
}
for old, new in fixture_identity_replacements.items():
    if old not in spreadsheet_test_text:
        raise SystemExit(f'missing PR46 fixture identity anchor: {old}')
    spreadsheet_test_text = spreadsheet_test_text.replace(old, new)

spreadsheet_test_path.write_text(spreadsheet_test_text)
