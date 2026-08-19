from pathlib import Path

root = Path(__file__).resolve().parents[1]
frontend = (root / "apps/desktop/frontend/authoritative-mutation-sync.js").read_text(encoding="utf-8")
undo = (root / "apps/desktop/frontend/undo-redo-ui.js").read_text(encoding="utf-8")
index = (root / "apps/desktop/frontend/index.html").read_text(encoding="utf-8")

required_sync_tokens = [
    "workspace_snapshot_complete",
    "activity_snapshot",
    "state.behaviorSnapshot",
    "state.activitySnapshot",
    "runCommandWithAuthoritativeSync",
    "render();",
]
for token in required_sync_tokens:
    assert token in frontend, f"missing authoritative mutation sync contract: {token}"

assert "smpSynchronizeAuthoritativeState" in undo, "Undo/Redo must use authoritative synchronization"
assert index.index('src="undo-redo-ui.js"') < index.index('src="authoritative-mutation-sync.js"'), (
    "authoritative mutation sync must load after Undo/Redo"
)
assert index.rstrip().endswith("</html>"), "desktop shell must remain complete"

print("general interaction synchronization contract passed")
