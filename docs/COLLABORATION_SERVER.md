# Collaboration server increment

This server wraps PR73 storage with authenticated HTTP. It is an administrator-run
service connected through the desktop Shared Projects window. Central hosted and private deployments
use the same binary. No public service is provisioned by this PR.

## Administrator setup

Build with `cargo build --locked -p systems-modeler-server --release`.
The executable supports these commands:

1. `systems-modeler-server init DATABASE NAME` creates a project and prints its ID.
   Use a dedicated server database. Do not point a running desktop at this database.
2. `systems-modeler-server credential` generates a random token, its SHA-256 hash,
   and an actor UUID. Deliver the token securely to that user; retain only the hash
   in configuration. Do not commit either credentials or live configuration to Git.
3. Write a JSON configuration with this shape, replacing placeholders:

```json
{
  "credentials": [{
    "actor": "ACTOR_UUID",
    "token_sha256": "64_LOWERCASE_HEX_HASH",
    "projects": [{"project": "PROJECT_UUID", "role": "editor"}]
  }]
}
```

Roles are `viewer` and `editor`. One actor and hash per credential; projects may
have multiple users. Configured projects must already exist. Keep the database and
configuration readable only by the service administrator/account.

4. `systems-modeler-server serve DATABASE CONFIG` listens on `127.0.0.1:4783`.

Remote access requires an HTTPS reverse proxy on the same host. The binary rejects
non-loopback listeners. Configure certificate validation, request/rate limits, and
proxy timeouts before exposing the HTTPS endpoint. Do not forward the HTTP port
publicly. HTTPS deployment is not yet qualified or provisioned by this increment.

Credentials are explicitly issued API tokens, not user passwords or SSO. They do
not expire automatically. Rotate or revoke a token by editing config and restarting
the service; removed credentials/grants are denied by the HTTP layer even if old
storage memberships remain. Offline database APIs are trusted administrative APIs,
not independently sandboxed from other code running as the service account.

## HTTP contract

Requests use `Authorization: Bearer TOKEN`. Tokens must be the generated 64-character
hex strings. Browser Origin requests and query parameters are rejected. Responses
are JSON with `Cache-Control: no-store`. POST requires `application/json`.

| Request | Result |
| --- | --- |
| GET /v1/capabilities | Protocol version, server package version, authenticated actor ID, and supported collaboration capabilities |
| GET /v1/projects | IDs and roles available to this credential |
| GET /v1/projects/ID | Semantic model, shared BDD presentations, and committed revision |
| POST /v1/projects/ID/operations | PR73 EditRequest; receipt includes revision and element ID |
| GET /v1/projects/ID/history | Authenticated actor's most recent 50 operations and reversal availability |
| GET /v1/projects/ID/presence | Active sessions for this authorized project |
| POST /v1/projects/ID/presence | Heartbeat with session UUID and display name; actor/role come from credentials |
| DELETE /v1/projects/ID/presence | Remove this actor's session UUID only |

Example operation body (UUIDs must refer to your project):

```json
{
  "operation_id": "NEW_OPERATION_UUID",
  "expected_revision": 0,
  "edit": {"CreateBlock": {"owner": "ROOT_ELEMENT_UUID", "name": "Engine"}}
}
```

The desktop calls the authenticated capabilities endpoint before project discovery.
Protocol `1` currently requires revisioned operations, shared BDDs, simple semantic
relationships, server-routed BDD relationship presentations, shared requirements,
authenticated actor identity, project presence and actor-scoped reversal. A new client will
not open a project through an older or incomplete server; deploy matching desktop
and server versions instead of attempting an operation with unknown semantics.

CreatePackage, RenameElement, CreateRequirement, UpdateRequirement and CreateTestCase
are also supported. Identity comes exclusively from
credential verification. Unknown top-level request fields are rejected. A retry
must retain the same operation ID and body. HTTP 409 reports revision conflicts;
resnapshot and resolve explicitly, never blindly overwrite. HTTP 401 means invalid
credentials; 403 means insufficient access; 422 means semantic rejection. Storage
errors do not expose internal SQL or filesystem paths.

