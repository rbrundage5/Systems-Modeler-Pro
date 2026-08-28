# PR34 Operation, Signal, and Sequence Runtime

PR34 extends the PR31–PR33 Rust execution session. It does not introduce a
second simulator, event queue, clock, value store, or frontend semantic store.

## Qualified semantics

- Operations execute on an exact `RuntimeInstanceId` after validating the
  Operation owner, runtime classifier conformance, named inputs, multiplicity,
  parameter direction, and runtime value type.
- `InOut` parameters deterministically echo their input. Authored default
  values provide bounded `Out` and `Return` results. Operation body text and
  arbitrary scripts are never executed.
- `CallOperationAction` invokes the modeled Operation runtime. The existing
  test-only/custom `OperationCallRuntime` seam remains available.
- `SendSignalAction` uses PR33 Port/Connector/ItemFlow/Reception routing when a
  structural runtime occurrence exists. The current Activity metamodel has no
  source-Port field, so execution requires exactly one compatible source Port.
- Receptions are explicitly typed by modeled Signals. New Reception creation
  requires an accepted Signal, and the Properties editor can assign or repair
  the accepted Signal on an existing Reception; Rust rejects non-Signal types.
- `AcceptEventAction` keeps typed Signal matching and exact target occurrence
  filtering. PR32 State Machine Signal triggers continue to consume the same
  `RuntimeEvent` queue.
- Sequence Lifelines resolve their represented structural property paths to
  exact runtime occurrences. Zero or multiple matches are rejected.
- Sequence message order comes from authored occurrence order, never pixels or
  route geometry. Synchronous/asynchronous Operation messages invoke modeled
  Operations; asynchronous Signal messages use the exact structural route to
  the represented target occurrence; Reply messages record completion.
- A Sequence participant with one modeled State Machine is initialized as an
  embedded engine on the same execution session. Operation CallEvents and
  SignalEvents are dispatched through the shared queue to the exact target
  occurrence, preserving PR32 trigger behavior. Direct embedded-engine dispatch
  applies the same semantic/runtime-instance address filter as normal queued
  State Machine execution, so another occurrence of the same classifier cannot
  consume an event addressed to its peer.
- Sequence controls are contextual and presentation-only. Runtime highlighting
  does not mutate Lifeline/message geometry, authored semantics, viewport, or
  routing.

## Explicit PARTIAL / UNSUPPORTED boundaries

- Required Out/Return parameters without an authored bounded default are
  rejected because the current metamodel has no safe executable Operation body.
- Activity SendSignalAction has no explicit source Port or target occurrence in
  the current metamodel; ambiguous structural source routes are unsupported.
- Sequence Create, Delete, Found, and Lost message execution is unsupported in
  PR34. Authoring remains available and unchanged.
- Multiple State Machines on one Sequence participant are rejected as
  ambiguous. The current metamodel has no participant-level behavior-selection
  reference.
- Sequence combined-fragment control execution is not inferred from diagram
  geometry. Existing fragment authoring remains intact.
- Arbitrary JavaScript, scripts, implementation-language bodies, debugger
  behavior, Parametric solving, and PR35 kernel work are out of scope.

## Qualification

`crates/model-core/tests/pr34_operation_signal_sequence.rs` covers occurrence
isolation, Operation parameter/target validation, deterministic returns,
structural Signal targeting, Reception Signal typing, semantic Lifeline binding,
Operation and Signal Sequence messages, exact target-instance State Machine
dispatch, deterministic ordering/reset, and authored-model immutability.

`scripts/validate_operation_signal_sequence_integration.py` is executed by both
Linux/core and Windows/desktop CI alongside the existing PR31–PR33 and
all-nine-family standard editing contracts.
