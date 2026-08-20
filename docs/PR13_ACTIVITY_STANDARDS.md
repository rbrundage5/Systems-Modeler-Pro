# PR13 — Activity Modeling Standards Baseline

## Purpose

PR #13 completes the next native behavioral-modeling slice after merged PR #12 by implementing SysML/UML Activity modeling behind the Rust semantic boundary. The target is CATIA Magic/Cameo-class modeling depth without copying the legacy browser modeler's architecture.

## Normative baseline

- OMG SysML 1.x Activity modeling semantics and notation.
- OMG UML 2.5.1 Activity semantics inherited by SysML 1.x.
- Repository architecture and roadmap requirements are binding implementation constraints.
- CATIA Magic/Cameo workflows are a professional UX reference only where they do not conflict with the standards.
- `modeler-proto` is a compatibility/workflow reference only and is not the completeness authority.

## Architecture rules

1. Rust owns Activity semantic identity, ownership, typing, validation, references, ordering, deletion rules, and persistence.
2. SQLite remains the authoritative local repository.
3. JavaScript remains presentation/input glue and may not become an Activity semantic engine.
4. Activity diagrams are presentations of semantic content and are never the semantic source of truth.
5. Stable semantic IDs must survive rename, movement, reparenting, diagram recreation, save/reopen, and later import/collaboration operations.
6. Existing Operations, Signals, classifiers, properties, and other model elements must be referenced by stable semantic ID rather than copied into Activity-local strings.
7. Shared routing/presentation infrastructure must be reused rather than creating an Activity-only router or storage path.
8. No Activity capability is complete without semantic, persistence, validation, and desktop interaction qualification appropriate to that capability.

## Required semantic scope

### Activity and parameters

- Activity with stable identity, external ID, name, documentation, and classifier context where applicable.
- Activity parameters with direction, type, multiplicity, ordering, and uniqueness semantics.
- ActivityParameterNode presentations referencing the Activity parameter semantic identity.

### Executable nodes

- OpaqueAction
- CallBehaviorAction
- CallOperationAction
- SendSignalAction
- AcceptEventAction
- AcceptTimeEventAction

CallBehaviorAction must reference another Activity/Behavior by stable semantic ID. CallOperationAction must reference an existing Operation by stable semantic ID. Send/Accept Signal actions must reference an existing Signal where required.

### Control nodes

- InitialNode
- ActivityFinalNode
- FlowFinalNode
- DecisionNode
- MergeNode
- ForkNode
- JoinNode

Topology and guard/join rules are validated in Rust, not inferred from rendered geometry.

### Object nodes and pins

- InputPin
- OutputPin
- ValuePin
- ObjectNode
- CentralBufferNode
- DataStoreNode

Pins are real semantic objects with stable identity, owner, type, multiplicity, direction/role, ordering, uniqueness, and connection eligibility. They are not decorative sub-shapes.

CallOperationAction pins must remain semantically compatible with the referenced Operation Parameters so operation rename/refactoring does not break the Activity model.

### Activity edges

- ControlFlow
- ObjectFlow

Edges have stable semantic identity independent of diagram route geometry. Support the standards-defined properties required for professional modeling, including source, target, guard, weight, object typing, and selection/transformation semantics where applicable.

ObjectFlow validation must reject or diagnose incompatible typed endpoints rather than accepting any graphical connection.

### Structured behavior

- ActivityPartition
- StructuredActivityNode
- ConditionalNode
- LoopNode
- SequenceNode
- ExpansionRegion
- InterruptibleActivityRegion

Membership/containment must be semantic and persisted independently from diagram coordinates.

## Diagram and interaction requirements

An Activity Diagram has independent presentation identity and geometry while referencing one semantic Activity context. The Rust workspace must expose the legal Activity palette and typed commands for creation/editing. The frontend may render symbols and collect pointer/keyboard input but may not invent legal node/edge kinds or mutate semantic storage directly.

Repository drag/drop must place or reference existing semantic content where appropriate rather than duplicating it. CallBehaviorAction should support drill-down to the referenced Activity/diagram when one exists.

## Routing requirements

Activity ControlFlows and ObjectFlows must use the shared Rust routing direction established for structural diagrams. Routes must avoid actions, control nodes, pins, partition headers, structured nodes, and other semantic presentations. Parallel flows must remain visually distinguishable. No JavaScript-only Activity router may be introduced.

## Validation baseline

Rust validation must cover at least:

- Activity/context integrity.
- legal node ownership and structured-node containment.
- legal ControlFlow/ObjectFlow endpoints.
- Initial/Final/FlowFinal topology.
- Decision/Merge rules.
- Fork/Join rules and join specification integrity where supported.
- pin ownership, direction, typing, and multiplicity.
- ObjectFlow type compatibility.
- referenced Operation/Activity/Signal integrity.
- partition/region membership.
- interrupting-edge integrity.
- safe deletion/reference protection.

## Persistence baseline

Create, edit, connect, move, save, close, and reopen must preserve semantic IDs, owners, references, pins, edge IDs/properties, partitions/regions, presentation IDs, geometry, and routes exactly enough for deterministic continued editing.

## Explicitly deferred from PR13

- full Activity execution/token simulation engine.
- Requirements/traceability metamodel.
- Use Case/Actor completion.
- Parametric solver/execution.
- verification execution/evidence framework.
- workbook import/reimport.
- collaboration/branching/version control.
- final cross-diagram Symbol Properties/style framework.
- global professional Clean Layout completion beyond Activity reuse of the shared routing foundation.
