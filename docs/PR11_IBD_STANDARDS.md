# PR11 IBD standards baseline

PR11 treats OMG SysML 1.6 and inherited UML structured-classifier semantics as normative. `SysML Distilled` is used as the project reference for notation and modeler-facing interpretation. `modeler-proto` remains a workflow/product baseline only; it is not evidence that an IBD implementation is complete.

## Complementary BDD/IBD rule

A Block Definition Diagram and an Internal Block Diagram are complementary views of the same semantic structure.

- PartProperty and ReferenceProperty identities are defined on the Block and must be reused on both BDD and IBD presentations.
- BDD compartments expose structural properties, ports and features.
- An IBD is owned in repository/package context but has a Block/AssociationBlock semantic context whose internal structure is being shown.
- Creating an additional presentation must never duplicate the semantic PartProperty, ReferenceProperty, Port, Connector, ConnectorEnd or ItemFlow.
- Canvas containment is presentation only and must never silently change semantic ownership.

## IBD semantic cohort

### Structured properties

- PartProperty: internal composite property; solid-border rectangle on IBD.
- ReferenceProperty: external/non-composite property; dashed-border rectangle on IBD.
- Nested structural property paths use stable semantic property IDs.

### Ports

- ProxyPort
- FullPort
- nested ports on structured-property boundaries
- boundary ports owned by the IBD context Block
- ProxyPort conjugation retained as semantic state
- FullPort conjugation rejected
- port type compatibility validated in Rust

### Connectors

- Connector is a first-class UML/SysML Connector, not an Association alias.
- ConnectorEnds preserve stable role, port and property-path identities.
- Assembly Connector connects compatible internal roles/ports.
- Delegation Connector connects exactly one context-boundary port to a compatible internal role/port.
- Connector name is optional.
- Connector typing, where used, must resolve to a compatible Association/AssociationBlock rather than a free-text renderer label.

### Item Flow

- ItemFlow realizes an existing Connector.
- conveyed items are stable semantic classifier IDs, never free-text labels.
- arrow orientation is presentation; FlowProperty direction and conveyed semantics remain model data.

## Presentation and routing

- Ports attach to valid structural boundaries.
- Connector endpoints remain attached after reroute.
- routing is orthogonal and deterministic.
- blocks/parts/reference properties/ports/labels are obstacles.
- parallel connectors receive deterministic lanes.
- no routing implementation may silently fall back to a diagonal through a model element.
- Route and future Clean commands must call the same Rust routing foundation.

## BDD corrections included in PR11

Only standards-required structural corrections are in scope:

- BDD and IBD reuse the same PartProperty/ReferenceProperty/Port semantics.
- FullPort cannot be conjugated.
- Connector and ItemFlow are not exposed as generic BDD relationships.
- structural features remain owned properties shown in classifier compartments rather than duplicate classifier nodes.
- Block-to-IBD drill-down resolves by semantic Block context.

## Explicit exclusions

- No SysML v2 concepts.
- BindingConnector remains parametric semantics, not a normal IBD Connector.
- Full cross-cutting right-click/Symbol Properties architecture is deferred.
- Behavior/sequence/activity edge semantics are not routed through the Connector model.
