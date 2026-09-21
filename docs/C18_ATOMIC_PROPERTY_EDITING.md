# C18.03.01 — Apply a structural element specification atomically

Work order: 2026-09-21. Baseline: `2b9750e8b3ca6de76f86ec7f367f155513063862`
(current GitHub main, verified at task start). Branch:
`codex/c18-property-editing-hardening`. Authorized by the user's pre-packaging
editing and SysML audit/fix request. This is one shared Properties workflow.

## Reproduced source findings

The generic structural Properties editor disables Type except for Receptions,
hides an unset default, and sends rename, detail and feature-semantic commands
separately. The legacy Rust detail/semantic setters mutate the live project before
their final validation. A rejected late field can therefore leave earlier fields
changed, and one Apply can create several undo entries. Specialized Parametric
editors already expose types and use staged mutations; preserve those editors.

## Scope and acceptance

Provide compatible type choices from existing Rust rules, qualified names and
stable identities; expose unset feature defaults; route the generic editor's
name, documentation, type, default, multiplicity and flags through one staged Rust
command. Reject malformed bounds, incompatible types, invalid aggregation,
conjugated FullPorts and broken dependent references without changing any authored
state or history. One successful edit has one undo/redo step. Reapplying identical
values has none. Errors preserve the user's draft. Save/reopen preserves edits.

Production paths: `crates/model-core/src/element_specification.rs`, its module
registration in `lib.rs`; `apps/desktop/src-tauri/src/workspace/feature_editing.rs`
and `history.rs`; IPC registration in `apps/desktop/src-tauri/src/main.rs`;
`apps/desktop/frontend/bdd-feature-editing.js` and `undo-redo-ui.js`.
Tests: module-local Rust tests and `scripts/test_element_specification.cjs`, loaded
by the existing `scripts/test_dialog_escape_ownership.cjs` CI entry point.
Documentation: this record. Do not change agent controls or CI policy.

Sources: supplied CATIA Magic guide section 2 (specification editing workflow;
secondary evidence only), supplied OMG SysML 1.6 sections 8.3.2.4 (Block) and 9.3.2.9 (FullPort),
and supplied UML 2.5.1 sections 7.8.8 (MultiplicityElement) and 7.8.22
(TypedElement). This change reuses existing
type/connector validators; it does not certify their complete standards coverage.

The lead performs this bounded work directly. No workers are launched: AGENTS.md
still disables delegation, and independent reviewer execution is unavailable.
Publication is a draft PR, never a merge. No model override is requested.

## Verification and remaining qualification

Local implementation checkpoint: the new Rust command, history integration and
Properties adapter are implemented. `node --test scripts/test_dialog_escape_ownership.cjs`
passes 11 tests, including 9 new production-handler tests. All 21 existing
`scripts/validate_*.py` source integration contracts pass after preserving the
Reception Signal control identity. JavaScript syntax and `git diff --check` pass.
These source contracts are not end-to-end feature qualification.

Seven new core Rust tests and three desktop tests cover atomic failures, typing,
bindings, no-op/history behavior, orphaned IBD ports and SQLite save/reopen. They
are written but NOT EXECUTED: this hosted environment has no cargo/rustfmt or native
WebView. Playwright is installed without its browser executable. Rust formatting,
compilation, lint, native tests and rendered Windows acceptance remain UNVERIFIED.

Publication was rejected twice by automatic approval review. After the first
rejection, read-only GitHub checks confirmed the authenticated login, matching
repository owner and admin/push permission; the reviewer still requires explicit
permission to publish. No alternative publication channel was attempted, no PR was
created and no merge occurred. Next: user approves pushing this branch and opening
a draft PR in rbrundage5/Systems-Modeler-Pro; execute/fix final-head CI and perform
independent/native review before merge or packaging.

Whole-product gaps remain: all-nine-family interaction and notation audit;
specialized editor coverage; standards leaf dispositions; deployed collaboration
and installed upgrade acceptance. Open PRs 104–106 are separate unmerged
collaboration leaves; this branch does not claim or integrate their capabilities.

Code checkpoint before this evidence-only update: `1a7cb2a108c5a89c882226f86747317cb9061465`.
The separate C16.06.01 cancellation fix is on `codex/c16-cancelled-drag-restore`
at `f3d9b827a0285fb0605fd07a6a615cf7eba422fe`; it also remains unpublished.

Publication authorization: the user explicitly approved pushing both branches and
opening draft PRs in the following turn. Publish through the connected GitHub
account, execute CI, and retain the independent-review/native-acceptance gates.
