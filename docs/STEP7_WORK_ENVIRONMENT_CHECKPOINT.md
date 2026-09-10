# Step 7 Work environment checkpoint — 2026-09-10

Status: **BLOCKED / NOT QUALIFIED**. Evidence-only setup maintenance; no product
changes, worker launches, configuration activation, or automatic merge.


## Current status after PR80

Current main observed through GitHub: `5431a45aeb4b17544b8631246e3cff12c465075a`.
PR80 is merged; its tested candidate was
`428910e37343c1194fca679c2bacb3c6a064253d`.

This section supersedes earlier next-action instructions below. Those sections
preserve historical observations from Work and the initial Codex handoff.

- The user supplied actual Codex Cloud inventory at the PR78 baseline:
  the configured /workspace/Systems-Modeler-Pro root exists, all ten TOMLs parse,
  the refusal script returns its expected diagnostic, and core Rust tools exist.
  This corrects the applicability of earlier Work-only missing-root/tool findings.
- Desktop setup initially failed on hosted package-proxy HTTP 502 responses.
  A subsequent setup supplied all checked desktop prerequisites.
- The user supplied a successful offline locked desktop Cargo check on the exact
  PR80 candidate: exit 0, 6m 04s, clean worktree.
- A separate user-supplied Codex review returned PASS with no findings, validating
  PNG provenance, bounded scope, preserved existing CI, and Linux compile coverage.
  This review covers PR80 only; it is not independent review of this document.
- Before PR80 merged, direct GitHub checks reported core, desktop-linux-check,
  desktop-check and configuration all successful on its candidate.

Remaining gates: actual custom-role loading/addressability, effective per-task
model/effort evidence, automatic guard behavior, enforced worker boundaries,
reference mounts, concurrency/reviewer controls, resumable attestation, and desktop
visual/manual capability. Codex's earlier live GitHub access gap also remains open;
GitHub access from Work does not establish access from Codex.

Next: resolve the supported hosted agent runtime and enforcement mechanism before
another worker qualification attempt. Do not repeat completed desktop build checks
on the unchanged candidate without a concrete reason. No agent activation follows
from PR80 or this evidence-only document.

PR79 remains a documentation checkpoint, independent of product/build fixes.
It may be reviewed and merged while environment qualification remains blocked;
merging it does not enable delegation. Independent document review is outstanding.

## Baseline and live GitHub evidence

- Baseline main: `78807b20310a38e5da166383b906e0dd6e825c79` (PR77 merge).
- Open PR search before this checkpoint returned PR78 only.
- PR78 head: `ec0a8f42c316a712f986a56a43163f3f9b2cbaff`.
- PR78 agent-setup-checks #40: success, [run 34469015799](https://github.com/rbrundage5/Systems-Modeler-Pro/actions/runs/34469015799).
- PR78 native-foundation-ci #1228: success, [run 34469015680](https://github.com/rbrundage5/Systems-Modeler-Pro/actions/runs/34469015680).
- A first PR metadata response reported mergeable=false; the subsequent REST
  pull response reported mergeable=true and mergeable_state=clean. The latter
  is the readiness observation. Recheck before merging; main advanced through PR77.
- Main agent-setup-checks #43: success. Main native-foundation-ci #1231:
  in_progress at inspection, not claimed passing.
- GitHub branch response reported main unprotected with no required check contexts.
  Both named workflows passed for PR78; this is not branch-protection enforcement.

Read current docs/README.md, AGENTS.md, docs/AGENT_WORKFLOW.md,
docs/AGENT_ENVIRONMENT_GATE.md, docs/AGENT_REFERENCES.md,
docs/STEP5_SCOPE_AND_ACCEPTANCE.md and docs/STEP5_AUDIT_COVERAGE.md.
Configuration, preflight, setup script and setup CI were read at the baseline SHA.
Historical completeness claims were not used.

## Observed environment

Client: ChatGPT Work, hosted Linux shell. Client build/version and supervisor
policy digest are unavailable. These omissions prevent trusted attestation.
Current task directory: `/workspace/scratch/3c2c4b04ea06`.

