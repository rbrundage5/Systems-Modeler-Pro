# C16.06.01 — Restore a cancelled HTML presentation gesture

Work order: 2026-09-21. Baseline: current GitHub main
`2b9750e8b3ca6de76f86ec7f367f155513063862`. Branch:
`codex/c16-cancelled-drag-restore`. This is separate from C18.03.01.

## Finding and acceptance

The shared HTML controller changes position/size during a drag. Its cancellation
callback removes the dragging class but does not restore geometry or State Machine
transition previews. It also leaves suppression enabled for the next selection
click. The diagram can therefore show a move/resize that Rust never committed.

The regression fixture drives the production pointer handlers. Before the fix,
eight cancellation cases fail and the two normal-gesture controls pass. A drag
from x=100 previews x=140 and remains there after pointercancel; a resize from
width=190 remains at width=230 after lostpointercapture.

Acceptance: restore all four original geometry values and transition preview,
allow the next selection click, and issue zero semantic/history commands on
cancel/lost capture. Preserve one commit on normal pointerup, zoom scaling,
drag threshold and pointer identity filtering.

Allowed production path: `apps/desktop/frontend/diagram-interaction.js` only.
Tests: `scripts/test_cancelled_presentation_gesture.cjs`, included by the existing
`scripts/test_dialog_escape_ownership.cjs` CI entry point. Documentation: this file.
No Rust semantic/routing/history changes, no replacement controller, no agent
configuration edits. Approved source: supplied CATIA guide section 2 (moving a
symbol is presentation editing), plus the repository's gesture/Rust authority
contract. No claim of complete CATIA behavior or all-family visual qualification.

The lead performs the bounded fix directly. AGENTS.md disables workers; no
independent reviewer has been launched and no model override is requested.

## Verification and publication

Post-fix `node --test scripts/test_dialog_escape_ownership.cjs`: all 12 tests
pass (10 gesture cases plus 2 existing Escape cases). The same gesture fixture
failed 8 of its 10 cases on the recorded baseline before the patch. JavaScript
syntax, presentation interaction, Rust-authority, workspace convergence and
`git diff --check` pass. These are production-handler tests in a simulated DOM;
native WebView2 visual acceptance remains UNVERIFIED. No Rust files changed.
Publishing requires explicit user approval after automatic review rejected the
related C18 branch's push. Do not retry publication through another channel.
No PR or automatic merge has been performed.

Publication authorization: the user explicitly approved pushing both branches and
opening draft PRs in the following turn. Publish through the connected GitHub
account, execute CI, and retain the independent-review/native-acceptance gates.
