"""Static PR14 architecture contracts; complements Rust unit and integration tests."""
from pathlib import Path

root = Path(__file__).resolve().parents[1]
frontend = root / "apps/desktop/frontend"
index = (frontend / "index.html").read_text()
workspace = (frontend / "shared-workspace.js").read_text()
dialogs = (frontend / "shared-dialogs.js").read_text()
styles = (frontend / "shared-workspace.css").read_text()
shell_styles = (frontend / "workspace-polish.css").read_text()
theme = (root / "apps/desktop/src-tauri/src/workspace/presentation_theme.rs").read_text()
main = (root / "apps/desktop/src-tauri/src/main.rs").read_text()

assert 'data-shared-workspace="true"' in index
assert 'shared-workspace.js' in index and 'shared-workspace.css' in index
family_contract = (root / "crates/model-core/src/diagram_family.rs").read_text()
for family in ["bdd", "ibd", "state-machine", "sequence", "activity"]:
    assert f'registerRenderer(\'{family}\'' in workspace
    assert f'"{family}"' in family_contract
for command in ["select", "clearSelection", "zoomIn", "zoomOut", "actualSize", "fitDiagram", "pan", "route", "cleanLayout"]:
    assert f'id: "{command}"' in theme
assert "active_diagram_router" in theme
assert "resolve_diagram_commands" in theme
assert "command.enabled" in workspace and "command.disabledReason" in workspace
for category in ["structural", "interface", "activity", "state", "requirement", "constraint", "data", "event", "verification", "annotation", "frame"]:
    assert f'category: "{category}"' in theme
assert "get_viewport_preference" in workspace and "get_panel_preferences" in workspace
assert "workspace-preferences.json" in (root / "apps/desktop/src-tauri/src/workspace/shared_workspace.rs").read_text()
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
assert "fit_diagram_viewport" in main and "fit_diagram_viewport" in workspace
assert "zoom_diagram_viewport" in main and "zoom_diagram_viewport" in workspace
assert "workspace-transform-spacer" in workspace and "setPointerCapture" in workspace
for interaction_command in ["workspace_interaction_snapshot", "set_workspace_interaction", "clear_workspace_interaction"]:
    assert interaction_command in main
    assert interaction_command in (root / "apps/desktop/src-tauri/src/workspace/shared_workspace.rs").read_text()
assert "publishInteraction" in workspace and "activeTool" in workspace
assert "expectedRevision" in workspace and "require_revision" in (root / "apps/desktop/src-tauri/src/workspace/shared_workspace.rs").read_text()
assert "for (const adapter of renderers.values())" in workspace and "window.addEventListener('keydown'" in workspace
assert "typeof tool === 'object'" in workspace and "tool.relationship_kind" in workspace
assert "ActiveWorkspaceSnapshot" in (root / "apps/desktop/src-tauri/src/workspace/shared_workspace.rs").read_text()
assert "activated.context" in workspace and "applyCommands(activated.commands)" in workspace
assert "aria-modal" in dialogs and "cancelActive" in dialogs
assert '.diagram-frame>.diagram-header{display:none!important}' in styles
assert '.canvas .activity-svg{background:transparent!important}' in styles
assert 'overflow:visible!important' in styles
assert 'data-family="activity"' in styles and "workspace-header').dataset.family" in workspace
presentation = (root / "apps/desktop/src-tauri/src/workspace/presentation_theme.rs").read_text()
assert 'data-semantic-kind=\\\"Lifeline\\\"]{background:transparent' in presentation
assert 'ACTIVITY_CALL' in presentation and 'ACTIVITY_OBJECT' in presentation
activity_ui = (frontend / "activity-ui.js").read_text()
for kind in ["OpaqueAction", "CallBehaviorAction", "CallOperationAction", "SendSignalAction", "AcceptEventAction"]:
    assert kind in activity_ui and f'("{kind}",' in presentation
print("Shared workspace convergence contract passed")
