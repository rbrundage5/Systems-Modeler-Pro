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
app_frontend = read("apps/desktop/frontend/app.js")
runtime_fixes = read("apps/desktop/frontend/interaction-runtime-fixes.js")
state_bar_frontend = read("apps/desktop/frontend/state-bar-resize.js")
sequence_frontend = read("apps/desktop/frontend/behavior-authoritative-renderer.js")
bdd_completion = read("apps/desktop/frontend/bdd-completion-ui.js")
bdd_extended = read("apps/desktop/frontend/bdd-extended-ui.js")
compartment_frontend = read("apps/desktop/frontend/bdd-compartment-visibility.js")
package_frontend = read("apps/desktop/frontend/workspace-ux.js")
ibd_frontend = read("apps/desktop/frontend/ibd-ui.js")
parametric_frontend = read("apps/desktop/frontend/parametric-ui.js")
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
assert "window.smpBeginPresentationGesture = beginPointerGesture" in frontend
assert "owner.addEventListener('pointermove', onMove, true)" in frontend
assert "owner.addEventListener('pointerup', onUp, true)" in frontend
assert "owner.addEventListener('pointercancel', onCancel, true)" in frontend
assert "owner.addEventListener('lostpointercapture', onLostCapture, true)" in frontend
assert "const htmlGeometryConfigs = new WeakMap()" in frontend
assert "geometryCanvas?.addEventListener('pointerdown', startHtmlGeometryGesture, true)" in frontend
assert "if (!htmlGeometryConfigs.has(node)) install();" in frontend
assert "event.stopImmediatePropagation();" in frontend
assert "window.smpInstallPresentationGeometry = install" in frontend
assert "DRAG_THRESHOLD_PX = 3" in frontend
assert "cancelTransientAuthoring" in frontend
assert ".smp-resize-handle { position: absolute; right: 2px; bottom: 2px;" in frontend
assert "await config.commit(next);" in frontend
assert "stopImmediatePropagation" in frontend, "resize click must not trigger a stale render"

# BDD, IBD, Use Case subject boundaries, and State Machine HTML presentations
# must consume the same pointer-gesture kernel. Family-specific code may choose
# geometry constraints and Rust commands, but it must not create another raw
# pointer lifecycle for these presentation families.
shared_html_geometry = frontend.split("function bindHtmlGeometry", 1)[1].split(
    "function installBdd", 1
)[0]
assert "beginPointerGesture(event" in shared_html_geometry
assert ".onpointerdown" not in shared_html_geometry
assert "addEventListener('pointerdown'" in shared_html_geometry

# The historical structural rebind competed with the generic controller. All
# structural families now share stable presentation lookup and command routing.
assert not (root / "apps/desktop/frontend/structural-interaction-rebind.js").exists()
assert "structural-interaction-rebind.js" not in index
assert "node.dataset.presentationId" in frontend
assert "box.dataset.presentationId = node.id" in app_frontend
assert "function surfaceScale(node)" in frontend
assert "(move.clientX - startX) / Math.max(scale.x || 1, 0.0001)" in frontend
assert "(move.clientY - startY) / Math.max(scale.y || 1, 0.0001)" in frontend
assert "new MutationObserver" in frontend
assert "selectedDiagramExists" in app_frontend and "ibd_diagrams" in app_frontend
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
assert "smpBeginPresentationGesture" in sequence_frontend
assert "smpPreviewSequenceLifelineGeometry" in sequence_frontend
assert "bindSequenceConnectedDrag" not in runtime_fixes
assert "smpPreviewSequenceLifelineGeometry" in runtime_fixes
assert "setAttribute('points'" in runtime_fixes
assert "smpBeginPresentationGesture" in state_bar_frontend
assert "smpBeginPresentationGesture" in parametric_frontend
activity_geometry = frontend.split("function installActivity", 1)[1].split("function install()", 1)[0]
assert "beginPointerGesture(event" in activity_geometry
assert ".onpointermove" not in activity_geometry
assert ".onpointerup" not in activity_geometry

# Rust commands reroute only notation attached to the moved/resized presentation,
# own the undo checkpoint, and preserve IBD boundary-port attachment. Unrelated
# stale routes must never make direct geometry editing fail.
assert "reroute_connected_bdd_edges" in interaction_rs, "BDD geometry must reroute incident edges without making unrelated routes block editing"
assert "validate_loaded_diagrams(project, diagrams)" not in interaction_rs, "Presentation-only BDD geometry must not be blocked by unrelated diagram validation"
assert "apply_ibd_property_geometry" in interaction_rs, "IBD property geometry must keep nested ports attached"
assert "affected_ids" in interaction_rs, "IBD property movement must reroute only incident connectors"
assert ".filter(|edge|" in interaction_rs and "edge.source_presentation_id == presentation_id" in interaction_rs
assert "ibd_nested_ports_follow_shared_property_move_and_resize_geometry" in interaction_rs
assert "route_ibd_edge" in interaction_rs, "IBD rerouting is not integrated"
assert "orthogonal_route" in interaction_rs, "Activity rerouting is not integrated"
assert "edge.source_node_id == presentation_id || edge.target_node_id == presentation_id" in interaction_rs
assert "reroute_incident_state_transitions" in interaction_rs
assert "reroute_incident_state_transitions" in behavior_rs
assert "reroute_incident_sequence_messages" in behavior_rs
assert "history::checkpoint_states(&state, &activity, &history)?;" in behavior_rs
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
