# Codex Cloud setup checkpoint

The setup files are tracked on main. Agent execution stays disabled. The candidate
configuration defines a PreToolUse hook for supported spawn_agent/Agent calls.
The hook calls the existing fail-closed preflight. Client loading is unverified;
this is not a general filesystem/network enforcement hook.

## Environment settings
Keep Systems-Modeler-Pro, universal image, caching on and agent internet off.
Choose Manual setup and enter:
```bash
bash /workspace/Systems-Modeler-Pro/scripts/codex_cloud_setup.sh
```
Use the current merged script, or an explicitly selected setup-maintenance branch
when testing its candidate. Record the actual commit used by the environment.
No personal computer commands, secrets or global local configuration changes.
The default core profile skips Ubuntu desktop package installation and fetches
locked Cargo dependencies during the hosted setup phase. This is dependency
preparation only, not a passing build. To prepare desktop dependencies separately,
set SMP_SETUP_PROFILE=desktop in environment variables and retest setup. Desktop
build and visual qualification remain mandatory. The script retains configured
package sources and does not silently switch mirrors. It does not launch Codex, workers or application tests.
Setup may require network; agent-phase internet remains off.
A missing tool, origin mismatch or dependency failure stops the script.

Setup now adds the approved HTTPS origin when absent, after checking the tracked
baseline. It preserves a valid origin and rejects unexpected effective fetch/push
URLs. Remote configuration does not supply GitHub authentication. An already
provisioned task can run `python3 scripts/ensure_repository_origin.py` alone without
installing dependencies. See [the capability contract](PUBLICATION_AND_EXECUTION_SETUP.md)
for connected GitHub publication, metadata-only make_pr handoffs, unavailable merge
operations and permitted direct work when delegated workers remain disabled.

## Bounded verification procedure
Use the environment's interactive setup terminal for initial verification, not an
audit/fix task. On the selected verified setup baseline, run:
```bash
cd /workspace/Systems-Modeler-Pro
git rev-parse HEAD
bash -n scripts/codex_cloud_setup.sh
python3 scripts/agent_preflight.py
```
The last command MUST exit 2 and print BLOCKED. This proves script behavior only,
not hook invocation. Do not enable agents or remove the block to test delegation.

After dependency setup, supervisor-run build checks may use:
```bash
cargo test --offline --locked --workspace --exclude systems-modeler-desktop
```

Only after the desktop profile completes:
```bash
cargo check --offline --locked -p systems-modeler-desktop
```
These checks may write build artifacts inside hosted infrastructure. They do not
prove desktop visual acceptance, reference access or worker isolation.

## Remaining gates
Record actual client/version and whether cloud tasks load project agent TOMLs and
inline hooks. Confirm actual hook invocation using a supported client diagnostic
or controlled supervisor test, with worker execution disabled. Verify effective
model availability without launching unqualified workers. Do not infer support
from TOML parsing or a matching filename.

The UI screenshot confirms repository, universal image, caching and internet off.
It does not establish read isolation, immutable rules, connector restrictions or
approved reference mounts. All synthetic negative-access tests in
AGENT_ENVIRONMENT_GATE.md remain pending and must be run by a trusted supervisor.
If cloud cannot supply these controls, report the exact unsupported requirement;
do not substitute prompts for enforcement or start workers.

ChatGPT uploaded PDFs are not automatically mounted in Codex Cloud. Reference
provisioning remains unresolved; do not upload commercial books into the public
repository. No complete operational setup claim until all required gates pass.
