# Systems-Modeler-Pro agent instructions

Work only on rbrundage5/Systems-Modeler-Pro and approved read-only Project references.
Read docs/STEP5_SCOPE_AND_ACCEPTANCE.md, docs/STEP5_AUDIT_COVERAGE.md and
docs/AGENT_WORKFLOW.md. Historical PR documents are evidence, not current work orders.

## Execution boundary
Worker execution is DISABLED pending environment qualification. Do not spawn audit,
implementation or review workers until a trusted supervisor enforces and verifies
the boundary in docs/AGENT_ENVIRONMENT_GATE.md. Configuration and a successful
repository script do not prove isolation. No personal filesystem access, unrelated
repositories, browsing, new downloads or unrelated connectors. Do not modify or
publish uploaded references. Use only approved sources for evidentiary claims.
Missing information is a reported gap, not permission to search externally.

Setup-only edits may be prepared via this repository's GitHub tools without launching
workers. Do not install hooks on the user's computer or modify global configuration.

## Engineering
Rust owns semantics, validation, commands, transactions, routing/layout, persistence,
history and execution. Reuse existing systems. Preserve all nine families' authoring,
editing, save/reopen, import and runtime behavior. No arbitrary JavaScript line cap.
Use the active work order's baseline, allowed paths, acceptance and verification.
No overlapping writers on shared modules; no automatic merge or force push.

## Task scope
The normal unit of work is one leaf child check from STEP5_AUDIT_COVERAGE.md, not a
whole Cxx/Fxx/Axx parent. Each work order names one concrete workflow, exact allowed
paths, acceptance evidence and a relevant negative/rollback case. Large feature gaps
must be specified and split into dependency-ordered increments before implementation.

Auditors are read-only and return evidence. The implementer fixes exactly one
validated finding or one explicitly bounded feature slice; no opportunistic cleanup,
broad refactor, or unrelated second fix. If the correct change crosses independent
subsystems, stop and request a split. The independent reviewer checks scope first and
returns SCOPE_TOO_LARGE when a candidate combines unrelated work.

Coverage ownership is non-overlapping: code_auditor C01/C02/C19/C21/C22;
sysml_auditor C03-C11; behavior_auditor C12-C15; ux_auditor C16/C18;
notation_auditor C17; collaboration_auditor C20.

## Delegation and evidence
Lead chooses available models by scope/risk; see workflow. Give each agent a bounded
task and approved reference IDs. Findings need reproduction and expected behavior.
Implementers cannot approve their own changes. Report exact commits, executed checks,
manual limitations and next steps. Preserve unresolved findings. Do not claim a
feature complete from enum presence or a historical qualification claim.
Workers may not edit AGENTS.md, .codex/, enforcement scripts or workflow controls.
Only a separately scoped setup-maintenance task may propose those changes.
