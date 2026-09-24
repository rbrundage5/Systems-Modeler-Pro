# TC-05: atomic legacy element details

Baseline: `f3e45cc0d1bf5ff40001afe75ac03fc1e4bfa8c7`.
One workflow: the still-registered legacy details command must reject invalid
metadata/type edits without retaining earlier fields or consuming undo/redo.

Allowed paths: workspace/bdd_elements.rs, frontend/bdd-completion-ui.js,
frontend/undo-redo-ui.js, existing property-editor tests and this record.
Reuse ElementSpecificationEdit and the established structural specification/history
transaction, including dependent views and linked composition properties. Do not
add another validator. The legacy Apply form sends name and fields in one existing
specification command instead of committing a rename before a fallible detail edit.

Acceptance: invalid unit/type leaves complete state and redo unchanged; successful
edit creates one checkpoint, preserves identity, updates linked views and survives
undo/redo; no-op creates none. Existing property tests and native CI must pass.
Direct work; independent review and native rendered acceptance remain outstanding.
