# C20.01 — Shared requirement authoring and verification traceability

Baseline: `2b9750e8b3ca6de76f86ec7f367f155513063862`.
Work order: finish one shared engineering workflow from the existing authenticated
revisioned edit boundary. This work is authorized by the user's continuation to
finish the tool; it does not enable agent workers or automatic merges.

## User workflow

An editor creates a Requirement with a name, human-readable ID and multiline text,
then creates a TestCase and uses the existing Verify relationship controls to
connect it. A Block can Satisfy the same Requirement. Another authenticated client
reads the same stable identities, text and traceability. An editor can load and
update the Requirement without breaking either relationship.

The form displays the saved requirement alongside the editable draft. A draft
retains its original project and revision across explicit refreshes. Stale drafts
cannot silently overwrite newer content. After reviewing the latest saved values,
the user explicitly keeps the draft on that revision or clears it. New-project
open and disconnect require saving or clearing the draft. Polling pauses while a
draft is being composed. Pending network retries retain the existing exact
operation identity; the form is cleared only after successful confirmation.

Server operations are `CreateRequirement`, `UpdateRequirement` and `CreateTestCase`.
They use the existing model-core constructors/update methods and the same SQLite
transaction as the revision and operation receipt. The update changes name, ID
and text together. It preserves semantic UUID and unrelated fields. Existing Copy
protection and supplier-text propagation remain authoritative in model-core.
The new `shared-requirements-v1` capability prevents a new client from connecting
to a server that lacks these operations. Old BDD operation payloads remain valid.

The existing 16 KiB HTTP request-body limit applies, including JSON escaping and
UTF-8 bytes. Existing core ownership rules apply; nested Requirement ownership is
not introduced by this change. No new normative SysML rule is claimed.

## Scope and verification

Allowed production paths: persistence `collaboration.rs`; desktop
`collaboration-ui.js` and `collaboration.css`. Focused tests are in persistence
`collaboration_requirements.rs`, server `tests/server.rs`, desktop
`collaboration_integration_tests.rs`, and `scripts/test_collaboration_ui.cjs`.
This document and the desktop collaboration workflow document record the behavior.
No other production subsystem, agent policy, CI gate, or dependency is changed.

Acceptance: two Rust desktop sessions use the real loopback server to create a
requirement, TestCase, Block and Verify/Satisfy links; update the requirement;
reject a stale update; converge on the same state; and recover that state after
server restart. Persistence tests cover retry after database reopen, unchanged
identity and links, duplicate/blank ID rejection, invalid ownership/kind, viewer
rejection, stale revisions, and Copy protection/propagation. Invalid edits must
leave model, presentation, revision and operation receipts unchanged.

Local frontend regression, syntax, Rust-authority and diff checks are available.
Rust is not installed in the hosted authoring workspace; GitHub CI must compile,
format, test and lint the exact published head before merge. Test presence is
not test success. Final results belong in the PR checkpoint.

Independent review remains required. Worker delegation remains disabled by the
repository's environment gate. Native Windows rendering and deployed two-device
acceptance remain unverified. This workflow does not complete collaboration:
normal-workspace integration, other diagram families, specialized structural
operations, presence, collaborative undo and crash-persistent pending edits remain
separate follow-up work.
