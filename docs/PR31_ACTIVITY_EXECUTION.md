# PR31 Native Activity Execution

PR31 adds a deterministic Activity token engine behind the shared
`ExecutionEngine` boundary. The authored `ActivityRepository` remains model
state. Tokens, activation state, call frames, waiting actions, scheduled events,
runtime values, diagnostics, and trace entries live only in an
`ExecutionSession` plus its transient `ActivityExecutionEngine`.

## Execution boundary

The complete path is:

```text
authored ActivityRepository
  -> ActivityExecutionEngine
  -> ExecutionSession / ExecutionManager
  -> ActivityExecutionSnapshot
  -> Tauri activity-execution commands
  -> Activity ribbon and runtime overlays
```

The frontend requests Rust state and renders it. It does not select flows,
evaluate guards, move tokens, dispatch events, or advance simulation time.

Execution sessions are tied to the authored Project + ActivityRepository used to initialize them. If the authored Activity changes after initialization, the desktop execution boundary invalidates the stale engine/session and rebuilds from the current authored model before Run, Step, Resume, or Reset can advance execution. A stale runtime snapshot is not returned as if it still matched the edited Activity.

## Implemented semantics

- Control tokens for Initial, Action, Decision, Merge, Fork, Join, Flow Final,
  and Activity Final nodes.
- Distinct object/value tokens and stores for pins, object nodes, central
  buffers, data stores, and Activity parameter nodes.
- Multiplicity, uniqueness, FIFO/LIFO, object-flow weight, selection, and
  transformation handling where those properties are represented by the
  authored Activity model.
- A bounded Rust expression service for boolean and numeric literals, runtime
  value references, comparisons, arithmetic, boolean operators, and
  parentheses.
- Nested CallBehavior frames with parameter transfer and caller resumption.
- SendSignal and AcceptEvent through the shared deterministic event queue.
- AcceptTimeEvent through `SimulationTime`, without wall-clock waits.
- An explicit `OperationCallRuntime` extension boundary. The default boundary
  returns a named diagnostic because the model does not provide Operation
  implementations yet.
- OpaqueAction bodies limited to the bounded pure-expression language.
  Assignments, statements, function calls, and arbitrary script execution are
  rejected with diagnostics.
- Interrupting edges clear the represented interruptible-region runtime state.
  Activity partitions remain allocation/presentation metadata and do not own
  execution.

## Structured Activity qualification

The current authored metamodel represents containment for Structured,
Sequence, Loop, Conditional, Expansion, and Interruptible nodes, but does not
represent executable clause/test/setup/body partitions or expansion
input/output nodes and modes.

- Structured, Sequence, and Loop containers execute their contained explicit
  control/object flows. No extra behavior is inferred from containment.
- Conditional containers emit a runtime warning that clause semantics are not
  represented; their explicit contained flows still execute.
- Interruptible regions are supported when an authored edge identifies the
  region as interrupting.
- Expansion regions are intentionally non-executable as expansion semantics.
  They emit an explicit runtime warning. Follow-on work requires authored
  expansion nodes, an expansion mode, collection token mapping, and output
  collection rules before execution can be implemented honestly.

## Runtime controls and visualization

Activity diagrams expose Initialize, Run, Step, Pause, Resume, Reset, and
Terminate in the existing ribbon. Snapshots drive visually separate runtime
states for enabled/active, waiting, completed, failed, active/completed flows,
token counts, simulation time, diagnostics, and recent trace entries. Opening
or creating a project clears transient sessions; save data is unchanged.

## Manual smoke test

1. Open a project and create an Activity diagram.
2. Add `Initial -> OpaqueAction -> ActivityFinal` with Control Flows. Leave the
   OpaqueAction empty, or set its body to the pure expression `1 + 2 == 3`.
3. Select **Initialize** and confirm the Initial node is enabled and the runtime
   panel reports `Initialized` at `0 ns`.
4. Select **Step** repeatedly. Confirm node and flow highlighting follows the
   semantic progression and the trace grows without changing diagram geometry.
5. Select **Reset** and confirm tokens, node state, step count, and simulation
   time return to their initial values.
6. Select **Run** and confirm the session reaches `Completed`; then Reset and
   repeat to confirm the same trace ordering.
7. Change the OpaqueAction body to `x = 1`, initialize, and step. Confirm the
   session fails with a bounded-expression diagnostic rather than executing the
   assignment.
8. Edit or delete that Action from the authored model, then Run/Step/Reset again. Confirm the stale `x = 1` runtime does not survive the authored-model change.
9. Save, close, and reopen the project. Confirm the authored Activity remains
   present and no prior runtime token, highlight, value, or trace was persisted.
