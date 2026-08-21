from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def read(relative: str) -> str:
    return (ROOT / relative).read_text(encoding="utf-8")


model = read("crates/model-core/src/model.rs")
desktop = read("apps/desktop/src-tauri/src/workspace/requirements.rs")
main = read("apps/desktop/src-tauri/src/main.rs")
frontend = read("apps/desktop/frontend/app.js")
ibd_ui = read("apps/desktop/frontend/ibd-ui.js")
bdd_completion = read("apps/desktop/frontend/bdd-completion-ui.js")
bdd_extended = read("apps/desktop/frontend/bdd-extended-ui.js")
bdd_feature_editing = read("apps/desktop/frontend/bdd-feature-editing.js")
palette_icons = read("apps/desktop/frontend/palette-icons.js")
visibility = read("apps/desktop/frontend/bdd-compartment-visibility.js")
families = read("crates/model-core/src/diagram_family.rs")
theme = read("apps/desktop/src-tauri/src/workspace/presentation_theme.rs")
catia = read("apps/desktop/frontend/catia-light.css")

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
assert "selectedBehaviorDiagramId: null" in frontend
assert "selectedActivityDiagramId: null" in frontend
assert "diagram?.family === 'requirement' ? 'Requirement'" in ibd_ui
assert "diagram.family === 'requirement' ? 'REQ' : 'BDD'" in bdd_completion
assert "active?.family === 'requirement'" in bdd_completion
assert "createStructuralPaletteElementAt(item, x, y)" in bdd_completion
assert "renderStructuralCanvas" in bdd_completion and "renderStructuralProperties" in bdd_completion
assert "diagram?.family === 'requirement') return baseRenderCanvasExtended()" not in bdd_extended
assert "box.innerHTML = extendedElementMarkup(project, element)" in bdd_extended
assert "box.dataset.semanticKind = element.kind" in bdd_extended
assert "box.dataset.presentationId = node.id" in bdd_extended
assert "diagram.family === 'requirement' ? 'req' : 'bdd'" in bdd_extended
assert "create_traceability_relationship" in bdd_extended
assert "element.kind === 'Requirement'" in bdd_extended
assert 'class="classifier-header"' in bdd_extended
assert ".classifier-header .block-name{background:transparent!important}" in theme
assert ".classifier-header{" in catia and ".bdd-block.kind-comment::after" in catia
assert "const renderStructuralProperties = renderProperties" in bdd_feature_editing
assert "element?.kind === 'Requirement' || element?.kind === 'TestCase'" in bdd_feature_editing
assert "return renderStructuralProperties()" in bdd_feature_editing
assert 'compartment-title">id' in frontend and 'compartment-title">text' in frontend
assert "Presentation Display" in visibility and "['id', 'text'" in visibility
assert "visibleCompartments.length" in visibility
assert "compartments-collapsed" in visibility
assert "box.querySelectorAll('.compartment')" in visibility
assert "smpCompartmentDisplay" in visibility
assert "window.smpRequirementDisplay = window.smpCompartmentDisplay" in visibility
assert "const compartmentDisplay=window.smpCompartmentDisplay?.(element.id)" in bdd_feature_editing
assert "Choose which compartments are visible" in bdd_feature_editing
assert "show-requirement-id" in frontend and "smpRequirementDisplay" in visibility
assert "ElementKind::Requirement => (260.0, 180.0)" in desktop
assert "DeriveRequirement: 'R┄➤'" in palette_icons
for semantic_kind in ["AssociationBlock", "InterfaceBlock", "ConstraintBlock", "ValueType", "DataType", "PrimitiveType", "Enumeration", "Signal", "Unit", "QuantityKind", "InstanceSpecification", "Comment"]:
    assert f'"{semantic_kind}"' in main, f"Requirement palette missing supported model element: {semantic_kind}"
assert "localStorage" not in frontend, "Requirement integration must not create browser semantic persistence"

print("Requirements integration contract passed")
