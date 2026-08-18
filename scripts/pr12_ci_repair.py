from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

def replace(path, old, new):
    p = ROOT / path
    text = p.read_text(encoding='utf-8')
    if old not in text:
        raise SystemExit(f'expected fragment not found in {path}: {old[:80]!r}')
    p.write_text(text.replace(old, new), encoding='utf-8')

# Use the existing full-diagram validator instead of leaving it dead.
replace(
    'apps/desktop/src-tauri/src/workspace/bdd_elements.rs',
    '    let current_file = state\n        .current_file\n        .lock()\n        .map_err(|_| "project path lock poisoned")?;\n    Ok(CompleteWorkspaceSnapshot {',
    '    if let Some(project) = project.as_ref() {\n        validate_complete_diagrams(project, &diagrams)?;\n    }\n    let current_file = state\n        .current_file\n        .lock()\n        .map_err(|_| "project path lock poisoned")?;\n    Ok(CompleteWorkspaceSnapshot {'
)
replace(
    'apps/desktop/src-tauri/src/workspace/bdd_elements.rs',
    '#[tauri::command]\npub fn create_bdd_feature(',
    '#[tauri::command]\n#[allow(clippy::too_many_arguments)] // Stable Tauri IPC contract; frontend sends named fields.\npub fn create_bdd_feature('
)
replace(
    'apps/desktop/src-tauri/src/workspace/feature_editing.rs',
    '#[tauri::command]\npub fn update_bdd_feature_semantics(',
    '#[tauri::command]\n#[allow(clippy::too_many_arguments)] // Stable Tauri IPC contract; frontend sends named fields.\npub fn update_bdd_feature_semantics('
)

# Rust 1.97 Clippy: elide needless lifetimes.
replace(
    'apps/desktop/src-tauri/src/workspace/behavior_workspace.rs',
    "fn find_region_mut<'a>(regions: &'a mut [Region], wanted: RegionId) -> Option<&'a mut Region> {",
    'fn find_region_mut(regions: &mut [Region], wanted: RegionId) -> Option<&mut Region> {'
)
replace(
    'apps/desktop/src-tauri/src/workspace/behavior_workspace.rs',
    "fn find_vertex_mut<'a>(regions: &'a mut [Region], wanted: VertexId) -> Option<&'a mut Vertex> {",
    'fn find_vertex_mut(regions: &mut [Region], wanted: VertexId) -> Option<&mut Vertex> {'
)
replace(
    'apps/desktop/src-tauri/src/workspace/behavior_workspace.rs',
    "fn find_vertex<'a>(regions: &'a [Region], wanted: VertexId) -> Option<&'a Vertex> {",
    'fn find_vertex(regions: &[Region], wanted: VertexId) -> Option<&Vertex> {'
)

# Keep current named-field Tauri interfaces stable during this migration PR.
replace(
    'apps/desktop/src-tauri/src/workspace/behavior_workspace.rs',
    '#[tauri::command]\npub fn add_state_transition(',
    '#[tauri::command]\n#[allow(clippy::too_many_arguments)] // Stable named-field Tauri IPC boundary.\npub fn add_state_transition('
)
replace(
    'apps/desktop/src-tauri/src/workspace/behavior_workspace.rs',
    '#[tauri::command]\npub fn add_sequence_message(',
    '#[tauri::command]\n#[allow(clippy::too_many_arguments)] // Stable named-field Tauri IPC boundary.\npub fn add_sequence_message('
)

print('Applied PR12 CI repair')
