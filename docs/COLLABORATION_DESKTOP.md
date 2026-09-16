# Desktop shared-project increment

Merged baseline: `98e92f681929f9656713d7da89c70ce21711616c` (PR75).
Shared BDD increment: PR77, resumed from `ecedbda897b91edf23b53617ae7e0888586b7450`.

The desktop Shared Projects window exposes server-authoritative element edits,
the supported simple semantic relationship operations, and shared BDD presentation
operations. Rust owns credentials, transport, snapshots, revision checks, semantic
validation, and retry identity. The server remains the committed model authority.
The existing offline workspace and SQLite database are not replaced or connected
to remote state by this increment.

## Workflow

1. Configure the [server](COLLABORATION_SERVER.md) and obtain its HTTPS origin and
   administrator-issued access token. Loopback HTTP is supported for local tests.
2. Click **Shared Projects** in the desktop status bar, enter the address/token,
   and connect. The client first verifies the server's authenticated protocol and
   required capabilities; incompatible installations stop with a version remedy
   before project discovery. Choose an authorized project and click **Open project**.
3. Select an owner/element, choose Create Package, Create Block, or Rename element,
   enter a name, and save. The relationship controls create or delete the supported
   source/target/namespace-owned semantic relationships. Viewer credentials cannot
   submit edits.
4. The window refreshes every five seconds while visible and no name is being
   composed. Explicit Refresh retrieves the current revision. Each user's local
   canvas, selection, and offline project remain independent.
5. On revision conflict, Refresh and review the model before submitting a new edit.
   On uncertain network failure, Retry pending edit sends the identical operation
   ID and body. New edits, opening another project, and disconnect are blocked
   until that pending operation is resolved.
6. Disconnect clears the in-memory session. Closing the window retains it until
   application exit. Tokens are never written to project files or local storage.

### Shared requirements and verification

The Requirements form creates a Requirement with its name, ID and multiline text.
Load an existing Requirement to inspect and update it. The saved version remains
visible above the draft. Create Test Case in Repository edits, then use the existing
relationship controls to connect TestCase → Requirement with Verify and
Block → Requirement with Satisfy. Updates preserve semantic identity and these links.

Drafts preserve their original revision through Refresh. After a conflict, inspect
the saved text and explicitly choose **Keep draft after reviewing latest revision**
before resubmitting, or clear the draft. Polling pauses during composition; project
switching/disconnect require saving or clearing. An uncertain network result must
use Retry pending edit. If an edit was accepted but snapshot refresh failed, refresh
and inspect the saved state before deciding whether another edit is needed.

Matching clients and servers must advertise `shared-requirements-v1`. Validation,
Copy protection, transaction/retry behavior and test scope are recorded in
[the bounded workflow contract](C20_SHARED_REQUIREMENTS.md). A dedicated shared
Requirement diagram and nested Requirement ownership are not introduced here.

HTTPS certificate validation stays enabled, redirects are disabled, and the client
bypasses environment proxy discovery. An administrator-provided direct HTTPS origin
is required; deployments requiring an explicit outbound proxy are not supported yet.
Responses are bounded to 32 MiB and requests time out after 20 seconds.

## Active participants

Enter a display name when connecting. While Shared Projects is open, a separate
ten-second heartbeat lists active sessions, authenticated account IDs and roles.
The heartbeat continues during draft composition and does not refresh model state
or advance its revision. Display names are participant-selected labels. Presence
does not grant permissions or lock model elements. Silent sessions expire after
45 seconds; closing the window or losing the network can leave a temporary entry
until expiry. Disconnect/project switch attempts immediate departure. See
[the presence contract](C20_PROJECT_PRESENCE.md).

## Qualification and limits

Local checks: frontend syntax, Rust authority regression gate, and diff hygiene.
Rust compilation, focused transport tests, full native CI, and UI acceptance must
be recorded against the final PR head. Cargo is unavailable in the authoring
workspace. The CI-generated formatting/dependency patch was applied; the dedicated
client job now checks strict formatting and locked dependencies. Visual testing was
attempted but browser security denied access to the local fixture; no UI pass is
claimed. Native two-device acceptance remains required.

