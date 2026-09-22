# C04.09.02 — connector typing implementation specification

Prepared 22 September 2026 against PR122, e432beb82a3892597785ece20de5a89c2d882740.
Main remains 2b9750e8b3ca6de76f86ec7f367f155513063862. This is a design work
order, not implemented functionality or an independent review. Allowed paths:
this file and one entry in docs/README.md. Lead direct work; workers remain disabled.

## Approved normative evidence

The supplied UML 2.5.1, formal-17-12-05, clauses 11.8.10.5 and 11.8.10.7
(printed pages 227–228; PDF 269–270) defines optional Association typing and
ordered end/type conformance. Clauses 11.8.11.4–11.8.11.6 (printed 228–229;
PDF 270–271) define corresponding association ends, role/part-with-port consistency
and connector-end multiplicity restrictions.

The supplied SysML 1.6, formal-19-11-01, 8.3.2.7 (PDF 86–87), 8.3.2.13
(PDF 92–93) and 9.4.5 (PDF 140–143) connects AssociationClass-based blocks
with connector properties, participant properties and reusable internal connection
structure. These are distinct capabilities. Reference text and diagrams are not
copied into the repository.

## Current representation and hazards

| Concern | Current code evidence | Required treatment |
| --- | --- | --- |
| Connector typing | `ibd.rs::Connector` has context, kind and two ends, without an Association reference | Explicit optional stable type identity and legacy migration. |
| Association identity | `model.rs::Relationship` owns ordered AssociationEnd records with stable end IDs | Reuse identity; no name-based type keys or guessed end order. |
| AssociationBlock | Classifier kind exists without an explicit association/class pairing | Define a coherent representation before offering these blocks as connector types. |
| End multiplicity | ConnectorEnd has path/role/port only | Explicit connector-end multiplicity; do not substitute part multiplicity. |
| Compatibility | `validate_connector_compatibility` compares endpoint types to each other | Typed ends must conform to corresponding Association ends; preserve separate port contract validation. |
| Deletion | `delete_bdd_relationship` directly removes the relationship and BDD edges | Stage reference checks before mutation; repair before exposing type references. |
| Duplicate | `duplicate_relationship` remaps connector context and feature paths | Reuse the type unless the type itself is intentionally duplicated. |
| Runtime | `structural_runtime.rs` represents runtime connector links | Type metadata does not instantiate AssociationBlock internals or participants. |

The repository's nested `ConnectorEnd::role_id` identifies its property occurrence;
`port_id` identifies the terminal Port. These names are not a literal mapping of
the UML metamodel's role/partWithPort fields. PR122 enforces the current internal
path/role contract. Exporters must map by meaning.

## Complete authoring journey

1. Create/reuse an Association in its package with ordered, named, typed ends and
   multiplicities. Keep stable relationship and end identities.
2. Select an IBD connector; choose a qualified compatible Association type and
   inspect corresponding source/target ends and connector-end multiplicities.
   Preserve an explicit Untyped choice for existing workflows.
3. Apply through the established Rust specification transaction. Validate type,
   ordered ends, multiplicities, port contracts and every dependent diagram.
   Rejection names the incompatible end and retains the form draft.
4. Rename/retype, duplicate, undo/redo, save/reopen and export/reimport without
   identity loss or unintended ItemFlow reversal. Referenced-type deletion rejects
   with uses listed until affected connectors are explicitly untyped/retyped.
5. AssociationBlock authoring additionally requires association/class identity,
   participant mappings, connector properties and navigable decomposition.
   Runtime initialization reports unsupported decomposition until implemented.

## Dependency-ordered implementation leaves

| Leaf | Outcome | Negative/rollback evidence |
| --- | --- | --- |
| C04.09.02a | Backward-compatible type/end-multiplicity representation and core validation for ordinary binary Associations | Missing/non-Association type; wrong arity/order; nonconforming end; invalid multiplicity; old projects reopen; rejected staged edit leaves model unchanged. |
| C04.09.02b | Type lifecycle through deletion, endpoint/Association editing, Duplicate, persistence and native interchange | Referenced deletion/retyping rejects before mutation; reused vs copied type IDs remain correct; unchanged reimport is a no-op; no partial history. |
| C04.09.02c | Existing connector Properties API/form exposes types and end multiplicities | Qualified choices, retained failures/drafts, late-response isolation, one undo checkpoint, all views and ItemFlows consistent. |
| C04.09.02d | AssociationBlock association/class identity and participant mappings | Wrong owner/end/type and duplicate mappings reject; inheritance/retype effects validated; native/portable identity survives. |
| C04.09.02e | AssociationBlock typing, connector properties and navigable internal decomposition | Wrong context or inconsistent connector-property name/type rejects; participant wiring validated; diagram ownership and reuse remain distinct. |
| C04.09.02f | Established runtime, adapters and shared-operation contract cover the authored slice | Deterministic initialization/reset; unsupported cases explicit; genuine vendor fixtures before compatibility claims; existing revision rules protect shared operations. |

Each leaf names exact paths after checking the actual baseline and conflicting
PRs. Schema consumers are part of a representation change, including mechanical
constructor/adapter updates. The type references the Association's stable identity
and retains ordered correspondence. Do not automatically accept either orientation
or infer mapping from names. An explicit reorder preserves physical ItemFlow
direction through the established staging logic.

## Qualification boundary

Preserve all nine families, untyped connectors, stable external IDs, copy-vs-
Duplicate semantics, atomic rejection, SQLite/portable compatibility and authored/
runtime isolation. Include duplicate names, generalization conformance, repeated
occurrences and two diagrams showing one connector. Type labels use shared notation;
JavaScript does not acquire routing, layout or semantic authority.

Ordinary binary Association typing is an intermediate outcome, not complete
AssociationBlock support. Multi-ended connector presentations, connector
redefinition, inherited/nested port traversal beyond the current part-path slice,
full interface contracts and vendor interchange remain separately accounted.
Independent review, installed-desktop acceptance and CI on the integrated release
remain mandatory. This specification does not close the association-typing gap.
