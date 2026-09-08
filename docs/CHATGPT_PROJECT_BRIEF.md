# Systems Modeler Pro — ChatGPT Project brief

Source repository: https://github.com/rbrundage5/Systems-Modeler-Pro
Documentation entry point: https://github.com/rbrundage5/Systems-Modeler-Pro/blob/main/docs/README.md
Prepared 2026-09-08. This is stable project context, not a live status report.
If supplied from an unmerged PR, use that PR branch for review; main remains the
implementation baseline until the PR is merged.

## Goal

Develop a professional SysML systems-engineering tool with CATIA/Cameo-class
workflow depth while preserving existing working behavior. Planned work includes
full code/functionality, SysML semantics, notation/visual, and behavior audits;
verified fixes; complete feature additions; UI/UX cleanup; and collaboration.

## Architecture and preservation

Rust owns semantic state, validation, commands, routing/layout, history,
persistence, and execution. Tauri frontend code renders and captures input.
Reuse established core, transaction, workspace, and execution systems. Preserve
SQLite project compatibility, stable semantic identity, offline desktop operation,
and existing authoring/editing/import workflows. Collaboration direction includes
peer/LAN operation and optional on-premises hosting.

Cover BDD, IBD, Requirement, Use Case, Package, Activity, State Machine, Sequence,
and Parametric workflows. Existing baseline documents identify SysML 1.6 and
inherited UML 2.5.1; future standards audits must consult the actual references.
Reference-product behavior does not establish normative correctness.

## Working method

Read current repository instructions if present, the documentation index, and
relevant code/tests before deciding current status. Record the baseline commit.
Distinguish implemented, exposed in UI, verified, defective, missing, and unverified.
A parser/enum or a passing unit test alone does not prove complete user workflow.

Within an authorized work package, the lead may delegate bounded audit, fix,
and independent review tasks, select available models by difficulty/risk, and
escalate when needed. Minimize duplicate exploration and keep ownership of shared
files explicit. Audit findings need evidence and acceptance criteria before fixes.
Large features need complete workflow specifications and dependency-ordered PRs.

Current preparation order: documentation/source cleanup, then agent setup,
then a bounded pilot. This brief does not itself authorize starting implementation,
merging, deployment, or an unattended background process. Follow the user's active
work order and existing authorization; routine handoffs inside it need not be
reconfirmed. Do not merge automatically.

## Source freshness

GitHub is authoritative for versioned project material. Uploaded copies and old
chat summaries are snapshots. Fetch current sources before technical decisions;
if unavailable, identify the limitation. Historical PR checklists/status reports
are not the current task queue. Keep stable context here; keep current findings,
commits, tests, dependencies, and next steps in repository/GitHub records.
