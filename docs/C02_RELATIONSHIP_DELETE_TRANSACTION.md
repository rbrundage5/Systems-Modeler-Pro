# C02 leaf work order: relationship deletion integrity

Depends on PR #171's guarded structural transaction. Finding TC-07: the generic
native BDD deletion command accepts any relationship ID and removes it directly,
without validating dependent ItemFlows, profiles, repositories or history.

Allowed paths: desktop `workspace/relationship_editing.rs`, `frontend/app.js`,
`frontend/undo-redo-ui.js`, `scripts/test_relationship_delete_history.cjs`, this file.

Require an actual presentation in the selected view, stage semantic deletion and
all structural presentations, validate dependencies, and checkpoint only successful
publication. Reject a missing presentation or dependent semantic/view reference
without losing redo. Keep reusable classifier and linked usage elements intact.
Verify multi-view removal with one undo/redo transaction and negative cases.

This command does not cascade IBD connector/ItemFlow or profile deletion. Requests
with such dependencies must fail visibly and leave the project intact. Broader
delete planners and symbol-only removal are separate workflows.
