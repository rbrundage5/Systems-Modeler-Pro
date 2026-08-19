from pathlib import Path

root = Path(__file__).resolve().parents[1]


def text(path: str) -> str:
    return (root / path).read_text(encoding="utf-8")


def require(path: str, *needles: str) -> None:
    payload = text(path)
    for needle in needles:
        if needle not in payload:
            raise SystemExit(f"{path}: missing required history contract: {needle}")


require(
    "apps/desktop/src-tauri/src/main.rs",
    "mod history;",
    "HistoryState",
    ".manage(HistoryState::default())",
    "history_checkpoint",
    "history_undo",
    "history_redo",
    "history_reset",
)
require(
    "apps/desktop/src-tauri/src/workspace/history.rs",
    "behavior: BehaviorRepository",
    "behavior_diagrams: Vec<behavior_workspace::BehaviorDiagram>",
    "activity_repository: ActivityRepository",
    "activity_diagrams: Vec<activity_workspace::ActivityDiagram>",
    "const HISTORY_LIMIT: usize = 100",
)
require(
    "apps/desktop/frontend/undo-redo-ui.js",
    "history_checkpoint",
    "history_undo",
    "history_redo",
    "history_reset",
    "await refresh();",
    "window.smpUndo",
    "window.smpRedo",
)
require(
    "apps/desktop/frontend/ui-shell.js",
    "data-action=\"undo\"",
    "data-action=\"redo\"",
    "window.smpUndo?.()",
    "window.smpRedo?.()",
    "History",
)

index = text("apps/desktop/frontend/index.html")
behavior = index.find('<script src="behavior-refresh-authority.js"></script>')
history = index.find('<script src="undo-redo-ui.js"></script>')
if behavior < 0 or history < 0 or history < behavior:
    raise SystemExit(
        "index.html: Undo/Redo must load after the validated Behavior refresh authority"
    )

undo = text("apps/desktop/frontend/undo-redo-ui.js")
for forbidden in (
    "authoritative-mutation-sync",
    "workspace_snapshot_complete",
    "activity_snapshot",
    "smpSynchronizeAuthoritativeState",
):
    if forbidden in undo:
        raise SystemExit(
            f"undo-redo-ui.js: history must reuse qualified refresh(), not {forbidden}"
        )

if (root / "apps/desktop/frontend/authoritative-mutation-sync.js").exists():
    raise SystemExit(
        "authoritative-mutation-sync.js must not return; it caused the STM/SEQ regression"
    )

print("PR13 history integration preserves qualified cross-diagram refresh authority and visible shell controls")
