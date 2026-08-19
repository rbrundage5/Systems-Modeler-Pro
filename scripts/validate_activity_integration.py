from pathlib import Path

root = Path(__file__).resolve().parents[1]
main_rs = (root / "apps/desktop/src-tauri/src/main.rs").read_text(encoding="utf-8")
workspace_rs = (root / "apps/desktop/src-tauri/src/workspace/activity_workspace.rs").read_text(encoding="utf-8")
frontend = (root / "apps/desktop/frontend/activity-ui.js").read_text(encoding="utf-8")
index = (root / "apps/desktop/frontend/index.html").read_text(encoding="utf-8")

required_commands = [
    "activity_snapshot",
    "create_activity_diagram",
    "add_activity_node",
    "add_activity_edge",
    "save_activity_workspace",
    "load_activity_workspace",
    "reset_activity_workspace",
]
for command in required_commands:
    assert command in main_rs, f"Activity Tauri command is not registered: {command}"
    assert command in workspace_rs, f"Activity command implementation is missing: {command}"

assert '"Activity" => Ok(vec![' in main_rs, "Rust-owned Activity palette is missing"
assert 'diagramType: \'Activity\'' in frontend, "Activity frontend does not request the Rust palette"
assert "create_activity_diagram" in frontend, "Activity creation is not forwarded to Rust"
assert "add_activity_node" in frontend, "Activity node creation is not forwarded to Rust"
assert "add_activity_edge" in frontend, "Activity flow creation is not forwarded to Rust"
assert "save_activity_workspace" in frontend and "load_activity_workspace" in frontend, "Activity project lifecycle integration is incomplete"
assert '<script src="activity-ui.js"></script>' in index, "Activity frontend is not loaded"
assert '<link rel="stylesheet" href="activity.css" />' in index, "Activity notation stylesheet is not loaded"

# Frontend may maintain selection/presentation state, but semantic Activity objects
# must only arrive from Rust snapshots and commands.
for forbidden in [
    "ActivityRepository =",
    "new ActivityRepository",
    "activity.edges.push",
    "activity.nodes.push",
]:
    assert forbidden not in frontend, f"JavaScript appears to own Activity semantics: {forbidden}"

print("Activity desktop integration contract passed")
