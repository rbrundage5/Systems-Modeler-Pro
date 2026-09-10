# Desktop shared-project increment

Merged baseline: `98e92f681929f9656713d7da89c70ce21711616c` (PR75).
Shared BDD increment: PR77, resumed from `ecedbda897b91edf23b53617ae7e0888586b7450`.

This increment exposes the server's three semantic edits and shared BDD presentation operations in a desktop
Shared Projects window. Rust owns credentials, transport, snapshots, revision
checks, and retry identity. The server remains the committed model authority.
The existing offline workspace and SQLite database are not replaced or connected
to remote state by this increment.

## Workflow

1. Configure the [server](COLLABORATION_SERVER.md) and obtain its HTTPS origin and
   administrator-issued access token. Loopback HTTP is supported for local tests.
2. Click **Shared Projects** in the desktop status bar, enter the address/token,
   and connect. Choose an authorized project and click **Open project**.
3. Select an owner/element, choose Create Package, Create Block, or Rename element,
   enter a name, and save. Viewer credentials cannot submit edits.
4. The window refreshes every five seconds while visible and no name is being
   composed. Explicit Refresh retrieves the current revision. Each user's local
   canvas, selection, and offline project remain independent.
5. On revision conflict, Refresh and review the model before submitting a new edit.
   On uncertain network failure, Retry pending edit sends the identical operation
   ID and body. New edits, opening another project, and disconnect are blocked
   until that pending operation is resolved.
6. Disconnect clears the in-memory session. Closing the window retains it until
   application exit. Tokens are never written to project files or local storage.

HTTPS certificate validation stays enabled, redirects are disabled, and the client
bypasses environment proxy discovery. An administrator-provided direct HTTPS origin
is required; deployments requiring an explicit outbound proxy are not supported yet.
Responses are bounded to 32 MiB and requests time out after 20 seconds.

## Qualification and limits

Local checks: frontend syntax, Rust authority regression gate, and diff hygiene.
Rust compilation, focused transport tests, full native CI, and UI acceptance must
be recorded against the final PR head. Cargo is unavailable in the authoring
workspace. The CI-generated formatting/dependency patch was applied; the dedicated
client job now checks strict formatting and locked dependencies. Visual testing was
attempted but browser security denied access to the local fixture; no UI pass is
claimed. Native two-device acceptance remains required.

Focused tests cover endpoint restrictions, exact retry identity over HTTP,
conflict handling, and accepted edits with failed snapshot refresh. These do not
establish deployed multi-device usability.

Manual acceptance remains required: connect two desktop instances to the same
server, create/rename in one and observe refresh in the other; verify viewer
rejection, stale-edit conflict, token rejection, reconnect, and unchanged offline
create/edit/save/reopen workflows. Test the window's keyboard focus and sizing.

Shared relationships/routing, other diagram families, complete authoring operations, presence, collaborative
undo, project administration, and production HTTPS deployment remain subsequent
work. Pending retries are memory-only; after an application crash, reload the
server snapshot and inspect it before recreating an unconfirmed edit. There is no
automatic offline merge or crash-persistent outbox in this increment.

## Shared BDD workflow and qualification

After opening a shared project, choose a Model or Package owner, enter a diagram
name, and select Create BDD. Select an existing supported element and Place on BDD.
Drag a node to commit geometry; another session observes the same layout on refresh.
Rename BDD, Remove selected node, and Delete BDD are revisioned operations. Removing
a presentation does not remove its semantic element. Geometry supports server-side
resize, but this UI currently exposes movement only. Relationship editing, normal
workspace integration, frame/compartment parity, zoom/pan, and other families are
not qualified by this bounded canvas.

A rejected repository edit retains its typed name for review and resubmission.
Pointer gestures are blocked during requests, capture their starting revision,
and do not commit on a stationary click. Network uncertainty retains exact retry
identity; a revision conflict requires explicit refresh. Pending retries remain
memory-only.

The real-server integration test runs two independent Rust desktop sessions over
loopback HTTP, covering a stale writer and shared BDD create/place/move convergence.
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

Next acceptance: run two native desktop instances through create/place/move,
rename/remove/delete, stale edits, viewer rejection, retry and reconnect; verify
unchanged offline editing/save/reopen. Then extend shared relationship operations
through existing Rust semantics before broadening diagram-family coverage.
