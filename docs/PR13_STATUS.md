# PR13 Status

Current branch: `agent/pr13-activity-rust`

Baseline: merged PR #12 / `fe671378dbca700f60eb38a3950faf47f2f46edd`.

Current state: final routing-quality qualification in progress.

Completed implementation boundaries:
- Rust Activity semantic core is implemented and exported from `systems-modeler-core`.
- Focused Activity semantic conformance tests exercise the public model-core API.
- Typed Activity repository persistence is implemented over the existing transactional SQLite metadata channel.
- Activity repository save/reload qualification preserves semantic Activity, node, and edge identities.
- The Rust/Tauri Activity workspace is implemented with Activity state, Activity diagram presentation metadata, Activity snapshots, diagram creation, executable node/flow commands, metadata save/load, Rust-owned palette entries, and shared obstacle-safe routing reuse.
- The Activity desktop create/connect/save/reopen/render checkpoint passed on Linux and Windows.
- Richer Activity editing exposes CallBehaviorAction, CallOperationAction, SendSignalAction, AcceptEventAction, AcceptTimeEventAction, ActivityParameterNodes, operation-derived pins, partitions, structured regions, node assignment, and Rust-backed semantic property editing.
- The richer Activity semantic-editing checkpoint passed Linux/core and Windows/desktop qualification.
- Semantic PinId endpoints are supported for Object Flows with Rust direction/type validation.
- Action pins, ActivityPartitions, and structured/interruptible regions render from authoritative Rust snapshots.
- CallBehaviorAction drill-down navigates by stable Activity ID.
- Activity node/edge deletion, flow reconnection, and full diagram rerouting are implemented as explicit Rust commands.
- Node deletion removes incident pin/node flows transactionally; failed semantic mutations restore both Activity and presentation snapshots.
- Activity rerouting reuses the shared Rust orthogonal obstacle-aware router.
- Focused manual acceptance passed for flow reconnection, node deletion with incident-flow cleanup, and full Activity rerouting.
- Activity reroute now assigns deterministic diagram-wide lanes so decision/merge/fork/join branches do not collapse onto identical orthogonal corridors.

Current qualification state:
- Previous automated checkpoint passed formatting, core/persistence tests, strict Clippy, frontend syntax, Activity integration contract, Windows desktop check/tests/lint, and Cargo.lock cleanliness.
- New diagram-wide Activity routing-lane change is awaiting CI qualification and focused visual verification.

Remaining completion gate:
- CI must pass on the routing-lane checkpoint.
- Focused visual Activity routing verification must confirm improved branch separation while retaining legal endpoint attachment and obstacle avoidance.

No completion claim is made until the final routing-quality gate is satisfied.