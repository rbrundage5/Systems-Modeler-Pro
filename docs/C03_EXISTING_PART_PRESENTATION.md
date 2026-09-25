# Show an existing typed part as a BDD composition

Leaf C03.REUSE.01. Baseline main:
`2d1f51b0faef8abb840d7c848adca2f458bb92cc`.

The BDD relationship tool always creates a new usage. A preexisting PartProperty
could be selected/edited but had no command to expose its own composition, forcing
users to make another part to get a line. This leaf adds an explicit alternative:
select the existing property in the repository while a BDD is active, then choose
**Show composition on BDD**. Apply any pending property changes first.

Rust resolves identity by property ID, reuses its linked Association if present,
and otherwise creates one through the existing core Property-association authority.
Missing Block symbols are placed in the selected BDD. Block and part counts remain
unchanged. Repeated requests are no-ops; another BDD reuses the same relationship.
Two named parts of the same Block type remain different usages with parallel edges.
No name matching, implicit merging, new schema or automatic legacy conversion.

The native authored transaction guards all repositories, stages/routs/validates
before publication and records one checkpoint only for a real change. Existing
edit_authored callers retain their original behavior; the new conditional helper
adds explicit no-op support for this workflow. Rejection preserves model/views,
identity and redo. The frontend prevents display refresh from discarding a draft.

Scope: property_presentation.rs, the existing presentation_interaction geometry
validator's visibility, command registration in main.rs, history helper,
bdd-feature-editing.js, native history label, focused tests, and this record.
References: supplied SysML 1.6 section 8.1 and section 8.2.1 (Block definitions and
association-end property notation), existing core create_property_association.

Five new frontend tests pass (16 in the Properties suite), including a draft
edited during pending IPC and prevention of a concurrent Apply. Native tests cover
shared typing, two named usages, repeated display, cross-view relationship reuse,
project database round trip, undo/redo, rejection, invalid geometry and lock failure.
Native CI exposed that the low-level router accepted a negative-width source.
This command now reuses the existing presentation geometry validator before
staging the relationship. The failing rollback test is retained unchanged.
Native CI is required; the local runtime has no Rust toolchain. Independent review
and rendered Windows acceptance remain outstanding.

Self-typed part routing remains explicitly unsupported by this new command;
ReferenceProperty presentation, usage-tree notation, and new semantic kinds are
separate leaves. Other unverified capabilities remain in the completion register.
No automatic merge, deployment or whole-tool completeness claim.
