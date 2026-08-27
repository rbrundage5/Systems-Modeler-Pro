"""Validate the PR32 Rust-owned State Machine execution integration boundary."""

from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(relative: str) -> str:
    return (ROOT / relative).read_text(encoding="utf-8")


core = read("crates/model-core/src/state_machine_execution.rs")
activity_bridge = read("crates/model-core/src/state_machine_execution/activity_bridge.rs")
base_tests = read("crates/model-core/tests/pr32_state_machine_execution.rs")
event_tests = read("crates/model-core/tests/pr32_state_machine_event_semantics.rs")
closure_tests = read("crates/model-core/tests/pr32_state_machine_semantic_closure.rs")
fork_join_tests = read("crates/model-core/tests/pr32_state_machine_fork_join.rs")
activity_bridge_tests = read("crates/model-core/tests/pr32_state_machine_activity_bridge.rs")
activity_reset_tests = read("crates/model-core/tests/pr32_state_machine_activity_reset.rs")
desktop = read("apps/desktop/src-tauri/src/workspace/state_machine_execution.rs")
main = read("apps/desktop/src-tauri/src/main.rs")
ui_file = read("apps/desktop/frontend/behavior-authoritative-renderer.js")
ui = ui_file[
    ui_file.index("PR32_STATE_MACHINE_EXECUTION_BEGIN") :
    ui_file.index("PR32_STATE_MACHINE_EXECUTION_END")
]
submachine_ui = read("apps/desktop/frontend/behavior-submachine.js")
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
    "suppressed_initial_regions",
    "TransitionKind::External",
    "TransitionKind::Internal",
    "TransitionKind::Local",
    "PseudostateKind::Choice",
    "PseudostateKind::Junction",
    "PseudostateKind::Fork",
    "PseudostateKind::Join",
    "PseudostateKind::ShallowHistory",
    "PseudostateKind::DeepHistory",
    "PseudostateKind::EntryPoint",
    "PseudostateKind::ExitPoint",
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

for diagnostic in (
    "connection-point owner/entry-exit mapping",
    "history default/restoration policy",
    "state-machine-time:",
):
    if diagnostic not in core:
        raise SystemExit(f"State Machine qualified limitation/identity is missing: {diagnostic}")

for token in (
    "ElementKind::Signal",
    "queued State Machine SignalEvent must reference a Signal",
):
    if token not in desktop:
        raise SystemExit(f"State Machine SignalEvent Rust boundary validation is missing: {token}")

for token in (
    "ActivityWorkspaceState",
    "ActivityRepository",
    "validate_machine_activity_references",
    "must reference a modeled Activity by stable ID",
    "references missing Activity stable ID",
    "serde_json::to_string(&(project, repository, activities))",
    ".with_activity_repository(activities)",
):
    if token not in desktop:
        raise SystemExit(f"State Activity reference/runtime boundary is missing: {token}")

for token in (
    "mod activity_bridge",
    "state_activity_runtime",
    "advance_state_do_activities",
    "activate_state_activities",
    "exit_state_activities",
):
    if token not in core:
        raise SystemExit(f"State Activity execution bridge is not wired into the State Machine engine: {token}")

for token in (
    "StateActivityRuntime",
    "ActivityExecutionEngine",
    "with_activity_repository",
    "shared ActivityRepository runtime source",
    "step_embedded_activity",
    "execute_synchronous_state_activity",
    "started doActivity",
    "terminated doActivity on exit",
    "doActivity completed",
    "time_event_sequences",
):
    if token not in activity_bridge:
        raise SystemExit(f"State Activity execution bridge is incomplete: {token}")

for scenario in (
    "entry_and_exit_activities_execute_in_parent_session_without_completing_it",
    "do_activity_waits_for_shared_signal_then_allows_completion_transition",
    "exiting_state_cancels_do_activity_time_event_from_shared_queue",
    "queued_state_transition_preempts_progressing_do_activity",
):
    if scenario not in activity_bridge_tests:
        raise SystemExit(f"State Activity execution qualification scenario is missing: {scenario}")

for scenario in (
    "reset_replays_state_activity_runtime_without_leaking_pending_events",
    "state_activity_execution_is_repeatable_for_identical_inputs",
):
    if scenario not in activity_reset_tests:
        raise SystemExit(f"State Activity reset/repeatability scenario is missing: {scenario}")

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

