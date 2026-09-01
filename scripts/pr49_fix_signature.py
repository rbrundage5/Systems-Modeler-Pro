from pathlib import Path
import re

path = Path("apps/desktop/src-tauri/src/workspace/spreadsheet_import/pr49_semantics.rs")
text = path.read_text()
pattern = r"fn signature_matches\([\s\S]*?\n\}\n\nfn binding_endpoint"
replacement = """fn signature_matches(
    existing: &Option<MessageSignature>,
    build: &Option<MessageSignatureBuild>,
) -> bool {
    match (existing, build) {
        (None, None) => true,
        (
            Some(MessageSignature::Operation(left)),
            Some(MessageSignatureBuild::Operation(BuildReference::Existing(right))),
        ) => left == right,
        (
            Some(MessageSignature::Signal(left)),
            Some(MessageSignatureBuild::Signal(BuildReference::Existing(right))),
        ) => left == right,
        _ => false,
    }
}

fn binding_endpoint"""
updated, count = re.subn(pattern, replacement, text, count=1)
if count != 1:
    raise SystemExit(f"expected exactly one signature_matches function, found {count}")
path.write_text(updated)
