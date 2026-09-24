# REL-COMP-01I: publish the complete composition workflow to main

Baseline: `bfdd50ea63b1bc1eec7445c2e52c94c087d538ee` (2026-09-24).
Direct primary-session integration. No worker dispatch or independent-review claim.

PR157 reached main, but PR159, PR160 and PR162 were subsequently merged into
their feature-branch bases. Their merged status did not put native authoring,
cross-view editing, duplication or explicit legacy linking into main. The
documentation and commands for those workflows are absent from the baseline.

Merge source: `09a3d48bcc60df35114d76d70066f110c5cbaf1c`, the actual merged
composition-linked-editing branch. Preserve its ancestry and the current main
changes, including the BDD deletion lock-order repair and inherited Sequence
lifeline selection. This integration has no new semantic design or workflow-control
changes. Allowed production paths are exactly the carried PR159/160/162 paths;
their existing work-order documents and regressions remain authoritative.

Acceptance: the resulting main-targeted PR contains both BDD composition commands,
linked property editing, cross-view staging, native retype history, distinct
duplicate Property identities and explicit legacy linking. Run the native CI and
existing rollback regressions on the combined candidate, then verify the real
PR base is main and it is mergeable. A local integration is not a GitHub merge.

Independent review and rendered acceptance remain outstanding. Workbook/XMI
linkage, broader deletion/runtime semantics and large-model acceptance remain
separate gaps. Future dependent PRs must be checked for actual reachability from
main, not merely a merged PR badge.
