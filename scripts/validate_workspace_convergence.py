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
assert "event.ctrlKey" in workspace and "event.clientX" in workspace
assert "semantic_presentation_manifest" in main and "diagram_command_manifest" in main
assert "min-width:0" in styles and "overflow:auto" in styles
assert "minmax(0,1fr)" in shell_styles
assert "minmax(540px,1fr)" not in shell_styles
for panel_command in ["showRepository", "showElements", "showProperties"]:
    assert f'id: "{panel_command}"' in theme
assert "setPanelVisibility" in workspace and "configuredWidth" in workspace
assert "set_viewport_preference" in main and "activate_diagram" in main
assert "workspace-transform-spacer" in workspace and "setPointerCapture" in workspace
assert "aria-modal" in dialogs and "cancelActive" in dialogs
print("Shared workspace convergence contract passed")
