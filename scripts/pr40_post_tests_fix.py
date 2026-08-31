from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
path = ROOT / "apps/desktop/src-tauri/src/workspace/spreadsheet_import.rs"
text = path.read_text(encoding="utf-8")

old = '''        assert_eq!(\n            state\n                .project\n                .lock()\n                .unwrap()\n                .as_ref()\n                .unwrap()\n                .relationship(note)\n                .err(),\n            Some(systems_modeler_core::ModelError::RelationshipNotFound(\n                systems_modeler_core::RelationshipId(note.0)\n            ))\n        );\n'''
if old not in text:
    # cargo fmt may not have run yet here; accept the compact source shape emitted by the test script.
    old = '''        assert_eq!(state.project.lock().unwrap().as_ref().unwrap().relationship(note).err(), Some(systems_modeler_core::ModelError::RelationshipNotFound(systems_modeler_core::RelationshipId(note.0))));\n'''
if old not in text:
    raise SystemExit("obsolete PR40 test assertion anchor missing")
text = text.replace(
    old,
    '''        assert!(state.project.lock().unwrap().as_ref().unwrap().element(note).is_ok());\n''',
    1,
)

# A CATIA-style mixed relationship worksheet may expose Association-specific columns
# for every row. For non-Association rows, those columns are semantically absent when
# every mapped Association-end cell is blank. Preserve strict rejection when any such
# cell actually contains a value.
old_end_presence = '''    let mapped = |property| values.contains_key(&property);\n    if ![role_property, multiplicity_property, navigable_property, aggregation_property]\n        .into_iter().any(mapped)\n    {\n        return Ok(None);\n    }\n'''
new_end_presence = '''    if ![role_property, multiplicity_property, navigable_property, aggregation_property]\n        .into_iter()\n        .any(|property| non_empty_value(values, property).is_some())\n    {\n        return Ok(None);\n    }\n'''
if old_end_presence not in text:
    raise SystemExit("association-end presence anchor missing")
text = text.replace(old_end_presence, new_end_presence, 1)

path.write_text(text, encoding="utf-8")
print("PR40 post-test fix applied")
