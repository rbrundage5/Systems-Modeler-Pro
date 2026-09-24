# TC-02: retire history with complete Open publication

Baseline: `f3e45cc0d1bf5ff40001afe75ac03fc1e4bfa8c7`.
One workflow: Open publishes all staged authored state and retires old undo/redo
before any subsequent frontend snapshot or rendering operation can fail.

Allowed production paths: workspace/bdd_elements.rs, workspace/history.rs,
frontend/project-open-compat.js and frontend/undo-redo-ui.js. Tests: native
history.rs fixtures and scripts/test_open_project_outcome.cjs. This leaf does not
change the file schema, loading validation or runtime registry contract.

Acceptance: successful native Open cannot undo into the preceding project even
when frontend refresh fails. Poisoned history or late authored locks reject
without publishing a new project or clearing either old history stack. Missing
and malformed files preserve the session. Both history guards are acquired before
either is cleared, while all publication guards are held. Cancellation stays a no-op.

Primary-session implementation; independent review and native rendered acceptance
remain outstanding. Final-head native CI is recorded on the PR. Runtime retirement
and unsaved-revision protection remain separate leaves. No automatic merge.
