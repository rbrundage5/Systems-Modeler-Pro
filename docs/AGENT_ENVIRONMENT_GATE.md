# Step 7 environment qualification

Status: BLOCKED / NOT VERIFIED. No trusted worker supervisor has been configured.
Current chat tool availability is broader than the requested worker boundary.
Do not start workers or call this setup operational until every applicable item
below has evidence. Do not change the user's personal machine or global settings.

## Required external enforcement
A trusted supervisor must provide isolated hosted task checkouts, no personal/home
mounts, read-only approved reference mounts, preinstalled toolchain/dependencies,
and worker credentials with no GitHub publication authority. Worker filesystem
writes are limited to assigned task paths plus isolated build/temp directories.
Git metadata and control files are supervisor-owned. Resolve canonical paths and
reject symlink/path-traversal escapes. Deny network/tool egress except a supervisor-
controlled model transport that cannot serve as arbitrary browsing or data access.
Remove unrelated MCP/connectors and do not expose secrets in environment or logs.
The repository-scoped publication supervisor validates diffs before pushing.
A content policy or ordinary Git hook cannot substitute for these controls.

## Tests using synthetic data only
- Outside-root canary read and write denied.
- Traversal, symlink and alternate absolute-path escape denied.
- Read-only reference write/delete denied; reference contents never published.
- AGENTS/config/hook/policy and Git metadata changes denied to workers.
- Network browsing, dependency download and unrelated connector/repository access denied.
- Approved in-scope read/write succeeds; assigned-path violation denied.
- Build/test temp/cache writes stay inside allocated scratch.
- Spawned agents inherit restrictions; worker cannot spawn unrestricted descendants.
- Missing/expired qualification and changed supervisor policy prevent startup.
- Effective model/effort and actual Codex configuration loading verified.
- Required sources are available and hashed per run; absent sources produce gaps.
- Application build/test and hosted desktop access checked without personal mounts.

Record for each: test ID, environment/client version, supervisor policy digest,
baseline, expected/actual outcome and sanitized evidence. Do not create a fabricated
PASS record. Repeat after boundary/configuration changes. A repo JSON flag is not
trusted attestation because a worker might edit it.

## Current deliverable boundary
scripts/agent_preflight.py deliberately exits nonzero. The script neither installs
a sandbox nor launches agents. A separately reviewed supervisor integration must
replace this placeholder with verified enforcement and tests. Do not invent hook
event names for an unverified client. During setup, configuration-only PRs through
GitHub remain permitted; worker execution and pilot remain blocked.
