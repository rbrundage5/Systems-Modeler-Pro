# C02.typed-feature.atomic-creation

Baseline: `bfdd50ea63b1bc1eec7445c2e52c94c087d538ee`. Independent main-targeted PR.
Allowed paths: `crates/model-core/src/model.rs`, new core regression file
`crates/model-core/tests/typed_feature_atomicity.rs`, and this record.

`Project::create_typed_feature` inserts a Property before checking its type and
final multiplicity. An error returns after insertion and leaves an orphan or
invalid Property in the caller's Project. This shared API is used throughout
authoring/import; requiring every caller to remember cleanup is not an atomic
construction contract.

Retain the existing diagnostics, initialization and validation path. On any
post-insertion failure, remove exactly the newly created element before returning
the original error. The exclusive mutable Project borrow prevents intermediate
publication. No full-project clone, unrelated cleanup, schema change or new index
is needed. Existing elements, relationships and IDs must remain byte-equivalent
at the semantic JSON-value level after rejected creation.

Acceptance: missing/incompatible types and late invalid multiplicity leave the
entire original model unchanged; successful Part/port creation retains expected
type, multiplicity and aggregation. Run native core tests/lint and integration CI.
Independent review remains outstanding; this is constructor integrity, not a
claim that all mutation APIs or UI workflows have been qualified.
