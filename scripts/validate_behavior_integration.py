from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

def text(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")

def require(path: str, *needles: str) -> None:
    payload = text(path)
    missing = [needle for needle in needles if needle not in payload]
    if missing:
        raise SystemExit(f"{path}: missing required PR12 integration markers: {missing}")

# Tauri must register every behavior command that the migrated UI calls.
require(
    "apps/desktop/src-tauri/src/main.rs",
    "behavior_snapshot",
    "create_state_machine_diagram",
    "create_sequence_diagram",
    "add_state_vertex",
    "add_state_region",
    "update_state_behaviors",
    "add_state_transition",
    "move_state_vertex",
    "behavior_lifeline_candidates",
    "add_sequence_lifeline",
    "move_sequence_lifeline",
    "add_sequence_message",
    "add_execution_specification",
    "add_combined_fragment",
    "add_state_invariant",
)

# Project lifecycle must persist and restore behavior semantics and presentation.
require(
    "apps/desktop/src-tauri/src/workspace.rs",
    "BehaviorRepository::default()",
    "behavior_diagrams",
    "save_behavior_metadata",
    "load_behavior_metadata",
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

print("PR12 behavior desktop integration markers are complete")
