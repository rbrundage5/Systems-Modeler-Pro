# PR32 Native State Machine Execution

PR32 adds a Rust-authoritative State Machine execution engine on the shared
runtime foundation introduced by PR29 and reused by PR31 Activity execution.
Authored State Machine semantics and diagram geometry remain separate from all
transient runtime state.

## Architecture

```text
BehaviorRepository / StateMachine
  -> StateMachineExecutionEngine
  -> shared ExecutionSession / ExecutionManager
  -> shared event queue and SimulationTime
  -> PR31 ActivityExecutionEngine for State behaviors
  -> StateMachineExecutionSnapshot
  -> thin Tauri commands
  -> presentation-only runtime controls and overlays
```

The frontend never selects transitions, evaluates guards/effects, processes
events, or advances simulation time. It only invokes Rust commands and renders
runtime snapshots. Runtime overlays are input-transparent and do not replace the
shared workspace surface.

## Implemented and qualified

- Shared execution lifecycle: Initialize, Run, Step, Pause, Resume, Reset, and
  Terminate.
- Deterministic signal, call, time, change, completion, and any-receive trigger
  matching where the authored model provides the required semantic data.
- Bounded run-to-completion traversal with deterministic transition priority and
  explicit ambiguity diagnostics instead of container/hash-order selection.
- Shared bounded expression evaluation for guards and pure transition effects.
- External, local, and internal transition handling, including cross-hierarchy
  exit/entry domains.
- Active composite ancestry, nested regions, simultaneous orthogonal-region
  configurations, and nested-target entry with sibling-region initialization.
- Initial, Choice, Junction, Fork, Join, Terminate, and FinalState execution.
  Fork/Join is qualified with explicit targets across orthogonal Regions and
  does not spuriously initialize sibling-region defaults.
- Executable Submachine States using the same ExecutionSession, event queue,
  SimulationTime, runtime values, trace, and shared ActivityRepository as the
  parent machine.
- State entry, doActivity, and exit author stable modeled Activity IDs and
  execute through the PR31 ActivityExecutionEngine. Entry/exit Activities run
  synchronously inside State Machine run-to-completion; doActivities progress
  asynchronously, can wait on shared Signal/Time events, and are cancelled on
  State exit.
- State Machine transition events that are already due can preempt a progressing
  doActivity; future TimeEvents do not advance simulation time ahead of zero-time
  Activity work.
- Embedded State Activities reuse the PR31 Activity engine's shared semantic-step
  budget accounting without an additional State Machine charge.
- Shared SimulationTime scheduling for relative and absolute TimeEvents; no
  browser or wall-clock timing. State activation generations plus deterministic
  queue cleanup prevent obsolete timers from firing or accumulating after exit
  and re-entry.
- ChangeEvent false-to-true edge semantics, addressed-event preservation, and
  deterministic dispatch that does not let unrelated queued events starve a
  relevant State Machine event.
- Authored default runtime values are available to State Machine guards through
  the shared ExecutionSession.
- State Activity references and externally queued Signal IDs are validated at
  the Rust boundary rather than trusted from frontend input.
- Authored-model fingerprint invalidation includes Behavior and Activity runtime
  sources, with reset/reinitialization, transient runtime isolation,
  deterministic traces, and bounded failure diagnostics.
- Runtime visualization for active/waiting/final states, active regions,
  enabled/last-fired transitions, events, time, status, diagnostics, and trace.
- PR31 all-nine-family authoring/editing and Rust-authority/frontend-debt
  regression contracts remain required.

## Explicitly unsupported / deferred

- EntryPoint and ExitPoint vertices are authorable, but the current metamodel
  does not identify a qualified connection-point owner and entry/exit mapping.
  Reaching either is rejected with an engineer-readable diagnostic rather than
  guessed execution semantics.
- ShallowHistory and DeepHistory vertices exist, but the authored metamodel has
  no qualified default/history restoration policy. Reaching either produces a
  bounded engineer-readable diagnostic instead of approximate behavior.
- Transition effects remain bounded pure-expression evaluation only. Arbitrary
  scripts/statements are never executed. Rich typed operation/signal effects are
  deferred to later execution integration.
- Full structural runtime instances, Parts, Ports, Connectors and distributed
  runtime addressing belong to PR33. Parametric solving and Sequence execution
  remain later work.

## Qualification

PR32 qualification includes:

- `crates/model-core/tests/pr32_state_machine_execution.rs`
- `crates/model-core/tests/pr32_state_machine_event_semantics.rs`
- `crates/model-core/tests/pr32_state_machine_semantic_closure.rs`
- `crates/model-core/tests/pr32_state_machine_fork_join.rs`
- `crates/model-core/tests/pr32_state_machine_activity_bridge.rs`
- `crates/model-core/tests/pr32_state_machine_activity_reset.rs`
- `crates/model-core/tests/pr32_state_machine_activity_budget.rs`
- `crates/model-core/tests/pr32_state_machine_composition.rs`
- `scripts/validate_state_machine_execution.py`

Linux/core and Windows/desktop CI also retain the existing PR31 Activity,
behavior, shared workspace, routing/frame, presentation interaction,
repository-editing, Rust-authority, and all-nine-family standard-authoring
contracts.
