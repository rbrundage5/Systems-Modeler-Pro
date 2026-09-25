# C04.NESTED.04 — explicit type-level IBD navigation

Depends on PR188. Authorised user workflow: open the reusable type definition,
open/create its IBD, and return to the enclosing diagram without changing scope.
Allowed: new ibd_navigation.rs, ibd_structure.rs port helper visibility, main.rs,
ibd-ui.js and focused tests/docs. Reuse existing diagram registry/history.

Rust resolves Block versus typed-property IDs, validates context/owner, finds an
existing type IBD or stages a populated new one with one undo checkpoint. Opening
an existing IBD is a no-op in authored history. Frontend only dispatches the target
stable ID and maintains the existing-style view navigation stack. Scope is named
explicitly as type-level; contextual expansion remains in the enclosing IBD.

Acceptance: repeated navigation reuses one diagram, creation/population atomic,
invalid target leaves history unchanged, property-to-type resolution retains
model IDs, UI return navigation and no duplicate frontend checkpoint. Installed
UI/persistence workflow and independent review remain open gates.
