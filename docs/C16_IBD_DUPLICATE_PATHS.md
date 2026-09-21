# C16.08.03 — semantic Duplicate of connected IBD parts and ports

Main baseline: 2b9750e8b3ca6de76f86ec7f367f155513063862.
Dependency: PR113, dd26736b31b8827d7bd583f9240788528015c2aa.

## Work order

Repair IBD Duplicate identity, occurrence paths and attachment. Copying a part
must create one new semantic property of the same type, new child presentation
IDs, and paths through the new property. Ports inherited from the unchanged type
remain the same semantic ports. A port explicitly duplicated without its parent
creates a new semantic port owned by that same classifier. Selecting a parent and
its contained port must not also modify the reusable type. Connector ends must
reference the newly duplicated occurrences; results must be selection-order
independent. Repeated presentations of a selected semantic property share one new
semantic identity. Invalid results reject before model/history publication.

Allowed paths: apps/desktop/src-tauri/src/workspace/standard_editing.rs and
standard_editing_bridge.rs; apps/desktop/frontend/standard-editing-ui.js;
scripts/test_standard_editing_clipboard.cjs; this document. Lead implementation;
workers disabled by AGENTS.md, independent review outstanding. Existing shared
Rust projection, validation and history are reused; no new semantic engine.

## Evidence and acceptance

The baseline clones child presentation IDs and paths unchanged when duplicating a
part. The same diagram then contains repeated presentation IDs; validation rejects
the operation. Explicitly selected ports use an independent path that can create
a new port on the original type, miss the new presentation mapping, and detach
geometry. An eager map insertion also creates unused duplicate semantic elements
when one semantic property has multiple selected presentations.

Exercise connected part/port/connector selections in both orders, repeated part
presentations, standalone typed-port duplication, frame adoption, rollback,
undo/redo and serialized snapshot round trip. Existing copy/paste retains its
presentation-only identity rules. The supplied CATIA workflow guide distinguishes
presentation copy from new-element copy; this fix implements the repository's
existing Duplicate command contract, not vendor file/shortcut compatibility.

Qualification pending CI; native Windows Duplicate interaction and independent
review remain open. Next: connector and ItemFlow specification editing.

Local: all 36 frontend tests, standard-editing and Rust-authority gates, syntax
and diff hygiene pass. Four new Rust tests await native CI execution.