Focused tests cover endpoint restrictions, exact retry identity over HTTP,
protocol negotiation/version rejection, conflict handling, and accepted edits with
failed snapshot refresh. These do not establish deployed multi-device usability.

Manual acceptance remains required: connect two desktop instances to the same
server, create/rename in one and observe refresh in the other; verify viewer
rejection, stale-edit conflict, token rejection, reconnect, and unchanged offline
create/edit/save/reopen workflows. Test the window's keyboard focus and sizing.

Specialized Association/Connector/ItemFlow/BindingConnector and import operations,
other diagram families, standard-workspace integration, complete element
authoring/configuration, collaborative undo, project administration, and
production HTTPS deployment remain subsequent work. Submitted pending edits are
recorded in a separate application-data SQLite outbox before transmission. After
restart, reconnect to the same server with credentials for the same authenticated
actor and use Retry pending edit. A rotated access token for that same actor can
recover the operation; another actor cannot. Credentials remain memory-only.
The original operation ID and revision are retained even if the server already
committed it. New writes are blocked if recovery storage cannot reserve the edit.
Unsubmitted drafts are still memory-only; automatic offline merge is not supported.

The simple relationship set is Dependency, Generalization, Realization, Allocate,
DeriveRequirement, Satisfy, Verify, Refine, Trace, Copy, Include, and Extend. Each
operation uses the existing model-core endpoint, direction, ownership, duplicate,
and cycle rules. Invalid edits do not advance the shared revision. Relationship
deletion is limited to this set; kinds with additional required payloads cannot be
deleted through the simple operation.

## Shared BDD workflow and qualification

After opening a shared project, choose a Model or Package owner, enter a diagram
name, and select Create BDD. Select an existing supported element and Place on BDD.
Drag a node to commit geometry; another session observes the same layout on refresh.
Rename BDD, Remove selected node, and Delete BDD are revisioned operations. Removing
a presentation does not remove its semantic element. Geometry supports server-side
resize, but this UI currently exposes movement only. After both relationship
endpoints are presented, select its semantic relationship and choose Show on BDD.
The server computes obstacle-clear orthogonal points and a label anchor through the
shared model-core router. Moving a node reroutes every edge atomically; a routing
failure preserves the prior diagram and revision. Remove edge removes only the
presentation, while deleting its semantic relationship or removing an endpoint node
cascades the affected BDD edge presentations. Route edges deterministically reruns
the authoritative batch router. Normal workspace integration, frame/compartment
parity, zoom/pan, and other families are not qualified by this bounded canvas.

A rejected repository edit retains its typed name for review and resubmission.
Pointer gestures are blocked during requests, capture their starting revision,
and do not commit on a stationary click. Network uncertainty retains exact retry
identity; a revision conflict requires explicit refresh. Pending retries remain
retained across desktop restart by the Rust recovery outbox.

The real-server integration test runs two independent Rust desktop sessions over
loopback HTTP, covering a stale writer and shared BDD node/relationship presentation
create, routing, endpoint movement, removal, and convergence.
The frontend regression script exercises production event handlers in a minimal
Node DOM fixture; it does not establish native rendering or two-device acceptance.
Run `node scripts/test_collaboration_ui.cjs` alongside the client CI workflow.

September 10 continuation: original head passed core/persistence CI but failed
native desktop compilation (test assertion required Debug), and both dedicated
workflows failed locked-dependency verification. The missing server dependency in
Cargo.lock and the assertion are corrected without relaxing either gate. Local
JavaScript syntax, Rust-authority, and diff checks pass. Cargo is unavailable in
this authoring environment; final native, server, and client results must be read
from GitHub CI for the published head before merge.

Next acceptance: run two native desktop instances through element and relationship
create/delete plus BDD create/place/move/rename/remove/delete and edge show/route/
remove, stale edits, viewer rejection, retry and reconnect; verify unchanged offline
editing/save/reopen. Then broaden the shared command boundary through another bounded
diagram-family or reliability slice.
