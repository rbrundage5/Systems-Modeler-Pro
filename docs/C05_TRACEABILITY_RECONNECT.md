# C05 leaf work order: reconnect a Requirement traceability endpoint

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
