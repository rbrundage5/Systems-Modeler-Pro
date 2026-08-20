"""Static PR14 architecture contracts; complements Rust unit and integration tests."""
from pathlib import Path

root = Path(__file__).resolve().parents[1]
frontend = root / "apps/desktop/frontend"
def read(path: Path) -> str:
    return path.read_text(encoding="utf-8")

index = read(frontend / "index.html")
workspace = read(frontend / "shared-workspace.js")
dialogs = read(frontend / "shared-dialogs.js")
styles = read(frontend / "shared-workspace.css")
shell_styles = read(frontend / "workspace-polish.css")
ibd_ui = read(frontend / "ibd-ui.js")
behavior_ui = read(frontend / "behavior-authoritative-renderer.js")
theme = read(root / "apps/desktop/src-tauri/src/workspace/presentation_theme.rs")
main = read(root / "apps/desktop/src-tauri/src/main.rs")

assert 'data-shared-workspace="true"' in index
assert 'shared-workspace.js' in index and 'shared-workspace.css' in index
family_contract = read(root / "crates/model-core/src/diagram_family.rs")
for family in ["bdd", "ibd", "state-machine", "sequence", "activity"]:
    assert f'registerRenderer(\'{family}\'' in workspace
    assert f'"{family}"' in family_contract
for abbreviation, context_kind in [("bdd", "Package"), ("ibd", "Block"), ("stm", "StateMachine"), ("seq", "Interaction"), ("act", "Activity")]:
    assert f'"{abbreviation}"' in family_contract and f'"{context_kind}"' in family_contract
assert "modelElementName" in workspace and "state.context.frameLabel" in workspace
assert "sysml-diagram-frame" in workspace and "sysml-frame-label" in styles
assert "get_diagram_frame_preference" in workspace and "set_diagram_frame_preference" in workspace
assert "validFrame(storedFrame)?storedFrame:null" in workspace
assert "setTimeout(()=>persistDiagramFrame(diagramId,preference)" in workspace
assert "event.stopImmediatePropagation(); canvas.setPointerCapture" in workspace
assert "state.frameElement.style.transform=transform" in workspace
assert "frame.dataset.diagramId=state.context.diagramId" in workspace
for command in ["select", "clearSelection", "zoomIn", "zoomOut", "actualSize", "fitDiagram", "pan", "route", "cleanLayout"]:
    assert f'id: "{command}"' in theme
assert "active_diagram_router" in theme
assert "active_diagram_router" in main
assert "active_diagram_layout" in main
assert "checkpoint_states" in read(root / "apps/desktop/src-tauri/src/workspace/history.rs")
assert "hierarchical_positions" in read(root / "apps/desktop/src-tauri/src/workspace/layout.rs")
assert "pub fn active_diagram_router" in read(root / "apps/desktop/src-tauri/src/workspace/shared_workspace.rs")
assert "pub fn route_bdd" in read(root / "apps/desktop/src-tauri/src/workspace.rs")
assert "route_diagram_geometry" in main
router = read(root / "apps/desktop/src-tauri/src/workspace/routing.rs")
assert "route_diagram_geometry" in router
for routing_contract in ["DiagramRouteEdge", "RouteRect", "reserved_routes", "allow_shared_departure"]:
    assert routing_contract in router
for family in ["bdd", "ibd", "state-machine", "sequence", "activity"]:
    assert family in family_contract
assert "resolve_diagram_commands" in theme
assert "command.enabled" in workspace and "command.disabledReason" in workspace
for category in ["structural", "interface", "activity", "state", "requirement", "constraint", "data", "event", "verification", "annotation", "frame"]:
    assert f'category: "{category}"' in theme
assert "get_viewport_preference" in workspace and "get_panel_preferences" in workspace
shared_workspace = read(root / "apps/desktop/src-tauri/src/workspace/shared_workspace.rs")
assert "frame_model_element_type" in family_contract and "frame_label" in shared_workspace
assert "workspace-preferences.json" in shared_workspace
assert "event.ctrlKey" in workspace and "event.clientX" in workspace
assert "semantic_presentation_manifest" in main and "diagram_command_manifest" in main
assert "semantic_presentation_stylesheet" in main and "semantic_presentation_stylesheet" in workspace
assert "min-width:0" in styles and "overflow:auto" in styles
assert "minmax(0,1fr)" in shell_styles
assert "minmax(540px,1fr)" not in shell_styles
for panel_command in ["showRepository", "showElements", "showProperties"]:
    assert f'id: "{panel_command}"' in theme
assert "setPanelVisibility" in workspace and "configuredWidth" in workspace
assert "set_viewport_preference" in main and "activate_diagram" in main
assert "get_diagram_frame_preference" in main and "set_diagram_frame_preference" in main
assert "fit_diagram_viewport" in main and "fit_diagram_viewport" in workspace
assert "zoom_diagram_viewport" in main and "zoom_diagram_viewport" in workspace
assert "workspace-transform-spacer" in workspace and "setPointerCapture" in workspace
for interaction_command in ["workspace_interaction_snapshot", "set_workspace_interaction", "clear_workspace_interaction"]:
    assert interaction_command in main
    assert interaction_command in shared_workspace
assert "publishInteraction" in workspace and "activeTool" in workspace
assert "expectedRevision" in workspace and "require_revision" in shared_workspace
assert "for (const adapter of renderers.values())" in workspace and "window.addEventListener('keydown'" in workspace
assert "typeof tool === 'object'" in workspace and "tool.relationship_kind" in workspace
assert "ActiveWorkspaceSnapshot" in shared_workspace
assert "activated.context" in workspace and "applyCommands(activated.commands)" in workspace
assert "aria-modal" in dialogs and "cancelActive" in dialogs
assert '.diagram-frame>.diagram-header{display:none!important}' in styles
assert '.canvas .activity-svg{background:transparent!important}' in styles
assert '.canvas .ibd-frame::after{display:none!important}' in styles
assert 'overflow:visible!important' in styles
assert 'data-family="activity"' in styles and "workspace-header').dataset.family" in workspace
assert 'frameGeometry:() => state.frame' in workspace and 'outerFramePoint' in ibd_ui
assert 'if(source)points[0]=outerFramePoint(source)' in ibd_ui
assert 'labelPoint={x:(first.x+last.x)/2' in behavior_ui
presentation = read(root / "apps/desktop/src-tauri/src/workspace/presentation_theme.rs")
assert 'data-semantic-kind=\\\"Lifeline\\\"]{background:transparent' in presentation
assert 'ACTIVITY_CALL' in presentation and 'ACTIVITY_OBJECT' in presentation
activity_ui = read(frontend / "activity-ui.js")
for kind in ["OpaqueAction", "CallBehaviorAction", "CallOperationAction", "SendSignalAction", "AcceptEventAction"]:
    assert kind in activity_ui and f'("{kind}",' in presentation
print("Shared workspace convergence contract passed")
