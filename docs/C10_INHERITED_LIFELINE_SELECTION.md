# REL-GEN-01S: inherited parts in Sequence lifeline selection

Baseline: `596e24f1f41ccfdcf1bb4c0b4a679858aaaa0a53`. Independent leaf against main.
Direct primary-session implementation; no delegated or independent review claim.
Allowed paths: `apps/desktop/src-tauri/src/workspace/behavior_workspace.rs`
(native selector and colocated command regressions), and this work-order record.

The Sequence selector uses direct children while the native structural-path
validator already accepts inherited accessible features. Reuse
`Project::classifier_features` at each selected path level so the selector and
authoring command agree. Preserve original Property IDs and semantic owners;
deduplicate diamond inheritance, exclude private ancestor features and retain
own private features. Propagate native resolution errors instead of returning a
partial successful choice list. Reference: supplied UML 2.5.1 sections 9.2.3.2-3;
the existing core authority implements the applicable membership rules.

Acceptance: choose an inherited part through a diamond and a nested inherited
reference, author a lifeline through the actual native command, validate Behavior
and retain its represented IDs. Invalid private-ancestor paths must leave both
Behavior semantics and presentations unchanged. No JavaScript changes are needed.

The existing depth-six selector limit remains an explicit separate pagination/
recursive-type workflow gap. This leaf does not claim full-depth or large-model
selector qualification, inherited operation signatures, redefinition/subsetting,
runtime parity or rendered acceptance. Native CI and independent review are
recorded separately on the published candidate.
