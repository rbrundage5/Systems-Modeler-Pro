# Publication and execution capability contract

Setup-maintenance leaf `SETUP-PUB-01`. Baseline:
`81819988179921958c7fe99f3c100cbf6eedf67c`.

The reported checkout had no remote and exposed only a `make_pr` metadata handoff.
The existing hosted setup explicitly tolerated the missing remote without adding
it. A separate worker-isolation block was then reported alongside publication.
These capabilities must be diagnosed independently; absence of merge access or
delegation is not evidence that all authorized primary-session work is blocked.

Allowed paths: `scripts/ensure_repository_origin.py`, its focused test,
`scripts/codex_cloud_setup.sh`, `.github/workflows/agent-setup-checks.yml`,
`AGENTS.md`, `docs/AGENT_WORKFLOW.md`, `docs/CODEX_CLOUD_SETUP.md` and this record.
This is supervisor-prepared setup maintenance, not a worker assignment. No
application behavior, role sandbox, agent configuration or refusal hook changes.

## Start-of-session capability check

Record each capability separately with observed evidence:

| Capability | Evidence needed | If unavailable |
| --- | --- | --- |
| Repository implementation | Authorized isolated checkout, actual read/write scope, baseline and toolchain | Continue only tasks permitted by the current environment; report the specific missing capability |
| Remote configuration | Approved effective origin fetch and push URLs | In the approved hosted checkout run `python3 scripts/ensure_repository_origin.py`; never replace an unexpected origin |
| Branch publication | Successful authenticated push or connected repository API write, with returned/verified commit and branch | Preserve local commits and a checkpoint; mark publication blocked and use the host's supported handoff if present |
| PR publication | Real PR URL/number and live repository/head/base metadata | A title/body record alone is `PR_HANDOFF_PENDING`, not a published PR |
| CI and review retrieval | Successful live check/review reads for the actual candidate | Mark those gates unverified; do not substitute local success or self-review |
| Merge | Exposed authorized operation, required checks/reviews and explicit merge authorization | Leave PR open for the user; continue other authorized work |
| Delegated workers | Trusted qualification required by `AGENT_ENVIRONMENT_GATE.md` plus supported enabled role dispatch | Do not launch workers; use permitted primary-session work and record independent review as outstanding |

Inspect advertised tools and their actual results. A tool's name, presence of
`gh`, an `origin` URL, a GitHub read response or a green build does not establish
write/merge permission. Do not perform dummy writes or merges to test permissions;
qualify publication by publishing the real reviewed task candidate. Do not
print tokens, credential-bearing URLs or authenticated command traces.

## Approved remote repair

The helper performs local Git metadata operations only. It checks that it is
called for the repository root with tracked project markers, and then adds the
canonical HTTPS origin only if absent. Existing valid origin configuration is
preserved. Existing effective fetch and push URLs must all target the approved
repository. When origin is missing, any matching Git URL-rewrite rule rejects
repair conservatively before adding it. Unexpected destinations reject without
changing configuration. It does not fetch, push, install tools, obtain credentials or
grant permissions. Checkout markers are an error-prevention check, not an
isolation or repository-authenticity attestation.

The hosted dependency setup invokes this helper after its existing approved
baseline check. For an already provisioned task, run the helper alone; rerunning
dependency installation is unnecessary. Never run hosted setup on personal files.

## Publication routes

1. Use an actually exposed connected GitHub integration for this repository when
   it supports branch/commit/PR publication. It need not depend on shell Git
   credentials. Verify the real PR and candidate head after publication.
2. Otherwise use already provisioned authenticated Git/CLI capabilities within
   the allowed environment. A missing remote can be repaired by the helper;
   missing credentials or denied network access cannot be repaired by that helper.
3. If only a metadata-only `make_pr` tool exists, record the local branch/commit,
   its result and `PR_HANDOFF_PENDING`. A repository script cannot expose a new
   host tool or provision account access. Continue useful permitted local work;
   report the exact host connection needed when no further useful work can proceed.

Never claim a local merge, synthetic integration commit, PR metadata record or
successful push is a GitHub PR merge. Only live GitHub state can establish that
the intended PR was merged. Merge authorization remains separate from publication.

## Primary-session continuation and agents

Worker isolation remains blocked until independently qualified. The read-only
registered coordinator does not become a writer. An authorized primary session
with its own permitted repository-write scope may perform bounded implementation
directly without dispatching workers or pretending to be an independent reviewer.
This does not add permissions to a restricted session or lift any source, network,
filesystem or publication boundary.

Treat each bounded PR as a checkpoint. While a PR awaits CI, review or a user
merge, continue a permitted independent leaf. Do not stop at an unavailable merge
operation when publication/local implementation can proceed. If every useful task
is blocked, name the specific external capability or approval needed, preserving
the exact commits and next action. Do not claim unattended continuation after the
session ends.

## Verification and remaining limits

The focused tests cover missing/present origins, idempotence, foreign fetch/push
destinations, multiple URLs, URL rewrites, missing tracked markers, nested paths
and non-repositories using disposable Git fixtures inside the test checkout.
They perform no network requests. Existing agent-setup CI still requires disabled
workers, the unchanged hook and exit-2 refusal behavior, and now runs these tests.
Local result: all nine Git-fixture tests passed; the existing configuration/refusal
contract, shell syntax and diff/link checks passed. The complete hosted dependency
installer was not run, and its environment was not reconfigured from this session.

The connected session used to prepare this change can read live PR/check/review
state and publish repository branches/PRs. That does not grant the same tools to
the different checkout described in the report. Remote setup is not account
authentication. No external supervisor qualification, native modeling acceptance,
independent review or automatic merge is claimed by this setup-only change.

Historical open setup PR81/PR83 also touch agent-setup CI. They are not included or
qualified by this candidate; their hook/configuration changes need reconciliation
against current main before any future merge. The new test is appended without
changing the existing disabled-agent assertions or historical hook behavior.
