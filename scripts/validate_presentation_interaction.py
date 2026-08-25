from pathlib import Path

root = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (root / path).read_text(encoding="utf-8")


main_rs = read("apps/desktop/src-tauri/src/main.rs")
interaction_rs = read(
    "apps/desktop/src-tauri/src/workspace/presentation_interaction.rs"
)
behavior_rs = read("apps/desktop/src-tauri/src/workspace/behavior_workspace.rs")
history_rs = read("apps/desktop/src-tauri/src/workspace/history.rs")
frontend = read("apps/desktop/frontend/diagram-interaction.js")
runtime_fixes = read("apps/desktop/frontend/interaction-runtime-fixes.js")
state_bar_frontend = read("apps/desktop/frontend/state-bar-resize.js")
sequence_frontend = read("apps/desktop/frontend/behavior-authoritative-renderer.js")
bdd_completion = read("apps/desktop/frontend/bdd-completion-ui.js")
bdd_extended = read("apps/desktop/frontend/bdd-extended-ui.js")
compartment_frontend = read("apps/desktop/frontend/bdd-compartment-visibility.js")
package_frontend = read("apps/desktop/frontend/workspace-ux.js")
ibd_frontend = read("apps/desktop/frontend/ibd-ui.js")
index = read("apps/desktop/frontend/index.html")

commands = [
    "update_bdd_presentation_geometry",
    "update_ibd_property_geometry",
    "update_ibd_port_geometry",
    "update_state_presentation_geometry",
    "update_activity_presentation_geometry",
]
for command in commands:
    assert command in main_rs, f"Presentation command is not registered: {command}"
    assert command in interaction_rs, f"Presentation command is not implemented: {command}"
    assert command in frontend, f"Frontend does not delegate presentation update: {command}"

for selector in [
    ".bdd-block",
    ".ibd-property",
    ".ibd-port",
    ".state-vertex",
    ".sequence-lifeline",
    ".activity-node",
]:
    assert selector in frontend, f"Shared interaction layer is missing {selector}"

# One shared Rust-authoritative commit sequence owns every supported resize:
# local preview -> one awaited command -> authoritative refresh -> render by refresh.
commit_body = frontend.split("async function commit", 1)[1].split(
    "window.smpCommitPresentationGeometry", 1
)[0]
assert "await runCommand" in commit_body
assert "await refresh();" in commit_body
assert commit_body.index("await runCommand") < commit_body.index(
    "await refresh();"
)
assert "finally" in commit_body
assert "render();" not in commit_body, "commit must not render a pre-refresh snapshot"
assert "window.smpCommitPresentationGeometry = commit" in frontend
assert "handle.onpointerup = async" in frontend
assert "await config.commit(next);" in frontend
assert "stopImmediatePropagation" in frontend, "resize click must not trigger a stale render"

# The historical structural rebind competed with the generic controller. All
# structural families now share stable presentation lookup and command routing.
assert not (root / "apps/desktop/frontend/structural-interaction-rebind.js").exists()
assert "structural-interaction-rebind.js" not in index
assert "node.dataset.presentationId" in frontend
assert "diagram.family === 'parametric'" in frontend
assert "update_parametric_presentation_geometry" in frontend
assert "update_use_case_subject_boundary_geometry" in frontend
assert "family === 'package'" in package_frontend
assert "presentation.dataset.presentationId = node.id" in package_frontend
assert "box.dataset.presentationId = node.id" in bdd_completion
assert "box.dataset.presentationId = node.id" in bdd_extended
assert "node.dataset.presentationId = port.id" in ibd_frontend
assert "box.dataset.presentationId = property.id" in ibd_frontend

# Explicit Rust geometry must survive every structural renderer refresh. Hidden
# or absent compartments may change content, never the presentation box height.
for renderer in [bdd_completion, bdd_extended]:
    assert "style.height = `${node.height}px`" in renderer
    assert "style.height = 'auto'" not in renderer
assert "box.style.height = `${presentation.height}px`" in compartment_frontend
assert "box.style.minHeight = '0'" in compartment_frontend
assert "box.style.height = visibleCompartments" not in compartment_frontend

# Notation-specific previews reuse the same final commit/refresh mechanism.
assert "applyActivityShapeGeometry" in frontend
assert "bindActivityVisibleResize" not in runtime_fixes
assert "presentation.width = next.width" not in runtime_fixes
assert "presentation.height = next.height" not in runtime_fixes
assert "window.smpCommitPresentationGeometry" in state_bar_frontend
assert "window.smpCommitPresentationGeometry" in sequence_frontend
assert "resize_sequence_lifeline_timeline" in sequence_frontend

# Rust commands reroute attached notation and own the undo checkpoint. Focused
# Rust tests cover immediate snapshots, persistence, and undo/redo for a BDD,
# Package Diagram, and a behavioral Sequence presentation.
assert "route_relationship" in interaction_rs, "BDD rerouting is not integrated"
assert "route_ibd_edge" in interaction_rs, "IBD rerouting is not integrated"
assert "orthogonal_route" in interaction_rs, "Activity rerouting is not integrated"
assert "port.y = y.clamp" in interaction_rs, "IBD ports are not boundary constrained"
sequence_resize = behavior_rs.split("pub fn resize_sequence_lifeline_timeline", 1)[1].split(
    "pub fn add_sequence_message", 1
)[0]
assert "history::checkpoint_states" in sequence_resize
assert "resize_sequence_lifeline_timeline_in" in sequence_resize
assert "behavior_metadata_database_round_trip_preserves_stm_and_seq_diagrams" in behavior_rs
assert "timeline_end_y == 960.0" in behavior_rs
assert "structural_and_package_resize_snapshot_persistence_and_history_round_trip" in interaction_rs
assert "save_metadata" in interaction_rs and "load_metadata" in interaction_rs
assert "history::undo_states" in interaction_rs and "history::redo_states" in interaction_rs
assert "pub(super) fn undo_states" in history_rs
assert "pub(super) fn redo_states" in history_rs

# State Machine Fork/Join keeps its notation-specific thickness semantics while
# delegating the final write/refresh to the shared path.
assert ".state-fork, #canvas .state-join" in state_bar_frontend
assert "bar.style.width = '100%'" in state_bar_frontend
assert "STORED_HEIGHT_OFFSET = 12" in state_bar_frontend
assert "MIN_BAR_THICKNESS = 8" in state_bar_frontend
assert "storedHeightForThickness" in state_bar_frontend
assert "return clampThickness(thickness) + STORED_HEIGHT_OFFSET" in state_bar_frontend
assert "updateIncidentTransitions" in state_bar_frontend
assert "presentation.width = next.width" not in state_bar_frontend
assert "presentation.height = next.height" not in state_bar_frontend

assert '<script src="diagram-interaction.js"></script>' in index
assert '<script src="state-bar-resize.js"></script>' in index

print("Shared presentation interaction contract passed")
