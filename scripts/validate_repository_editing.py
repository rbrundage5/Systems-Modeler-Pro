"""Validate the PR22 repository-governance integration boundary."""

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
main = read("apps/desktop/src-tauri/src/main.rs")
repository_ui = read("apps/desktop/frontend/repository-tree-ui.js")
application_ui = read("apps/desktop/frontend/app.js")
shared_ui = read("apps/desktop/frontend/shared-workspace.js")

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
    main,
    [
        "mod repository_editing;",
        "move_repository_element,",
        "delete_model_element,",
        "move_repository_diagram,",
        "delete_repository_diagram,",
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
    ["window.smpRepositoryEditing?.handleDelete?.(event)"],
    "shared keyboard routing",
)

print(
    "Repository editing contract passed: Rust-owned move/delete and diagram "
    "governance are registered, visible, history-backed, and cross-family validated"
)
