# S7.03 — Portable delegation-refusal command

Status: setup candidate; delegation remains disabled. This change fixes only
hook-command path resolution. It does not qualify native hook invocation, role
dispatch, permissions, models, or desktop behavior.

## Observed input

The user reported a GitHub Codespace checkout at
/workspaces/Systems-Modeler-Pro, commit
9383c9e3f8dd6f925cc4706f172606ca6dac97a3, Codex CLI 0.154.0,
and successful ChatGPT login. These are user-supplied observations; this Work
session did not access that Codespace or its credentials.

The previous hook command hard-coded /workspace/Systems-Modeler-Pro. The mismatch
prevented it from locating the refusal script in the reported Codespace.

## Change and verification

Resolve the repository root with git rev-parse --show-toplevel, then invoke the
existing script using a quoted absolute path. Failed Git discovery prevents the
Python command from starting. The original hook matcher, disabled setting,
concurrency cap, role files and scripts/agent_preflight.py remain unchanged.

Run:

    python3 -m unittest discover -s scripts -p 'test_agent_preflight_path.py' -v

The tests execute the actual configured shell command in disposable Git fixtures:
both hosted directory layouts, a nested working directory, and a path containing
spaces must return the existing BLOCKED diagnostic and exit 2. Missing Git context
must not invoke Python; a missing guard must fail without claiming it executed.

These are synthetic command tests, not native Codex interception tests. The Git
root identifies a checkout; it is not a repository-identity or sandbox guarantee.

## Scope and next gate

Allowed changed files: .codex/config.toml,
.github/workflows/agent-setup-checks.yml, scripts/test_agent_preflight_path.py,
and this document. No workers or product tests are needed for this path-only fix.
An independent review of the exact candidate and its CI is required before merge.

PR81 is paused diagnostic work and overlaps the config/CI paths. Do not merge it
unchanged after this fix; reconcile its hard-coded diagnostic path and its guard
equality assertion in a separate reviewed update if that probe is resumed.

scripts/codex_cloud_setup.sh is intentionally still limited to its configured
/workspace root. Do not run it in Codespaces. Dependency installation and setup
script portability are outside this hook-only change.

After review, inspect actual project loading and native hook trust in the hosted
CLI. Do not bypass trust or launch workers to test the refusal. First use a bounded,
independently reviewed non-delegating diagnostic. All requirements in
AGENT_ENVIRONMENT_GATE.md remain outstanding unless separately evidenced.
