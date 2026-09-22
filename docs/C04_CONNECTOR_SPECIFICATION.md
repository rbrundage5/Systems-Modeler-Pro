# C04.08.01a — atomic connector specification and endpoint choices

Main baseline: 2b9750e8b3ca6de76f86ec7f367f155513063862.
Candidate dependency: PR115, db0c580f21647cbaf12bacffd251bbfba76887c0.

## Feature split and work order

The IBD Properties override exposes no connector Apply form. Deliver in order:
(a) Rust semantic staging, endpoint choices, all-presentation remapping and atomic
history; (b) draft-preserving Properties UI; (c) editing existing ItemFlow names,
directions and conveyed classifiers. Explicit association typing is a distinct
metamodel/migration capability and is not implied by this workflow.

This leaf implements (a). Allowed paths: crates/model-core/src/ibd.rs;
apps/desktop/src-tauri/src/main.rs; workspace/connector_editing.rs, history.rs and
ibd.rs under apps/desktop/src-tauri/src; this document. Lead direct implementation;
workers remain disabled; independent review is outstanding.

## Contract

Edit connector name, kind and presented source/target in its existing context.
Preserve relationship/presentation identities, ownership and metadata. Validate
with existing core connector/type/topology rules. Endpoint choices resolve from
the active IBD and include semantic paths and qualified labels. Other diagrams
must already present suitable endpoints; otherwise reject the complete edit with
a remedy. Do not silently delete a presentation or create a new model element.

Update dependent ItemFlows when an endpoint is replaced, preserving direction
relative to the replaced end. Swapping connector end order alone preserves the
existing physical ItemFlow direction. Validate the final project and all dependent
presentations. Reroute only changed connector endpoints and their label anchors.
Use one existing history transaction; rejection and no-op preserve undo/redo.
Native/portable schemas stay unchanged.

Acceptance: valid rewire in multiple diagrams; preserved stable IDs/flow direction;
invalid topology, incompatible types, missing presented endpoint and routing failure
reject without partial name/semantic/geometry/history changes; undo/redo and no-op.

Qualification pending CI. UI exposure belongs to leaf (b). Existing ItemFlow form,
association typing, native Windows acceptance and independent review remain open.
