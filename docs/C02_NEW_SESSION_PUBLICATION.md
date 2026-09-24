# TC-01: atomic New Project authored-session publication

Baseline: `f3e45cc0d1bf5ff40001afe75ac03fc1e4bfa8c7`.
User work order: implement the completion audit, preserving existing behavior.
One workflow: New Project replaces every authored repository, exchange/file state
and undo/redo together, or leaves the old session unchanged.

Allowed production paths: workspace.rs, workspace/history.rs, frontend/app.js,
frontend/undo-redo-ui.js and frontend/project-reset-integrity.js. Focused tests:
history.rs and scripts/test_new_project_outcome.cjs. Existing model-script and
history integration contracts must remain valid.

Acquire all authored/exchange/path guards before clearing history or publishing
the new project. Busy authored state rejects and can be retried. No frontend
Activity reset or second history reset is needed after native New succeeds.
Negative cases include poisoned late locks, history failure, contention and
cancelled New. Success creates a clean, distinct project and cannot undo into the
old project. Execution-registry retirement and dirty-revision prompts are separate
dependent leaves; this change does not claim to close those gaps.

Direct primary-session implementation. Delegated workers remain disabled;
independent review and installed acceptance remain outstanding. Native CI results
are recorded on the actual PR/head. No automatic merge.
