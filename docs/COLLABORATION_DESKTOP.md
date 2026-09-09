# Desktop shared-project increment

Baseline: `c18c770de554a32a345412819c6f35dcd8249f82` (merged PR74).

This increment exposes the existing server's three semantic edits in a desktop
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

Shared diagram presentation, complete authoring operations, presence, collaborative
undo, project administration, and production HTTPS deployment remain subsequent
work. Pending retries are memory-only; after an application crash, reload the
server snapshot and inspect it before recreating an unconfirmed edit. There is no
automatic offline merge or crash-persistent outbox in this increment.
