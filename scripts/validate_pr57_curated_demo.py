"""Static contract for the curated PR57 EV digital-engineering sales demo."""
from __future__ import annotations

from collections import Counter
from pathlib import Path
import json
import math
import re

ROOT = Path(__file__).resolve().parents[1]
DEMO = ROOT / "examples" / "model-script" / "ev-digital-engineering-demo"
EXPECTED_MODULES = [
    "00-foundation.groovy",
    "10-architecture-structure.groovy",
    "11-architecture-execution-features.groovy",
    "20-requirements-verification.groovy",
    "21-use-cases-traceability.groovy",
    "30-vehicle-connectors.groovy",
    "31-subsystem-connectors.groovy",
    "40-core-activities.groovy",
    "41-extended-activities.groovy",
    "50-state-machines.groovy",
    "51-sequences.groovy",
    "52-parametrics-runtime-diagrams.groovy",
]
EXPECTED_NAMESPACE = "demo:ev-digital-engineering:v3"
EXPECTED_DIAGRAMS = {
    "D_PKG": ("Package", "EV Digital Engineering Project Overview"),
    "D_REQ_SYS": ("Requirement", "Core EV System Requirements"),
    "D_REQ_VER": ("Requirement", "Verification Requirements and Test Cases"),
    "D_BDD_VEH": ("BDD", "Electric Vehicle System Breakdown"),
    "D_BDD_PT": ("BDD", "Powertrain Component Breakdown"),
    "D_IBD_VEH": ("IBD", "Electric Vehicle Internal Interfaces"),
    "D_IBD_ESS": ("IBD", "Energy Storage and Charging Internal View"),
    "D_UC": ("Use Case", "EV Operational Use Cases"),
    "D_ACT_OPERATE": ("Activity", "Operate Electric Vehicle"),
    "D_ACT_START": ("Activity", "Start Vehicle — Drill-Down"),
    "D_SM_VEH": ("State Machine", "Vehicle Operational Modes"),
    "D_SEQ_START": ("Sequence", "Vehicle Startup Sequence"),
    "D_PAR_FORCE": ("Parametric", "Tractive Force Analysis — Expected 4.5 kN"),
    "D_PAR_RANGE": ("Parametric", "Driving Range Analysis — Expected 400 km"),
}
EXPECTED_FAMILY_COUNTS = Counter(
    {
        "Package": 1,
        "Requirement": 2,
        "BDD": 2,
        "IBD": 2,
        "Use Case": 1,
        "Activity": 2,
        "State Machine": 1,
        "Sequence": 1,
        "Parametric": 2,
    }
)


def parse_module(path: Path) -> dict:
    source = path.read_text(encoding="utf-8")
    match = re.search(r"modelScript\('''\s*(\{.*\})\s*'''\)\s*$", source, re.S)
    assert match, f"{path.name}: expected modelScript triple-quoted JSON payload"
    return json.loads(match.group(1))


def token_id(value: str) -> str | None:
    value = value.strip()
    if value.startswith("handle:"):
        return value.removeprefix("handle:")
    if value.startswith("ext:"):
        return value.removeprefix("ext:")
    return None


def references(value):
    if isinstance(value, dict):
        for child in value.values():
            yield from references(child)
    elif isinstance(value, list):
        for child in value:
            yield from references(child)
    elif isinstance(value, str):
        found = token_id(value)
        if found:
            yield found


module_paths = [DEMO / name for name in EXPECTED_MODULES]
assert all(path.is_file() for path in module_paths), "PR57 demo module set is incomplete"
documents = [parse_module(path) for path in module_paths]
assert {doc["source_namespace"] for doc in documents} == {EXPECTED_NAMESPACE}

operations = [op for doc in documents for op in doc.get("operations", [])]
diagrams = [diagram for doc in documents for diagram in doc.get("diagrams", [])]
external_ids = [op["external_id"] for op in operations if op.get("external_id")]
assert len(external_ids) == len(set(external_ids)), "operation External IDs must be globally unique"
known = set(external_ids)
by_id = {op["external_id"]: op for op in operations if op.get("external_id")}

for op in operations:
    for ref in references(op):
        assert ref in known, f"unresolved operation reference {ref} in {op.get('external_id', op.get('op'))}"
for diagram in diagrams:
    for ref in references(diagram):
        assert ref in known, f"unresolved diagram reference {ref} in {diagram['external_id']}"

