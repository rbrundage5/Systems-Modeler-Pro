# Step 7 Work environment checkpoint — 2026-09-10

Status: **BLOCKED / NOT QUALIFIED**. Evidence-only setup maintenance; no product
changes, worker launches, configuration activation, or automatic merge.

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
