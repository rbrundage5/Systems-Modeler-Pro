# PR8 BDD standards baseline

PR8 treats OMG SysML 1.6 as the normative systems-modeling baseline and UML 2.5.1 as the normative source for inherited UML semantics. `modeler-proto` remains a product-behavior baseline, not the authority for completeness.

## Migration rule

1. Preserve standards-compatible behavior already present in `modeler-proto`.
2. Implement semantic identity, ownership, typing, multiplicity, validation, persistence, and BDD eligibility in Rust.
3. Keep JavaScript limited to presentation and collection of user input.
4. Do not expose palette tools that cannot be executed by the Rust engine.
5. Represent owned features in classifier compartments instead of creating duplicate standalone semantic classifiers.

## PR8 BDD semantic families

### Classifiers presented as BDD nodes
- Block
- InterfaceBlock
- ValueType
- DataType
- Enumeration
- ConstraintBlock

### Owned features represented in classifier compartments
- PartProperty
- ReferenceProperty
- ValueProperty
- ConstraintProperty
- ProxyPort
- FullPort
- Operation
- Reception
- Parameter
- EnumerationLiteral

### Relationships currently migrated
- Association
- shared aggregation through Association end aggregation
- composite aggregation through Association end aggregation
- Generalization
- Dependency
- Realization

## Required behavior

- stable semantic IDs are authoritative and persisted
- ownership rules are enforced in Rust
- typed features require compatible stable type references
- multiplicity is validated in Rust
- PartProperty is composite
- ReferenceProperty cannot be composite
- EnumerationLiteral is owned by Enumeration
- Parameter is owned by Operation
- BDD node presentation reuses an existing semantic classifier rather than duplicating it
- Save/Open validates both semantic and presentation data
- Generalization cycle prevention remains enforced by the Rust core

Further SysML/UML BDD families that are not yet represented by the current Rust `ElementKind` enum must be added to the core before PR8 is marked complete; the product baseline alone is not sufficient evidence of completeness.
