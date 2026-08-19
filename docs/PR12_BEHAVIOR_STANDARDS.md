# PR12 State Machine and Sequence standards baseline

PR12 implements behavior semantics as native Rust model data. The frontend may render and invoke commands, but it must not become the authoritative source of behavioral semantics.

## Governing sources

1. OMG UML 2.5.1 semantics inherited by SysML 1.x for State Machines and Interactions.
2. SysML 1.x ownership/context rules and the project governance/roadmap.
3. CATIA Magic / Cameo behavior and notation workflows as the professional UX baseline where they do not conflict with OMG semantics.
4. `modeler-proto` only as a migration/workflow reference; legacy behavior is not evidence of completeness.

## State Machine scope

- StateMachine owned in model/package repository context and associated with a classifier context.
- Regions, composite states and orthogonal regions.
- State entry / do / exit behaviors.
- Initial, Final, Choice, Junction, Fork, Join, Shallow History, Deep History, Entry Point, Exit Point and Terminate pseudostates.
- External, Internal and Local transitions.
- Signal, Call, Time, Change and AnyReceive events/triggers.
- Guards and transition effects.
- Correct transition label notation: `trigger [guard] / effect` with absent portions omitted.
- Initial transitions are triggerless and guardless.
- Initial pseudostates have no incoming transition and exactly one outgoing transition.
- Final states have no outgoing transition.
- Fork/Join cardinality is validated in Rust.
- Semantic validation must not depend on diagram geometry.

## Sequence / Interaction scope

- Interaction with a classifier context.
- Lifelines represent semantic properties of the context, including nested stable property paths.
- Message occurrence ordering is semantic and independent of pixel coordinates.
- Synchronous call, asynchronous call, asynchronous signal, Reply, Create, Delete, Lost and Found messages.
- Call messages reference Operations and carry argument values.
- Signal messages reference Signals.
- ExecutionSpecifications with valid start/finish occurrences on one lifeline.
- CombinedFragments: alt, opt, loop, break, par, critical, neg, assert, strict, seq, ignore and consider.
- Interaction operands with guards and valid occurrence ranges.
- StateInvariants on lifelines.
- Found/Lost message endpoint rules are enforced in Rust.

## CATIA/Cameo workflow parity goals

- Diagram palettes expose only elements legal for the active behavior diagram.
- Selecting a transition/message exposes its semantic specification, not a diagram-local text surrogate.
- Lifelines may represent nested properties and display their path using dot notation.
- Call Message specification supports selecting an Operation signature and arguments.
- Transition specification supports event/trigger, guard and effect independently.
- Composite/orthogonal state editing must preserve real Region ownership.
- Messages remain attached to lifelines/executions when presentations move.
- Ordering cannot silently change because a user drags a label or resizes a symbol.

## Architecture requirements

- Rust owns validation, creation, reconnection, ordering and persistence.
- SQLite remains the local authoritative repository.
- JavaScript is intentionally minimal: rendering, pointer interaction and Tauri command invocation only.
- No JavaScript-only semantic object, transition rule, message rule or occurrence-order algorithm is acceptable.
- Saved models must round-trip without regenerating behavior IDs.
- State Machine and Sequence presentations must remain separate from the semantic model.

## Explicitly deferred

- Full simulation/execution engine.
- Full cross-cutting context-menu / Symbol Properties framework.
- Activity behavior and Use Case behavior unless a narrowly required shared semantic foundation must be introduced.
