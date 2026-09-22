# C16.08.02 — standalone IBD port paste boundary

Main baseline: 2b9750e8b3ca6de76f86ec7f367f155513063862.
Dependency: PR112, 32e074ea8a70440835e872ef0aaf46cbff737680.

## Bounded work order

Correct presentation paste of individual nested/context ports. Preserve semantic
identity and correct owner, project through the shared Rust port geometry, and
supply the actual visible frame for legacy context ports. Parent-plus-child paste
continues to reuse the parent's copied child. Atomic rejection and history remain
part of acceptance. Semantic Duplicate is a separate leaf.

Allowed paths: workspace/standard_editing.rs and standard_editing_bridge.rs under
apps/desktop/src-tauri/src; apps/desktop/frontend/standard-editing-ui.js;
scripts/test_standard_editing_clipboard.cjs and its existing test entry point;
this record. Lead works directly; delegation remains disabled, independent review
is not claimed. No reference uploads or enforcement controls change.

## Evidence and acceptance

The prior standalone path added a diagonal offset directly to port coordinates,
so a left/right/top/bottom port could move into the owning rectangle. Reuse the
existing Rust projection for new presentations. Before pasting a legacy boundary
port, adopt the supplied visible frame on the staged snapshot. Reject a missing or
invalid frame before authored state/history changes.

Test all four sides for nested and context ports; retain semantic identity and
valid connector mapping. Exercise target context mismatch, missing parent, legacy
frame adoption and rejection, and undo/redo. Frontend passes frame data only for IBD
paste; it performs no geometry or semantic computation.

Qualification: pending CI and local checks. Native Windows clipboard/visual
acceptance and independent review remain required. No whole-product completion
or CATIA compatibility claim. Next separate leaf: semantic Duplicate child paths.
