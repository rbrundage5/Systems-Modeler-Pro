# REL-GEN-02 / C03 Block specialization invariant

Baseline: `596e24f1f41ccfdcf1bb4c0b4a679858aaaa0a53`.
Authorized direct primary-session work; registered workers disabled.
Production scope: `crates/model-core/src/model.rs`; regression scope:
`crates/model-core/tests/block_specialization.rs`.

Finding: generalization accepted any two classifiers except the existing Actor
and UseCase restrictions. A Signal or DataType could therefore specialize a Block.
Supplied SysML 1.6 section 8.3.2.4 constraint 8 requires the specialization to
have Block or a specialization of that stereotype applied.

Both native creation and whole-project validation now enforce this invariant.
Built-in Block, AssociationBlock, InterfaceBlock and ConstraintBlock count as
Block specializations. Plain strings in applied_stereotypes do not promote an
arbitrary classifier kind. This does not implement redefinition/subsetting or
additional constraints specific to individual Block specializations.

Acceptance: the Block-kind matrix is accepted by this constraint; seven other
classifier kinds cannot specialize any Block kind. Failed creation preserves the
serialized project exactly. Invalid deserialized/reconnected endpoints are
rejected by the same project validation used before Open/import publication.
Ordinary Signal-to-Signal generalization remains valid.

Verification: hosted CI required; no local Rust compiler is installed.
Independent review and full rendered authoring acceptance remain outstanding.
