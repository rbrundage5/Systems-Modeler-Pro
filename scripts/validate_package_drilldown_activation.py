"""Validate Package Diagram drill-down survives selection-driven DOM re-rendering."""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def read(relative: str) -> str:
    return (ROOT / relative).read_text(encoding="utf-8")


index = read("apps/desktop/frontend/index.html")
workspace = read("apps/desktop/frontend/workspace-ux.js")
activation = read("apps/desktop/frontend/interaction-runtime-fixes.js")

assert '<script src="interaction-runtime-fixes.js"></script>' in index
assert "presentation.ondblclick" in workspace
assert "await drillDown(element)" in workspace
assert "render();" in workspace.split("async function selectPackageNode", 1)[1].split("function renderPackageCanvas", 1)[0]

# First-click selection currently rebuilds Package presentations. The runtime
# bridge therefore keys a double activation by stable presentation id and
# delegates to the existing drill-down handler instead of duplicating navigation.
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

# The activation bridge remains presentation-only: no alternate diagram creator
# or independent keyboard/pan controller is introduced by this fix.
for forbidden in (
    "create_package_diagram",
    "keydown",
    "keyup",
):
    assert forbidden not in activation

print("PR26B Package drill-down activation contract passed")
