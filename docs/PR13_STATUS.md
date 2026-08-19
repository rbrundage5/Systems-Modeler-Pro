# PR13 Status

Current branch: `agent/pr13-activity-rust`

Baseline: merged PR #12 / `fe671378dbca700f60eb38a3950faf47f2f46edd`.

Current state: implementation in progress.

Completed implementation boundaries:
- Rust Activity semantic core is implemented and exported from `systems-modeler-core`.
- Focused Activity semantic conformance tests exercise the public model-core API.
- Typed Activity repository persistence is implemented over the existing transactional SQLite metadata channel.
- Activity repository save/reload qualification preserves semantic Activity, node, and edge identities.
- Checkpoint 2 passed formatting, core/persistence tests, strict Clippy, frontend syntax, and the existing behavior integration contract.
- The Rust/Tauri Activity workspace is implemented with Activity state, Activity diagram presentation metadata, Activity snapshots, diagram creation, executable node/flow commands, metadata save/load, Rust-owned palette entries, and shared obstacle-safe routing reuse.
- The Activity desktop create/connect/save/reopen/render checkpoint passed on Linux and Windows.
- Richer Activity editing now exposes CallBehaviorAction, CallOperationAction, SendSignalAction, AcceptEventAction, AcceptTimeEventAction, ActivityParameterNodes, operation-derived pins, partitions, structured regions, node assignment, and Rust-backed semantic property editing.
- The richer Activity semantic-editing checkpoint passed Linux/core and Windows/desktop qualification.

Current implementation slice:
- Add true semantic pin endpoints for Object Flows while preserving the existing `add_activity_edge` command contract.
- Render action pins from Rust snapshots and allow pin selection only for Object Flow endpoints.
- Render ActivityPartitions as swimlane geometry and structured Activity nodes/regions as derived presentation frames without moving semantic ownership into JavaScript.
- Follow with deletion/reconnection, drill-down/navigation, and final routing qualification.

Remaining completion gates include deletion/reconnection/drill-down, final routing qualification, and focused manual create/edit/connect/save/reopen acceptance.

No completion claim is made until the gates in `PR13_IMPLEMENTATION_CHECKLIST.md` are satisfied.