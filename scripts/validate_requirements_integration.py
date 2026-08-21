from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def read(relative: str) -> str:
    return (ROOT / relative).read_text(encoding="utf-8")


model = read("crates/model-core/src/model.rs")
desktop = read("apps/desktop/src-tauri/src/workspace/requirements.rs")
main = read("apps/desktop/src-tauri/src/main.rs")
frontend = read("apps/desktop/frontend/app.js")
families = read("crates/model-core/src/diagram_family.rs")

for token in [
    "Requirement",
    "TestCase",
    "DeriveRequirement",
    "Satisfy",
    "Verify",
    "Refine",
    "Trace",
    "Copy",
    "requirement_id",
    "requirement_text",
]:
    assert token in model, f"missing Rust Requirement contract: {token}"

for command in [
    "create_requirement_diagram",
    "create_requirement",
    "create_test_case",
    "update_requirement",
    "place_on_requirement_diagram",
    "create_traceability_relationship",
]:
    assert command in desktop, f"missing Rust Requirement command: {command}"
    assert command in main, f"Requirement command not registered: {command}"

assert '"requirement"' in families and '("req", "Package")' in families
assert "create_traceability_relationship" in frontend
assert "update_requirement" in frontend
assert "localStorage" not in frontend, "Requirement integration must not create browser semantic persistence"

print("Requirements integration contract passed")
