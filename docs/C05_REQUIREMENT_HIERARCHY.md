# C05.HIERARCHY.01 — nested Requirement ownership

Baseline main: `2d1f51b0faef8abb840d7c848adca2f458bb92cc`.
Source: supplied SysML 1.6, section 16.3.2.5. Requirements may contain nested
Requirements; containment is distinct from DeriveReqt. Deleting a compound
Requirement retires its nested Requirements through the normal ownership tree.

This leaf enables Requirement-under-Requirement ownership in the existing core
authority. It reuses stable Element IDs, owner_id, qualified-name traversal,
cycle checks and connected deletion. It introduces no alternative tree or schema.
Other element kinds do not become legal children merely because a Requirement is
a classifier. Diagram ownership remains with its package.

Allowed changes: model.rs ownership validation, requirement_hierarchy.rs core
regressions, requirements_persistence.rs regressions and this record.

Acceptance covers three levels, rename/reparent with stable identity, invalid
parent kinds and cycles without mutation, duplicate-ID creation rollback,
recursive deletion with surviving design elements, SQLite and JSON round trips.
Existing flat projects retain their existing representation. Native checks use
the repository's CI; this workspace has no Rust toolchain. Independent review and
installed-desktop acceptance remain outstanding.

## Dependency-ordered workflow completion

1. This core containment/compatibility leaf.
2. Native transactional nested creation and user-facing parent selection, including
   one history entry, invalid-ID/owner rollback, repository navigation and package
   diagram ownership. Existing shared reparent/delete paths must retain identities.
3. Recursive Copy with explicit stable correspondence for each nested Requirement:
   stage the entire subtree and Copy links, preserve local Requirement IDs, enforce
   supplier-controlled text, reject cycles/conflicts before mutation, and persist
   the correspondence. Define supplier add/move/delete synchronization before
   claiming a continuously synchronized copied tree. No name-based inference.
4. Existing-tree correspondence, safe import/reimport and adapters; qualification
   of hierarchy notation and requirement-to-design-to-test navigation.

The parent TC-09 remains open until those dependent workflows are implemented and
qualified. This leaf does not claim recursive Copy, diagram nesting notation,
requirement matrices/evidence, or external-vendor interoperability.
