# C12.structured-preflight — unsupported Activity execution

Baseline: `2d1f51b0faef8abb840d7c848adca2f458bb92cc`.
Finding: TC-17, narrowed to truthful execution preflight. Allowed paths are
`crates/model-core/src/activity_execution.rs`, its existing PR31 integration test,
and this record. No authored schema, UI, persistence or CI-control changes.

The current authored structured-node record contains identity, name, kind and
parent only. Conditional clauses, expansion modes/nodes, LoopNode setup/test/body
and loop variables, and SequenceNode executable-node order are not represented.
The previous engine warned for two of those kinds and otherwise ran contained
flows without the missing semantics. A completed run could therefore be misleading.

Initialization, embedded initialization and reset now reject these four kinds
before changing the session. Preflight walks the root Activity and its transitive
CallBehavior references with a visited set, including recursive call graphs. It
names the Activity and structured node in the error. An unrelated unsupported
Activity does not block a supported root. All constructs remain authorable and
persisted without conversion or deletion.

This does not implement the missing semantics. Explicit control-flow cycles can
still execute through supported nodes; that is distinct from LoopNode semantics.
Other action/operation runtime limits and full structured/interruptible behavior
qualification remain separate coverage leaves.

Native regressions cover all four unsupported kinds and all three entry points,
preserving an initialized session with an existing runtime value, plus transitive
calls, recursion termination and an unrelated unsupported Activity. Existing
deterministic Activity tests remain the supported-behavior regression baseline.
The workspace has no Rust toolchain; native execution, formatting and lint must
be verified through the existing GitHub CI. Independent review and rendered
desktop acceptance remain outstanding.
