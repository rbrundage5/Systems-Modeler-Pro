from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def text(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def require(path: str, *needles: str) -> None:
    payload = text(path)
    missing = [needle for needle in needles if needle not in payload]
    if missing:
        raise SystemExit(f"{path}: missing required PR12 integration markers: {missing}")


# Tauri must register every Rust-authoritative behavior command used by the migrated UI.
require(
    "apps/desktop/src-tauri/src/main.rs",
    "behavior_snapshot",
    "create_state_machine_diagram",
    "create_sequence_diagram",
    "add_state_vertex",
    "add_composite_state",
    "add_submachine_state",
    "add_state_region",
    "update_state_behaviors",
    "add_state_transition_complete",
    "update_state_transition",
    "move_state_vertex",
    "behavior_lifeline_candidates",
    "add_sequence_lifeline",
    "move_sequence_lifeline",
    "add_sequence_message",
    "update_sequence_message_complete",
    "add_execution_specification",
    "update_execution_specification",
    "add_combined_fragment",
    "add_combined_fragment_operand",
    "update_combined_fragment_operand",
    "add_state_invariant",
    "update_state_invariant",
    "delete_behavior_item",
)

# Project lifecycle must persist and restore behavior semantics and presentation.
require(
    "apps/desktop/src-tauri/src/workspace.rs",
    "BehaviorRepository::default()",
    "behavior_diagrams",
    "save_behavior_metadata",
    "load_behavior_metadata",
)
require(
    "crates/persistence/tests/behavior_persistence.rs",
    "behavior_repository_round_trip_preserves_submachine_and_occurrence_identity",
    "restored_state.submachine",
    "restored_message.send_event",
    "restored_message.receive_event",
)

# The authoritative ribbon shell must expose behavior creation on Home and Diagram.
require(
    "apps/desktop/frontend/ui-shell.js",
    'data-action="new-state-machine"',
    'data-action="new-sequence"',
    "smpCreateStateMachineForSelectedBlock",
    "smpCreateSequenceForSelectedBlock",
)

# Creation must select the exact Rust-created diagram without relying on a chained refresh race.
require(
    "apps/desktop/frontend/behavior-ribbon.js",
    "create_state_machine_diagram",
    "create_sequence_diagram",
    "behavior_snapshot",
    "selectedBehaviorDiagramId = diagramId",
)

# Required migrated State Machine and Sequence capabilities must remain available.
require(
    "apps/desktop/frontend/behavior-ui.js",
    "StateMachine",
    "Sequence",
    "Initial",
    "FinalState",
    "Choice",
    "Junction",
    "Fork",
    "Join",
    "ShallowHistory",
    "DeepHistory",
    "EntryPoint",
    "ExitPoint",
    "Terminate",
    "Transition",
    "Lifeline",
    "SynchCall",
    "AsynchCall",
    "AsynchSignal",
    "Reply",
    "Create",
    "Delete",
    "Lost",
    "Found",
    "Execution",
    "alt",
    "opt",
    "loop",
    "par",
    "critical",
    "State Invariant",
)

# Completion bridges must be present and actually loaded by the desktop shell.
require(
    "apps/desktop/frontend/index.html",
    'src="behavior-safe-transition.js"',
    'src="behavior-completion-ui.js"',
    'src="behavior-atomic-message.js"',
    'src="behavior-delete-ui.js"',
    'src="behavior-message-notation.js"',
    'href="behavior-message-notation.css"',
    'src="behavior-submachine.js"',
    'href="behavior-submachine.css"',
    'src="behavior-nested-transition-notation.js"',
    'src="behavior-region-placement.js"',
    'src="behavior-command-authority.js"',
    'src="behavior-runtime-hardening.js"',
)
require(
    "apps/desktop/frontend/behavior-safe-transition.js",
    "add_state_transition_complete",
)
require(
    "apps/desktop/frontend/behavior-command-authority.js",
    "create_state_machine_diagram_staged",
    "create_sequence_diagram_staged",
    "add_state_transition_complete",
)
require(
    "apps/desktop/frontend/behavior-runtime-hardening.js",
    "clearBehaviorSelection",
    "clearStructuralSelection",
    "enforceSingleMode",
    "ensureStatePresentations",
    "smpActivateDiagramMode",
)
require(
    "apps/desktop/frontend/behavior-atomic-message.js",
    "update_sequence_message_complete",
)
require(
    "apps/desktop/frontend/behavior-delete-ui.js",
    "delete_behavior_item",
)
require(
    "apps/desktop/frontend/behavior-completion-ui.js",
    "add_composite_state",
    "update_state_transition",
    "update_execution_specification",
    "add_combined_fragment_operand",
    "update_combined_fragment_operand",
    "update_state_invariant",
)
require(
    "apps/desktop/frontend/behavior-message-notation.js",
    "created-lifeline",
    "sequence-destruction-marker",
    "message.sort === 'Create'",
    "message.sort === 'Delete'",
    "message.sort === 'Reply'",
)
require(
    "apps/desktop/frontend/behavior-submachine.js",
    "SubmachineState",
    "add_submachine_state",
    "«submachine»",
)
require(
    "apps/desktop/frontend/behavior-nested-transition-notation.js",
    "nestedTransitions",
    "nested-state-transition",
    "transition.source_id",
    "transition.target_id",
)
require(
    "apps/desktop/frontend/behavior-region-placement.js",
    "REGION_VERTEX_TOOLS",
    "regionIdValue: region.id",
    "add_composite_state",
    "add_state_vertex",
)
require(
    "crates/model-core/src/behavior.rs",
    "pub submachine: Option<StateMachineId>",
    "UnknownSubmachine",
    "SelfSubmachine",
    "SubmachineCycle",
)

print("PR12 behavior desktop integration markers are complete")
