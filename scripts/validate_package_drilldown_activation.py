"""Validate Package/BDD/Activity drill-down activation and visible child-diagram notation."""
from pathlib import Path
import json
import re

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

# The shipped complete demo must remain parseable as the bounded Groovy wrapper,
# logically packaged, cover every supported diagram family, and give every
# authored Activity/StateMachine/Interaction a diagram presentation.
demo_source = read("examples/model-script/complete-vehicle-demo.groovy")
match = re.search(r"modelScript\('''\s*(\{.*\})\s*'''\)\s*$", demo_source, re.S)
assert match, "complete demo must remain a modelScript triple-quoted JSON payload"
demo = json.loads(match.group(1))
operations = demo["operations"]
diagrams = demo["diagrams"]
external_ids = [op["external_id"] for op in operations if op.get("external_id")]
assert len(external_ids) == len(set(external_ids)), "demo operation External IDs must be unique"
known = set(external_ids)

# Mirror the exact public enum spellings consumed by the Rust model-script host.
ALLOWED_OPS = {
    "element", "relationship", "connector", "item_flow", "activity",
    "activity_partition", "structured_activity_node", "activity_node", "pin",
    "activity_edge", "state_machine", "region", "vertex", "transition",
    "interaction", "lifeline", "occurrence", "message", "execution",
    "combined_fragment", "operand", "state_invariant", "parametric_metadata", "binding",
}
ALLOWED_ELEMENTS = {
    "Model", "Package", "ModelLibrary", "Block", "AssociationBlock", "InterfaceBlock",
    "ConstraintBlock", "ValueType", "DataType", "PrimitiveType", "Enumeration",
    "EnumerationLiteral", "Signal", "Unit", "QuantityKind", "InstanceSpecification",
    "Slot", "PartProperty", "ReferenceProperty", "ValueProperty", "FlowProperty",
    "ConstraintProperty", "ConstraintParameter", "ProxyPort", "FullPort", "Operation",
    "Parameter", "Reception", "Requirement", "TestCase", "Actor", "UseCase", "Comment",
}
ALLOWED_RELATIONSHIPS = {
    "Dependency", "PackageImport", "ElementImport", "PackageMerge", "Association",
    "Composition", "Generalization", "Realization", "Allocate", "Connector", "ItemFlow",
    "DeriveRequirement", "Satisfy", "Verify", "Refine", "Trace", "Copy", "Include",
    "Extend", "BindingConnector",
}
ALLOWED_ACTIVITY_NODES = {
    "initial", "activity_final", "flow_final", "decision", "merge", "fork", "join",
    "opaque_action", "call_behavior", "call_operation", "send_signal", "accept_event",
    "accept_time_event", "object", "activity_parameter",
}
ALLOWED_EDGE_KINDS = {"ControlFlow", "ObjectFlow"}
ALLOWED_CONNECTORS = {"Assembly", "Delegation"}
ALLOWED_PSEUDOSTATES = {
    "Initial", "Choice", "Junction", "Fork", "Join", "ShallowHistory", "DeepHistory",
    "EntryPoint", "ExitPoint", "Terminate",
}
ALLOWED_MESSAGE_SORTS = {
    "SynchCall", "AsynchCall", "AsynchSignal", "Reply", "Create", "Delete", "Lost", "Found",
}
ALLOWED_FAMILIES = {
    "BDD", "IBD", "Requirement", "Use Case", "Package",
    "Activity", "State Machine", "Sequence", "Parametric",
}

for op in operations:
    assert op["op"] in ALLOWED_OPS, f"unsupported demo operation: {op['op']}"
    if op["op"] == "element":
        assert op["kind"] in ALLOWED_ELEMENTS, f"unsupported ElementKind: {op['kind']}"
    if op["op"] == "relationship":
        assert op["kind"] in ALLOWED_RELATIONSHIPS, f"unsupported RelationshipKind: {op['kind']}"
    if op["op"] == "connector":
        assert op["kind"] in ALLOWED_CONNECTORS, f"unsupported ConnectorKind: {op['kind']}"
    if op["op"] == "activity_node":
        assert op["node"]["kind"] in ALLOWED_ACTIVITY_NODES, f"unsupported Activity node kind: {op['node']['kind']}"
    if op["op"] == "activity_edge":
        assert op["kind"] in ALLOWED_EDGE_KINDS, f"unsupported ActivityEdgeKind: {op['kind']}"
    if op["op"] == "vertex" and op["vertex"]["kind"] == "pseudostate":
        assert op["vertex"]["pseudostate"] in ALLOWED_PSEUDOSTATES
    if op["op"] == "message":
        assert op["sort"] in ALLOWED_MESSAGE_SORTS, f"unsupported MessageSort: {op['sort']}"


def handles(value):
    if isinstance(value, dict):
        for child in value.values():
            yield from handles(child)
    elif isinstance(value, list):
        for child in value:
            yield from handles(child)
    elif isinstance(value, str) and value.startswith("handle:"):
        yield value.removeprefix("handle:")


for operation in operations:
    for target in handles(operation):
        assert target in known, f"unresolved demo handle: {target}"
for diagram in diagrams:
    for target in handles(diagram):
        assert target in known, f"unresolved demo diagram handle: {target}"

families = {diagram["family"] for diagram in diagrams}
assert families == ALLOWED_FAMILIES
package_owners = {
    op["external_id"]: op["owner"]
    for op in operations
    if op.get("op") == "element" and op.get("kind") == "Package"
}
for package_id in (
    "PKG_COMMON", "PKG_REQ", "PKG_UC", "PKG_STRUCTURE",
    "PKG_BEHAVIOR", "PKG_PARAM", "PKG_CONFIG",
):
    assert package_owners[package_id] == "handle:PKG"
for child in ("PKG_ACTIVITIES", "PKG_STATES", "PKG_INTERACTIONS", "PKG_SIGNALS"):
    assert package_owners[child] == "handle:PKG_BEHAVIOR"

semantic_diagrams = {
    diagram.get("semantic", "").removeprefix("handle:")
    for diagram in diagrams if diagram.get("semantic")
}
for semantic_op in ("activity", "state_machine", "interaction"):
    authored = {op["external_id"] for op in operations if op.get("op") == semantic_op}
    assert authored <= semantic_diagrams, f"unpresented {semantic_op}: {sorted(authored - semantic_diagrams)}"

assert any(d["family"] == "IBD" and d.get("context") == "handle:VEH" for d in diagrams)
assert any(d["family"] == "IBD" and d.get("context") == "handle:FLEET" for d in diagrams)
assert any(
    op.get("op") == "activity_node"
    and op.get("node", {}).get("kind") == "call_behavior"
    and op.get("node", {}).get("activity") == "handle:ACT_PREFLIGHT"
    for op in operations
)
assert any(d.get("semantic") == "handle:ACT_PREFLIGHT" for d in diagrams)

relationship_kinds = {
    op.get("kind") for op in operations if op.get("op") == "relationship"
}
assert "PackageImport" in relationship_kinds
assert "DeriveRequirement" in relationship_kinds

print("PR56 drill-down indicator + complete demo contract passed")
