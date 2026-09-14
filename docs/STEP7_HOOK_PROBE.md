# S7.02B — automatic project hook diagnostic

Status: PREPARED / runtime NOT VERIFIED. Baseline:
`5431a45aeb4b17544b8631246e3cff12c465075a`.

S7.02A showed synthetic Bubblewrap process filesystem feasibility. It did not
show that Codex tools use that boundary. Before implementing a supervisor launcher,
test whether the actual client loads and invokes project tool hooks at all.

This branch enables the hooks feature and adds one Bash diagnostic handler.
agents.enabled remains false. The existing delegation refusal guard and all role
definitions remain unchanged. The handler reads only its JSON stdin, emits a fixed
diagnostic, and never executes the supplied command, reads files, or starts workers.
It denies commands containing one fixed marker; unrelated understood shell inputs
return zero. Invalid payloads return a different diagnostic and cannot count as PASS.

## Independent setup review and hosted test

Review the four control/test changes plus this document before trusting this
diagnostic hook. Start a fresh Codex Cloud task on the candidate branch, not a
follow-up whose client configuration may already be loaded. If the client presents
a native project/hook trust review, review the exact definition using that supported
flow. Do not fabricate trust records, change global controls, or bypass hook trust.
If the required native flow is unavailable, record that limitation.

After verifying candidate HEAD, submit each as a separate direct shell tool call:

1. `printf '%s\\n' 'S7_02B_CONTROL_OK'`
2. `printf '%s\\n' 'S7_02B_PROBE_7F4C8D'`

Do not call the Python hook directly, simulate its JSON input, or wrap the marker
inside another script for this runtime test. Do not manually refuse the harmless
marker just because this document describes the expected denial: only actual
runtime interception is evidence. If higher-priority policy prevents the probe,
report NOT RUN and the reason.

Record tool results:
- Control prints its marker with exit zero.
- Probe is blocked by the runtime with S7_02B_HOOK_OBSERVED; its printf does not run:
  OBSERVED for this shell-hook path only.
- Probe prints its marker: NOT OBSERVED; do not activate or invent a workaround.
- Invalid-input, missing-file, trust, configuration or unrelated denial: BLOCKED or
  INCONCLUSIVE; preserve the exact sanitized error, not a PASS.

A successful result does not prove custom-role loading, effective model/effort,
immutability, filesystem sandboxing, network/connector isolation, hosted tool
coverage, or Step 7 qualification. No subagent test is authorized by this branch.
Remove or replace this diagnostic through a later reviewed setup change after its
result is recorded; it is not the production security policy.

## Static verification

`python3 scripts/test_agent_hook_probe.py` tests input handling, expected denial,
normal-command allowance, and absence of execution. Setup CI also preserves its
existing role/delegation checks and validates the new diagnostic configuration.
Passing those checks proves only the script/configuration contract.

## Source

Official hook documentation, consulted for this setup diagnostic:
https://learn.chatgpt.com/docs/hooks

It documents inline TOML hooks, Bash as the shell/unified-exec matcher, exit code 2
as a pre-tool denial, and native trust requirements. It also excludes hosted tools
from local hook coverage and warns that hooks are not a complete enforcement
boundary. Actual availability in this user's Cloud client remains the subject of
this test. No external product or standards audit claim is made.
