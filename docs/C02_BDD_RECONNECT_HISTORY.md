# C02 leaf work order: atomic BDD reconnect history

Baseline: `f3e45cc0d1bf5ff40001afe75ac03fc1e4bfa8c7`.

Finding: TC-03's reproduced BDD reconnect rejection consumes redo through the
frontend pre-checkpoint even though Rust preserves semantic and presentation state.

Allowed paths: `workspace/history.rs`, `workspace/relationship_editing.rs`,
`frontend/app.js`, `frontend/undo-redo-ui.js` under the desktop application,
`scripts/test_bdd_reconnect_history.cjs`, and this work order.

Reuse the existing Rust structural transaction for BDD reconnect; retain selected
view endpoint checks, cross-view staging, and dependent repository validation.
Acceptance: rejected reconnect preserves authored state and redo, valid reconnect
creates exactly one checkpoint, no-op creates none, and undo/redo round-trips every
affected view. Exercise the production frontend handler to verify no pre-checkpoint.

Only BDD reconnect is in scope. Other generic mutations, Requirement reconnect,
association-end editing, deletion, and feature breadth remain separate leaves.
Native CI is the available compiler/test authority; independent review is outstanding.
