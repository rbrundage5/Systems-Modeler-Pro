# Codex Cloud setup checkpoint

PR71 remains draft. Agent execution stays disabled. This update fixes coordinator
delegation and configures a PreToolUse hook for supported spawn_agent/Agent calls.
The hook calls the existing fail-closed preflight. Client loading is unverified;
this is not a general filesystem/network enforcement hook.

## Environment settings
Keep Systems-Modeler-Pro, universal image, caching on and agent internet off.
Choose Manual setup and enter:
```bash
bash /workspace/Systems-Modeler-Pro/scripts/codex_cloud_setup.sh
```
The file is currently on agent/step6-agent-setup, not main. Only test against that
branch. If the environment setup tester cannot select that branch, do not run a
main-based setup that references the missing file: use the full script from the
PR's Files changed tab in the setup editor, or defer until the file is merged.
No personal computer commands, secrets or global local configuration changes.
The default core profile skips Ubuntu desktop package installation and fetches
locked Cargo dependencies during the hosted setup phase. This is dependency
preparation only, not a passing build. To prepare desktop dependencies separately,
set SMP_SETUP_PROFILE=desktop in environment variables and retest setup. Desktop
build and visual qualification remain mandatory. The script retains configured
package sources and does not silently switch mirrors. It does not launch Codex, workers or application tests.
Setup may require network; agent-phase internet remains off.
A missing tool, origin mismatch or dependency failure stops the script.

## Bounded verification procedure
Use the environment's interactive setup terminal for initial verification, not an
audit/fix task. On the setup branch, run:
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
