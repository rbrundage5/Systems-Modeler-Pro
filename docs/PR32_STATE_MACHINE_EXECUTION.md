# PR32 Native State Machine Execution

PR32 adds a Rust-authoritative State Machine execution engine on the shared
runtime foundation introduced by PR29 and used by PR31 Activity execution.
Authored State Machine semantics and diagram geometry remain separate from all
transient runtime state.

## Architecture

```text
BehaviorRepository / StateMachine
  -> StateMachineExecutionEngine
  -> shared ExecutionSession / ExecutionManager
  -> shared event queue and SimulationTime
  -> StateMachineExecutionSnapshot
  -> thin Tauri commands
  -> presentation-only runtime controls and overlays
```

The frontend never selects transitions, evaluates guards/effects, processes
events, or advances simulation time. It only invokes commands and renders Rust
snapshots. Runtime overlays are input-transparent and do not replace the shared
workspace surface.

## Implemented and qualified

- Shared execution lifecycle: Initialize, Run, Step, Pause, Resume, Reset, and
  Terminate.
- Deterministic signal, call, time, change, completion, and any-receive trigger
  matching where the authored model provides the required semantic data.
- Bounded run-to-completion traversal with deterministic transition priority and
  explicit ambiguity diagnostics instead of container/hash-order selection.
- Shared bounded expression evaluation for guards and pure transition effects.
- External, local, and internal transition handling.
- Active composite ancestry, nested regions, simultaneous orthogonal-region
  configuration, cross-hierarchy exits, and nested-target entry.
- Initial, Choice, Junction, Fork, Join, Terminate, and FinalState execution.
- Executable Submachine States using the same ExecutionSession, event queue,
  SimulationTime, runtime values, and deterministic trace as the parent machine.
- Shared SimulationTime scheduling for relative and absolute TimeEvents; no
  browser or wall-clock timing. Activation generations prevent an exited
  state's stale TimeEvent from firing after re-entry.
- Authored-model fingerprint invalidation, reset/reinitialization, transient
  runtime isolation, deterministic traces, and bounded failure diagnostics.
- Runtime visualization for active/waiting/final states, active regions,
  enabled/last-fired transitions, events, time, status, diagnostics, and trace.
- State Properties can select modeled Activities for entry, doActivity, and
  exit by stable Activity ID; the frontend does not interpret those references.

## Partial or explicitly unsupported

- State entry, doActivity, and exit now author stable Activity IDs, but the
  State Machine execution engine does not yet invoke the PR31
  ActivityExecutionEngine for those references. Until that bridge is qualified,
  execution emits a diagnostic and never interprets arbitrary model text.
- EntryPoint and ExitPoint vertices are authorable, but the current metamodel
  does not identify a qualified connection-point owner and entry/exit mapping.
  Reaching either is therefore rejected with an engineer-readable diagnostic.
- ShallowHistory and DeepHistory vertices exist, but the authored metamodel has
  no qualified default/history restoration policy. Reaching either produces a
  bounded engineer-readable diagnostic instead of approximate behavior.
- An obsolete TimeEvent is prevented from firing by activation-generation
  matching, but the obsolete queue record is not yet purged. Repeated exit and
  re-entry can therefore accumulate inert timer records and must be closed
  before PR32 is considered runtime-complete.
- Full structural runtime instances, ports, connectors, distributed targets,
  Parametric solving, and Sequence execution remain outside PR32 and belong to
  later work.

## Qualification

- `crates/model-core/tests/pr32_state_machine_execution.rs`
- `crates/model-core/tests/pr32_state_machine_event_semantics.rs`
- `crates/model-core/tests/pr32_state_machine_semantic_closure.rs`
- `scripts/validate_state_machine_execution.py`
- Existing PR31 Activity, behavior, shared workspace, routing/frame,
  presentation interaction, Rust-authority, and all-nine-family standard
  authoring contracts remain required in CI on Linux and Windows.
