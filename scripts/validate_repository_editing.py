"""Validate the PR22 repository-governance and standard-editing boundary."""

from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(relative: str) -> str:
    return (ROOT / relative).read_text(encoding="utf-8")


def require(source: str, tokens: list[str], label: str) -> None:
    missing = [token for token in tokens if token not in source]
    if missing:
        raise SystemExit(f"{label} is missing: {', '.join(missing)}")


model = read("crates/model-core/src/model.rs")
commands = read("apps/desktop/src-tauri/src/workspace/repository_editing.rs")
standard = read("apps/desktop/src-tauri/src/workspace/standard_editing.rs")
bridge = read("apps/desktop/src-tauri/src/workspace/standard_editing_bridge.rs")
behavior = read("apps/desktop/src-tauri/src/workspace/behavior_workspace.rs")
main = read("apps/desktop/src-tauri/src/main.rs")
repository_ui = read("apps/desktop/frontend/repository-tree-ui.js")
application_ui = read("apps/desktop/frontend/app.js")
shared_ui = read("apps/desktop/frontend/shared-workspace.js")
standard_ui = read("apps/desktop/frontend/standard-editing-ui.js")
index = read("apps/desktop/frontend/index.html")
workflow = read(".github/workflows/ci.yml")

require(
    model,
    [
        "pub fn move_element(",
        "ModelError::ProtectedProjectRoot(id)",
        "ModelError::OwnershipCycle",
        "pub fn delete_element(",
    ],
    "model-core repository invariants",
)
require(
    commands,
    [
        "pub fn move_repository_element(",
        "pub fn delete_model_element(",
        "pub fn move_repository_diagram(",
        "pub fn delete_repository_diagram(",
        "history::checkpoint_states",
        "validate_behavior_workspace",
        "activity_repository",
        "remove_bdd_presentations",
        "remove_ibd_presentations",
    ],
    "Rust repository commands",
)
require(
    standard,
    [
        "pub struct StandardEditingState",
        "fn collect_clipboard(",
        "fn paste_clipboard(",
        "fn duplicate_selection_items(",
        "fn remove_presentations(",
        "fn move_selection_items(",
        "EditingFamily::Bdd",
        "EditingFamily::Requirement",
        "EditingFamily::Ibd",
        "EditingFamily::StateMachine",
        "EditingFamily::Sequence",
        "EditingFamily::Activity",
        "history::checkpoint_states",
    ],
    "Rust cross-diagram standard editing",
)
require(
    bridge,
    [
        "workspace_interaction_snapshot",
        "standard_editing::copy_selection",
        "standard_editing::paste_selection",
        "standard_editing::duplicate_selection",
        "standard_editing::delete_active_selection",
        "standard_editing::move_active_selection",
        "from_model: Option<bool>",
        "delete_selected_relationship_from_model",
        "history::checkpoint_states",
    ],
    "Rust standard-editing selection bridge",
)
require(
    behavior,
    [
        "pub struct BehaviorPresentationCopy",
        "pub hidden_semantic_ids: Vec<String>",
        "pub presentation_copies: Vec<BehaviorPresentationCopy>",
        "#[serde(default)]",
    ],
    "Behavior presentation persistence",
)
require(
    main,
    [
        "mod repository_editing;",
        "mod standard_editing;",
        "mod standard_editing_bridge;",
        "move_repository_element,",
        "delete_model_element,",
        "move_repository_diagram,",
        "delete_repository_diagram,",
        "copy_selection,",
        "delete_active_selection,",
        "duplicate_selection,",
        "move_active_selection,",
        "paste_selection,",
        ".manage(StandardEditingState::default())",
    ],
    "Tauri command registration",
)
require(
    repository_ui,
    [
        "application/x-smp-repository-element-id",
        "move_repository_element",
        "delete_model_element",
        "move_repository_diagram",
        "delete_repository_diagram",
        "Delete from Model",
        "Delete Diagram",
        "repository-drop-target",
        "window.smpRepositoryEditing",
    ],
    "repository user interface",
)
require(
    application_ui,
    ["window.smpRepositoryEditing?.renderProperties?.();"],
    "central property rendering",
)
require(
    shared_ui,
    [
        "window.smpRepositoryEditing?.handleDelete?.(event)",
        "window.smpStandardSelections",
        "await publishInteraction();",
    ],
    "shared keyboard and authoritative-selection routing",
)
require(
    standard_ui,
    [
        "Copy",
        "Paste",
        "Duplicate",
        "Remove from Diagram",
        "Delete from Model",
        "fromModel: true",
        "window.smpStandardSelections",
        "publishInteraction",
        "hidden_semantic_ids",
        "presentation_copies",
        "smp-standard-context-menu",
        "standard-editing-properties",
        "standard-editing-ribbon-group",
    ],
    "standard editing user interface",
)
require(
    index,
    ['<script src="standard-editing-ui.js"></script>'],
    "standard editing script load",
)
require(
    workflow,
    ["apps/desktop/frontend/standard-editing-ui.js"],
    "standard editing frontend syntax gate",
)

if "IBD model deletion is awaiting" in standard_ui:
    raise SystemExit("IBD connector Delete from Model must not regress to a frontend exception")

print(
    "Repository/editing contract passed: Rust-owned repository governance, clipboard, "
    "multi-selection, presentation removal, semantic deletion, persistence, and history "
    "are registered and cross-family wired"
)
