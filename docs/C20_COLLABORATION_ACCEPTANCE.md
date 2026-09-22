# Collaboration review and native acceptance record

Implementation checkpoint: 2026-09-16. Main remains
`2b9750e8b3ca6de76f86ec7f367f155513063862`. The combined candidate includes PR99's
installer/requirements changes, PR100 recovery, PR101 presence and PR102 reversal.
The collaboration source checkpoint is
`05bf54622e4187b29879642735f3f56f10247536`; subsequent integration documentation
does not change executable code. Final CI/artifact links belong in the integration
PR and identify its exact head, including the CI-generated merge commit when used.

This record does not approve a merge, enable agent workers, provision a server or
declare full collaboration complete. Independent review and the rendered native
checks below remain outstanding.

## Automated evidence covered by the candidate

- Persistence: exact pending-operation reservation/cleanup, reopen and isolation;
  actor-scoped inverse capture, disjoint changes, affected-record/dependency
  conflicts, atomic rollback, reverse-the-reversal and legacy history behavior.
- Server: authenticated capability identity, permission checks, project-isolated
  presence/expiry/capacity, own-change history and idempotent reversal.
- Real HTTP desktop sessions: restart before transmission, lost response after
  commit, rotated-token recovery, editor/viewer presence and two-client reversal.
- Frontend controller fixture: 24 checks through production event handlers,
  including draft preservation during heartbeat, exact recovery and history actions.
  The fixture does not establish rendered desktop usability.
- Native CI: existing core/persistence/desktop regression suites, strict formatting,
  lint, locked dependencies, frontend authority checks and Linux desktop build.
  Read the final candidate's CI result; do not substitute a preceding head's result.
- Windows packaging: a development installer and disposable-host install/launch
  smoke test. A running process does not prove successful interactive modeling.

## Native acceptance procedure

Use synthetic project data, two desktop instances on separate devices, an
administrator-managed HTTPS origin and matching desktop/server versions. Give A
and B different editor actor IDs and V a viewer token. Keep token values out of
screenshots, recordings and issue bodies. Configure the server using
[the administrator guide](COLLABORATION_SERVER.md). Record exact binary source
commits, operating systems and server configuration version, without credentials.

Every row is **NOT EXECUTED** in this session. Record actual results and sanitized
evidence before changing its status. Failed checks remain visible.

| ID | Steps | Required result |
| --- | --- | --- |
| N01 discovery and permissions | Connect A/B/V; open one authorized shared project; try a project with no grant | Authorized users see the same revision; unauthorized access is denied; V cannot mutate |
| N02 semantic editing | A creates a Package, Block, Requirement and TestCase; B refreshes and adds Satisfy/Verify links | Stable identities and requirement text agree across clients; links survive reconnect |
| N03 BDD authoring | A creates a BDD, places both relationship endpoints, presents and routes the relationship; B moves one endpoint | Both clients converge on server-owned geometry; endpoint movement reroutes without semantic duplication |
| N04 stale requirement draft | B edits Requirement text; A commits a conflicting update; B refreshes and tries to submit | B's draft survives, saved text is visible and explicit review is required; no silent overwrite |
| N05 presence during composition | Both users enter distinct labels; B composes a draft while heartbeats run; close A's window | Labels/account IDs/roles display correctly, B's draft survives, A expires after the documented interval |
| N06 restart with uncertain edit | Interrupt an edit response after server commit, close/restart the desktop, reconnect as the same actor, Retry pending edit | Original operation/revision is recovered exactly once; no duplicate model content; outbox clears after definite completion |
| N07 credential rotation and isolation | Repeat N06, rotate the token while keeping actor UUID; separately connect as another actor | The same actor recovers its operation; another actor cannot access that recovery request |
| N08 unrelated concurrent work | A and B each create a Block; A loads own history and reverses A's creation | A's Block disappears in both clients; B's Block and its identity remain intact |
| N09 affected-record/dependency conflict | A renames a Block; B renames it again; A attempts reversal. Repeat with A's Package and B's child Block | Reversal is rejected with a useful message; model, revision and history remain unchanged |
| N10 reversal of reversal | Reverse an eligible operation, then reverse that reversal; restart clients/server and reopen | Restored identity/state agree across clients and persisted history; repeated request does not duplicate revisions |
| N11 offline preservation | Separately create/edit/undo/save/reopen an offline project and representative diagrams from all nine families | Existing offline behavior remains functional and separate from the remote project |
| N12 accessibility and recovery feedback | Use keyboard-only navigation and narrow/large displays; inspect disconnected, viewer and pending states | Focus remains usable, labels/actions are clear, failed heartbeat does not erase drafts, pending requests cannot be overwritten |

Only N01–N12 for the existing bounded shared command surface are described here.
Ordinary-workspace/all-nine shared editing is still missing and needs its own
per-family acceptance after the [remaining implementation leaves](C20_COLLABORATION_COMPLETION.md).
Passing this table alone cannot close the full collaboration work package.

## Review and delivery

Review PR100, PR101 and PR102 as independent feature changes, then check the
combined candidate against main. The integration candidate contains their code;
it is not an additional competing implementation. Use one reviewed integration
path, retain the component PRs as evidence, and let the repository owner control
merge. No production signing keys, release activation or deployed HTTPS service
were introduced by these collaboration changes.
