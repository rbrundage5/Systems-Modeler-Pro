# Collaboration server increment

This server wraps PR73 storage with authenticated HTTP. It is an administrator-run
service, not yet connected to the desktop UI. Central hosted and private deployments
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
| GET /v1/projects | IDs and roles available to this credential |
| GET /v1/projects/ID | Semantic model plus committed revision |
| POST /v1/projects/ID/operations | PR73 EditRequest; receipt includes revision and element ID |

Example operation body (UUIDs must refer to your project):

```json
{
  "operation_id": "NEW_OPERATION_UUID",
  "expected_revision": 0,
  "edit": {"CreateBlock": {"owner": "ROOT_ELEMENT_UUID", "name": "Engine"}}
}
```

CreatePackage and RenameElement are also supported. Identity comes exclusively from
credential verification. Unknown top-level request fields are rejected. A retry
must retain the same operation ID and body. HTTP 409 reports revision conflicts;
resnapshot and resolve explicitly, never blindly overwrite. HTTP 401 means invalid
credentials; 403 means insufficient access; 422 means semantic rejection. Storage
errors do not expose internal SQL or filesystem paths.

Request bodies are limited to 16 KiB, at most 32 connections are active, request
body timeout is 5 seconds, and the connection lifetime is bounded to 15 seconds.
An interrupted response may correspond to a committed operation: retry the exact
operation ID to recover its receipt. This is a serialized single-process foundation,
not a high-availability or large-model performance qualification.

## Remaining work

Desktop Connect/Open controls, presence, change streaming, diagram metadata,
complete editing operation coverage, collaborative undo, user-facing project
administration, token expiry/SSO, and deployed multi-device acceptance remain open.
Snapshot fetch supports reconnect at this API level; it does not implement offline
merge. Existing individual offline desktop workflows remain separate.

## Validation

The server tests cover authenticated edit/retry/conflict, viewer access, unknown
project denial, malformed/oversized inputs, forged identity, and real loopback HTTP
authentication. Required full CI and server checks must pass before merge. Public
HTTPS deployment and desktop behavior have not been tested by these server tests.
