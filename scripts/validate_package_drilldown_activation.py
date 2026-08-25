"""Validate Package Diagram drill-down survives selection-driven DOM re-rendering."""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def read(relative: str) -> str:
    return (ROOT / relative).read_text(encoding="utf-8")


index = read("apps/desktop/frontend/index.html")
workspace = read("apps/desktop/frontend/workspace-ux.js")
activation = read("apps/desktop/frontend/package-drilldown-activation.js")

assert '<script src="workspace-ux.js"></script>\n  <script src="package-drilldown-activation.js"></script>' in index
assert "presentation.ondblclick" in workspace
assert "await drillDown(element)" in workspace
assert "render();" in workspace.split("async function selectPackageNode", 1)[1].split("function renderPackageCanvas", 1)[0]

# First-click selection currently rebuilds Package presentations. The activation
# bridge must therefore key a double activation by stable presentation id and
# delegate to the existing drill-down handler instead of duplicating navigation.
for token in (
    "DOUBLE_CLICK_WINDOW_MS",
    ".package-diagram [data-presentation-id]",
    "presentation.dataset.presentationId",
    "lastActivation.key === key",
    "typeof presentation.ondblclick !== 'function'",
    "Promise.resolve(presentation.ondblclick(event))",
    "No existing diagram is owned by or context-bound to",
):
    assert token in activation

# The bridge is presentation-only: no model command, alternate diagram creator,
# or independent keyboard/pan controller is allowed here.
for forbidden in (
    "__TAURI__",
    "create_package_diagram",
    "create_bdd",
    "keydown",
    "keyup",
):
    assert forbidden not in activation

print("PR26B Package drill-down activation contract passed")
