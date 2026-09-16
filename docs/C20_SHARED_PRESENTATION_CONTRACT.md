# C20.06a — shared structural presentation contract

Baseline: `320582838fb2d446a65ae1956076839a716a73ec` (PR103).
Main: `2b9750e8b3ca6de76f86ec7f367f155513063862`.

One leaf: extract the existing native structural presentation records and validation
into model-core so future desktop/server commands can use the same authority.
Allowed production paths: model-core lib.rs and structural_presentation.rs;
desktop workspace.rs and workspace/package_diagrams.rs. Supporting paths: the
focused core contract tests and this document. No other author owns these paths
in the open follow-up PRs inspected at implementation start.

The desktop re-exports the core types and delegates existing validation to the
same moved functions. JSON field names, defaults, diagnostics and accepted values
are preserved. The `bdd-diagrams` metadata key and existing file format remain
unchanged. Package helper predicates and endpoint checks also have one authority.
There is no server endpoint or capability announcement in this leaf.

Acceptance: old sparse records deserialize with legacy defaults; stable IDs and
serialized fields survive re-encoding; malformed IDs, invalid owners/contexts,
duplicate presentations, missing and reversed relationship endpoints reject without
semantic mutation. Existing desktop native/interchange/family tests must continue
passing. This extraction deliberately preserves existing validation behavior;
additional network-boundary geometry/size/family restrictions belong to the typed
command leaf before this contract is accepted from a remote client.

Verification: Rust compilation/tests/formatting/lint run in existing GitHub CI;
this hosted workspace has no Rust toolchain. Final-head results belong in the PR.
Independent review is outstanding; worker delegation remains disabled. No merge.

Next: C20.06b typed BDD commands and transactional server storage, followed by
C20.07a normal-workspace BDD integration. All-nine collaboration, administration,
deployed HTTPS and native two-device acceptance remain open. This extraction alone
does not change the user-visible shared editing surface or finish collaboration.

## Local verification checkpoint

Implementation commit: `d20dfb8`.
- Rust-authority gate: PASS.
- Shared-workspace convergence contract: PASS.
- Existing collaboration frontend regression: 24/24 PASS.
- Direct comparison of extracted validator/helper bodies to baseline: identical
  after module-path/function-name adjustments.
- Diff whitespace check: PASS.
- Added Rust compatibility/negative tests: NOT RUN (no local Rust toolchain).
- Rust formatting, compilation, native regression and independent review: pending.

Publication was blocked by automatic approval review: the push to the configured
GitHub repository was rejected for lack of explicit authorization to share the
new source with that destination. No alternative publication route was attempted.
Next action requires explicit approval to push this branch to
rbrundage5/Systems-Modeler-Pro and open a draft PR against
codex/c20-collaboration-review-candidate. Run existing CI, resolve any diagnostics,
and qualify this leaf before implementing its dependent command integration.

Publication was explicitly approved by the user on 2026-09-16. Draft PR104 is
open against PR103. The authenticated GitHub connector publishes the candidate
because shell Git has no credential. The earlier approval blocker is resolved.
