"""Validate the PR24 Use Case semantic and shared-workspace integration boundary."""

from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(relative: str) -> str:
    return (ROOT / relative).read_text(encoding="utf-8")


model = read("crates/model-core/src/model.rs")
families = read("crates/model-core/src/diagram_family.rs")
desktop = read("apps/desktop/src-tauri/src/workspace/use_cases.rs")
workspace = read("apps/desktop/src-tauri/src/workspace.rs")
shared = read("apps/desktop/src-tauri/src/workspace/shared_workspace.rs")
standard = read("apps/desktop/src-tauri/src/workspace/standard_editing.rs")
theme = read("apps/desktop/src-tauri/src/workspace/presentation_theme.rs")
main = read("apps/desktop/src-tauri/src/main.rs")
frontend = read("apps/desktop/frontend/use-case-ui.js")
styles = read("apps/desktop/frontend/use-case.css")
index = read("apps/desktop/frontend/index.html")

for token in [
    "Actor",
    "UseCase",
    "Include",
    "Extend",
    "extension_points",
    "use_case_specification",
    "represented_classifier_id",
    "extension_condition",
    "extension_location",
    "validate_use_case_relationship_endpoints",
    "InvalidUseCaseGeneralization",
]:
    assert token in model, f"missing Rust Use Case semantic contract: {token}"

for command in [
    "create_use_case_diagram",
    "create_use_case_element",
    "update_actor_details",
    "update_use_case_specification",
    "update_use_case_diagram_subject",
    "update_use_case_subject_boundary_geometry",
    "update_use_case_actor_notation",
    "place_on_use_case_diagram",
    "create_use_case_relationship",
    "reconnect_use_case_relationship",
    "update_extend_specification",
    "delete_use_case_relationship",
]:
    assert f"pub fn {command}(" in desktop, f"missing Rust Use Case command: {command}"
    assert command in main, f"Use Case command not registered: {command}"

assert '"use-case"' in families
assert '("uc", "Package")' in families
assert 'PreferredFlowDirection::LeftToRight' in families
assert '"bdd" | "requirement" | "use-case"' in shared
assert 'family: "use-case".into()' in desktop
assert "semantic_context_id" in workspace and "semantic_context_id" in desktop
assert "EditingFamily::UseCase" in standard
assert "use_case_compatible" in standard
assert '("Actor", USE_CASE)' in theme and '("UseCase", USE_CASE)' in theme
assert "registerRenderer('use-case'" in read("apps/desktop/frontend/shared-workspace.js")

for notation in [
    "actor-figure",
    "actor-rectangle",
    "use-case-presentation",
    "use-case-subject-boundary",
    "extension-point-compartment",
    "relationship-include",
    "relationship-extend",
]:
    assert notation in frontend or notation in styles, f"missing Use Case notation: {notation}"

for behavior in [
    "create_use_case_relationship",
    "place_on_use_case_diagram",
    "update_bdd_presentation_geometry",
    "createRelationshipLayer",
    "application/x-smp-repository-element-id",
    "extensionLocation",
    "subject_boundary",
    "update_use_case_subject_boundary_geometry",
    "update_association_end",
]:
    assert behavior in frontend or behavior in read(
        "apps/desktop/frontend/structural-interaction-rebind.js"
    ), f"missing Use Case workspace behavior: {behavior}"

assert "use-case.css" in index and "use-case-ui.js" in index
assert "UseCaseSubjectBoundary" in standard
assert "actor_notation" in workspace and "actor_notation" in frontend
assert "subject_boundary" in workspace and "subject_boundary" in frontend
assert "localStorage" not in frontend, "Use Case semantics must not use browser persistence"
assert "new Map" not in desktop, "Use Case adapter must reuse existing workspace stores"

print("PR24 Use Case integration contract passed")