| Test | Expected | Actual and disposition |
| --- | --- | --- |
| S7.01 configured repository root | Existing isolated approved checkout | Python isdir('/workspace/Systems-Modeler-Pro') returned false. FAIL for this environment. |
| S7.02 exact configured guard command | Execute repository refusal guard | python3 /workspace/Systems-Modeler-Pro/scripts/agent_preflight.py exited 2 with file-not-found. FAIL: this is not the guard's BLOCKED diagnostic. |
| S7.03 client/toolchain discovery | Available client and Rust tools | shutil.which found no codex, cargo or rustc. BLOCKED; no Rust build/test executed. |
| S7.04 supporting tools | Available setup prerequisites | git 2.51.1 and node v24.19.0 available; pkg-config absent from PATH. Partial inventory only. |
| S7.05 desktop qualification | Hosted desktop/test capability | Xvfb absent from PATH and DISPLAY unset. Desktop execution/rendered/manual tests NOT RUN. |
| S7.06 GitHub supervisor read access | Read this repository and live CI | PASS for branch, PR, workflow and source retrieval through GitHub connector. Does not qualify worker credentials or git clone/push transport. |
| S7.07 automatic config/instruction loading | Client loads project controls and roles | NOT VERIFIED. Documents were explicitly fetched, not automatically loaded from a checkout. |
| S7.08 role addressability | Named TOML role dispatch supported | NOT VERIFIED. Hosted spawn interface advertises tasks/model/effort but no project-role selector or per-worker filesystem policy. No worker launched to bypass the gate. |
| S7.09 model routing | Requested/effective model and effort verifiable | NOT VERIFIED. Task overrides advertised; no execution receipt obtained. Requested/effective worker model and effort: N/A, no dispatch. |
| S7.10 isolation and immutable controls | Enforced per-worker path/tool/network boundaries | BLOCKED. No trusted supervisor policy/attestation exposed; current shell has unrestricted filesystem permissions. No synthetic denial suite executed. |
| S7.11 reference availability | Approved inputs readable and immutable for workers | Six supplied inputs readable/hashable by lead; worker read-only mounts, editions and immutability NOT VERIFIED. |
| S7.12 ownership/review/checkpoint | Enforced writer exclusion and independent review | Checkpoint preserved; worker leases, reviewer separation, expiry/restart and descendant restrictions NOT VERIFIED. |

S7.02 reproduces the exact command from .codex/config.toml. Setup CI instead invokes
the script relative to its checkout and separately asserts the hard-coded command
string. Passing that CI does not verify the configured hook executes in Work.
scripts/codex_cloud_setup.sh also starts by changing to the absent absolute root.
It was inspected but not run: it includes dependency downloads and system setup.
No personal/global settings, dependencies, hooks or source uploads were modified.

Network restrictions are advertised by the host, but repository-only worker
connector isolation and model-transport restrictions have not been demonstrated.
Do not substitute the lead's available connectors or instruction compliance for
an enforced worker boundary.

## Approved input digests

SHA-256 of supplied copies; hashing confirms lead readability only. No source
contents are published. Exact editions still require verification in the future
qualified reference environment.

| Input | SHA-256 |
| --- | --- |
| Uploaded CHATGPT_PROJECT_BRIEF.md snapshot | 86c847cf2499865a863715ec9c02b86cb16f1f49447f75c05beae2fbb9832844 |
| CATIA Magic guide | 8342557e2cf34bd25ab7b191115e7eee773f36f69e9caf61fd0622b518664daa |
| formal-19-11-01.pdf | b42cca045093046877768fb8915793e77a2640346be04fec067f81bbc9d3868f |
| SysML Distilled supplied copy | e362c49b5ffa4f420b047f0b4cfa8c0d5fa4e16605e1532a32a00b7ed3f1d379 |
| formal-17-12-05.pdf | 416b57e1933780eb48bd60fe513e031da220c28a521bdd334a366bebc78a463e |
| Practical Guide supplied copy | 75c363e13ca8082e5e89a614828885d301b470b8d0f1c08e83e430d304e74cde |

## Target execution environment

The user confirmed during this checkpoint that all future work will move to Codex
once the agents are ready. Codex is therefore the qualification target. Work is
used here for setup preparation and evidence recording; its missing executables
or checkout are not proof of a defect in the future Codex environment.

Final readiness necessarily includes a setup-only qualification session in the
actual selected Codex client/environment before production delegation. Record
which Codex client/version is used rather than assuming all clients support the
same TOML roles, hooks, model receipts or isolation controls. No Work-specific
fallback or weakened gate is introduced.

## Resume work orders

1. User may merge PR78 after checking its live head/clean status. This checkpoint
   does not depend on or alter its six changed files.
2. S7.01 environment integration: provide an isolated hosted Systems-Modeler-Pro
   checkout with a supported client, preinstalled Rust/desktop dependencies and
   trusted supervisor controls. Record the real canonical root, client version and
   policy digest before proposing a portable setup/guard invocation. Merely creating
   the missing directory or changing a path cannot qualify the environment.
3. S7.02 enforcement qualification: under that supervisor, execute every synthetic
   negative/positive case in AGENT_ENVIRONMENT_GATE.md, including reference/control
   immutability, traversal/symlink escapes, connector/network denial, worker
   credentials, model receipts, role loading and descendant restrictions.
4. S7.03 runtime qualification: execute recorded core/desktop build/test checks;
   verify independent reviewer isolation, writer leases and resumable evidence.
5. Only after all applicable gates have evidence, prepare a separate independently
   reviewed delegation-enablement PR. Product feature work remains separate.

