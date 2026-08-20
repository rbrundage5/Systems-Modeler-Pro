"""Static PR14 architecture contracts; complements Rust unit and integration tests."""
from pathlib import Path

root = Path(__file__).resolve().parents[1]
frontend = root / "apps/desktop/frontend"
index = (frontend / "index.html").read_text()
workspace = (frontend / "shared-workspace.js").read_text()
dialogs = (frontend / "shared-dialogs.js").read_text()
styles = (frontend / "shared-workspace.css").read_text()
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
assert "active_diagram_router" in theme and "requiredCapability" in workspace
for category in ["structural", "interface", "activity", "state", "requirement", "constraint", "data", "event", "verification", "annotation", "frame"]:
    assert f'category: "{category}"' in theme
assert "get_viewport_preference" in workspace and "get_panel_preferences" in workspace
assert "event.ctrlKey" in workspace and "event.clientX" in workspace
assert "semantic_presentation_manifest" in main and "diagram_command_manifest" in main
assert "min-width:720px" in styles and "overflow:auto" in styles
assert "set_viewport_preference" in main and "activate_diagram" in main
assert "workspace-transform-spacer" in workspace and "setPointerCapture" in workspace
assert "aria-modal" in dialogs and "cancelActive" in dialogs
print("Shared workspace convergence contract passed")
