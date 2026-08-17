# BDD Modeling Rules for Systems Modeler Pro

These rules are the tool-facing interpretation used by the Rust BDD foundation.

## BDD purpose

A Block Definition Diagram presents reusable definitions and their relationships. A BDD does not represent one runtime occurrence of a system. Blocks and other classifiers may be presented on multiple diagrams while retaining one semantic identity.

## Blocks and classifiers

A Block defines structural and behavioral features that instances may possess. Interface Blocks define reusable interaction/interface structure. Value Types define engineering values and may reference unit and quantity-kind identities. Data Types define structured non-Block data. Enumerations define a closed set of literals. Constraint Blocks define reusable constraints for later parametric binding.

## Features

A part property represents composite structure and therefore carries composite aggregation. A reference property references a separately owned object and cannot silently become composition. A value property represents a value/data feature. A constraint property is typed by a Constraint Block. Ports are owned features and are presented on a classifier boundary when shown graphically.

## Multiplicity

Multiplicity is semantic data, not free-form diagram text. The engine stores lower and upper bounds; no upper bound means unbounded. Invalid bounds are rejected.

## Association

Association meaning is stored using association ends. Each end can contain a role name, classifier, multiplicity, navigability and aggregation. A filled diamond is derived from composite aggregation. A hollow diamond is derived from shared aggregation. The visual diamond does not create the semantics by itself.

## Generalization and inheritance

The source/specific classifier inherits from the target/general classifier. Inheritance cycles are invalid. Inherited features must be queryable so later diagrams and property inspectors can show effective structure without cloning inherited features.

## Dependency and realization

Dependency is a directed client-to-supplier semantic relationship. Realization is a directed realization relationship using UML realization notation. They remain independent of diagram geometry.

## Units and values

Value-type quantity kind and unit references are stored by stable identifier. A default value belongs to the property definition. Future parametric and verification capabilities will reuse this semantic data.

## Compartments

Classifier compartments are views of owned semantic features. Hiding a compartment never deletes the underlying feature. Moving a feature between visual compartments is not permitted unless its semantic kind changes through a valid model command.

## Cross-diagram use

BDD definitions are reused by other diagram families. IBD parts reference Block definitions; sequence lifelines and activity object types may reference classifiers; state machines may be owned by classifiers; parametrics reuse value and constraint definitions; requirements link to the same elements through traceability relationships.

## Notation

The UI must render notation from semantic kind and relationship/end metadata. It must not infer SysML semantics from color, icon shape, SVG path, label placement or arbitrary frontend-only flags.
