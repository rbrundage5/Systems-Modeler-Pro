from pathlib import Path

root = Path(__file__).resolve().parents[1]
main_rs = (root / "apps/desktop/src-tauri/src/main.rs").read_text(encoding="utf-8")
interaction_rs = (
    root / "apps/desktop/src-tauri/src/workspace/presentation_interaction.rs"
).read_text(encoding="utf-8")
frontend = (root / "apps/desktop/frontend/diagram-interaction.js").read_text(
    encoding="utf-8"
)
index = (root / "apps/desktop/frontend/index.html").read_text(encoding="utf-8")

commands = [
    "update_bdd_presentation_geometry",
    "update_ibd_property_geometry",
    "update_ibd_port_geometry",
    "update_state_presentation_geometry",
    "update_activity_presentation_geometry",
]
for command in commands:
    assert command in main_rs, f"Presentation command is not registered: {command}"
    assert command in interaction_rs, f"Presentation command is not implemented: {command}"
    assert command in frontend, f"Frontend does not delegate presentation update: {command}"

for selector in [
    ".bdd-block",
    ".ibd-property",
    ".ibd-port",
    ".state-vertex",
    ".sequence-lifeline",
    ".activity-node",
]:
    assert selector in frontend, f"Shared interaction layer is missing {selector}"

assert "move_sequence_lifeline" in frontend or "sequence-lifeline" in frontend
assert "resize_sequence_lifeline_timeline" in (
    root / "apps/desktop/frontend/behavior-authoritative-renderer.js"
).read_text(encoding="utf-8")
assert "route_relationship" in interaction_rs, "BDD rerouting is not integrated"
assert "route_ibd_edge" in interaction_rs, "IBD rerouting is not integrated"
assert "orthogonal_route" in interaction_rs, "Activity rerouting is not integrated"
assert "port.y = y.clamp" in interaction_rs, "IBD ports are not constrained to boundaries"
assert '<script src="diagram-interaction.js"></script>' in index

print("Shared presentation interaction contract passed")
