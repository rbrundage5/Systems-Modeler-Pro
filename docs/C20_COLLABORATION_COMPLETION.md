# Collaboration completion work package

Authorized by the user's request to finish collaboration. Main baseline is
`2b9750e8b3ca6de76f86ec7f367f155513063862`; implementation continues from integration
candidate `b74264073aa85aed96cc3f1e05ade1e6fe5b785d` (PR99). Existing candidates
remain unmerged. Worker delegation remains disabled; independent review and an
owner-controlled merge remain required.

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
