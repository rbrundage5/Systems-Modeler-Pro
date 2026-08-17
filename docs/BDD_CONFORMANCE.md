# SysML Block Definition Diagram (BDD) Conformance

This document defines the PR #2 BDD semantic and notation contract for Systems Modeler Pro.

## Scope

A BDD is a presentation of reusable semantic definitions. Diagram nodes never become the source of truth for Blocks, properties, ports, value types, data types, constraints, operations, requirements, or relationships.

## Semantic elements

The BDD foundation supports:

- Block
- Interface Block
- Value Type
- Data Type
- Enumeration and Enumeration Literal
- Constraint Block
- Part Property
- Reference Property
- Value Property
- Constraint Property
- Proxy Port
- Full Port
- Operation and Parameter
- Reception
- Association and Association End
- Generalization
- Dependency
- Realization
- Composition and shared aggregation through association-end aggregation

## Property semantics

Properties are owned features of a classifier and must carry a stable type reference when typed. Multiplicity is represented by lower and upper bounds where `*` is unbounded. Part properties require composite aggregation. Reference properties require non-composite aggregation. Value properties are typed by a value/data/primitive/enumeration-compatible classifier. Constraint properties are typed by a Constraint Block.

## Port semantics

Proxy Ports expose features of the owning Block through an interface/type without adding a separate internal behavioral object. Full Ports represent interaction points with their own identity and may be typed by an Interface Block or other compatible classifier. Port conjugation is preserved as semantic state and is not encoded only in notation.

## Associations and ends

Associations own two or more ends. Each end can carry role name, multiplicity, navigability, aggregation, and classifier endpoint. Composition is represented by `composite` aggregation on the whole-side end, not as an unrelated visual-only edge kind.

## Generalization

Generalization connects a specific classifier to a general classifier. Cyclic inheritance is invalid. The engine must support inherited features and deterministic lookup before BDD completion is claimed.

## Value types, quantities, and units

Value Types may reference a quantity kind and unit by stable external identifier. Unit and quantity metadata are semantic data so parametric and analysis features can reuse them later. Default property values are model data, not diagram text.

## Stereotypes

Elements may carry applied stereotypes identified independently of their displayed stereotype labels. This PR provides the semantic storage surface; profile-definition and stereotype-property execution may be expanded in later profile work.

## BDD notation contract

Notation follows SysML/UML conventions rather than arbitrary application icons.

- Block: classifier rectangle with `«block»` above the name.
- Interface Block: classifier rectangle with `«interfaceBlock»` above the name.
- Value Type: classifier rectangle with `«valueType»` above the name.
- Constraint Block: classifier rectangle with `«constraint»` above the name.
- Enumeration: classifier rectangle with `«enumeration»` and a literals compartment.
- Part/reference/value/constraint properties appear in appropriate compartments using `name : Type [multiplicity]` form.
- Operations use operation signature notation.
- Ports are boundary presentations and reference semantic ports; they are not free-standing duplicate model elements.
- Generalization is a solid line with a hollow triangular arrowhead toward the general classifier.
- Association is a solid line. Navigability, role names, and multiplicities are presentation labels derived from association ends.
- Composition uses a filled diamond at the composite/whole end; shared aggregation uses a hollow diamond.
- Dependency uses a dashed line with an open arrow toward the supplier.
- Realization uses a dashed line with a hollow triangular arrowhead toward the realized classifier.

## Compartments

Supported classifier compartments include parts, references, values, constraints, operations, receptions, ports, and enumeration literals where applicable. Compartment visibility is presentation state; compartment content is derived from owned semantic features.

## Cross-diagram reuse

The same semantic identifiers are reused by IBD, parametric, activity, sequence, state-machine, requirements, use-case, package, and traceability presentations. Creating another presentation must never clone the semantic element.

## Validation gates

The BDD semantic foundation must reject at least:

- missing owners
- invalid feature owners
- unresolved classifier/type references
- invalid multiplicities
- invalid property-kind/type combinations
- invalid constraint-property typing
- association ends whose classifiers do not exist
- generalization cycles
- duplicate external IDs within a project
- deleting an element that still owns semantic children

A capability is not considered complete until semantic, persistence, and UI/presentation qualification exists for that capability.
