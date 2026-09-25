# C03.06: explicit part/reference conversion

Baseline: main `2d1f51b0faef8abb840d7c848adca2f458bb92cc`.
Finding: the property editor offered only composite aggregation on a part, so
the requested part-to-reference change was impossible without deleting identity.

Allowed paths: `crates/model-core/src/element_specification.rs`,
`apps/desktop/frontend/bdd-feature-editing.js`, their focused tests, this record.
Use the existing specification transaction/history and supplied SysML 1.6/UML
2.5.1 property/association semantics. No classifier cloning or ownership change.

The explicit structural-usage selector offers composite part, noncomposite
reference and shared reference. Rust stages the appropriate property kind and
aggregation together, refreshes linked association ends and validates the whole
candidate. IDs, owner, type, multiplicity and existing contextual paths survive.
Inverse association multiplicity is preserved: converting a reference to a part
rejects if a linked whole end permits multiple owners, until that dependency is
explicitly resolved. Rejection preserves the original model/form/history.

Acceptance: convert a linked part to a reference, verify identity and end
aggregation, convert back, and reject malformed aggregation without mutation.
Frontend dispatch test verifies the explicit control and one native transaction.
Native CI and direct desktop checks remain required; no independent review or
packaged acceptance is claimed by the primary-session implementation.
