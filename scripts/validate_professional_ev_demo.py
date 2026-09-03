"""PR57 contract for the single-file professional EV SysML sales demo."""
from pathlib import Path
import json
import re

ROOT = Path(__file__).resolve().parents[1]
DEMO = ROOT / "examples/model-script/professional-ev-demo.groovy"
source = DEMO.read_text(encoding="utf-8")

assert source.count("modelScript('''") == 1, "demo must be one directly importable modelScript file"
match = re.search(r"modelScript\('''\s*(\{.*\})\s*'''\)\s*$", source, re.S)
assert match, "demo must be a modelScript triple-quoted JSON payload"
demo = json.loads(match.group(1))
operations = demo["operations"]
diagrams = demo["diagrams"]

external_ids = [op["external_id"] for op in operations if op.get("external_id")]
assert len(external_ids) == len(set(external_ids)), "operation External IDs must be unique"
known = set(external_ids)


def handles(value):
    if isinstance(value, dict):
        for child in value.values():
            yield from handles(child)
    elif isinstance(value, list):
        for child in value:
            yield from handles(child)
    elif isinstance(value, str) and value.startswith("handle:"):
        yield value.removeprefix("handle:")


for record in [*operations, *diagrams]:
    for target in handles(record):
        assert target in known, f"unresolved demo handle: {target}"

families = {diagram["family"] for diagram in diagrams}
assert families == {
    "Package", "BDD", "IBD", "Requirement", "Use Case",
    "Activity", "State Machine", "Sequence", "Parametric",
}
assert len(diagrams) == 12, f"sales demo must stay compact; found {len(diagrams)} diagrams"
assert all(diagram.get("populate", True) for diagram in diagrams)
assert all(diagram.get("clean_layout", True) for diagram in diagrams)
assert all(diagram.get("route", True) for diagram in diagrams)

elements = [op for op in operations if op.get("op") == "element"]
relationships = [op for op in operations if op.get("op") == "relationship"]
requirements = [op for op in elements if op.get("kind") == "Requirement"]
test_cases = [op for op in elements if op.get("kind") == "TestCase"]
assert len(requirements) == 16
assert len(test_cases) == 5
required_trace_kinds = {"DeriveRequirement", "Satisfy", "Verify", "Refine", "Trace", "Copy"}
assert required_trace_kinds <= {op["kind"] for op in relationships}
assert any(d["family"] == "Requirement" and d["external_id"] == "D_REQ" for d in diagrams)
assert any(d["family"] == "Requirement" and d["external_id"] == "D_VERIFY" for d in diagrams)

assert not any(op.get("kind") == "Composition" for op in relationships)
parts = [op for op in elements if op.get("kind") == "PartProperty"]
assert {op["external_id"] for op in parts} >= {
    "P_POWERTRAIN", "P_BATTERY", "P_CTRL", "P_HMI", "P_BRAKES", "P_THERMAL",
    "P_INVERTER", "P_MOTOR",
}
assert any(op.get("kind") == "FullPort" for op in elements)
assert any(op.get("kind") == "ProxyPort" for op in elements)
connectors = [op for op in operations if op.get("op") == "connector"]
item_flows = [op for op in operations if op.get("op") == "item_flow"]
assert len(connectors) == 11
assert len(item_flows) == 11
assert {op["kind"] for op in connectors} == {"Assembly", "Delegation"}
assert {op["connector"].removeprefix("handle:") for op in item_flows} == {
    op["external_id"] for op in connectors
}
assert any(d["family"] == "IBD" and d.get("context") == "handle:VEH" for d in diagrams)
assert any(d["family"] == "IBD" and d.get("context") == "handle:POWERTRAIN" for d in diagrams)

activities = {op["external_id"] for op in operations if op.get("op") == "activity"}
activity_diagrams = {
    d.get("semantic", "").removeprefix("handle:")
    for d in diagrams if d["family"] == "Activity"
}
assert activities == {"ACT_START", "ACT_OPERATE"}
assert activities == activity_diagrams
assert any(
    op.get("op") == "activity_node"
    and op.get("node", {}).get("kind") == "call_behavior"
    and op["node"].get("activity") == "handle:ACT_START"
    for op in operations
)
assert any(op.get("op") == "state_machine" for op in operations)
assert any(op.get("op") == "interaction" for op in operations)
assert any(d["family"] == "State Machine" for d in diagrams)
assert any(d["family"] == "Sequence" for d in diagrams)
assert any(
    op.get("op") == "lifeline"
    and op.get("represented_path") == ["handle:P_POWERTRAIN", "handle:P_INVERTER"]
    for op in operations
), "sequence must demonstrate nested structural path resolution"

use_case_kinds = {
    op["kind"] for op in relationships
    if op["kind"] in {"Include", "Extend", "Generalization"}
}
assert use_case_kinds == {"Include", "Extend", "Generalization"}

metadata = {
    op["element"].removeprefix("handle:"): op["constraint_expression"]
    for op in operations if op.get("op") == "parametric_metadata"
}
assert metadata == {
    "CB_FORCE": "F = m * a",
    "CB_POWER": "P = F * v",
    "CB_RANGE": "R = E / c",
}
assert sum(op.get("op") == "binding" for op in operations) == 9
values = {
    op["external_id"]: float(op["default_value"])
    for op in elements
    if op.get("kind") == "ValueProperty" and op.get("default_value") is not None
}
force = values["V_MASS"] * values["V_ACCEL"]
power = force * values["V_SPEED"]
vehicle_range = values["V_ENERGY"] / values["V_CONSUMPTION"]
assert force == 4500.0
assert power == 135000.0
assert vehicle_range == 400.0
assert any(d["family"] == "Parametric" and d.get("context") == "handle:VEH" for d in diagrams)

print("PR57 professional EV single-file demo contract passed")
