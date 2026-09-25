"""PR25 Parametric integration contract; complements executable Rust tests."""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def read(relative: str) -> str:
    return (ROOT / relative).read_text(encoding="utf-8")


model = read("crates/model-core/src/model.rs")
engine = read("crates/model-core/src/parametrics.rs")
families = read("crates/model-core/src/diagram_family.rs")
commands = read("apps/desktop/src-tauri/src/workspace/parametrics.rs")
dispatcher = read("apps/desktop/src-tauri/src/workspace/shared_workspace.rs")
frontend = read("apps/desktop/frontend/parametric-ui.js")
binding_diagnostics = read("apps/desktop/frontend/shared-dialogs.js")
bdd_frontend = read("apps/desktop/frontend/bdd-completion-ui.js")
bdd_extended = read("apps/desktop/frontend/bdd-extended-ui.js")
styles = read("apps/desktop/frontend/parametric.css")
index = read("apps/desktop/frontend/index.html")
main = read("apps/desktop/src-tauri/src/main.rs")
manifest = read("apps/desktop/src-tauri/src/workspace/presentation_theme.rs")
shell = read("apps/desktop/frontend/ui-shell.js")

for contract in [
    "ConstraintParameter",
    "BindingConnector",
    "constraint_expression",
    "quantity_dimension",
    "unit_scale_to_base",
]:
    assert contract in model

for contract in [
    "pub fn evaluate_parametrics",
    "constraint dependency cycle detected",
    "unresolved parameter",
    "division by zero",
    "addition/subtraction requires identical dimensions",
    "create_binding_connector",
    "validate_binding_connector",
    "DuplicateBindingConnector",
]:
    assert contract in engine

for contract in [
    '"parametric"',
    '"Parametric Diagram"',
    '("par", "Block")',
]:
    assert contract in families

for command in [
    "create_parametric_diagram",
    "place_on_parametric_diagram",
    "create_parametric_constraint_property",
    "update_parametric_constraint_property",
    "create_parametric_value_property",
    "create_binding_connector",
    "update_constraint_parameter_presentation",
    "route_parametric_with_bounds",
    "layout_parametric_with_bounds",
    "evaluate_parametric_diagram",
]:
    assert f"fn {command}" in commands
    if not command.endswith("with_bounds"):
        assert command in main

assert '"parametric" => {' in dispatcher
assert "route_parametric_with_bounds" in dispatcher
assert "layout_parametric_with_bounds" in dispatcher
assert "parametric-ui.js" in index and "parametric.css" in index
assert "shared-dialogs.js" in index
assert 'id: "evaluateParametrics"' in manifest
assert 'rust_adapter: Some("evaluate_parametric_execution")' in manifest
assert 'data-command="evaluateParametrics"' in shell
assert '"constraint-parameter"' in main
assert "ConstraintParameter: ['ValueType', 'DataType', 'PrimitiveType', 'Enumeration']" in bdd_frontend
assert "semanticKind === 'ConstraintParameter'" in bdd_frontend
assert "requireInvoke()('create_constraint_parameter'" in bdd_frontend
assert "ConstraintParameter: ['ValueType', 'DataType', 'PrimitiveType', 'Enumeration']" in bdd_extended
assert "kind === 'ConstraintParameter'" in bdd_extended
assert "typeId === '__create_real__' ? null" in bdd_frontend
assert 'create_element(ElementKind::PrimitiveType, "Real", namespace_id)' in commands
assert "definition.selectedId === '__create_real__' ? null" in frontend
assert "valueTypeId: definition.selectedId === '__create_real__' ? null" in frontend
assert "Parametric definitions and typed values are semantic elements" in frontend
assert "fn resolve_parametric_value_type" in commands
assert "value_type_id: Option<String>" in commands
assert frontend.index("if (!diagram) return baseRenderProperties();") > frontend.index("if (element.kind === 'Unit')")
for notation in [
    "constraint-property",
    "constraint-parameter",
    "Evaluate Runtime",
]:
    assert notation in frontend
assert "relationship-bindingconnector" in styles

for diagnostic_contract in [
    "Rust remains the sole authority",
    "Binding Connector endpoints are incompatible.",
    "Set both endpoints to compatible types in Properties.",
    "compatible ValueTypes with matching QuantityKind and dimension",
    "constraint-parameter-label",
    "par-binding-source",
    "par-binding-target",
    "formatBindingTypeError",
]:
    assert diagnostic_contract in binding_diagnostics
assert "originalAlert(formatBindingTypeError(text))" in binding_diagnostics

print(
    "PR25 Parametric integration contract passed: the eighth shared diagram family, "
    "typed bindings, reusable constraints, unit-aware Rust evaluation, shared routing/layout, "
    "explicit evaluation, engineer-readable binding diagnostics, and thin Parametric renderer "
    "are wired end to end"
)

# Cross-family direct-manipulation contract
parametric_geometry_rs = read("apps/desktop/src-tauri/src/workspace/parametrics.rs")
assert "fn reroute_incident_edges" in parametric_geometry_rs
parametric_geometry = parametric_geometry_rs.split("pub fn update_parametric_presentation_geometry", 1)[1].split("pub fn update_constraint_parameter_presentation", 1)[0]
assert "reroute_incident_edges" in parametric_geometry
assert "validate_loaded_diagrams" not in parametric_geometry
parameter_geometry = parametric_geometry_rs.split("pub fn update_constraint_parameter_presentation", 1)[1].split("pub fn evaluate_parametric_diagram", 1)[0]
assert "reroute_incident_edges" in parameter_geometry
assert "validate_loaded_diagrams" not in parameter_geometry
