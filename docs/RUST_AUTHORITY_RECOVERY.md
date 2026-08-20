# Rust Authority Recovery

Baseline: merged `main` at `d117fc1` (PR14 and PR15).

Systems Modeler Pro is a Rust application with a thin Tauri web renderer. The
frontend is not an application engine. It may draw authoritative snapshots,
capture input, preview an in-progress pointer gesture, and invoke typed Tauri
commands. Rust owns semantics, identity, validation, command eligibility,
selection/tool state, routing, layout, history, persistence, presentation
metadata, and committed workspace preferences.

## Measured PR15 baseline

- Rust: 14,374 source lines.
- Frontend JavaScript: 7,241 source lines across 39 files.
- Direct `state.*` assignments: 332.
- Renderer wrapper assignments: 32.
- Blocking `prompt()`/`alert()` calls: 73.
- Independent keydown controllers: 7.

Rust is the majority by source volume, but the frontend controller count and
mutable-state surface are too high. These values are migration debt, not a
target architecture.

The first recovery slice adds a 38-line compatibility bridge so existing
renderers publish their interaction mirror to Rust without losing behavior.
The enforced post-bridge JavaScript ceiling is therefore 7,279 lines. No later
feature or recovery slice may increase it.

## Recovery rules

1. No feature PR may increase any frontend debt metric enforced by
   `scripts/validate_rust_authority.py`.
2. Recovery PRs lower the ceilings after moving a complete behavior into Rust.
3. Do not replace all frontend code at once. Preserve qualified behavior while
   removing one authoritative controller chain at a time.
4. A Rust manifest with a JavaScript implementation is not Rust authority.
   Rust must execute or validate the operation and return the resulting state.
5. Static source-presence checks supplement but never replace Rust unit tests,
   integration tests, and desktop interaction tests.

## Ordered recovery

1. Centralize active diagram, selection, pending tool, and cancellation state in
   Rust; renderers receive that state in workspace snapshots.
2. Replace chained render-function reassignment with one renderer host contract.
3. Move content-bound computation, routing, and clean layout to typed Rust
   geometry adapters for every diagram family.
4. Replace blocking browser dialogs with one thin dialog renderer whose
   candidates and submitted IDs are Rust-owned and validated.
5. Consolidate keyboard and command dispatch into the Rust command manifest and
   one frontend input bridge.
6. Replace full DOM rebuilds with keyed scene updates and add large-model
   interaction benchmarks.
7. Delete superseded compatibility and runtime-fix scripts after their behavior
   is covered by the shared Rust path.

The first recovery slice introduces Rust-owned workspace interaction snapshots,
selection/tool validation, monotonic interaction revisions, and authoritative
clear/cancel commands. Existing diagram-family state remains as a compatibility
mirror until each renderer consumes the Rust snapshot directly; this preserves
qualified behavior while authority migrates.

## Performance gates

Language choice alone cannot guarantee performance. Each recovered subsystem
must include representative large-model benchmarks for snapshot generation,
serialization, routing/layout, render-update count, selection latency, pan/zoom
frame time, and save/open. New diagram families must reuse these paths rather
than add controller layers.
