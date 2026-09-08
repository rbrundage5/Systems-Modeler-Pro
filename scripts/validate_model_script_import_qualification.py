"""Qualification contract for complete model-script generation and presentation.

This complements the native Rust model-script tests. It protects the blank-workspace
lifecycle and post-commit UI qualification required by full nine-family imports.
"""
from pathlib import Path

root = Path(__file__).resolve().parents[1]
frontend = root / "apps/desktop/frontend"
backend = root / "apps/desktop/src-tauri/src/workspace"

model_script_ui = (frontend / "model-script-ui.js").read_text(encoding="utf-8")
index = (frontend / "index.html").read_text(encoding="utf-8")
model_script_rs = (backend / "model_script.rs").read_text(encoding="utf-8")
workspace_rs = (root / "apps/desktop/src-tauri/src/workspace.rs").read_text(encoding="utf-8")

# The native host must retain explicit support for all nine qualified families.
for family in [
    "bdd", "ibd", "requirement", "use-case", "package", "activity",
    "state-machine", "sequence", "parametric",
]:
    assert f'"{family}"' in model_script_rs, f"model-script host lost {family} support"

# The native representative model-script test must prove committed, idempotent
# all-nine-family generation across ordinary, IBD, Activity and Behavior stores.
assert "representative_script_builds_native_semantics_and_all_nine_diagram_families" in model_script_rs
assert "let first_diagram_count = workspace.diagrams.lock().unwrap().len()" in model_script_rs
assert "+ workspace.ibd_diagrams.lock().unwrap().len()" in model_script_rs
assert "+ activity.diagrams.lock().unwrap().len()" in model_script_rs
assert "+ workspace.behavior_diagrams.lock().unwrap().len();" in model_script_rs
assert "assert_eq!(first_diagram_count, 9);" in model_script_rs
assert "second_diagram_count, 9," in model_script_rs

# New Project natively clears Behavior semantics/presentations. A blank model-
# script run must then unconditionally reset the separately managed Activity
# workspace before preview rather than trying to infer whether stale state exists.
new_project = workspace_rs.split("pub fn new_project", 1)[1].split("pub fn save_project_file", 1)[0]
assert "BehaviorRepository::default()" in new_project
assert "behavior_diagrams" in new_project and ".clear()" in new_project

for required in [
    "qualifyBlankProjectBaseline",
    "projectIsSemanticallyBlank",
    "activityStatePresent",
    "behaviorStatePresent",
    "repository?.external_ids",
    "invoke('new_project'",
    "invoke('reset_activity_workspace')",
    "invoke('clear_activity_executions')",
    "Qualified a clean Project, Activity, and Behavior baseline",
    "BLANK_PROJECT_BASELINE_INCOMPLETE",
    "requestedDiagrams",
    "committedDiagrams",
    "qualifyCommittedDiagramSet",
    "DIAGRAM_COMMIT_INCOMPLETE",
    "DIAGRAM_COMMIT_COUNT_MISMATCH",
    "DIAGRAM_UI_REGISTRATION_INCOMPLETE",
    "ALL_NINE_FAMILY_QUALIFICATION_FAILED",
    "window.smpLoadBehaviorSnapshot",
    "document.querySelectorAll('#diagram-tabs .diagram-tab')",
    "Model script applied and diagram set qualified",
]:
    assert required in model_script_ui, f"missing model-script import qualification contract: {required}"

baseline = model_script_ui.split("async function qualifyBlankProjectBaseline", 1)[1].split("function normalizeDiagramFamily", 1)[0]
assert baseline.index("invoke('new_project'") < baseline.index("invoke('reset_activity_workspace')")
assert baseline.index("invoke('reset_activity_workspace')") < baseline.index("invoke('activity_snapshot')")
assert "if (!activityStatePresent" not in baseline, "blank import must not skip authoritative specialized-state reset"

# A complete nine-family import must explicitly require the exact qualified set.
required_nine_literal = "new Set(['package', 'requirement', 'use-case', 'bdd', 'ibd', 'activity', 'state-machine', 'sequence', 'parametric'])"
assert required_nine_literal in model_script_ui

# The UI adapters that expose committed Behavior and Activity diagrams must be
# loaded in the desktop shell, along with the model-script command itself.
for script in ["behavior-ui.js", "model-script-ui.js", "activity-ui.js", "repository-tree-ui.js"]:
    assert f'<script src="{script}"></script>' in index, f"desktop shell is missing {script}"

print("Model-script full-import qualification contract passed")
