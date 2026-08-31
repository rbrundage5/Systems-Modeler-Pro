from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
path = ROOT / "apps/desktop/src-tauri/src/workspace/spreadsheet_import.rs"
text = path.read_text(encoding="utf-8")

# Keep the relationship update comparison explicit; no sentinel semantic ID is needed.
old = '''    let owner_changed = !relationship_reference_matches(&owner.reference, relationship.owner_id.unwrap_or(project_sentinel_element_id()));\n'''
new = '''    let owner_changed = match (&owner.reference, relationship.owner_id) {\n        (BuildReference::Existing(id), Some(existing)) => *id != existing,\n        (BuildReference::Existing(_), None) => true,\n        (BuildReference::External(_), _) => true,\n    };\n'''
if old not in text:
    raise SystemExit("relationship owner-change anchor missing")
text = text.replace(old, new, 1)

sentinel = '''\nfn project_sentinel_element_id() -> ElementId {\n    ElementId(uuid::Uuid::nil())\n}\n'''
if sentinel not in text:
    raise SystemExit("relationship sentinel helper missing")
text = text.replace(sentinel, "\n", 1)

# RelationshipId is not needed by the spreadsheet mapping implementation itself.
text = text.replace(
    '''    Relationship, RelationshipId, RelationshipKind, VisibilityKind,\n''',
    '''    Relationship, RelationshipKind, VisibilityKind,\n''',
    1,
)

path.write_text(text, encoding="utf-8")
print("PR40 focused fixes applied")
