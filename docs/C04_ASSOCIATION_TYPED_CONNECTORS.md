# C04.09.02a–c — reusable Association types for connectors

Baseline: `84255e7b5aff5afaf797ae8b4ee6a8e670f3abfa`, the structural integration
candidate over main `2d1f51b0faef8abb840d7c848adca2f458bb92cc`.
User work order: complete generic SysML semantics and workflows, 2026-09-25.
Primary session implementation; delegated workers remain disabled and independent
review remains outstanding.

The supplied UML 2.5.1 §§11.8.10.5–11.8.11.6 requires optional Association typing,
ordered end/type conformance, and connector-end multiplicities no more general
than corresponding Association ends. Existing part multiplicities and contextual
endpoint paths are distinct from those connector-end multiplicities.

Dependency-ordered leaves:

1. C04.09.02a: additive serde-default type and ordered multiplicity fields; native
   validation; mechanical constructor updates; generic positive/negative tests.
   Allowed paths: core ibd.rs/model.rs, constructor consumers in crates and desktop
   workspace, focused tests, and this work record. Existing update consumers must
   retain metadata until the explicit specification editor is added.
2. C04.09.02b: dependency-safe deletion/retyping, duplicate type-reference handling,
   SQLite and portable round trips, native history/rollback checks. Allowed paths:
   existing standard/relationship/connector editing, persistence and interchange
   tests, and this record. Do not infer a replacement type or silently untype.
3. C04.09.02c: native type/end choices and editing transaction, Properties controls,
   shared native notation, retained drafts, and focused UI/native tests. Allowed
   paths: connector_editing.rs, core connector notation, connector-properties.js,
   ibd-ui.js if required for rendering, their tests, and this record.

Acceptance uses arbitrary package/classifier names, unequal endpoint types,
subtypes, repeated properties, same-named Associations distinguished by ID,
explicit ordered ends, and independent endpoint/part multiplicities. Wrong type,
wrong order, missing type, overly broad multiplicity and dependent deletion must
reject without modifying authored state or history. Old connectors remain untyped
with multiplicity 1 at each end. Vehicle remains a separate test example only.

AssociationBlock identity/participants, connector redefinition, n-ary connector
presentation, full port contracts and runtime link-cardinality realization are
separate leaves; this ordinary binary Association slice does not close them.
Packaged UI/rendered and independent review gates remain explicit.