Allowed path for this change: this checkpoint document only. Independent review
is outstanding; disabled worker execution was respected. No passing qualification
or feature-completeness claim follows from publishing this document.

## Post-PR78 handoff update

GitHub confirmed PR78 merged. New main baseline:
`9b7d498f4e4a0b8502b82ac2d7998e4e959a33e8`.
At this follow-up check, agent-setup-checks run 34471094330 succeeded and
native-foundation-ci run 34471094422 was in progress. The earlier observations
above remain historical evidence, not current main status. The checkpoint branch
still originates at the recorded PR77 baseline; this update does not claim a rebase.
Current main AGENTS.md and AGENT_WORKFLOW.md were reread after the merge.

### First Codex qualification task

Scope: S7.01 only — establish the actual client, repository and enforcement
capabilities. Start from live main and record its SHA. Read the documentation index,
AGENTS.md, workflow, environment gate, references and Step 5 scope/coverage.
Read this checkpoint from PR79 if it has not been merged.

Keep agents disabled. Do not run product audit, implementation or review workers.
Inspect supported client configuration and advertised supervisor capabilities
without starting a worker. Verify the canonical checkout path, current client
version, role discovery mechanism, per-task model/effort selection and effective
settings evidence, filesystem/tool/network enforcement, immutable controls,
preinstalled build tools and hosted desktop capability. Distinguish unavailable,
unverified and demonstrably supported behavior.

Do not execute dependency-installing setup scripts merely to collect inventory.
Do not change personal/global configuration. Do not replace unsupported TOML role
dispatch with ad-hoc agents. If a trustworthy enforcement mechanism is unavailable,
record the precise missing capability and stop before worker startup.

Publish sanitized observations as a setup-only checkpoint on a separate branch
from current main (or extend PR79 only after checking its current state and ownership).
No production paths or activation controls may change in this inventory task.
Return baseline, client/version, actual root, executed checks, evidence for each
claim, unresolved blockers, and the exact next bounded integration/qualification
task. PR79 remains an evidence document requiring independent review; merging it
does not qualify or activate the agents.

## S7.02A supplied filesystem feasibility evidence

The user supplied a Codex Cloud report at main
`5431a45aeb4b17544b8631246e3cff12c465075a`: FEASIBLE for synthetic process
filesystem isolation with /usr/bin/bwrap, Bubblewrap 0.9.0.

Approved reference reads and assigned-directory writes succeeded. Reference/mock
control writes and deletes, outside-canary reads/writes, traversal and symlink
access were denied. An ordinary child shell inherited the tested restrictions.
Host-side canaries remained unchanged and temporary fixtures were removed.

A probe requiring a fresh /proc mount failed with Operation not permitted. The
successful narrower probe omitted /proc and /dev. An initial runner's redirection
to absent /dev/null produced invalid test outcomes; only the corrected runner's
results count. Network namespaces were requested, but network denial itself was
not tested. Environment sanitization, inherited file descriptors, real toolchain
execution and model-tool integration were not established.

This is user-supplied evidence for the tested mount namespace, not evidence that
hosted Codex tools or subagents are forced to execute through it. A repository
launcher that can be bypassed by direct model tools is not a trusted supervisor.
No actual worker-model dispatch occurred; requested/effective model and effort N/A.

Next bounded integration check: establish actual client loading and invocation of
a diagnostic project PreToolUse hook on an ordinary synthetic shell command.
Keep agents disabled and the existing refusal guard intact. This check would
verify only a local tool hook; connector/hosted-tool restrictions and immutable
supervisor enforcement remain separate gates.

## S7.02B result — NOT OBSERVED

User-supplied independent review of PR81 candidate
`a85de1a6f688e62d879ac2e8ba363e8c1017dbeb` against
`5431a45aeb4b17544b8631246e3cff12c465075a`: PASS, no findings.
Role definitions and original delegation refusal were unchanged.

In a hosted task at that exact candidate, separate direct shell calls both exited
zero and printed their respective control/probe markers. The runtime did not emit
S7_02B_HOOK_OBSERVED or prevent the probe's printf from executing. Worktree remained
clean; no workers, dependencies, edits or bypass were reported.

Conclusion: project PreToolUse invocation NOT OBSERVED in the tested configuration.
This is not a code-review failure and not proof that every Codex Cloud configuration
lacks hooks. Loading, trust, supported client surface and tool-path behavior remain
possible causes; the supplied result does not identify which.

Do not merge PR81 as an operational enforcement fix. Keep delegation disabled.
Do not repeat the same probe unchanged or build a launcher assuming interception
exists. Next action requires an identified supported native project-config/hook
trust and loading mechanism for this hosted client, or a separately scoped hosted
runtime integration. A platform capability/support response is needed if that
mechanism is not exposed. Personal-machine installations remain outside scope.
