# Requirement Apply history ownership

Leaf C05.EDIT.HISTORY.01; baseline `2d1f51b0faef8abb840d7c848adca2f458bb92cc`.
The native Requirement update is already transactional, but the production Apply
handler passes through the generic frontend pre-checkpoint wrapper. Both positive
and rejected edits reproduced an extra history_checkpoint in an executable UI test.

Recognize Updating Requirement as native-checkpointed. Preserve its draft on a
rejected ID/text edit and let the existing Rust transaction own undo/redo. Two
production-handler regressions now pass; existing native Requirement tests cover
transitive Copy, one-step history, no-op and failure rollback. No semantic or
persistence changes. Native CI and independent/rendered acceptance remain gates.
