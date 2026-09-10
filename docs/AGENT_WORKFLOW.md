# Agent setup and operating workflow

Status: Step 6 configuration prepared; execution disabled. Step 7 isolation and
client qualification remain outstanding. The setup branch was originally created
from the PR70 baseline; every future work order must use the actual current `main`
SHA at dispatch time. A setup baseline is never a product-work baseline.

## Sequence
1. Review/merge the setup PR after configuration checks and required CI.
2. Qualify the hosted environment using AGENT_ENVIRONMENT_GATE.md.
3. Record actual client/version, model availability and source access.
4. Only after qualification, explicitly enable delegation in a reviewed setup update.
5. Authorize a pilot work order. Lead assigns one leaf audit, implementer fixes one
   validated finding if needed, independent reviewer checks it, and supervisor prepares a PR.
6. User merges qualified changes. Resume from recorded checkpoints.

## Role ownership
Coverage ownership is intentionally non-overlapping. A role receives only a leaf
child check from its rows, never the whole parent row.

| Role | Coverage ownership |
| --- | --- |
| code_auditor | C01, C02, C19, C21, C22 |
| sysml_auditor | C03-C11 |
| behavior_auditor | C12-C15 |
| ux_auditor | C16, C18 |
| notation_auditor | C17 |
| collaboration_auditor | C20 |
| implementer | one validated finding or one explicitly specified small feature slice |
| reviewer | one candidate patch for one leaf work order |

The coordinator owns decomposition, dependencies, file ownership and task-level
model/reasoning selection; it does not perform implementation or independent review.

## Leaf-scope rule
`STEP5_AUDIT_COVERAGE.md` explicitly requires semicolon-separated topics to become
child checks such as C04.01. Those child checks are the normal dispatch unit.

A worker task must name one child ID, one concrete user/model/runtime workflow,
explicit allowed paths, acceptance evidence and at least one relevant negative or
rollback case. Do not assign requests such as "finish BDD", "audit all imports",
"fix collaboration", "clean the UI", or "complete runtime" as one task.

An audit may reveal more than one defect, but independent defects become separate
finding IDs and separate implementation work orders. An implementation patch fixes
one finding or one deliberately bounded feature slice. It may touch several files
only when they are all necessary for that single behavior. If the correct change
requires independent semantic, storage, UI, runtime or protocol changes, split it
into dependency-ordered patches before writing. Do not use arbitrary line-count
limits; use semantic and review boundaries instead.

A shared-contract child may intentionally span diagram families only when the shared
contract itself is the subject under test, for example Space-drag pan or a common
frame renderer. The implementation must still modify the shared authority rather
than duplicating family-specific fixes.

Before assigning a writer, the coordinator/supervisor checks current task records
and available PR state for overlapping production paths. Conflicting writers are
serialized or re-scoped. If current PR state cannot be checked, record that gap and
do not assume exclusive ownership of a shared module.

## Model routing
Agent role files intentionally do not pin a model or reasoning effort, and the
project `[agents]` configuration intentionally provides no model default. For every
leaf dispatch, the coordinator chooses both from the models actually available in
the current environment.

Choose based on the task itself: ambiguity, semantic/architectural risk, context
size, expected verification burden, and cost/latency. Routine, bounded and easily
verified work should use a lighter available configuration; difficult judgment,
cross-cutting reasoning or consequential review should receive stronger reasoning.
Do not create permanent role-to-model mappings. Escalate or re-dispatch only when
the evidence shows the first choice is insufficient or the task has changed.

Record the requested and effective model/reasoning effort for every dispatched task.
If task-level selection is unsupported or the requested setting is unavailable,
record the limitation and do not pretend the override took effect.

Start with at most three independent read-only specialists in parallel and serialize
shared-file writes. More agents do not guarantee lower subscription usage.

## Work order required before dispatch
Record:
- Task ID and one leaf Cxx.yy finding/feature scope.
- Current baseline SHA and isolated task branch/checkpoint.
- One concrete objective and explicit non-goals.
- Exact allowed production paths; tests/docs may be separately named.
- Acceptance scenario, failure/rollback case and relevant reference clause/figure.
- Effective role, requested/effective model and effort, dependencies and exclusive file ownership.
- Required focused checks and any integration/manual gate.
- Environment qualification record tied to the supervisor configuration.
- Stop condition, remaining correction budget and publication owner.

## Audit, implementation and review
Auditors inspect the recorded baseline and return evidence; they do not patch. The
coordinator validates reproduction and duplicates before implementation. Missing
large features require a bounded workflow specification and dependency split first.

The implementer writes only the assigned finding/slice. No opportunistic refactor,
adjacent cleanup, multiple unrelated fixes, or family-by-family copy of a shared fix.
The reviewer first checks scope. If unrelated changes or multiple independent fixes
are combined, return `SCOPE_TOO_LARGE` and require a split before technical approval.
Independent review uses the exact candidate commit and does not weaken tests or
requirements to accept it.

Focused verification runs during development. Required integration/CI gates run on
the final candidate head. Manual/visual gaps remain explicit; never report an
unexecuted test as passing.

## Checkpoint
Record task IDs, effective models, baseline/candidate SHAs, allowed paths, sources
used, findings, executed tests/results, unresolved decisions, active/conflicting PRs
when known, and the exact next action in a repository PR or task record. No session
continues unattended by assumption. Do not copy commercial reference text into GitHub.

## Protection limits
AGENTS.md and sandbox_mode are defense in depth, not proof of read isolation,
network isolation, connector restriction or immutable controls. The startup script
remains fail-closed until a trusted supervisor and Step 7 evidence exist. Do not
remove that block solely because configuration syntax or CI passes. Hosted GitHub-
only preparation does not prove worker capability or desktop visual qualification.
