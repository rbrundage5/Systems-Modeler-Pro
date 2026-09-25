# Vehicle structural decomposition: candidate workflow and qualification

This guide describes the structural candidate, not an installed release. Main and
existing installed applications are unchanged until the leaf PRs are merged and
an updated application is installed. Do not rebuild existing diagrams in advance.

## Work order and delivery boundary

C04.05: qualify the combined Vehicle workflow from C03 composition authoring and
C04 contextual expansion, contained editing, connector projection and navigation.
Allowed changes: this guide, the test-only registration in
`workspace/property_presentation.rs`, and `workspace/vehicle_workflow_tests.rs`.
Integration conflict formatting is allowed; no independent production feature is
bundled into this qualification increment. Primary-session implementation only;
independent review remains outstanding. References: supplied SysML 1.6 Block,
property and nested-connector definitions and UML 2.5.1 Property/Association rules.

The assembly branch `codex/structural-candidate-2026-09-25` combines the bounded
leaf candidates for testing. It is not a merge unit. Land the reviewed leaf PRs
individually, retargeting stacked children to main after their parent lands.
Retarget the qualification PR only after those dependencies land.

## Build the example manually

1. New project; create package `VehicleDefinitions`. Create a BDD in that package.
2. Create four Block definitions in that package: `Vehicle`, `Engine`, `Piston`,
   `Ring`. Place these existing definitions on the BDD. Their names stay plain
   classifier names; do not rename Engine to `engine: Engine`.
3. Choose Composition and connect Vehicle (whole) to Engine (part type). Choose
   **Create a new named part property**, enter `engine` and multiplicity `1`.
   Composite aggregation is explicit in this dialog. Repeat Engine → Piston with
   `pistons`, `4`, and Piston → Ring with `rings`, `3`.
4. If the property already exists, choose its existing usage in the composition
   dialog. Alternatively select that PartProperty in the repository and use
   **Show composition on BDD**. Both paths reuse its stable classifier reference.
5. Check the filled diamond at each whole, with role name and multiplicity at the
   part/type end. The repository package must still own all four Blocks. Each
   Block owns its respective PartProperty.
6. Double-click Vehicle to open/create its type-level IBD. Use **Show existing
   parts** if the diagram already existed before the property was authored.
7. Select `engine: Engine [1]`; in Properties choose **Expand internal structure**.
   Select its `pistons: Piston [4]` occurrence and expand again to show
   `rings: Ring [3]`. This is a contextual view of shared definitions. It represents
   three rings per piston: one Engine, four Pistons, twelve Rings in one Vehicle.
8. **Open type-level IBD** opens the reusable type's separate diagram;
   **Back to enclosing diagram** returns to the prior view. **Select type
   definition** selects the classifier. The scope message identifies edits that
   affect every usage of that shared type.
9. Add `rearEngine: Engine [1]` by drawing another Vehicle → Engine composition and
   explicitly choosing a new property. Show existing parts and expand it. Both
   piston occurrences reference the same property ID but have different paths.
10. Edit the shared pistons property's name/multiplicity/type through Properties
    and Apply. Check every diagram and linked association. Changes are validated
    by Rust; invalid dependent connections/views reject the edit atomically.
11. Collapse, drag, resize, Route and Clean Workspace. Remove from diagram hides
    presentations only; Delete property from model uses native dependency cleanup
    and must retain Engine/Piston/Ring definitions. Exercise Undo and Redo.
12. Save, close and reopen the native project. Verify IDs, geometry, paths and
    collapsed state. Repeat New/Open/Save/Reopen through the packaged application,
    including a real project created by the previous release.

## Automated evidence

The cross-feature native fixture creates the four Blocks and three properties
through the composition authoring transaction; checks BDD nodes/relationships,
owners/types, the 1/1/4/12 runtime populations, nested expansion without semantic
creation, linked-end synchronization, SQLite model plus BDD/IBD metadata reopening,
presentation removal, property deletion retaining classifiers, and undo/redo.
It is test data only, never application startup content.

Related leaf tests cover two Engine usages with distinct contextual paths;
collapsed visibility; expansion at depth 32; legacy metadata without the new
optional fields; no-op and rejected transaction history; contained movement,
resize rejection, cleanup; projected existing connectors and type navigation.
Frontend tests call production renderer and dispatch functions using a DOM test
double. They are not browser screenshots or a substitute for a packaged UI test.

Local combined result before the new native fixture: 196 frontend tests passed;
all repository `validate_*.py` checks passed. Native fixture execution and final
combined Rust checks are recorded in the qualification PR's exact-head CI.

## Gates still requiring direct desktop evidence

No native desktop or installed browser executable is available in this hosted
checkout. No package was installed and no screenshots were fabricated. Therefore
the real UI A–M walkthrough, rendered BDD/expanded-IBD screenshots, interactive
layout quality/performance, a representative user-owned older project, and the
packaged command-path permission error remain unverified here. CI/native tests
alone do not close those gates or establish that the user's installation is fixed.

Existing valid IDs and layout are retained. New presentation fields have serde
defaults; no destructive model rewrite is introduced. Shared property edits are
resolved by ID when views render. Newly authored child definitions appear on
Show existing parts/expansion. Recursive types remain legal; expansion is lazy,
bounded to 32 levels and 4096 property occurrences per diagram.

Further limitations: no per-occurrence cloned type overrides; changing types or
deleting referenced classifiers can require resolving dependencies first. The
broad all-SysML audit remains open, including unsupported structured Activity
execution. This structural candidate is not a claim of complete SysML coverage.
