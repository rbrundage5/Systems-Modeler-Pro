from pathlib import Path

root = Path(__file__).resolve().parents[1]
main_rs = (root / "apps/desktop/src-tauri/src/main.rs").read_text(encoding="utf-8")
workspace_rs = (root / "apps/desktop/src-tauri/src/workspace/activity_workspace.rs").read_text(encoding="utf-8")
editing_rs = (root / "apps/desktop/src-tauri/src/workspace/activity_editing.rs").read_text(encoding="utf-8")
mutation_rs = (root / "apps/desktop/src-tauri/src/workspace/activity_mutation.rs").read_text(encoding="utf-8")
frontend = (root / "apps/desktop/frontend/activity-ui.js").read_text(encoding="utf-8")
rich_frontend = (root / "apps/desktop/frontend/activity-rich-ui.js").read_text(encoding="utf-8")
navigation_frontend = (root / "apps/desktop/frontend/activity-navigation-ui.js").read_text(encoding="utf-8")
mutation_frontend = (root / "apps/desktop/frontend/activity-mutation-ui.js").read_text(encoding="utf-8")
index = (root / "apps/desktop/frontend/index.html").read_text(encoding="utf-8")

base_commands = [
    "activity_snapshot",
    "create_activity_diagram",
    "add_activity_node",
    "add_activity_edge",
    "save_activity_workspace",
    "load_activity_workspace",
    "reset_activity_workspace",
]
for command in base_commands:
    assert command in main_rs, f"Activity Tauri command is not registered: {command}"
    assert command in workspace_rs, f"Activity command implementation is missing: {command}"

rich_commands = [
    "add_activity_action",
    "add_activity_parameter_node",
    "add_activity_partition",
    "assign_activity_node_partition",
    "add_structured_activity_node",
    "assign_activity_node_structured_parent",
    "update_activity_node_semantics",
]
for command in rich_commands:
    assert command in main_rs, f"Rich Activity command is not registered: {command}"
    assert command in editing_rs, f"Rich Activity command implementation is missing: {command}"
    assert command in rich_frontend, f"Rich Activity command is not forwarded by the frontend: {command}"

mutation_commands = [
    "delete_activity_item",
    "reconnect_activity_edge",
    "route_activity_diagram",
]
for command in mutation_commands:
    assert command in main_rs, f"Activity mutation command is not registered: {command}"
    assert command in mutation_rs, f"Activity mutation implementation is missing: {command}"
    assert command in mutation_frontend, f"Activity mutation command is not forwarded by the frontend: {command}"

for semantic_kind in [
    "CallBehaviorAction",
    "CallOperationAction",
    "SendSignalAction",
    "AcceptEventAction",
    "AcceptTimeEventAction",
    "ActivityParameterNode",
    "ActivityPartition",
    "StructuredActivityNode",
    "InterruptibleActivityRegion",
]:
    assert semantic_kind in main_rs, f"Rust-owned Activity palette is missing {semantic_kind}"

assert '"Activity" => Ok(vec![' in main_rs, "Rust-owned Activity palette is missing"
assert 'diagramType: \'Activity\'' in frontend, "Activity frontend does not request the Rust palette"
assert "create_activity_diagram" in frontend, "Activity creation is not forwarded to Rust"
assert "add_activity_node" in frontend, "Activity node creation is not forwarded to Rust"
assert "add_activity_edge" in frontend, "Activity flow creation is not forwarded to Rust"
assert "save_activity_workspace" in frontend and "load_activity_workspace" in frontend, "Activity project lifecycle integration is incomplete"
assert 'strip_prefix("pin:")' in workspace_rs, "Rust Activity edge command does not accept semantic pin endpoint tokens"
assert "ActivityEndpoint::Pin" in workspace_rs, "Rust Activity edge command does not persist PinId endpoints"
assert "ObjectFlow pin direction is invalid" in workspace_rs, "Pin direction validation is missing from ObjectFlow creation"
assert "ObjectFlow pin types are incompatible" in workspace_rs, "Pin type compatibility validation is missing from ObjectFlow creation"
assert "activity-pin-anchor" in rich_frontend, "Activity pin presentation anchors are missing"
assert "activity-partition-frame" in rich_frontend, "Activity partition presentation geometry is missing"
assert "activity-structured-frame" in rich_frontend, "Structured Activity presentation geometry is missing"
assert "pin:${pin.id}" in rich_frontend, "Activity frontend does not forward stable PinId endpoint tokens"
assert "CallBehavior" in navigation_frontend and "smpSelectActivityDiagram" in navigation_frontend, "CallBehavior Activity drill-down is missing"
assert "original_activity" in mutation_rs and "original_diagram" in mutation_rs, "Activity mutations are not transactionally recoverable"
assert "incident" in mutation_rs and "activity.edges.retain" in mutation_rs, "Activity node deletion does not remove incident flows"
assert "reroute_diagram" in mutation_rs and "orthogonal_route" in mutation_rs, "Activity mutation routing is not using the shared Rust router"
assert "selectedActivityEdgeId" in mutation_frontend, "Activity flow selection is missing"
assert "Delete" in mutation_frontend and "Backspace" in mutation_frontend, "Activity keyboard deletion is missing"
assert '<script src="activity-ui.js"></script>' in index, "Activity frontend is not loaded"
assert '<script src="activity-rich-ui.js"></script>' in index, "Rich Activity frontend is not loaded"
assert '<script src="activity-navigation-ui.js"></script>' in index, "Activity navigation frontend is not loaded"
assert '<script src="activity-mutation-ui.js"></script>' in index, "Activity mutation frontend is not loaded"
assert '<link rel="stylesheet" href="activity.css" />' in index, "Activity notation stylesheet is not loaded"

# Activity branch/merge/fork/join flows must not collapse back onto the same
# duplicate-endpoint-only lanes. Each flow gets a deterministic diagram-wide
# lane while the shared Rust router continues to enforce node obstacle clearance.
assert "route_semantic_edge(&snapshot, activity, semantic, index)?" in mutation_rs, "Activity flows are not assigned diagram-wide deterministic lanes"
assert "Assign every Activity flow a deterministic diagram-wide lane" in mutation_rs, "Activity routing-lane intent is not documented in the implementation"
assert "candidate.source_node_id == presentation.source_node_id" not in mutation_rs, "Activity routing regressed to duplicate-endpoint-only lanes"

# Frontend may maintain selection/presentation state, but semantic Activity objects
# must only arrive from Rust snapshots and commands.
for source in [frontend, rich_frontend, navigation_frontend, mutation_frontend]:
    for forbidden in [
        "ActivityRepository =",
        "new ActivityRepository",
        "activity.edges.push",
        "activity.nodes.push",
        "activity.partitions.push",
        "activity.structured_nodes.push",
    ]:
        assert forbidden not in source, f"JavaScript appears to own Activity semantics: {forbidden}"

print("Activity desktop integration contract passed")