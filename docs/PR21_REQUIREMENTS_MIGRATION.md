# PR 21 Requirements and Traceability Migration

## Authority and compatibility

- Rust owns Requirement/TestCase semantics, identity, ownership, validation, mutation, history, persistence, diagram geometry, and traceability legality.
- Requirement diagrams use the shared diagram-family, BDD geometry, obstacle-aware routing, clean-layout, presentation-manifest, and history infrastructure.
- The frontend remains an input/rendering adapter; no JavaScript semantic repository is introduced.

## Modeler Proto migration checklist

| Capability | Systems Modeler Pro implementation | Migration decision |
|---|---|---|
| Requirement name, ID, text, documentation | First-class persisted Rust fields and commands | Preserved; identity, External ID, display ID, and text are explicitly separated |
| Requirement containment | Existing Package/Model ownership | Preserved and corrected; relationships do not become containment children |
| Requirement hierarchy | Package containment plus semantic `deriveReqt`, `copy`, and traceability links | Standards correction; visual nesting is not substituted for semantic relationships |
| Existing-element diagram placement | Shared diagram presentation collection and Repository drag/drop command | Preserved |
| Requirement editing | Rust `update_requirement` command | Preserved; multiline text is a single command payload and stable identity is protected |
| Traceability | `deriveReqt`, `satisfy`, `verify`, `refine`, `trace`, `copy` relationship kinds | Improved from generic-arrow behavior to validated semantic relationships |
| TestCase verification | First-class TestCase endpoint with Rust legality checks | Improved |
| Route/Clean | Shared family capabilities and shared persisted edge geometry | Preserved; no Requirements-specific router |
| Undo/Redo | Existing whole-workspace Rust history checkpoints | Preserved from the first mutation path |
| Save/reopen | Existing SQLite element/relationship payload plus diagram metadata | Preserved; UUID and External ID round-trip tests added |

No modeler-proto Requirement capability is intentionally deferred at the semantic-contract layer. Desktop visual smoke testing remains required for pointer interaction, multiline focus behavior, notation appearance, and Route/Clean quality.

## Windows smoke test

1. Create a Package and a Requirement Diagram; confirm `req` frame and package ownership.
2. Create two Requirements and one TestCase; edit name, ID, multiline text, and documentation.
3. Drag an existing Requirement and Block from Repository to the diagram.
4. Create each traceability relationship and confirm illegal endpoints are rejected.
5. Move/resize nodes, select edges, press Escape, Route, and Clean Layout.
6. Undo and redo every mutation above.
7. Save, close, reopen, and confirm identity, content, ownership, routes, and presentation geometry.
8. Smoke BDD, IBD, Activity, State Machine, and Sequence switching, palette, drag/drop, selection, Route, and Undo/Redo.
