# PR13 — Activity Modeling Implementation Checklist

PR13 is not complete until every required item below is implemented and qualified on the PR branch.

## Semantic core

- [ ] Add ActivityRepository/Activity semantic ownership behind the Rust model boundary.
- [ ] Add stable typed IDs for Activity, nodes, edges, pins, partitions, and structured regions.
- [ ] Add Activity parameters and ActivityParameterNodes.
- [ ] Add Initial, ActivityFinal, FlowFinal, Decision, Merge, Fork, and Join nodes.
- [ ] Add OpaqueAction, CallBehaviorAction, CallOperationAction, SendSignalAction, AcceptEventAction, and AcceptTimeEventAction.
- [ ] Add InputPin, OutputPin, ValuePin, ObjectNode, CentralBufferNode, and DataStoreNode.
- [ ] Add ControlFlow and ObjectFlow semantics.
- [ ] Add ActivityPartition, StructuredActivityNode, ConditionalNode, LoopNode, SequenceNode, ExpansionRegion, and InterruptibleActivityRegion.

## References and typing

- [ ] CallOperationAction references an existing Operation by stable ElementId.
- [ ] CallBehaviorAction references an existing Activity/Behavior by stable ID.
- [ ] Signal actions reference an existing Signal by stable ElementId.
- [ ] Pins retain semantic typing/multiplicity and validate against their owning action/reference.
- [ ] ObjectFlow validates endpoint/type compatibility.

## Validation

- [ ] Activity context/ownership validation.
- [ ] ControlFlow endpoint validation.
- [ ] ObjectFlow endpoint/type validation.
- [ ] Initial/Final/FlowFinal topology validation.
- [ ] Decision/Merge validation.
- [ ] Fork/Join validation.
- [ ] Guard/weight validation.
- [ ] pin ownership/direction/type validation.
- [ ] structured-node/region membership validation.
- [ ] interrupting-edge validation.
- [ ] referenced Operation/Activity/Signal deletion protection.

## Persistence

- [ ] SQLite round-trip preserves Activity semantic IDs and references.
- [ ] Activity presentation metadata is persisted independently from semantics.
- [ ] save/reopen preserves node/edge IDs, pins, guards, types, partitions, geometry, and routes.
- [ ] legacy projects without Activity metadata reopen without migration failure.

## Desktop workspace

- [ ] Activity Diagram can be created with a semantic Activity context.
- [ ] Rust-owned diagram palette exposes only legal Activity tools.
- [ ] click/drop creation invokes typed Rust commands.
- [ ] repository placement does not duplicate semantic elements.
- [ ] properties editing invokes Rust commands and refreshes authoritative snapshots.
- [ ] CallBehaviorAction supports semantic drill-down to referenced Activity where available.
- [ ] selection/delete/reconnect behavior remains Rust authoritative.

## Presentation and routing

- [ ] render standard action/control/object/pin notation.
- [ ] render Activity partitions/swimlanes.
- [ ] render ControlFlow/ObjectFlow distinctions and labels.
- [ ] reuse the shared Rust obstacle-safe router.
- [ ] flows do not pass behind/through actions, pins, structured nodes, or partition headers.
- [ ] parallel flows use deterministic separate lanes.

## Qualification

- [ ] focused Activity model-core tests.
- [ ] persistence round-trip tests.
- [ ] desktop command/integration tests.
- [ ] frontend syntax/integration contract updated for Activity files.
- [ ] `cargo fmt --all --check` passes.
- [ ] non-desktop workspace tests pass.
- [ ] non-desktop Clippy passes with warnings denied.
- [ ] Windows desktop `cargo check` passes.
- [ ] Windows desktop tests pass.
- [ ] Windows desktop Clippy passes with warnings denied.
- [ ] `Cargo.lock` remains clean after CI checks.
- [ ] focused manual desktop Activity create/edit/connect/save/reopen checks completed before readiness is claimed.