# The standard sales presentation is deliberately bounded.
assert len(diagrams) == 14, f"expected 14 curated diagrams, found {len(diagrams)}"
assert len({d["external_id"] for d in diagrams}) == 14
assert Counter(d["family"] for d in diagrams) == EXPECTED_FAMILY_COUNTS
assert set(d["external_id"] for d in diagrams) == set(EXPECTED_DIAGRAMS)
for diagram in diagrams:
    expected_family, expected_name = EXPECTED_DIAGRAMS[diagram["external_id"]]
    assert diagram["family"] == expected_family
    assert diagram["name"] == expected_name
    assert diagram.get("populate", True), f"{diagram['external_id']} must be populated"
    assert diagram.get("clean_layout", True), f"{diagram['external_id']} must use Clean Layout"
    assert diagram.get("route", True), f"{diagram['external_id']} must be routed"

assert set(EXPECTED_FAMILY_COUNTS) == {
    "Package", "Requirement", "BDD", "IBD", "Use Case",
    "Activity", "State Machine", "Sequence", "Parametric",
}

# Requirements are the anchor of the demonstration.
requirements = [op for op in operations if op.get("op") == "element" and op.get("kind") == "Requirement"]
test_cases = [op for op in operations if op.get("op") == "element" and op.get("kind") == "TestCase"]
assert len(requirements) >= 20, f"expected substantial requirement model, found {len(requirements)}"
assert len(test_cases) >= 9, f"expected verification TestCases, found {len(test_cases)}"
requirement_ids = [op.get("requirement_id") for op in requirements]
assert all(requirement_ids), "every Requirement must have a human-readable requirement_id"
assert len(requirement_ids) == len(set(requirement_ids)), "requirement IDs must be unique"
for req in requirements:
    assert req.get("requirement_text", "").strip(), f"{req['external_id']} has no requirement text"

relationship_ops = [op for op in operations if op.get("op") == "relationship"]
relationship_kinds = {op["kind"] for op in relationship_ops}
for required in ("DeriveRequirement", "Satisfy", "Verify", "Refine", "Trace", "Copy"):
    assert required in relationship_kinds, f"missing requirement semantic {required}"
assert "Composition" not in relationship_kinds, "PartProperty composition must not be replaced by legacy Composition relationships"

# Multi-level parts breakdown must remain real semantic composition usages.
parts = [op for op in operations if op.get("op") == "element" and op.get("kind") == "PartProperty"]
assert len(parts) >= 20, f"expected multi-level PartProperty architecture, found {len(parts)}"
for required in (
    "P_PT", "P_ESS", "P_CTRL", "P_HMI", "P_TMS", "P_BRAKES", "P_SENS",
    "P_INV", "P_MOTOR", "P_BAT", "P_BMS", "P_CHARGER", "P_VCU",
):
    assert required in by_id, f"missing required parts-breakdown usage {required}"

# Interface rigor: typed interfaces, Proxy/Full ports, semantic connectors and ItemFlows.
interface_blocks = [op for op in operations if op.get("op") == "element" and op.get("kind") == "InterfaceBlock"]
proxy_ports = [op for op in operations if op.get("op") == "element" and op.get("kind") == "ProxyPort"]
full_ports = [op for op in operations if op.get("op") == "element" and op.get("kind") == "FullPort"]
assert len(interface_blocks) >= 7
assert len(proxy_ports) >= 20
assert len(full_ports) >= 3
connectors = [op for op in operations if op.get("op") == "connector"]
item_flows = [op for op in operations if op.get("op") == "item_flow"]
assert len(connectors) >= 25, f"expected connected architecture, found {len(connectors)} connectors"
assert {op["kind"] for op in connectors} >= {"Assembly", "Delegation"}
connector_ids = {op["external_id"] for op in connectors}
flow_connector_ids = []
for flow in item_flows:
    connector = token_id(flow["connector"]) or flow["connector"]
    assert connector in connector_ids, f"ItemFlow {flow['external_id']} references unknown Connector {connector}"
    flow_connector_ids.append(connector)
    assert flow.get("conveyed_items"), f"ItemFlow {flow['external_id']} must convey modeled items"
assert set(flow_connector_ids) == connector_ids, "every demo Connector must be realized by at least one ItemFlow"
assert len(item_flows) == len(connectors), "demo uses one focused ItemFlow per Connector"

