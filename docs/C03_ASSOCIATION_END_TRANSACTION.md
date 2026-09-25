# Association end editing transaction

Leaf C03.ASSOC.EDIT.01. Baseline main:
`2d1f51b0faef8abb840d7c848adca2f458bb92cc`.

The end editor changed the core Project in isolation; the frontend checkpointed
before validation. It did not validate the dependent authored repositories before
publication. Scope: relationship_editing.rs, the native-history label in
undo-redo-ui.js, focused frontend/native tests, and this record.

Use the existing structural specification transaction for a role, multiplicity,
navigability or aggregation edit. Core association/Property identity rules remain
authoritative. Validate dependent views and behavior before publishing; rejected
edits and no-ops preserve redo. Commit one checkpoint; undo restores the linked
Property and relationship together. Existing project format is unchanged.

Evidence: production UI success/rejection tests pass. Native tests cover linked
part identity/owner/type, role/multiplicity consistency, two views, database reopen,
one-step undo/redo, no-op, invalid role/navigation and malformed dependent views.
Native CI is required because the local runtime lacks Rust. Independent review and
rendered desktop acceptance remain outstanding. No automatic merge or worker use.

Acceptance: edit a composition role/multiplicity, inspect the owning Block's parts
and its IBD, undo/redo, save/reopen, then attempt an invalid edit with redo pending.
Other mutators, new feature semantics and whole-tool completion are separate work.
