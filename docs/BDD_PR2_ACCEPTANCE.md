# PR #2 Acceptance Gates — BDD Semantic Foundation

PR #2 is complete only when all of the following are true.

## Semantic creation

- Packages can own BDD classifiers.
- Blocks, Interface Blocks, Value Types, Data Types, Enumerations, and Constraint Blocks are distinct semantic kinds.
- Blocks/compatible classifiers can own part, reference, value, and constraint properties, ports, operations, and receptions.
- Operations can own parameters.
- Enumerations can own literals.

## Type rules

- Part properties are typed by Blocks/Interface Blocks and are composite.
- Reference properties are non-composite and use compatible classifier/data types.
- Value properties are typed by Value Type/Data Type/Enumeration.
- Constraint properties are typed by Constraint Blocks.
- Ports and parameters retain stable semantic type references.

## Relationship rules

- Generalization is classifier-to-classifier and rejects cycles.
- Associations retain semantic association ends, including role, multiplicity, navigability, and aggregation.
- Composite/shared aggregation is held on association ends so notation is not the only source of composition meaning.
- Dependency and Realization remain first-class semantic relationships.

## Engineering data

- Multiplicity supports exact, ranged, and unbounded values.
- Value Types retain quantity-kind and unit identifiers.
- Properties retain default values and derived/read-only state.
- Applied stereotypes are retained independently from display labels.

## Notation contract

- Classifier stereotype labels follow SysML/UML conventions.
- Property labels use `name : Type [multiplicity]` with derived/default-value additions when present.
- Generalization, dependency, realization, association, composition, and shared aggregation have deterministic line/end-decoration metadata.
- Feature-to-compartment mapping is deterministic.

## Integrity

- Duplicate external IDs are rejected.
- Invalid ownership and typing are rejected.
- Referenced elements cannot be deleted silently.
- Inherited features can be queried across generalization chains.
- BDD semantic data round-trips through SQLite without loss.

## Deliberately deferred

The following are not blockers for PR #2 because they belong to subsequent vertical slices:

- interactive BDD canvas editing
- persisted diagram-node geometry and compartment visibility
- project file/open/save UI
- complete profile/stereotype definition engine
- IBD connector/item-flow semantics
- requirements and behavior semantics
- routing/layout

Those later features must consume this semantic foundation rather than creating duplicate UI-owned model data.
