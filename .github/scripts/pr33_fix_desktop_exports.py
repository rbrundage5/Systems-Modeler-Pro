from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count == 1:
        return text.replace(old, new, 1)
    if new in text:
        return text
    raise SystemExit(f"{label}: expected one match, found {count}")


main = Path("apps/desktop/src-tauri/src/main.rs")
text = main.read_text(encoding="utf-8")
text = replace_once(
    text,
    """    pub use activity_execution::{\n        ActivityExecutionState, activity_execution_snapshot, clear_activity_executions,\n        initialize_activity_execution, pause_activity_execution, reset_activity_execution,\n        resume_activity_execution, run_activity_execution, step_activity_execution,\n        terminate_activity_execution,\n    };\n""",
    """    pub use activity_execution::{\n        ActivityExecutionState, activity_execution_runtime_selection, activity_execution_snapshot,\n        clear_activity_executions, configure_activity_execution_runtime,\n        initialize_activity_execution, pause_activity_execution, preview_activity_execution_runtime,\n        reset_activity_execution, resume_activity_execution, run_activity_execution,\n        step_activity_execution, terminate_activity_execution,\n    };\n""",
    "activity execution re-export",
)
text = replace_once(
    text,
    """    pub use state_machine_execution::{\n        StateMachineExecutionState, clear_state_machine_executions,\n        initialize_state_machine_execution, pause_state_machine_execution,\n        queue_state_machine_signal, reset_state_machine_execution, resume_state_machine_execution,\n        run_state_machine_execution, state_machine_execution_snapshot,\n        step_state_machine_execution, terminate_state_machine_execution,\n    };\n""",
    """    pub use state_machine_execution::{\n        StateMachineExecutionState, clear_state_machine_executions,\n        configure_state_machine_execution_runtime, initialize_state_machine_execution,\n        pause_state_machine_execution, preview_state_machine_execution_runtime,\n        queue_state_machine_signal, reset_state_machine_execution, resume_state_machine_execution,\n        run_state_machine_execution, state_machine_execution_runtime_selection,\n        state_machine_execution_snapshot, step_state_machine_execution,\n        terminate_state_machine_execution,\n    };\n""",
    "state machine execution re-export",
)
text = replace_once(
    text,
    """    active_diagram_command_manifest, active_diagram_layout, active_diagram_router,\n    activity_execution_snapshot, activity_snapshot, add_activity_action, add_activity_edge,\n""",
    """    active_diagram_command_manifest, active_diagram_layout, active_diagram_router,\n    activity_execution_runtime_selection, activity_execution_snapshot, activity_snapshot,\n    add_activity_action, add_activity_edge,\n""",
    "activity runtime selection import",
)
text = replace_once(
    text,
    """    behavior_lifeline_candidates, behavior_snapshot, clear_activity_executions,\n    clear_state_machine_executions, clear_workspace_interaction, copy_selection,\n""",
    """    behavior_lifeline_candidates, behavior_snapshot, clear_activity_executions,\n    clear_state_machine_executions, clear_workspace_interaction, configure_activity_execution_runtime,\n    configure_state_machine_execution_runtime, copy_selection,\n""",
    "runtime configure imports",
)
text = replace_once(
    text,
    """    open_project_file, open_project_file_complete, paste_selection, pause_activity_execution,\n    pause_state_machine_execution, place_bdd_element, place_element_on_bdd,\n""",
    """    open_project_file, open_project_file_complete, paste_selection, pause_activity_execution,\n    pause_state_machine_execution, place_bdd_element, place_element_on_bdd,\n    preview_activity_execution_runtime, preview_state_machine_execution_runtime,\n""",
    "runtime preview imports",
)
text = replace_once(
    text,
    """    set_panel_preferences, set_viewport_preference, set_workspace_interaction,\n    state_machine_execution_snapshot, step_activity_execution, step_state_machine_execution,\n""",
    """    set_panel_preferences, set_viewport_preference, set_workspace_interaction,\n    state_machine_execution_runtime_selection, state_machine_execution_snapshot,\n    step_activity_execution, step_state_machine_execution,\n""",
    "state runtime selection import",
)
main.write_text(text, encoding="utf-8")

activity = Path("apps/desktop/src-tauri/src/workspace/activity_execution.rs")
text = activity.read_text(encoding="utf-8")
text = text.replace(
    "ActivityExecutionEngine, ActivityExecutionSnapshot, ActivityId, ActivityRepository, ElementId,\n    ElementKind,",
    "ActivityExecutionEngine, ActivityExecutionSnapshot, ActivityId, ActivityRepository, ElementKind,",
)
text = replace_once(
    text,
    "fn is_structural_root(kind: ElementKind) -> bool {\n    matches!(\n        kind,",
    "fn is_structural_root(kind: &ElementKind) -> bool {\n    matches!(\n        kind,",
    "borrow structural root kind",
)
text = text.replace("if !is_structural_root(root.kind) {", "if !is_structural_root(&root.kind) {")
text = text.replace("matches!(\n                element.kind,", "matches!(\n                &element.kind,")
activity.write_text(text, encoding="utf-8")

state = Path("apps/desktop/src-tauri/src/workspace/state_machine_execution.rs")
text = state.read_text(encoding="utf-8")
text = text.replace("    if !matches!(\n        root.kind,", "    if !matches!(\n        &root.kind,")
state.write_text(text, encoding="utf-8")
