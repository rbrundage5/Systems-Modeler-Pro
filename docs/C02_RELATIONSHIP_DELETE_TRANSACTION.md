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

## Integration recovery, 25 September 2026

Main baseline: `2d1f51b0faef8abb840d7c848adca2f458bb92cc`. PR173 merged
into the already-merged PR171 feature branch and never delivered this fix to main.
This main-based candidate restores only this leaf from merge
`4a9d061b6604f7634505edaf045cf8077c83bee0`. It retains PR174's connected-element
deletion and shared authored transaction. The restored presentation staging removes retired relationship views before validation.
Native CI must run against this candidate; old CI is not present acceptance.
Independent review/rendered acceptance remain open; no automatic merge.
