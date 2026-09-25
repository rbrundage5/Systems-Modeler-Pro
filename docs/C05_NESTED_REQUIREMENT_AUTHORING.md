# C05.HIERARCHY.02 — transactional nested Requirement authoring

Depends on PR183's core ownership support. This leaf adds **Add Nested Requirement**
to Requirement Properties. The existing repository Move selector offers legal
Requirement parents and excludes cycles. Creation uses the selected parent's stable
ID; diagrams remain package-owned and the new child can be dragged onto a view.

The existing create_requirement IPC now uses the shared native structural
transaction, including dependent-view validation and one history entry. The frontend
does not add a pre-checkpoint. Failed creation preserves model and redo; duplicate
IDs and missing parents publish nothing. The dialog retains entered values for
correction. Cancel changes nothing; post-commit refresh failure never retries the
creation. A pending nested-creation dialog prevents duplicate button submissions.

Allowed paths: requirements.rs creation command/tests, app.js nested authoring,
repository-tree-ui.js parent choices, undo-redo-ui.js native command label,
test_nested_requirement_creation.cjs, and this record. Core changes belong to PR183.

Native regression: nested creation, stable owner, undo/redo, duplicate-ID and missing
parent rollback, busy authored repository. Four production-handler regressions cover
success, rejected-draft retry, cancel and refresh failure. Native CI, independent
review and installed rendered acceptance remain separate gates. Existing generic
repository Move transaction and palette create-then-place are separate workflow
leaves, not made atomic by this creation change. Recursive Copy remains subsequent
work in C05_REQUIREMENT_HIERARCHY.md.
