# C04.NESTED.01 — contextual structural occurrences

Baseline: main `2d1f51b0faef8abb840d7c848adca2f458bb92cc`.
User work order: nested BDD/IBD decomposition, 2026-09-25.
Reference: supplied SysML 1.6 §§8.3.2.3, 8.3.2.12 and 9; inherited UML 2.5.1
Property and Connector semantics. The certification study guide is not authority.

## Dependency-ordered implementation

1. This leaf: native lazy contextual occurrence expansion/collapse, existing parts
   population, stable occurrence selection and shared-definition scope in the UI.
   Existing IDs and semantic ownership never change during expansion. Additive
   serde-default presentation metadata keeps old files readable.
2. C04.NESTED.02: projected type-owned connectors and ports, contextual endpoint
   validation, contained movement/resize/removal, nested Clean Workspace/routing.
3. C03.STRUCTURE.03: integrate composition reuse/editing, type-level navigation,
   importable Vehicle fixture and full authoring/persistence/rendered acceptance.

Allowed paths for this leaf: workspace/ibd.rs, new workspace/ibd_structure.rs,
main.rs registration, ibd-ui.js, ibd.css, undo-redo-ui.js, native constructor sites
requiring the default collapse field, focused tests and this work order.
No CI, agent controls, new dependencies or startup demo data.

Acceptance: Vehicle/Engine/Piston/Ring definition ownership; expansion with no
semantic mutation; distinct engine usages/path IDs; repeat expansion no duplicates;
collapse/expand retains IDs/layout; JSON and SQLite metadata round trip; old
metadata defaults; bounded lazy recursion; rejected requests preserve history.
Native transactions own undo/redo. Frontend renders authored geometry and dispatches.

Independent review and installed application UI qualification remain open until
recorded with concrete evidence. A native test alone does not close user A–M.