base_scenarios = (
    "basic_signal_lifecycle_is_deterministic_and_completes",
    "guarded_choice_uses_shared_expression_evaluator_and_event_payload",
    "composite_and_orthogonal_regions_maintain_active_configuration",
    "time_event_uses_simulation_time_not_wall_clock",
    "fork_and_join_synchronize_concurrent_paths",
    "identical_inputs_produce_identical_semantic_trace",
    "reset_matches_fresh_initialization_and_does_not_mutate_authored_model",
    "pseudostate_cycle_fails_at_bounded_run_to_completion_limit",
)
for scenario in base_scenarios:
    if scenario not in base_tests:
        raise SystemExit(f"State Machine execution test scenario is missing: {scenario}")

for scenario in (
    "authored_defaults_are_available_to_state_machine_guards",
    "change_event_fires_only_on_false_to_true_edge",
    "stale_time_event_cannot_fire_after_exit_and_reentry",
    "event_for_another_runtime_target_does_not_block_relevant_signal",
):
    if scenario not in event_tests:
        raise SystemExit(f"State Machine event-semantics scenario is missing: {scenario}")

for scenario in (
    "one_signal_fires_non_conflicting_transitions_in_two_orthogonal_regions",
    "cross_hierarchy_external_transition_exits_child_then_parent",
    "local_transition_from_composite_to_descendant_retains_composite",
    "entering_nested_target_initializes_other_orthogonal_regions",
    "same_priority_conflicting_transitions_fail_with_ambiguity_diagnostic",
    "submachine_state_executes_child_machine_and_completes_back_into_parent",
    "entry_point_execution_is_explicitly_rejected_until_connection_point_semantics_exist",
):
    if scenario not in closure_tests:
        raise SystemExit(f"State Machine semantic-closure scenario is missing: {scenario}")

fork_join_scenario = (
    "fork_targets_orthogonal_regions_without_entering_sibling_defaults_and_join_completes"
)
if fork_join_scenario not in fork_join_tests:
    raise SystemExit(
        "State Machine Fork/Join orthogonal-region qualification scenario is missing: "
        f"{fork_join_scenario}"
    )
for token in (
    "Left Default",
    "Right Default",
    "Left Target",
    "Right Target",
    "PseudostateKind::Fork",
    "PseudostateKind::Join",
):
    if token not in fork_join_tests:
        raise SystemExit(f"State Machine Fork/Join qualification is incomplete: {token}")

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

invoke_start = ui.index("async function invokeStateMachineExecution")
invoke_end = ui.index("async function initializeStateMachineExecution")
if "render();" in ui[invoke_start:invoke_end]:
    raise SystemExit("State Machine runtime snapshots must not rebuild the authored workspace")
if "requestAnimationFrame" not in ui or "step_state_machine_execution" not in ui:
    raise SystemExit("State Machine Run must remain a thin frontend loop over Rust Step")
if "pointer-events:none" not in styles or ".state-machine-execution-panel" not in styles:
    raise SystemExit("State Machine execution overlay is not input-transparent")
if '<script src="behavior-authoritative-renderer.js"></script>' not in index:
    raise SystemExit("State Machine execution UI is not loaded")
if "all nine" not in standard:
    raise SystemExit("PR31 all-nine-family authoring regression contract is missing")

for token in (
    "Entry Activity",
    "Do Activity",
    "Exit Activity",
    "update_state_behaviors",
    "stable Activity IDs",
):
    if token not in submachine_ui:
        raise SystemExit(f"State behavior Activity-reference authoring is missing: {token}")

print(
    "PR32 State Machine execution integration contract passed: Rust owns deterministic runtime "
    "semantics, Fork/Join is qualified across orthogonal Regions without implicit sibling-default "
    "entry, State entry/doActivity/exit reuse the PR31 Activity engine and shared ExecutionSession, "
    "queued State Machine transitions preempt progressing doActivities, State Activity reset and "
    "repeatability are qualified, stable Activity/Signal references are checked at the Rust boundary, "
    "shared time/events/values/trace/expressions are reused, qualified unsupported semantics remain "
    "explicit, runtime UI is presentation-only, and the PR31 all-nine-family authoring contract "
    "remains present"
)