Simple semantic relationship creation uses the same endpoint and revision. For
example, a Block generalizing another Block is submitted as:

```json
{
  "operation_id": "NEW_OPERATION_UUID",
  "expected_revision": 2,
  "edit": {
    "CreateRelationship": {
      "kind": "Generalization",
      "source": "SPECIFIC_BLOCK_UUID",
      "target": "GENERAL_BLOCK_UUID",
      "owner": "MODEL_OR_PACKAGE_UUID"
    }
  }
}
```

Supported kinds are Dependency, Generalization, Realization, Allocate,
DeriveRequirement, Satisfy, Verify, Refine, Trace, Copy, Include, and Extend.
`DeleteRelationship` accepts the stable relationship UUID for those same kinds.
All endpoint/direction/owner/duplicate/cycle checks are performed by model-core and
the model, revision, and operation receipt commit atomically. Association,
Connector, ItemFlow, BindingConnector, package/element import, and their specialized
presentations require additional payloads and are deliberately rejected by the
simple relationship deletion path.

Once both semantic endpoints are nodes on a shared BDD, the relationship can be
presented and routed with the same project revision:

```json
{
  "operation_id": "NEW_OPERATION_UUID",
  "expected_revision": 6,
  "edit": {
    "PresentBddRelationship": {
      "diagram": "BDD_DIAGRAM_UUID",
      "edge": "NEW_PRESENTATION_UUID",
      "relationship": "SEMANTIC_RELATIONSHIP_UUID"
    }
  }
}
```

The server uses the shared model-core batch router; clients cannot submit arbitrary
points. `RouteBddDiagram` reroutes all edges, `RemoveBddEdge` removes only one
presentation, endpoint movement reroutes atomically, and semantic deletion removes
all affected BDD presentations. Invalid or impossible routes leave the model,
diagram, revision, and receipt unchanged.

Request bodies are limited to 16 KiB, at most 32 connections are active, request
body timeout is 5 seconds, and the connection lifetime is bounded to 15 seconds.
An interrupted response may correspond to a committed operation: retry the exact
operation ID to recover its receipt. This is a serialized single-process foundation,
not a high-availability or large-model performance qualification.

## Recovery, presence and change reversal

The desktop durably reserves each submitted request before sending it. Reconnecting
to the same origin as the same authenticated actor restores an unresolved request;
retrying its exact identity recovers a previously committed receipt. Token rotation
must retain the actor UUID to recover that actor's request. Credentials stay in
memory. Unsubmitted drafts are not yet persisted.

Presence is ephemeral, bounded and project-scoped. Heartbeats do not change model
revisions; silent sessions expire after 45 seconds. Display names are user-supplied
labels and do not establish identity. See [presence](C20_PROJECT_PRESENCE.md).

`UndoOperation` with an `operation` UUID reverses an operation by the authenticated
author. The server records typed inverse deltas atomically with commits and verifies
current affected records plus semantic/presentation dependencies before reversal.
Conflicts preserve state and history. Reversal creates a new revision and can itself
be reversed. Operations predating inverse recording remain visible but unavailable
for reversal. See [change reversal](C20_COLLABORATIVE_UNDO.md).

## Remaining work

Change streaming, specialized relationship editing and presentation,
additional diagram families, complete editing operation coverage, collaborative
editing through the ordinary workspace, user-facing project administration, token expiry/SSO, and deployed
multi-device acceptance remain open.
Snapshot fetch supports reconnect at this API level; it does not implement offline
merge. Existing individual offline desktop workflows remain separate.

## Validation

The server tests cover authenticated edit/retry/conflict, viewer access, unknown
project denial, malformed/oversized inputs, forged identity, real loopback HTTP
authentication, and authenticated BDD edge routing/cascade behavior. Required full
CI and server checks must pass before merge. Public HTTPS deployment and native
two-device behavior have not been tested by these server tests.
