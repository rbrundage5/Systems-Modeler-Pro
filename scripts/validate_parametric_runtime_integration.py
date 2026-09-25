from pathlib import Path

root = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (root / path).read_text(encoding="utf-8")


core = read("crates/model-core/src/parametric_execution.rs")
parametrics = read("crates/model-core/src/parametrics.rs")
tests = read("crates/model-core/tests/pr35_parametric_runtime.rs")
desktop = read("apps/desktop/src-tauri/src/workspace/parametric_execution.rs")
main = read("apps/desktop/src-tauri/src/main.rs")
frontend = read("apps/desktop/frontend/parametric-ui.js")
runtime_ui = read("apps/desktop/frontend/behavior-authoritative-renderer.js")
css = read("apps/desktop/frontend/structural-runtime.css")
workflow = read(".github/workflows/ci.yml")
editing = read("scripts/validate_standard_editing_integration.py")
pr25_tests = read("crates/model-core/tests/pr25_parametrics.rs")

for token in [
    "ParametricExecutionEngine",
    "ExecutionSession",
    "evaluate_parametrics(&mut scratch",
    "value_in_instance_context",
    "session.set_value",
    "runtime_instance_id",
    "runtime_instance_path",
    "session.complete",
]:
    assert token in core, f"missing shared Parametric runtime semantic: {token}"

assert "project.element_mut" not in core, "Parametric runtime must not mutate the authored Project"
assert "ParametricEvaluationScope" in parametrics
assert "Serialize, Deserialize" in parametrics

for token in [
    "force_equation_evaluates_into_runtime_without_mutating_authored_model",
    "repeated_vehicle_occurrences_keep_parametric_values_isolated",
    "reset_replays_authored_inputs_deterministically",
    "missing_runtime_input_fails_with_readable_diagnostic_and_no_authored_mutation",
    "unsupported_expression_is_rejected_instead_of_executed",
    "ambiguous_repeated_context_requires_explicit_occurrence_selection",
]:
    assert token in tests, f"missing PR35 Rust regression: {token}"

for token in [
    "ParametricExecutionState",
    "initialize_parametric_execution",
    "evaluate_parametric_execution",
    "step_parametric_execution",
    "reset_parametric_execution",
    "terminate_parametric_execution",
    "parametric_execution_runtime_selection",
]:
    assert token in desktop and token in main, f"missing desktop Parametric runtime command: {token}"

assert "ExecutionManager" in desktop, "Parametric runtime must reuse the shared ExecutionManager"

for token in [
    "data-parametric-execution",
    "parametric-execution-ribbon-group",
    "Evaluate Runtime",
    "evaluate_parametric_execution",
    "smpOpenStructuralRuntimeConfiguration?.('parametric'",
    "parametric-execution-panel",
]:
    assert token in frontend or token in css, f"missing thin Parametric runtime UI contract: {token}"

assert "requireInvoke()('evaluate_parametric_diagram'" not in frontend
assert "kind === 'parametric'" in runtime_ui
assert 'data-execution-family="parametric"' in css
assert "validate_parametric_runtime_integration.py" in workflow

for token in [
    "evaluation_rejects_constraint_dependency_cycles",
    "binding_rejects_incompatible_quantity_kinds_and_self_connections",
    "evaluation_reports_unbound_mandatory_parameters_without_mutating_values",
]:
    assert token in pr25_tests, f"PR25 Parametric safety regression lost: {token}"

for family in [
    "Bdd",
    "Ibd",
    "Requirement",
    "UseCase",
    "Parametric",
    "Package",
    "Activity",
    "StateMachine",
    "Sequence",
]:
    assert family in editing, f"all-nine-family editing contract lost {family}"

for forbidden in ["eval(", "new Function("]:
    assert forbidden not in frontend, f"frontend-owned Parametric evaluation detected: {forbidden}"

print(
    "PR35 Parametric runtime integration contract passed: PR25 bounded constraint semantics "
    "execute through the shared occurrence-scoped ExecutionSession, authored model data remains "
    "unchanged, contextual controls are presentation-only, and prior runtime/editing contracts remain wired."
)
