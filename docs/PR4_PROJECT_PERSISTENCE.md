# PR #4 — Project persistence and clean local development

This increment makes the first visible BDD workflow durable and removes local build artifacts from normal Git status.

## User-visible acceptance criteria

1. Running the Tauri application does not leave generated capability/schema files as uncommitted changes.
2. The committed workspace `Cargo.lock` remains authoritative; normal local builds should not require committing incidental lockfile churn.
3. A project can be saved to a local `.smproj` SQLite file.
4. A saved `.smproj` can be reopened in a later application session.
5. Semantic element IDs, external IDs, ownership, names, and BDD presentation IDs/geometry survive the round trip.
6. BDD presentations remain separate from semantic Blocks; opening a project never duplicates semantic elements.
7. Save/open failures are returned to the UI and do not silently replace the in-memory project.

## Architecture

The existing `systems-modeler-persistence` crate remains the semantic SQLite store. PR #4 extends the persistence boundary with project metadata for diagram presentation payloads. The frontend invokes explicit Rust save/open commands and never serializes the authoritative model itself.
