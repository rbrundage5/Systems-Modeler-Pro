# C05 leaf work order: reconnect a Requirement traceability endpoint

## Integration recovery, 25 September 2026

Baseline main: `2d1f51b0faef8abb840d7c848adca2f458bb92cc`.
PR172 merged into `codex/reconnect-history-atomic` after PR171 had already
merged into main. Comparing main's source to merge `4a9d061b6604f7634505edaf045cf8077c83bee0`
confirms this implementation was absent. This candidate restores only this leaf
and explicitly preserves the native history command label. The restored frontend
regression failed with an extra `history_checkpoint` before that label was fixed.
Native tests must run on this main-based candidate; prior PR success is historical
evidence. Rust is unavailable locally. No workers or automatic merge are used.

## Original work order

Depends on the guarded structural transaction introduced by PR #171.
Finding TC-06: reconnect can consume redo or publish an endpoint before routing
fails. Copy reconnect validates stale client text before supplier propagation.

Allowed paths: `crates/model-core/src/model.rs`,
`crates/model-core/tests/pr21_requirements.rs`, desktop `workspace/requirements.rs`,
`frontend/app.js`, `frontend/undo-redo-ui.js`,
`scripts/test_bdd_reconnect_history.cjs`, and this work order.

Stage the selected endpoint, reuse the core transitive Copy traversal, validate all
dependent repositories and views, and publish with exactly one history checkpoint.
Preserve relationship identity and local requirement IDs. Reject missing/invalid
presentations, conflicting suppliers and illegal endpoints without changing model,
views, or redo. Verify different-text supplier reconnect, chain/branch propagation,
multi-view routing, labels, no-op, undo/redo and persisted project reopen.

Creation, deletion, containment, recursive structural Copy, and other requirement
features are out of scope. Independent review and rendered UI acceptance remain open.
