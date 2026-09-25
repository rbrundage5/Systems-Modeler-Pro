# Retire obsolete PR38 source-writing automation

Task: `C01.EXEC.01`, separately scoped setup-maintenance candidate, 22 September 2026.
Baseline: `848b9e9650627ff819cea3d2e2cff382fa77d3af`.

The user's execution-scope review identified a one-time PR38 bootstrap workflow
that remains tracked after its spreadsheet implementation has become ordinary
application code. On pushes to `agent/pr38-spreadsheet-mapping`, it requests
`contents: write`, applies historical text replacements to source, formats/tests,
then commits and pushes to that branch. It also attempts to delete itself and its
helper. This was historical build automation, not evidence of malicious code, but
it has no ongoing product or qualification purpose.

Allowed changes for this maintenance leaf:

- Delete `.github/workflows/pr38-bootstrap.yml`.
- Delete its sole helper, `scripts/pr38_bootstrap.py`.
- Add this scope, rationale and verification record.

The deletion prevents the obsolete mutation path from being present on branches
that incorporate this commit. It does not rewrite or remove an older remote
branch that still contains a historical copy; such a branch must not be reused as
an automation entry point. No other workflow, agent control, release gate,
application code, dependency, or user file is changed.

Verification: repository-wide reference inspection found the helper was invoked
only by this workflow. After deletion, every remaining workflow that existed at
the baseline is byte-identical, and the changes are exactly the two deletions and
this record. Whitespace/diff checks pass. Source/runtime tests are unnecessary
for deleting this isolated historical bootstrap, and none are claimed as proof
of an application change. Existing normal spreadsheet tests and release/build
workflows remain available.

Acceptance: the current source tree contains neither executable bootstrap file
nor an active workflow invoking that helper, while all seven normal baseline
workflows remain unchanged. Negative case: no retained source or workflow
references a missing helper. Inspect the final PR scope and required checks
before merge. Independent review and merge are outstanding; no worker was used.
