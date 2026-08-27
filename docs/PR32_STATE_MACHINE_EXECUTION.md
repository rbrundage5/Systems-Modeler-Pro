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
- Bounded run-to-completion traversal with deterministic transition priority.
- Shared bounded expression evaluation for guards and pure transition effects.
- External, local, and internal transition handling.
- State entry/exit ordering, active composite ancestry, nested regions, and
  simultaneous orthogonal-region configuration.
- Initial, Choice, Junction, Fork, Join, EntryPoint, ExitPoint, Terminate, and
  FinalState traversal.
- Shared SimulationTime scheduling for relative and absolute TimeEvents; no
  browser or wall-clock timing.
- Authored-model fingerprint invalidation, reset/reinitialization, transient
  runtime isolation, deterministic traces, and bounded failure diagnostics.
- Runtime visualization for active/waiting/final states, active regions,
  enabled/last-fired transitions, events, time, status, diagnostics, and trace.

## Partial or explicitly unsupported

- `State.entry`, `State.exit`, and `State.do_activity` are currently stored as
  plain text. They are not stable Behavior/Activity references. PR32 preserves
  authoring and emits a diagnostic instead of interpreting arbitrary text or
  guessing an Activity by name. Consequently, typed PR31 Activity call-frame
  integration remains partial until the metamodel adds stable references.
- ShallowHistory and DeepHistory vertices exist, but the authored metamodel has
  no qualified default/history restoration policy. Reaching either produces a
  bounded engineer-readable diagnostic instead of approximate behavior.
- Full structural runtime instances, ports, connectors, distributed targets,
  Parametric solving, and Sequence execution remain outside PR32 and belong to
  later work.

## Qualification

- `crates/model-core/tests/pr32_state_machine_execution.rs`
- `scripts/validate_state_machine_execution.py`
- Existing PR31 Activity, behavior, shared workspace, routing/frame,
  presentation interaction, Rust-authority, and all-nine-family standard
  authoring contracts remain required in CI on Linux and Windows.
