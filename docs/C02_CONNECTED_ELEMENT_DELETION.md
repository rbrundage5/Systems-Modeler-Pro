# C02 work order: delete an element with its relationships

User request: delete a model element and apply the correct dependent effects.
Baseline: `f3e45cc0d1bf5ff40001afe75ac03fc1e4bfa8c7`.

Allowed paths: core `deletion.rs`, `lib.rs`, `activity.rs`, `behavior.rs`,
`tests/connected_deletion.rs`; desktop `workspace/history.rs`,
`workspace/repository_editing.rs`; `frontend/standard-editing-ui.js`,
`frontend/repository-tree-ui.js`;
`scripts/test_connected_element_delete.cjs`; this work order.

Implement Delete from Model as a staged Rust operation: delete the selected
element and owned descendants; remove incident/owned relationships, Connector
and ItemFlow dependents, and applicable profile applications; remove presentations
from all views, including diagrams and behaviors belonging to deleted contexts.
Keep reusable types and unrelated usages. Shared external type/behavior/profile
dependencies that cannot remain valid are rejected atomically with their validation
error; do not silently delete unrelated owners or behavior graphs.

The selected element, affected relationships, views and specialized authored state
are one undoable edit. Rejection, contention, or poisoned late/history locks must
leave authored state and redo intact. Save/reopen must retain the resulting model.
Delete presentation remains a distinct action from Delete from Model.

Acceptance covers a connected part with Connector/ItemFlow and association
presentations, owned children, retained reusable type and sibling usage,
all-view removal, root/external-dependency rejection, undo/redo and database reopen.
Independent review and rendered desktop acceptance are outstanding.
