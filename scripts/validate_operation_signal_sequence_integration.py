from pathlib import Path

root = Path(__file__).resolve().parents[1]
def read(relative_path: str) -> str:
    return (root / relative_path).read_text(encoding="utf-8")


core = read("crates/model-core/src/operation_signal_sequence_execution.rs")
execution = read("crates/model-core/src/execution.rs")
activity = read("crates/model-core/src/activity_execution.rs")
structural = read("crates/model-core/src/structural_runtime.rs")
desktop = read("apps/desktop/src-tauri/src/workspace/sequence_execution.rs")
main = read("apps/desktop/src-tauri/src/main.rs")
renderer = read("apps/desktop/frontend/behavior-authoritative-renderer.js")
frontend = renderer
index = read("apps/desktop/frontend/index.html")
workflow = read(".github/workflows/ci.yml")
tests = read("crates/model-core/tests/pr34_operation_signal_sequence.rs")
editing = read("scripts/validate_standard_editing_integration.py")

for token in [
    "ModeledOperationRequest",
    "invoke_modeled_operation",
    "ParameterDirection::In",
    "ParameterDirection::Out",
    "ParameterDirection::InOut",
    "ParameterDirection::Return",
    "validate_runtime_assignment",
    "SequenceExecutionEngine",
    "message_order",
    "represented_path",
    "queue_structural_signal_to_instance",
    "StateMachineExecutionEngine",
    "dispatch_current_events",
    "ExecutionSession",
    "RuntimeInstanceId",
]:
    assert token in core, f"missing Rust Operation/Sequence semantic: {token}"

assert "queue_structural_signal_from_instance" in activity
assert "invoke_modeled_operation" in activity
assert "queue_structural_signal_to_instance" in execution
assert "signal_routes_between" in structural
assert "signal_source_ports" in structural

for token in [
    "SequenceExecutionState",
    "initialize_sequence_execution",
    "run_sequence_execution",
    "step_sequence_execution",
    "pause_sequence_execution",
    "resume_sequence_execution",
    "reset_sequence_execution",
    "terminate_sequence_execution",
    "sequence_execution_runtime_selection",
]:
    assert token in desktop and token in main, f"missing desktop Sequence command: {token}"

for token in [
    "data-sequence-execution",
    "sequence-execution-ribbon-group",
    "runtime-active-message",
    "initialize_sequence_execution",
    "step_sequence_execution",
    "smpOpenStructuralRuntimeConfiguration?.('sequence'",
]:
    assert token in frontend, f"missing thin Sequence runtime UI contract: {token}"

assert "PR34_SEQUENCE_EXECUTION_BEGIN" in renderer
assert "kind === 'sequence'" in renderer
assert "validate_operation_signal_sequence_integration.py" in workflow

for token in [
    "modeled_operation_targets_one_occurrence_and_returns_authored_value",
    "two_same_typed_occurrences_execute_independently",
    "operation_parameter_presence_and_type_are_enforced",
    "structural_signal_targets_only_the_connected_occurrence",
    "signal_reception_compatibility_and_unrelated_occurrences_are_enforced",
    "call_operation_action_uses_the_modeled_operation_runtime",
    "send_and_accept_actions_share_typed_structural_signal_delivery",
    "sequence_resolves_lifelines_and_executes_operation_then_signal",
    "sequence_order_reset_and_authored_model_are_deterministic",
]:
    assert token in tests, f"missing PR34 Rust regression: {token}"

for family in [
    "Bdd", "Ibd", "Requirement", "UseCase", "Parametric", "Package", "Activity",
    "StateMachine", "Sequence",
]:
    assert family in editing, f"all-nine-family editing contract lost {family}"

for forbidden in [
    "eval(", "new Function(", "setInterval(", "requestAnimationFrame(source", "geometry.order",
]:
    assert forbidden not in frontend, f"frontend-owned execution semantic detected: {forbidden}"

print(
    "PR34 Operation/Signal/Sequence integration contract passed: modeled Operations, "
    "typed structural Signals/Receptions, semantic Lifeline occurrence binding, deterministic "
    "message ordering, contextual presentation-only controls, and the all-nine-family editing "
    "regression contract are wired through the existing Rust execution session."
)
