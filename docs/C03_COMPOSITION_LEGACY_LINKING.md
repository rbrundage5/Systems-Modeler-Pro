# REL-COMP-01D1: explicit composition-to-part linking for existing models

Depends on PR160/159/157. Direct primary-session implementation.
Allowed paths: core `association_properties.rs`, its identity regression test;
desktop `workspace/relationship_editing.rs`, `main.rs`; frontend `app.js` and
`undo-redo-ui.js`. This records a bounded legacy-linking workflow, not adapter D2.

Existing compositions must not silently infer identity from names or Block pairs.
The relationship inspector offers native-validated eligible PartProperties and
an explicit Create new part choice. Linking reuses the selected stable Property
or creates one in an atomic native history transaction. Existing relationship and
member-end IDs remain stable. Legacy missing-end Composition records receive new
member ends only because they had no previous member identities.

Legacy aggregation denotes the diagram diamond side. On explicit linking, convert
to canonical part-end aggregation while keeping the diamond at the whole. The
selected existing property's name/multiplicity are authoritative. Never reuse a
property already belonging to another association. Invalid ownership, typing,
whole multiplicity, payload shape or dependent validation rejects without changing
model, views or history. Repeated linking to the same Property is a no-op.

Acceptance: link both source-diamond and target-diamond legacy associations;
reuse a chosen part without duplicates; reject a foreign property without any
change; retain legacy relation/end IDs and roundtrip through current persistence.
Newly created parts appear in normal IBD population and inherited-feature queries.

Still open: workbook/XMI member-property ID roundtrips, broader migration fixtures,
full rendered interaction and runtime lifetime acceptance. Independent review is
outstanding; source/integration checks are not visual acceptance.
