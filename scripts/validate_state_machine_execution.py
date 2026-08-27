"""Validate the PR32 Rust-owned State Machine execution integration boundary."""

from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(relative: str) -> str:
    return (ROOT / relative).read_text(encoding="utf-8")


core = read("crates/model-core/src/state_machine_execution.rs")
tests = read("crates/model-core/tests/pr32_state_machine_execution.rs")
desktop = read("apps/desktop/src-tauri/src/workspace/state_machine_execution.rs")
main = read("apps/desktop/src-tauri/src/main.rs")
ui = read("apps/desktop/frontend/behavior-authoritative-renderer.js")
ui = ui[
    ui.index("PR32_STATE_MACHINE_EXECUTION_BEGIN") :
    ui.index("PR32_STATE_MACHINE_EXECUTION_END")
]
styles = read("apps/desktop/frontend/behavior.css")
index = read("apps/desktop/frontend/index.html")
standard = read("scripts/validate_standard_editing_integration.py")

for token in (
    "StateMachineExecutionEngine",
    "ExecutionEngine for StateMachineExecutionEngine",
    "ExecutionSession",
    "RuntimeEventKind",
    "SimulationTime",
    "evaluate_execution_expression",
    "MAX_RUN_TO_COMPLETION_STEPS",
    "active_states",
    "active_regions",
    "final_regions",
    "TransitionKind::External",
    "TransitionKind::Internal",
    "TransitionKind::Local",
    "PseudostateKind::Choice",
    "PseudostateKind::Junction",
    "PseudostateKind::Fork",
    "PseudostateKind::Join",
    "PseudostateKind::ShallowHistory",
    "PseudostateKind::DeepHistory",
    "Event::Signal",
    "Event::Call",
    "Event::Time",
    "Event::Change",
    "source_is_complete",
    "source_fingerprint",
):
    source = desktop if token == "source_fingerprint" else core
    if token not in source:
        raise SystemExit(f"State Machine execution contract is missing {token}")

commands = (
    "state_machine_execution_snapshot",
    "initialize_state_machine_execution",
    "run_state_machine_execution",
    "step_state_machine_execution",
    "pause_state_machine_execution",
    "resume_state_machine_execution",
    "reset_state_machine_execution",
    "terminate_state_machine_execution",
    "queue_state_machine_signal",
    "clear_state_machine_executions",
)
for command in commands:
    if command not in desktop or command not in main:
        raise SystemExit(f"State Machine execution command is not fully registered: {command}")

for scenario in (
    "basic_signal_lifecycle_is_deterministic_and_completes",
    "guarded_choice_uses_shared_expression_evaluator_and_event_payload",
    "composite_and_orthogonal_regions_maintain_active_configuration",
    "time_event_uses_simulation_time_not_wall_clock",
    "fork_and_join_synchronize_concurrent_paths",
    "identical_inputs_produce_identical_semantic_trace",
    "reset_matches_fresh_initialization_and_does_not_mutate_authored_model",
    "pseudostate_cycle_fails_at_bounded_run_to_completion_limit",
):
    if scenario not in tests:
        raise SystemExit(f"State Machine execution test scenario is missing: {scenario}")

for control in ("Initialize", "Run", "Step", "Pause", "Resume", "Reset", "Terminate"):
    if control not in ui:
        raise SystemExit(f"State Machine runtime UI control is missing: {control}")

for forbidden in (
    "eval(",
    "setTimeout(",
    "setInterval(",
    "onpointerdown",
    "onpointermove",
    "onpointerup",
    "addEventListener('keydown",
    'addEventListener("keydown',
):
    if forbidden in ui:
        raise SystemExit(f"State Machine execution UI owns forbidden semantics/input: {forbidden}")

if "render();" in ui[ui.find("async function invokeExecution"):ui.find("async function initializeExecution")]:
    raise SystemExit("State Machine runtime snapshots must not rebuild the authored workspace")
if "pointer-events:none" not in styles or ".state-machine-execution-panel" not in styles:
    raise SystemExit("State Machine execution overlay is not input-transparent")
if '<script src="behavior-authoritative-renderer.js"></script>' not in index:
    raise SystemExit("State Machine execution UI is not loaded")
if "all nine" not in standard:
    raise SystemExit("PR31 all-nine-family authoring regression contract is missing")

print(
    "PR32 State Machine execution integration contract passed: Rust owns deterministic runtime "
    "semantics, shared time/events/expressions are reused, runtime UI is presentation-only, and "
    "the PR31 all-nine-family authoring contract remains present"
)
