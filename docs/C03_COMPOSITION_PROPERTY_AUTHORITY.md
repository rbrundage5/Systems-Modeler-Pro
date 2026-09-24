# C03/C04 composition-property completion work order

Baseline: `596e24f1f41ccfdcf1bb4c0b4a679858aaaa0a53` (merged main).
Authorized primary-session implementation; registered workers remain disabled.
Independent review and rendered desktop acceptance remain outstanding.

## REL-COMP-01A: canonical identity and native authority

Allowed production paths: `crates/model-core/src/model.rs`,
`crates/model-core/src/lib.rs`, `crates/model-core/src/association_properties.rs`.
Tests: `crates/model-core/tests/association_property_identity.rs`,
`crates/persistence/tests/association_property_persistence.rs`.
This record specifies the dependency split; it does not claim all increments done.

References: supplied SysML 1.6 sections 8.3.1 and 8.3.2.4; supplied UML 2.5.1
sections 9.5.3 and 11.5.3.1. A classifier-owned member end and its property must
identify one usage. Composite aggregation belongs to the part-typed property;
the diamond appears at the opposite whole classifier. Definition deletion is
different from runtime instance lifetime.

Use an optional stable Property ID on the association member end. The linked
Property is the authority for name, type, owner, multiplicity and aggregation;
serialized end fields are validated projections, never independent editable
copies. New linked associations use UML property-end aggregation. The optional
link explicitly distinguishes them from old notation-oriented payloads. Legacy
missing-link and empty-end associations retain their interpretation and IDs;
never infer links from equal names or pairs of Blocks.

Initial native scope is a binary association with one classifier-owned part or
reference end and one anonymous inverse end. Each Property belongs to at most
one association. Unsupported/malformed mixed linkage must fail validation.
Composition inverse multiplicity is 0..1; reference inverse multiplicity is 0..*.
Creation must publish both Property and Association or neither. Two uses of the
same definition remain distinct. Property edits refresh projections by identity.
Deletion of a referenced Property is rejected; deleting just its association
unlinks the Property without deleting its reusable type or existing usage.

Acceptance: Vehicle owns distinct front:Wheel and rear:Wheel; each association
links to its exact property. ElectricVehicle inherits those identities. Renaming,
changing multiplicity and retyping update projections. Invalid creation and
mismatched/dangling/duplicate linkage reject. SQLite and JSON roundtrips retain
identities and legacy unlinked interpretation. No UI exposure is claimed by A.

## Dependency-ordered remaining increments

- B: native BDD composition creation and property inspector integration, with
  routing prepared before commit and stable history/save/reopen behavior.
- C: association-end edits, reconnect/retype/reparent across affected views,
  reference-aware deletion and copy/duplicate identity remapping.
- D: adapter roundtrips and explicit legacy linking workflow, including authored
  workbook and XMI mapping without silent identity loss.
- E: rendered BDD/IBD and runtime acceptance using the connected fixture above,
  ports/ItemFlows, repeated occurrences, dense routing, and performance evidence.

Each subsequent increment gets exact paths, rejection/rollback cases and actual
verification evidence before publication. Do not call the whole composition
workflow complete from the authority tests alone.
