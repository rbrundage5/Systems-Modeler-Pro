# C16.06.02 — IBD selection movement preserves attachment

One leaf: the existing `move_active_selection` command for IBD presentations.
Baseline main is `2b9750e8b3ca6de76f86ec7f367f155513063862`; this candidate is
stacked on PR109 head `1e35a8aaaa47c0f534120e5e0f52bf8c042960e0` to reuse its
Rust geometry and atomic history contract. Lead-only implementation under the
user's active work request; no workers, automatic merge or independent approval.

Reproduction: the current IBD selection mover clamps a part at x=0 but applies
the original unclamped delta to its ports. Selecting both a parent and its port
can move the port twice. Independently selected ports translate off the boundary,
and every connector is rerouted without moving its label.

Expected: move selected parents before ports; use the actual clamped displacement;
move an attached child once; project independent ports with the same Rust contract
as pointer gestures; reroute affected connectors and labels once; one atomic history
entry, including no-op and routing-failure protection. Legacy context-port movement
requires its visible frame explicitly, rather than guessing another rectangle.

Allowed paths: `apps/desktop/src-tauri/src/workspace/standard_editing.rs`,
`standard_editing_bridge.rs`, `ibd_geometry.rs` (shared test-fixture visibility only),
and this document. Reuse existing tests/fixtures and the repository's CI; do not
modify workflow controls. No new keyboard binding, clipboard change, semantic change
or general editing refactor. Approved evidence: CATIA guide section 2 and the
repository's Rust-owned presentation/history contract.

Acceptance: clamped parent move; parent-plus-port order independence; independent
nested/context-port movement; unrelated stale connector preservation; no-op keeps
redo; missing frame/invalid delta/routing failure leaves authored state/history
unchanged. Preserve all other families' command path. Tests must call the production
move/history helpers. Native multi-selection UX and independent review remain open.

Verification: implementation in progress; Rust tests will execute in CI because the
local toolchain is unavailable. The command is registered, but no general frontend
arrow-key binding is claimed.
