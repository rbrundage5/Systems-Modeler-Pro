# Agent setup and operating workflow

Status: Step 6 configuration prepared; execution disabled. Step 7 isolation and
client qualification remain outstanding. Baseline: PR70 merge dda6f4885c7539569522b85976be49e4525d066d.
No product audit or fix has started.

## Sequence
1. Review/merge this setup PR after configuration checks and required CI.
2. Qualify the hosted environment using AGENT_ENVIRONMENT_GATE.md.
3. Record actual client/version, model availability and source access.
4. Only after qualification, explicitly enable delegation in a reviewed setup update.
5. Authorize a pilot work order. Lead assigns, auditor reports, implementer fixes
   validated findings, reviewer checks, supervisor integrates and prepares a PR.
6. User merges qualified changes. Resume from recorded checkpoints.

## Model routing
Candidate defaults are explicit in role TOMLs. They are not account availability
claims. Terra medium handles bounded everyday work; Sol high handles ambiguous
semantics, execution and review. Luna low/medium may handle extraction or summaries.
Astra is an optional escalation for difficult architecture, not a default.
All model substitutions require checking actual availability first.
Custom-agent model settings can override spawn choices: use a reviewed role variant
or correctly configured explicit worker session for substitutions; record the
effective model/effort. Never pretend a requested override took effect.

Escalate after two unsuccessful focused correction cycles, or immediately for
cross-cutting consistency/semantic uncertainty. Send prior evidence so the next
worker does not repeat discovery. Do not keep retrying without a new hypothesis.
At most three spawned workers initially; serialize shared-file writes. More agents
do not guarantee lower subscription usage. Do not claim access to quota telemetry.

## Work order (required before dispatch)
- Task ID, scope and parent Cxx/Fxx/Axx from Step 5.
- Baseline SHA and task branch; exact allowed files/directories.
- Objective, non-goals, acceptance scenarios and negative cases.
- Reference IDs/clauses; source access confirmed for this worker.
- Effective model/effort, assigned role, dependencies and exclusive file ownership.
- Required focused checks and integration gates; manual acceptance plan.
- Environment qualification record tied to the supervisor configuration.
- Stop condition, remaining attempt budget and publication owner.

## Findings and implementation
Audit the recorded baseline; create child checks for every assigned subtopic.
Lead validates reproduction and duplicates before dispatch. Missing full features
need a specification first. A finding can be fixed while unrelated audit work
continues, in isolated checkouts; review must use the resulting candidate commit.
No invented bugs when the pilot finds none. Independent review precedes integration.
Read-only auditors may request supervisor-run tests in a disposable workspace;
read-only does not mean tests can write caches wherever they want.

## Checkpoint
Record task IDs, effective models, baseline/candidate SHAs, allowed paths, sources
used, findings, executed tests/results, unresolved decisions and exact next action
in a repository PR or task document. No session continues unattended by assumption.
No need to copy whole conversations or commercial reference text into GitHub.

## Protection limits
AGENTS.md and sandbox_mode are defense in depth, not proof of read isolation,
network isolation, connector restriction or immutable controls.
The startup script is a fail-closed placeholder until a trusted supervisor exists.
Do not remove that block solely because configuration syntax passes.
Hosted GitHub-only preparation does not prove hosted worker capability.
