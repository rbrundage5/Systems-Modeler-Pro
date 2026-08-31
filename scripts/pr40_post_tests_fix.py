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
text = text.replace(old, '''        assert!(state.project.lock().unwrap().as_ref().unwrap().element(note).is_ok());\n''', 1)
path.write_text(text, encoding="utf-8")
print("PR40 post-test fix applied")
