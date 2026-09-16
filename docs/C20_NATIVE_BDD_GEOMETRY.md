# C20.06b.01 — one native/shared BDD geometry command authority

Baseline: `1b8542cc1b76da9ef1de96a3870d9a39710ed588` (PR104 candidate).
Main remains `2b9750e8b3ca6de76f86ec7f367f155513063862`.

One workflow: move/resize a BDD presentation and reroute its relationships with the
same Rust command in the native workspace and authoritative collaboration server.
The command stages a diagram clone and publishes geometry only after successful
routing. No semantic mutation or whole-project replacement is introduced.

Allowed production paths: core structural_presentation.rs and its new geometry
module; desktop workspace.rs, workspace/presentation_interaction.rs and
workspace/routing.rs (remove the superseded routing re-export);
persistence collaboration.rs and its new native_bdd adapter module. Tests: focused
core BDD geometry and persistence transaction/compatibility tests. No frontend,
protocol, permission, inverse schema, dependencies or worker/workflow changes.
No parallel writer is active; worker delegation remains disabled.

Existing offline incident-edge routing and server all-edge routing policies remain
explicit caller choices. Shared JSON/database records remain unchanged; the adapter
maps their stable IDs and geometry into the native contract. The shared transaction
still owns permission, revision, exact retry receipt, validation and inverse capture.
Other diagram-family editing keeps its existing code paths.

Acceptance: native command and shared server move produce matching geometry when
using all-edge policy; malformed geometry, missing presentation and failed route
leave the candidate unchanged. Offline incident-only routing preserves unrelated
routes. Shared stale/viewer requests reject, exact retries do not duplicate and
reversal restores the original geometry after reopen. Existing desktop/history,
server and persistence suites must remain green on the final published head.

This is the geometry subleaf of C20.06b. Creation/deletion/property/relationship
commands and ordinary-workspace remote-session integration still need their own
bounded dispatch leaves. All-nine-family collaboration and native deployed two-device
acceptance remain open. Keep draft pending independent review; no automatic merge.
