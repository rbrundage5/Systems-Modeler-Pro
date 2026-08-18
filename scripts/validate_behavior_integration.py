from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def text(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def require(path: str, *needles: str) -> None:
    payload = text(path)
    missing = [needle for needle in needles if needle not in payload]
    if missing:
        raise SystemExit(f"{path}: missing required PR12 integration markers: {missing}")


def forbid(path: str, *needles: str) -> None:
    payload = text(path)
    present = [needle for needle in needles if needle in payload]
    if present:
        raise SystemExit(f"{path}: forbidden legacy PR12 integration markers remain active: {present}")


# Rust/Tauri must register every authoritative behavior operation used by the desktop UI.
require(
    "apps/desktop/src-tauri/src/main.rs",
    "behavior_snapshot",
    "create_state_machine_diagram_staged",
    "create_sequence_diagram_staged",
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

# Project lifecycle must persist and restore Rust behavior semantics and presentation.
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

# Shell creation commands must survive ribbon reconstruction and keep a stable context node.
require(
    "apps/desktop/frontend/ui-shell.js",
    'data-action="new-state-machine"',
    'data-action="new-sequence"',
    'id="active-diagram-summary"',
    "smpCreateStateMachineForSelectedBlock",
    "smpCreateSequenceForSelectedBlock",
)
forbid(
    "apps/desktop/frontend/ui-shell.js",
    "active-diagram-summary-shell",
)

# Creation must use staged Rust commands and select the exact returned behavior diagram.
require(
    "apps/desktop/frontend/behavior-ribbon.js",
    "create_state_machine_diagram_staged",
    "create_sequence_diagram_staged",
    "behavior_snapshot",
    "selectedBehaviorDiagramId = diagramId",
)

# behavior-ui.js is intentionally only a thin adapter. It must not regain a second
# palette, canvas renderer, or legacy transition implementation.
require(
    "apps/desktop/frontend/behavior-ui.js",
    "smpLoadBehaviorSnapshot",
    "smpSelectBehaviorDiagram",
    "smpBehaviorLifelineClick",
    "behavior_snapshot",
    "add_sequence_message",
)
forbid(
    "apps/desktop/frontend/behavior-ui.js",
    "STATE_PALETTE",
    "SEQUENCE_PALETTE",
    "function renderStateMachine",
    "function renderSequence",
    "function commitTransition",
    "create_state_machine_diagram'",
    "create_sequence_diagram'",
)

# Exactly one active behavior canvas renderer is loaded. Removed legacy override
# layers must not be reintroduced into the shell.
require(
    "apps/desktop/frontend/index.html",
    'src="behavior-ui.js"',
    'src="behavior-ribbon.js"',
    'src="behavior-completion-ui.js"',
    'src="behavior-atomic-message.js"',
    'src="behavior-delete-ui.js"',
    'src="behavior-message-notation.js"',
    'src="behavior-submachine.js"',
    'src="behavior-region-placement.js"',
    'src="behavior-command-authority.js"',
    'src="behavior-authoritative-renderer.js"',
)
forbid(
    "apps/desktop/frontend/index.html",
    'src="behavior-safe-transition.js"',
    'src="behavior-runtime-hardening.js"',
    'src="behavior-nested-transition-notation.js"',
)

# The final renderer must draw all primary State Machine/Sequence presentation
# families directly from Rust snapshot/presentation records.
require(
    "apps/desktop/frontend/behavior-authoritative-renderer.js",
    "renderAuthoritativeBehaviorCanvas",
    "renderStateMachine",
    "renderSequence",
    "statePresentationMap",
    "fallbackStatePresentation",
    "state-vertex",
    "state-transition",
    "sequence-lifeline",
    "sequence-message",
    "execution-spec",
    "combined-fragment",
    "state-invariant-box",
    "move_state_vertex",
    "move_sequence_lifeline",
)

# Rust-defined palette and semantic editing bridges remain available.
require(
    "apps/desktop/frontend/behavior-completion-ui.js",
    "diagram_palette",
    "add_composite_state",
    "update_state_transition",
    "update_execution_specification",
    "add_combined_fragment_operand",
    "update_combined_fragment_operand",
    "update_state_invariant",
)
require(
    "apps/desktop/frontend/behavior-command-authority.js",
    "create_state_machine_diagram_staged",
    "create_sequence_diagram_staged",
    "add_state_transition_complete",
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
    "apps/desktop/frontend/behavior-region-placement.js",
    "REGION_VERTEX_TOOLS",
    "regionIdValue: region.id",
    "add_composite_state",
    "add_state_vertex",
)

# Core owns true Submachine State semantics and cycle/error validation.
require(
    "crates/model-core/src/behavior.rs",
    "pub submachine: Option<StateMachineId>",
    "UnknownSubmachine",
    "SelfSubmachine",
    "SubmachineCycle",
)

print("PR12 consolidated Rust-authoritative behavior integration is complete")
