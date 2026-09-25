# C15.04.01 — typed connector runtime link cardinality

Depends on C04.09.02a–c. Baseline: connector authoring candidate
`a70785f95f90576e1b1175473d0dd7229f5cf447`. Primary implementation only.

The structural runtime currently expands every resolved source/target pair. New
authored connector-end multiplicities must not be silently ignored by that policy.
For Association-typed connectors, or explicitly nondefault end multiplicities,
check the number of resolved endpoints against the corresponding ordered end
before publishing runtime links. Under all-to-all realization this is the number
of links at that end for each opposite endpoint. An incompatible configuration
is a runtime realization error; do not mark a valid reusable type model invalid
or invent an arbitrary pairing. Optional absent ends can produce zero links when
their multiplicity permits it. Preserve legacy untyped/default realization.

Allowed production path: crates/model-core/src/structural_runtime.rs. Tests:
crates/model-core/tests/association_typed_connectors.rs. Reference: supplied UML
2.5.1 §§11.8.10–11.8.11. No new runtime engine, schema or JavaScript semantics.

Acceptance: four sources to one target with compatible 4/1 end constraints gives
four links; incompatible 1/1 fails without authored mutation; repeated build
gives stable identities; optional empty source yields no fabricated connection.
General link allocation/pairing and AssociationBlock internal instances remain
separate capabilities. CI and independent review must qualify this candidate.
