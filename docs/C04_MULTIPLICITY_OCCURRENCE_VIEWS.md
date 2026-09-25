# C04.OCCURRENCE — compact and allocated part population views

User work order: 25 September 2026, multiplicity to separate IBD symbols.
Baseline: structural/connector candidate `e4866c066205d470939d92aff658718ead0edf88`.
The Vehicle example remains a fixture only. Rust remains authoritative.

Semantic contract: a Block owns one typed part property. The IBD owns its
presentations, not another property definition. A symbol may represent the whole
property population or a named, numbered subset of that population. An occurrence
name is a display alias, not a new UML Property name or classifier. This is an
explicit occurrence-allocation view, not an implicit creation of InstanceSpecifications
or runtime instances. Ordinary classifier-level IBD notation remains available.
Reference: supplied SysML 1.6 Blocks/Internal Blocks and UML 2.5.1 Property,
MultiplicityElement and Connector semantics; no certification guide authority.

Dependency leaves:

1. C04.OCCURRENCE.01: additive occurrence ranges on each semantic path segment,
   names and coverage, native partition/compact editing, occurrence-aware nesting,
   movement and connector projection. Allowed: workspace ibd.rs, ibd_structure.rs,
   ibd_projection.rs, new ibd_occurrences.rs, connector_editing.rs, standard_editing.rs,
   main.rs registrations and mechanical constructors, focused tests and this record.
2. C04.OCCURRENCE.02: native labels/coverage snapshots, explicit occurrence editor,
   repository repeat-drop and compact/expanded controls. Allowed: those native
   snapshot/command paths, ibd-ui.js, focused frontend tests, and this record.

Preserve stable semantic IDs and definition ownership. Occurrence identity consists
of property path plus selected population ranges at every nesting level. Counts
apply per enclosing instance. Repeated presentations of the same selected range
do not allocate additional instances. Distinct allocations must not overlap;
coverage is counted once per selected range and per containing occurrence.

Count/range rejection and unsupported endpoint editing must be atomic. Existing
type-owned connectors can project within each separately displayed containing
occurrence. A classifier-level connector to an aggregate population must not be
silently attached to its first subset; keep the semantic connector and hide that
presentation until an aggregate endpoint exists. Creating per-individual wiring
requires an explicit instance-connection model, not a cloned definition.

Acceptance: generic property [4] -> four named [1] groups or two [2] groups;
partial [3]/[4] coverage; invalid [5]/[4], duplicate aliases and overlap rejection;
multi-level repeated groups expand/move/collapse independently; type definitions
unchanged; return to compact; remove-only; save/reopen and undo/redo; inherited
properties and bounded ranges. Existing metadata defaults to compact.
Independent review and packaged/rendered UI verification remain required.

## Authoring workflow

1. Create the reusable Block definitions under a Package. Create the part on its
   owning Block, selecting the existing type by ID and setting multiplicity and
   composite aggregation. Alternatively draw the BDD composition and supply the
   property role/multiplicity in the composition dialog. The definition remains
   owned by its Package; its part is owned by the whole Block.
2. Open that whole Block's type-level IBD and choose **Show existing parts**.
   An old project opens with compact population symbols by default. No diagrams
   need to be rebuilt to access these controls.
3. Select a part symbol, choose **Separate / edit occurrences**, and enter one
   `display name = count` per line. For an exact [4] property, use four [1] groups
   or two [2] groups. Names may be specific to any domain. Rust checks distinct
   aliases, positive counts, disjoint allocations and the property's upper bound.
4. Alternatively drag the existing property from the repository into the IBD
   repeatedly. Each drop asks for an occurrence display name and count. Drop
   an internal property on its containing occurrence, or repeat it on one of
   its existing symbols. Rust resolves the enclosing type and rejects the wrong
   context. Existing allocations retain their range and presentation IDs.
5. The selected symbol's Properties panel reports **Complete population** or
   **Partial population view**, the displayed total and the definition constraint.
   A partial view is permitted. An excessive count is rejected without committing
   a history entry; the dialog retains its input for correction. Counts apply
   per enclosing instance, including when the enclosing group has count >1.
6. Use **Show / expand internal parts** on any occurrence. Its children refer to
   the same type-owned definitions, with that occurrence's population selection
   in their path. Collapse, move, resize, Clean Workspace, remove from diagram,
   save/reopen and undo/redo operate on these contextual presentations.
7. Choose **Show compact population** to present the entire property again.
   Choose **Open type-level IBD** to edit the shared type's internal structure;
   **Back to enclosing diagram** returns to the earlier view. Property edits
   affect all occurrences. Display names only affect their occurrence view.

For exact multiplicities, complete means the total equals the definition count.
A range such as [2..*] does not specify an exact population size; a finite
occurrence allocation remains a partial view, not a runtime instance count.
Reducing a definition's upper bound below an existing allocation is rejected
until its affected occurrence views are adjusted. This preserves unresolved
content instead of silently dropping symbols.

Separate occurrence symbols are explicit presentation subsets. They do not
create UML InstanceSpecifications, rename the shared Property, or independently
wire selected runtime objects. Shared internal connectors project in each
enclosing occurrence. Aggregate connectors remain semantic connections when
their compact endpoint is absent; the renderer does not substitute one subset.
Arbitrary wiring between selected individual slots needs a separate explicit
instance-connection model. The occurrence editor intentionally changes the
population allocation when counts/order are edited; repeat-drop fills an
available range without renumbering existing groups.

## Verification record

Native foundation tests exercise four generic occurrences, nested ports and
delegation connectors, per-occurrence movement/collapse, Clean Workspace,
SQLite round trip, compact conversion, partial counts, repeated-symbol counting,
invalid allocations and undo/redo. The controls leaf adds repeated drops,
preserved ranges after removal, nested same-name groups in different containing
occurrences, and wrong-context rejection.

Production JavaScript controller/DOM tests exercise alias/count rendering,
occurrence-aware collapse, target/zoom-coordinate dispatch, native transaction
ownership, corrected retries, cancellation, busy/stale guards and native coverage
display. Local full frontend suite: 216 passed on 25 September 2026. These are
rendered DOM contracts, not screenshots or packaged desktop end-to-end evidence.
Packaged launch, actual New/Open/Save/Reopen interaction, visual fit at multiple
zoom levels and independent review remain release qualification gates.
