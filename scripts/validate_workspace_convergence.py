"""Static PR14 architecture contracts; complements Rust unit and integration tests."""
from pathlib import Path

root = Path(__file__).resolve().parents[1]
frontend = root / "apps/desktop/frontend"
index = (frontend / "index.html").read_text()
workspace = (frontend / "shared-workspace.js").read_text()
styles = (frontend / "shared-workspace.css").read_text()
theme = (root / "apps/desktop/src-tauri/src/workspace/presentation_theme.rs").read_text()
main = (root / "apps/desktop/src-tauri/src/main.rs").read_text()

assert 'data-shared-workspace="true"' in index
assert 'shared-workspace.js' in index and 'shared-workspace.css' in index
for family in ["BDD", "IBD", "StateMachine", "Sequence", "Activity"]:
    assert family in workspace and family in theme
for command in ["select", "clearSelection", "zoomIn", "zoomOut", "actualSize", "fitDiagram", "pan", "route", "cleanLayout"]:
    assert f'id: "{command}"' in theme
assert "active_diagram_router" in theme and "smpRouteActivityDiagram" in workspace
for category in ["structural", "interface", "activity", "state", "requirement", "constraint", "data", "event", "verification", "annotation", "frame"]:
    assert f'category: "{category}"' in theme
assert "stored.viewports" in workspace and "stored.panels" in workspace
assert "event.ctrlKey" in workspace and "event.clientX" in workspace
assert "semantic_presentation_manifest" in main and "diagram_command_manifest" in main
assert "min-width:720px" in styles and "overflow:auto" in styles
print("Shared workspace convergence contract passed")
