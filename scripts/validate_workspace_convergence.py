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
theme = read(root / "apps/desktop/src-tauri/src/workspace/presentation_theme.rs")
main = read(root / "apps/desktop/src-tauri/src/main.rs")

assert 'data-shared-workspace="true"' in index
assert 'shared-workspace.js' in index and 'shared-workspace.css' in index
family_contract = read(root / "crates/model-core/src/diagram_family.rs")
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
shared_workspace = read(root / "apps/desktop/src-tauri/src/workspace/shared_workspace.rs")
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
assert 'overflow:visible!important' in styles
assert 'data-family="activity"' in styles and "workspace-header').dataset.family" in workspace
presentation = read(root / "apps/desktop/src-tauri/src/workspace/presentation_theme.rs")
assert 'data-semantic-kind=\\\"Lifeline\\\"]{background:transparent' in presentation
assert 'ACTIVITY_CALL' in presentation and 'ACTIVITY_OBJECT' in presentation
activity_ui = read(frontend / "activity-ui.js")
for kind in ["OpaqueAction", "CallBehaviorAction", "CallOperationAction", "SendSignalAction", "AcceptEventAction"]:
    assert kind in activity_ui and f'("{kind}",' in presentation
print("Shared workspace convergence contract passed")
