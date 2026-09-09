# Collaboration delivery

Selected direction: a central Rust server serving multiple desktop devices,
with the same server deployable privately or through secured internet hosting.
No hosting service is provisioned by this change.

## Increment 1A: persistent operation boundary

This increment adds a server-side API to the existing persistence crate. It is
not a running server or a finished shared-project UI.

- Trusted administration provisions project membership as viewer or editor.
- A future authentication adapter supplies the actor identity; the library does
  not authenticate tokens, passwords, or network clients.
- Create Block, Create Package, and Rename Element reuse core model methods and
  validation. Unsupported operations are not advertised as collaborative.
- SQLite IMMEDIATE transactions serialize writers across database connections.
  Model changes, revision, and operation receipt commit atomically.
- Requests carry a unique operation ID and expected project revision. Exact
  retries return the original receipt; changed payloads or actors using that ID
  are rejected. Stale requests fail explicitly and must resynchronize.
- Authorized snapshots read the model and revision in one transaction.
- Ordinary save_project rejects shared projects to prevent stale whole-project
  replacement. Existing unshared project save behavior remains available.

The database API is trusted server infrastructure, not a security sandbox.
Existing raw load/metadata/activity APIs must never be exposed as remote handlers.
No desktop database is marked shared automatically. Diagram metadata is not yet
included in shared edits or reconnect snapshots. Model writes currently reuse
whole-model persistence internally; this is not a large-model performance claim.

## Remaining increments

1. Authenticated server: verified identities, project access lifecycle, encrypted
   transport/deployment configuration, bounded requests, and operation delivery.
2. Desktop integration: connect/open shared project, presence, semantic and diagram
   command coverage, independent local view state, explicit conflict UI.
3. Reliability: revision-based reconnect, membership revocation, concurrent writes,
   safe user-scoped undo, delete-versus-edit, restart and multi-device qualification.

Offline shared editing/merge, branching, and advanced governance are subsequent
scope. Disconnected shared clients must not silently accept unsynchronized edits.

## Verification

`cargo test --locked -p systems-modeler-persistence` includes reopen/idempotency,
separate-connection stale revision, access denial, invalid-edit rollback, rename,
role downgrade, and legacy-save protection cases. These are storage-level tests;
network, UI, and simultaneous device acceptance remain pending. Full existing
CI is required before merge. Never merge automatically.

The next HTTP adapter increment is described in [server setup and limits](COLLABORATION_SERVER.md).
