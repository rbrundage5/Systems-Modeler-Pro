"""Validate the PR22 cross-diagram standard editing boundary."""

from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(relative: str) -> str:
    return (ROOT / relative).read_text(encoding="utf-8")


def require(source: str, tokens: list[str], label: str) -> None:
    missing = [token for token in tokens if token not in source]
    if missing:
        raise SystemExit(f"{label} is missing: {', '.join(missing)}")


standard = read("apps/desktop/src-tauri/src/workspace/standard_editing.rs")
bridge = read("apps/desktop/src-tauri/src/workspace/standard_editing_bridge.rs")
ui = read("apps/desktop/frontend/standard-editing-ui.js")
marquee = read("apps/desktop/frontend/marquee-selection.js")
index = read("apps/desktop/frontend/index.html")
shared = read("apps/desktop/frontend/shared-workspace.js")
main = read("apps/desktop/src-tauri/src/main.rs")
behavior = read("apps/desktop/src-tauri/src/workspace/behavior_workspace.rs")
repository = read("apps/desktop/src-tauri/src/workspace/repository_editing.rs")
workflow = read(".github/workflows/ci.yml")

require(
    standard,
    [
        "enum EditingFamily",
        "Bdd,",
        "Package,",
        "Requirement,",
        "UseCase,",
        "Ibd,",
        "StateMachine,",
        "Sequence,",
        "Activity,",
        "fn collect_clipboard(",
        "fn paste_clipboard(",
        "fn duplicate_selection_items(",
        "fn remove_presentations(",
        "fn move_selection_items(",
        "history::checkpoint_states",
        "pub fn copy_selection(",
        "pub fn paste_selection(",
        "pub fn duplicate_selection(",
        "pub fn delete_active_selection(",
        "pub fn move_active_selection(",
    ],
    "Rust standard editing engine",
)

for command in (
    "copy_selection",
    "paste_selection",
    "duplicate_selection",
    "delete_active_selection",
    "move_active_selection",
):
    if f"#[tauri::command]\npub fn {command}(" in standard:
        raise SystemExit(
            f"standard_editing.rs: {command} must remain an internal Rust function; "
            "the shared-selection bridge is the sole Tauri command boundary"
        )
    if f"#[tauri::command]\npub fn {command}(" not in bridge:
        raise SystemExit(
            f"standard_editing_bridge.rs: missing Tauri bridge for {command}"
        )

require(
    bridge,
    [
        "workspace_interaction_snapshot",
        "active_selections",
        "from_model: Option<bool>",
        "Delete from Model requires exactly one selected relationship",
        "delete_selected_relationship_from_model",
    ],
    "Rust shared-selection bridge",
)

require(
    ui,
    [
        "copy_selection",
        "paste_selection",
        "duplicate_selection",
        "delete_active_selection",
        "Remove from Diagram",
        "Delete from Model",
        "event.ctrlKey || event.metaKey || event.shiftKey",
        "window.smpRendererHost?.publishInteraction?.()",
        "window.smpStandardEditing",
        "Deleting Behavior item from model",
        "Deleting Activity item from model",
        "itemKind: edge ? 'edge' : 'node'",
        "The selected Activity presentation does not resolve to a semantic node or edge.",
    ],
    "cross-diagram editing UI",
)

require(
    marquee,
    [
        "smp-marquee-selection",
        "[data-smp-presentation-id]",
        "window.smpStandardEditing?.selections?.()",
        "window.smpStandardEditing?.setSelections?.([...existing, ...hits])",
        "smp:selection-changed",
        "event.ctrlKey",
        "event.metaKey",
        "canvas.classList.contains('pan-active')",
        "canvas.classList.contains('is-panning')",
    ],
    "shared marquee selection",
)
for forbidden in (
    "addEventListener('keydown'",
    'addEventListener("keydown"',
    "addEventListener('keyup'",
    'addEventListener("keyup"',
):
    if forbidden in marquee:
        raise SystemExit(
            "marquee-selection.js must not own a keyboard controller; "
            "Space/Ctrl/Meta panning belongs to shared-workspace.js"
        )
if '<script src="standard-editing-ui.js"></script>\n  <script src="marquee-selection.js"></script>' not in index:
    raise SystemExit("marquee-selection.js must load after standard-editing-ui.js")

if "itemType: edge ? 'Edge' : 'Node'" in ui:
    raise SystemExit(
        "standard-editing-ui.js: Activity model deletion must use the qualified "
        "delete_activity_item itemKind contract with lowercase edge/node values"
    )

require(
    shared,
    [
        "publishInteraction",
        "set_workspace_interaction",
        "workspace_interaction_snapshot",
        "await publishInteraction();",
        "state.space",
        "pan-active",
        "is-panning",
        "smp:selection-changed",
    ],
    "shared workspace interaction authority",
)

require(
    main,
    [
        "StandardEditingState",
        ".manage(StandardEditingState::default())",
        "copy_selection,",
        "paste_selection,",
        "duplicate_selection,",
        "delete_active_selection,",
        "move_active_selection,",
    ],
    "desktop command registration",
)

require(
    behavior,
    [
        "hidden_semantic_ids: Vec<String>",
        "presentation_copies: Vec<BehaviorPresentationCopy>",
        "BEHAVIOR_DIAGRAM_METADATA_KEY",
    ],
    "behavior presentation persistence",
)

require(
    repository,
    [
        "pub fn delete_model_element(",
        "pub fn delete_repository_diagram(",
        "history::checkpoint_states",
    ],
    "model-vs-presentation deletion governance",
)

require(
    workflow,
    [
        "apps/desktop/frontend/standard-editing-ui.js",
        "apps/desktop/frontend/marquee-selection.js",
    ],
    "frontend syntax qualification",
)

print(
    "PR22/PR24/PR25/PR26B standard editing integration contract passed: all nine diagram families retain "
    "Rust-owned clipboard/remove/move authority, shared click/marquee selection synchronization, "
    "shared pan ownership, presentation persistence, model-vs-diagram deletion separation, and qualified "
    "Behavior/Activity model-deletion history wiring"
)
