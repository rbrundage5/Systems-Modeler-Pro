from pathlib import Path

path = Path(__file__).resolve().parents[1] / "apps/desktop/src-tauri/src/workspace/spreadsheet_import.rs"
text = path.read_text(encoding="utf-8")

old = 'temp_csv("ID,Kind,Source,Target,Owner\\nREL-2,Satisfy,VEH,ENG,Structure\\n"),'
new = 'temp_csv("ID,Kind,Source,Target,Owner\\nREL-2,Include,VEH,ENG,Structure\\n"),'
if old not in text:
    raise SystemExit("missing PR40 unsupported-kind test anchor")
text = text.replace(old, new, 1)

old = '''    let allow_requirement_id = match (kind, property) {
        (
            RelationshipKind::DeriveRequirement | RelationshipKind::Copy,
            SpreadsheetSemanticProperty::Source | SpreadsheetSemanticProperty::Target,
        ) => true,
        (
            RelationshipKind::Satisfy
            | RelationshipKind::Verify
            | RelationshipKind::Refine,
            SpreadsheetSemanticProperty::Target,
        ) => true,
        (
            RelationshipKind::Trace,
            SpreadsheetSemanticProperty::Source | SpreadsheetSemanticProperty::Target,
        ) => true,
        _ => false,
    };
'''
new = '''    let allow_requirement_id = matches!(
        (kind, property),
        (
            RelationshipKind::DeriveRequirement | RelationshipKind::Copy,
            SpreadsheetSemanticProperty::Source | SpreadsheetSemanticProperty::Target,
        ) | (
            RelationshipKind::Satisfy
                | RelationshipKind::Verify
                | RelationshipKind::Refine,
            SpreadsheetSemanticProperty::Target,
        ) | (
            RelationshipKind::Trace,
            SpreadsheetSemanticProperty::Source | SpreadsheetSemanticProperty::Target,
        )
    );
'''
if old not in text:
    raise SystemExit("missing PR41 clippy match anchor")
text = text.replace(old, new, 1)

path.write_text(text, encoding="utf-8")
