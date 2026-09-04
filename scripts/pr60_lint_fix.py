from pathlib import Path

path = Path("apps/desktop/src-tauri/src/workspace/model_script.rs")
text = path.read_text(encoding="utf-8")

old_enum = '''pub enum ModelScriptAction {
    Create,
    Update,
    NoChange,
    Blocked,
}
'''
new_enum = '''pub enum ModelScriptAction {
    Create,
    Update,
    NoChange,
}
'''
if text.count(old_enum) != 1:
    raise SystemExit("expected exactly one ModelScriptAction enum")
text = text.replace(old_enum, new_enum)

old_valid = '''impl ModelScriptPreview {
    fn valid(&self) -> bool {
        self.diagnostics.is_empty()
            && !self
                .items
                .iter()
                .any(|item| item.action == ModelScriptAction::Blocked)
    }
}
'''
new_valid = '''impl ModelScriptPreview {
    fn valid(&self) -> bool {
        // Model-script blocking is represented by structured diagnostics. The
        // old BLOCKED item action was never emitted and therefore made the
        // serialized action contract claim a state the host could not produce.
        self.diagnostics.is_empty()
    }
}
'''
if text.count(old_valid) != 1:
    raise SystemExit("expected exactly one ModelScriptPreview::valid implementation")
text = text.replace(old_valid, new_valid)

old_success = '''    ModelScriptPreview {
        host: SCRIPT_HOST,
        applied,
        source_namespace: compiled.document.source_namespace.clone(),
        items,
        diagnostics: Vec::new(),
    }
}

fn preview_impl(
'''
new_success = '''    let mut preview = ModelScriptPreview {
        host: SCRIPT_HOST,
        applied: false,
        source_namespace: compiled.document.source_namespace.clone(),
        items,
        diagnostics: Vec::new(),
    };
    // Keep one production validity gate for both preview and apply responses.
    // successful_preview has no diagnostics by construction, but using the
    // same predicate prevents applied=true from ever escaping with blockers if
    // this response gains additional validation in the future.
    preview.applied = applied && preview.valid();
    preview
}

fn preview_impl(
'''
if text.count(old_success) != 1:
    raise SystemExit("expected exactly one successful_preview return block")
text = text.replace(old_success, new_success)

path.write_text(text, encoding="utf-8")
