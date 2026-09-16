# Collaboration completion work package

Authorized by the user's request to finish collaboration. Main baseline is
`2b9750e8b3ca6de76f86ec7f367f155513063862`; implementation continues from integration
candidate `b74264073aa85aed96cc3f1e05ade1e6fe5b785d` (PR99). Existing candidates
remain unmerged. Worker delegation remains disabled; independent review and an
owner-controlled merge remain required.

## Current implementation checkpoint — 2026-09-16

| Area | Candidate state | Review leaf |
| --- | --- | --- |
| Shared semantic edits, requirements and BDD | Implemented in the separate Shared Projects window; bounded command coverage | PR99 and its preceding leaves |
| Pending operation recovery after restart | Implemented, including actor/server isolation and exact retry identity | PR100, `a18a140da2ffdb0e0f31ffa665f292dbed4bba2e` |
| Authenticated project presence | Implemented with heartbeat expiry, viewer/editor labels and draft preservation | PR101, `c247b9fff29231002fed0203f3dd985146399198` |
| Own-change history and reversal | Implemented with affected-record and dependency checks; legacy operations unavailable | PR102, `05bf54622e4187b29879642735f3f56f10247536` |
| Ordinary workspace / all nine families | Not implemented; extraction and command coverage remain dependencies | C20.06–C20.07 below |
| Project administration and deployed HTTPS journey | Administrator CLI/configuration exists; user-facing workflow and deployed acceptance remain open | C20.08 |
| Independent review and two-device rendered acceptance | Outstanding; automated client/server tests do not replace these gates | [Acceptance record](C20_COLLABORATION_ACCEPTANCE.md) |

The combined review candidate contains the preceding installer/requirements work
and all three new collaboration leaves. Main has not been changed. The PR bodies
record final-head CI and installer evidence; this table records implementation,
not a full collaboration or release qualification claim.

## Completion contract and dependencies

Completion requires two authenticated desktop users to author a persistent shared
engineering project through the ordinary workspace, with all nine supported
diagram families and their existing Rust semantic rules. Both users must observe
committed changes, recover after disconnection and restart, understand conflicts,
and preserve unrelated work when undoing an edit. Viewer permissions and server
authorization must cover every mutation. Deployment, protocol compatibility and
native two-device acceptance are part of completion.

The existing server implements a bounded semantic/BDD command set. The full
ModelBuildPlan, authored presentation contracts and workspace command handlers
currently live in the desktop crate. Full workspace collaboration therefore
requires reuse/extraction of that authority; forwarding arbitrary JSON or
replacing a whole project snapshot is not an acceptable substitute.

Implement and review these dependency-ordered leaves separately:

1. **C20.03 durable pending-edit recovery:** retain exact operation identity across
   desktop restart, bind recovery to authenticated actor/server, and never send an
   edit when its local recovery record cannot be committed.
2. **C20.04 presence:** authenticated project-scoped sessions, bounded heartbeat
   expiry and visible participants; no semantic revision changes from presence.
3. **C20.05 shared mutation history/undo:** server-owned actor-scoped inverse
   operations with dependency/conflict checks; never rewind another user's work.
4. **C20.06 shared authored-state/command contract:** extract the existing Rust
   validation and command boundary needed by desktop and server, preserving native
   files and existing tests. Split further by command family before implementation.
5. **C20.07 ordinary workspace integration:** route shared mutations through that
   authority; retain local view/selection and isolate offline projects. Establish
   the BDD journey first, then IBD/Requirement/Use Case/Package, then the existing
   Activity/State/Sequence/Parametric command families as separate leaves.
6. **C20.08 administration/deployment and integrated acceptance:** publish/open a
   project, grant/revoke access, authenticated HTTPS deployment, compatibility and
   native two-device authoring/restart/conflict/undo acceptance across all families.

### Remaining extraction split

The ordinary workspace is not connected merely by converting a snapshot. Its
commands currently mutate desktop `WorkspaceState` and `ActivityWorkspaceState`,
while the server's bounded commands use `ProjectDatabase::commit_shared_edit`.
Shared BDD presentation records also differ from the desktop records. A second
semantic engine, JavaScript mutation layer or whole-snapshot overwrite would lose
the authority and concurrency guarantees established by the existing leaves.