# Behavior depth remains in the model even though only the strongest views are presented.
activities = [op for op in operations if op.get("op") == "activity"]
state_machines = [op for op in operations if op.get("op") == "state_machine"]
interactions = [op for op in operations if op.get("op") == "interaction"]
assert len(activities) >= 8
assert len(state_machines) >= 2
assert len(interactions) >= 2
assert {d.get("semantic") for d in diagrams if d["family"] == "Activity"} == {"ext:ACT_OPERATE", "ext:ACT_START"}
assert {d.get("semantic") for d in diagrams if d["family"] == "State Machine"} == {"ext:SM_VEH"}
assert {d.get("semantic") for d in diagrams if d["family"] == "Sequence"} == {"ext:SEQ_START"}
assert any(
    op.get("op") == "activity_node"
    and op.get("activity") in {"handle:ACT_OPERATE", "ext:ACT_OPERATE"}
    and op.get("node", {}).get("kind") == "call_behavior"
    and op.get("node", {}).get("activity") in {"handle:ACT_START", "ext:ACT_START"}
    for op in operations
), "OperateElectricVehicle must drill down to StartVehicleBehavior"

trigger_kinds = {
    op.get("trigger", {}).get("kind")
    for op in operations
    if op.get("op") == "transition" and isinstance(op.get("trigger"), dict)
}
for required in ("signal", "call", "time", "change", "any_receive"):
    assert required in trigger_kinds, f"state-machine demo lost {required} trigger coverage"

# Sequence lifelines must resolve to real PartProperty paths and messages to executable definitions.
for lifeline in (op for op in operations if op.get("op") == "lifeline"):
    assert lifeline.get("represented_path"), f"{lifeline['external_id']} has no structural represented_path"
    for segment in lifeline["represented_path"]:
        ref = token_id(segment) or segment
        assert ref in by_id and by_id[ref].get("kind") == "PartProperty", (
            f"{lifeline['external_id']} path segment {ref} is not a PartProperty"
        )
for message in (op for op in operations if op.get("op") == "message"):
    signature = message.get("signature")
    if signature:
        ref = token_id(signature.get("operation", signature.get("signal", "")))
        assert ref in by_id, f"{message['external_id']} has unresolved executable signature"

# Four analyses remain executable; two are intentionally shown in the standard presentation.
constraints = [op for op in operations if op.get("op") == "element" and op.get("kind") == "ConstraintBlock"]
assert len(constraints) >= 4
metadata = {
    token_id(op["element"]): op.get("constraint_expression")
    for op in operations if op.get("op") == "parametric_metadata"
}
assert metadata["CB_FORCE"] == "F = m * a"
assert metadata["CB_POWER"] == "P = V * I"
assert metadata["CB_RANGE"] == "R = E / C"
assert metadata["CB_THERM"] == "M = L - Q"


def default(external_id: str) -> float:
    return float(by_id[external_id]["default_value"])


assert math.isclose(default("V_MASS") * default("V_ACCEL"), 4500.0)
assert math.isclose(default("V_VOLT") * default("V_CURR"), 150000.0)
assert math.isclose(default("V_ENERGY") / default("V_CONS"), 400.0)
assert math.isclose(default("V_COOL") - default("V_HEAT"), 6.0)
for binding_id in ("BF_M", "BF_A", "BF_F", "BP_V", "BP_I", "BP_P", "BR_E", "BR_C", "BR_R", "BT_L", "BT_Q", "BT_M"):
    assert binding_id in by_id, f"missing parametric binding {binding_id}"

# Runtime occurrence isolation demonstration reuses the same ElectricVehicle classifier twice.
assert by_id["FLEET"]["kind"] == "Block"
for usage in ("P_VEH_A", "P_VEH_B"):
    assert by_id[usage]["kind"] == "PartProperty"
    assert by_id[usage]["owner"] == "handle:FLEET"
    assert by_id[usage]["type_ref"] == "ext:VEH"
assert by_id["P_SUPPORT"]["type_ref"] == "handle:SUPPORT_SYS"

# Layering remains explicit for the sales narrative.
for package_import in ("IMP_ARCH_COMMON", "IMP_BEHAV_ARCH", "IMP_PARAM_ARCH", "IMP_REQ_ARCH"):
    assert package_import in by_id and by_id[package_import]["kind"] == "PackageImport"

print(
    "PR57 curated EV demo contract passed: "
    f"{len(requirements)} requirements, {len(test_cases)} TestCases, "
    f"{len(connectors)} connectors/{len(item_flows)} ItemFlows, "
    f"{len(activities)} Activities, {len(diagrams)} curated diagrams"
)
