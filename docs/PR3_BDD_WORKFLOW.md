# PR #3 — First Visible BDD Workflow

This increment connects the Rust BDD semantic foundation to the Tauri desktop shell.

## User-visible acceptance flow

1. Launch Systems Modeler Pro.
2. Create a new project.
3. Create a package under the model root.
4. Create Blocks under the selected package.
5. Create a Block Definition Diagram owned by a package.
6. Place existing Blocks onto that diagram.
7. Select a Block from either the repository or diagram.
8. Rename the Block from the Properties panel and see the same semantic element update everywhere it is presented.

## Architectural rules

- The frontend never creates authoritative semantic records itself.
- Model, Package, and Block creation goes through typed Tauri commands backed by `systems-modeler-core`.
- A diagram node references an existing semantic `ElementId`; placing a Block never creates another Block.
- Diagram presentation identity and geometry are separate from semantic identity.
- BDD presentation accepts classifier elements appropriate to the current structural slice; this PR exposes Blocks in the UI first.
- The repository is rendered from Rust snapshots rather than maintained as an independent JavaScript model.
- Presentation state is kept in the Rust desktop layer in this increment. Durable diagram persistence and project-file open/save follow in the persistence increment.

## Deferred deliberately

This PR does not yet implement associations on the canvas, persistence-backed diagram geometry, drag routing, workbook import, or the full BDD palette. Those features build on this vertical slice after the frontend-to-Rust authority boundary is proven.