Continue as separate, dependency-ordered work orders:

| Leaf | Shared authority to reuse | Required preservation/negative evidence |
| --- | --- | --- |
| C20.06a authored structural presentation contract | Existing DTOs and validators in `workspace.rs`, `workspace/package_diagrams.rs` and the portable interchange boundary | Existing serialized project records reopen unchanged; malformed identities, endpoints and contexts are rejected |
| C20.06b ordinary BDD command dispatch | Existing model-core operations plus desktop BDD authoring, routing and transaction paths | Desktop/server execute the same typed mutation; invalid command or route rolls back model, presentation, revision and inverse |
| C20.07a ordinary workspace BDD journey | Shared command dispatch plus session/recovery authority; local view and selection remain local | Two clients author through normal repository/canvas/properties; stale edits cannot mutate cached state or fall back to offline writes |
| C20.06c IBD contract and command dispatch | `workspace/ibd.rs`, feature/relationship editing, existing core connector/ItemFlow validation | Nested ports, occurrences and connector payloads preserve identity; invalid endpoints and concurrent deletion reject atomically |
| C20.06d Requirement / Use Case / Package commands | Existing family command and configuration paths, using the structural contract | Required metadata/context/relationship fields survive collaboration and reopen; role, conflict and inverse tests cover each family |
| C20.06e Activity authored state and commands | Existing ActivityRepository, activity workspace/mutation and bulk-model paths | Typed nodes/edges/regions and authored presentation share a transaction; runtime state is never synchronized as authored state |
| C20.06f State / Sequence authored state and commands | Existing BehaviorRepository and behavior workspace/completion paths, split into one family per writer | State regions/transitions and sequence occurrences/messages retain current semantic checks; failed apply preserves both repositories |
| C20.06g Parametric authored state and commands | Existing parametric/binding validation and workspace commands | Constraint definitions/usages and parameter endpoint identity remain distinct; invalid bindings and dependencies block commits/reversal |
| C20.07b–g remaining workspace journeys | The corresponding qualified shared command leaves | All-nine editing, import, save/reopen, history and runtime preservation; independent native evidence per family |
| C20.08a project lifecycle and administration | Existing trusted provisioning plus a specified authenticated management policy | Publish/open, grant/revoke and token lifecycle have an auditable permission boundary; revoked users cannot read, edit or retry a write |
| C20.08b deployment and integrated acceptance | Matching desktop/server builds and administrator-managed HTTPS | Actual two-device acceptance, reconnect/restart and service recovery; required independent review |

Before each implementation, narrow exact paths to that leaf, record the current
base/head and preserve existing tests. Broader import/profile/clipboard operations
must get explicit typed command/inverse coverage rather than silently forwarding
arbitrary payloads. This split is remaining work, not an executed test record.

The parent collaboration work remains open until all applicable leaves and native
acceptance have evidence. CI green on one leaf does not complete the parent.

## C20.03 work order

One workflow: an editor loses the application after sending an edit, reconnects
with credentials for the same actor, and explicitly retries the original operation
without duplication. Rejection remains definite; uncertain outcomes remain pending.

Allowed production paths: persistence collaboration capability and a dedicated
outbox module; desktop collaboration session and its recovery module; authenticated
server capability response; shared-project controller recovery feedback. Supporting
paths: focused persistence/server/desktop/frontend tests and collaboration docs.
No dependencies, CI/agent policy, model semantics, native project schema or runtime
behavior changes are included.

The outbox uses a separate Rust-owned SQLite database in application data. It stores
only server origin, authenticated actor, project and exact operation request. Access
tokens are memory-only. Atomic reservation prevents another application instance
from replacing an unresolved operation. An authenticated actor change must never
recover another actor's request. Definite completion clears the exact record only.

Acceptance covers process/session restart after server commit, restart before send,
same actor with a rotated token, actor/server isolation, rejected edit cleanup,
malformed storage, and a competing reservation. An unreadable or unwritable outbox
must fail before transmitting new edits. Existing transport/authorization tests and
native CI must pass at the published head. Native OS crash/power-loss and rendered
two-device acceptance remain separately recorded limitations.
