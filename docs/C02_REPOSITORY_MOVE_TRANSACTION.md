# C02.MOVE.01 — atomic repository reparenting

Baseline: `2d1f51b0faef8abb840d7c848adca2f458bb92cc`.
The previous Move command cloned and validated only part of the workspace, then
checkpointed and published Project and diagrams through separate fallible locks.
It did not retain a complete authored-state guard or validate Behavior/Activity/IBD
dependencies before publication. This can leave a partial or invalid reparent.

Move now stages the entire authored change under the existing guarded transaction.
It uses the conditional history primitive also introduced in PR179, unchanged,
so a same-owner operation preserves undo and redo. Core ownership/cycle validation,
linked-association presentation staging, existing Parametric presentation cleanup,
and dependent repository validation precede publication. Invalid surviving
dependencies reject the move instead of silently deleting unrelated model content.

Allowed changes: repository_editing.rs command and regression, history.rs's shared
conditional transaction primitive, and this record. PR179's identical helper is a
shared dependency; when integrating both, retain one copy. The production repository
UI already invokes this native command directly, so no new frontend checkpoint is
introduced or bypassed.

Native regression covers one successful history entry, stable owner/identity,
same-owner no-op, invalid parent and self-cycle rejection, undo/redo retention and
busy Activity-repository rollback. Existing deletion and Parametric tests remain
regression gates. Native CI is required; local Rust is unavailable. Independent
review and installed UI acceptance remain outstanding. This applies to nested
Requirements once their separate ownership feature is integrated; it does not add
new legal owner kinds or make every other command transactional.
