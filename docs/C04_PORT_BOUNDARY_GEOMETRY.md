# C04.03.01 — IBD port boundary geometry

Work order: one presentation workflow, moving/resizing a port on an internal
property or the context frame, including moving/resizing its owning boundary.
Baseline main: `2b9750e8b3ca6de76f86ec7f367f155513063862`.
Dependency: PR108 (`cd5ef99e50cb738b5377ccc6effbe2a0b8f0f976`), the cancelled
gesture restoration contract. Local isolated branch: `codex/c04-port-boundary-geometry`.

The user explicitly prioritized boundary dragging and authorized continued lead
implementation and draft PR publication. No workers are dispatched; environment
qualification and independent review remain outstanding. No settings override is
claimed. The lead owns all writes, with no overlapping writer.

## Reproduced defect and expected behavior

`presentation_interaction::update_ibd_port_geometry` snaps context ports to
0/42/1800/1100, while `ibd-ui::outerFramePoint` projects them using a second legacy
rectangle and the visible workspace frame. Pointer previews do not constrain ports;
their start coordinates differ from what is displayed. Routes update points without
updating labels. Moving a frame changes only machine preferences, outside authored
IBD history and native persistence.

One Rust geometry contract must project onto all four owner boundaries, preserve
attachment through owner movement/resize, stage connected routes and labels, and
commit one undo snapshot. The renderer must display the committed geometry. Preview
requests are read-only, coalesced, and discarded after cancellation. Frame geometry
used by context ports belongs to the authored IBD, including native/portable save
and history. Legacy diagrams remain readable and adopt the visible frame on their
first explicit context-boundary edit.

Sources: uploaded CATIA Magic guide, section 2 (presentation movement versus semantic
identity, secondary evidence); uploaded SysML 1.6, section 9 (ports and connectors).
These establish the workflow and semantic distinction, not a vendor gesture parity
claim. No copyrighted source text is copied. No external retrieval is needed.

## Scope and acceptance

Allowed production paths, relative to the repository:

- `apps/desktop/src-tauri/src/main.rs` (module and IPC registration)
- `apps/desktop/src-tauri/src/workspace/ibd_geometry.rs` (shared Rust geometry)
- `apps/desktop/src-tauri/src/workspace/ibd.rs` (authored frame, route reuse, creation)
- `apps/desktop/src-tauri/src/workspace/presentation_interaction.rs`
- `apps/desktop/src-tauri/src/workspace/shared_workspace.rs` (IBD frame commands)
- `apps/desktop/src-tauri/src/workspace/history.rs` (atomic IBD presentation edit)
- `apps/desktop/src-tauri/src/workspace/model_script.rs` (new field constructor only)
- `apps/desktop/src-tauri/src/workspace/portable_interchange.rs` (fixture constructor)
- `apps/desktop/frontend/diagram-interaction.js`
- `apps/desktop/frontend/ibd-ui.js`
- `apps/desktop/frontend/shared-workspace.js`

Tests: Rust unit/integration tests alongside these modules;
`scripts/test_ibd_port_gesture.cjs`, existing gesture fixture exports, and the existing
dialog regression entry point, plus `scripts/validate_presentation_interaction.py` and
`scripts/validate_workspace_convergence.py`
(updated to follow the extracted Rust geometry module without relaxing its checks). This work-order document records results.

Positive: four sides/corners; nested and context ports; zoom-scaled input; square
resize; frame/part proportional attachment; connected routes and labels; identity
unchanged; one undo/redo step; native/portable round trip and legacy deserialization.
Negative: nonfinite/oversize geometry, missing owner/port, routing failure,
pointercancel/lost capture and stale preview responses; no partial state/history.
No-op must not create history or erase redo. Unrelated routes stay unchanged.

Excluded: port semantic kinds/conjugation rules, connector semantic editing, a new
router, all-family frame history, collaboration protocol, whole-tool certification.
Follow-up findings are separate work orders. Required qualification: focused Rust
and Node tests, existing integration gates and CI on the published head. Native
WebView pointer/visual acceptance and independent review remain release gates.

## Verification

Implementation in progress. Local Rust tooling and an installed browser binary are
unavailable; Rust compilation/test execution will use the repository's existing CI.
No checks are claimed before execution.

Initial candidate evidence: 18 Node regressions pass through the existing dialog
entry point (10 gesture, 2 dialog, 6 port). All 21 static integration validators
pass after their geometry-location checks were updated. JavaScript syntax and
`git diff --check` pass. Six Rust regression tests are added but not yet executed.
CI results and native manual acceptance are pending.

Combined qualification: PR108 is stacked on PR107 so all candidates are checked
together while each PR keeps one leaf diff. The shared regression entry point
conflict retains every test. The combined Node suite passes 31 tests (including
four context-frame commit/cancel/undo/rejection regressions). No PR is merged.
