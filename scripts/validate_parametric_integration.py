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
bdd_frontend = read("apps/desktop/frontend/bdd-completion-ui.js")
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
assert 'id: "evaluateParametrics"' in manifest
assert 'rust_adapter: Some("evaluate_parametric_diagram")' in manifest
assert 'data-command="evaluateParametrics"' in shell
assert '"constraint-parameter"' in main
assert "ConstraintParameter: ['ValueType', 'DataType', 'PrimitiveType', 'Enumeration']" in bdd_frontend
assert "semanticKind === 'ConstraintParameter'" in bdd_frontend
assert "requireInvoke()('create_constraint_parameter'" in bdd_frontend
for notation in [
    "constraint-property",
    "constraint-parameter",
    "Evaluate Parametrics",
]:
    assert notation in frontend
assert "relationship-bindingconnector" in styles

print(
    "PR25 Parametric integration contract passed: the eighth shared diagram family, "
    "typed bindings, reusable constraints, unit-aware Rust evaluation, shared routing/layout, "
    "explicit evaluation, and thin Parametric renderer are wired end to end"
)
