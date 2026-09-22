# C05 — atomic requirement specification edit

Baseline: PR124, 860a00d7837fb93ade7a281dd07919578c89f4c8.
Lead direct implementation, no workers or independent review. Allowed paths:
apps/desktop/src-tauri/src/workspace/requirements.rs and this record.

Finding: update_requirement checkpoints before core validation. A rejected
duplicate ID or read-only Copy edit therefore consumes redo and adds undo history.

Stage the complete requirement ID, text, name and documentation on a cloned
project, validate it, and use the existing structural specification transaction.
Text propagation through Copy descendants is supplied by PR124. Publish once,
with one checkpoint; identical edits add no history. Keep the existing command API.

Acceptance: edit all specification fields and Copy descendants together; repeated
edit is a no-op; undo/redo restore the whole project; duplicate ID and read-only
Copy rejection preserve pending redo and all project state. Invalid ID and no open
project must not create history. Diagram geometry remains unchanged.

Local requirements, history integration and Rust authority checks pass. Native
tests and lint run in CI; no local Rust toolchain or native visual session is
available. Requirement creation, traceability reconnection and historical Copy
graph validation are separate outstanding leaves. No full lifecycle claim.
