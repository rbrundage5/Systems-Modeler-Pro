"""Validate Package/BDD/Activity drill-down activation and visible child-diagram notation."""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def read(relative: str) -> str:
    return (ROOT / relative).read_text(encoding="utf-8")


index = read("apps/desktop/frontend/index.html")
workspace = read("apps/desktop/frontend/workspace-ux.js")
activation = read("apps/desktop/frontend/interaction-runtime-fixes.js")
indicators = read("apps/desktop/frontend/drilldown-indicators.js")
activity_navigation = read("apps/desktop/frontend/activity-navigation-ui.js")
ibd = read("apps/desktop/frontend/ibd-ui.js")

assert '<script src="interaction-runtime-fixes.js"></script>' in index
assert '<script src="drilldown-indicators.js"></script>' in index
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

# PR56: a visible marker is derived only from an already-existing drill-down
# target. It is presentation-only and never creates model/diagram semantics.
for token in (
    "smp-child-diagram-indicator",
    "smp-sysml-call-behavior-rake",
    "hasPackageDrillDownTarget",
    "diagram.family === 'bdd'",
    "context_block_id",
    "calledActivityId",
    "Referenced Activity is described on another Activity Diagram",
    "Child diagram available — double-click to drill down",
    "MutationObserver",
):
    assert token in indicators

# The standard CallBehavior drill-down and BDD -> IBD drill-down remain the
# interaction authorities; the indicator layer only visualizes their targets.
assert "openCalledActivity" in activity_navigation
assert "bindCallBehaviorDrillDown" in activity_navigation
assert "bindBddIbdDrilldown" in ibd

for forbidden in (
    "create_package_diagram",
    "create_bdd",
    "create_ibd",
    "create_activity_diagram",
    "keydown",
    "keyup",
):
    assert forbidden not in indicators

print("PR56 drill-down activation + indicator contract passed")
