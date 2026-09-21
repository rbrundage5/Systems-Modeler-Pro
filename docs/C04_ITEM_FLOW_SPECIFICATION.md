# C04.09.01a — atomic existing ItemFlow specification

Baseline: 5cc1d94f338c1ee92e0605e9ac913742d748294e (PR117).
Finding: creation exists, but existing ItemFlows have no specification command or
editor. The notation query omits direction and the renderer always points forward.

Dependency-ordered increments:
1. This leaf: Rust-owned read/edit API for an existing flow's name, direction and
   conveyed classifier set, retaining connector and relationship identity.
2. Directional notation and authoritative reconciliation after history/reopen.
3. A Properties form on the selected connector using these APIs.

Allowed paths: crates/model-core/src/ibd.rs;
apps/desktop/src-tauri/src/workspace/item_flow_editing.rs; command registration in
apps/desktop/src-tauri/src/main.rs; this record. No unrelated semantic changes.

Acceptance: one atomic checkpoint; forward/reverse edits; qualified classifier
choices; stable identity; nonempty unique classifier validation; no-op suppression;
invalid edits preserve project, diagrams and redo; undo/redo and serialization.
Use existing Rust model validation and structural specification transaction.

Lead direct implementation. Workers remain disabled; independent review is open.
Native Rust CI and Windows UI acceptance are recorded separately. This API alone
does not qualify item-flow notation or the future form. Association typing, runtime
flow execution and external interchange conformance remain separate work.
