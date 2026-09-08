# Documentation review register

Reviewed 2026-09-08 at commit `82e1476ee60ed5213f9f583c01a4730ee30daf9a`.

## Scope and limits

Inventory covers all 31 tracked `.md`, `.txt`, `.rst`, and `.adoc` files at this baseline, including the icon README. Reviewed documentation status, internal contradictions, repository layout, and selected supporting code/tests. Long import contracts were checked for structure, baselines, status contradictions, and selected implementation references; their individual schema rules were not exhaustively requalified. No application, SysML conformance, UI, or runtime audit/test execution is claimed. Source comments, fixtures, scripts, and CI definitions are outside documentation cleanup scope.

## Disposition

No deletion is proposed: the reviewed files contain unique contracts, acceptance criteria, references, or milestone evidence. Historical records are marked in place so links remain valid. Do not upload historical status/checklist documents as active ChatGPT Project guidance. UPDATE REQUIRED entries are explicitly marked, not silently certified as current.

| File | Disposition | Evidence / action |
| --- | --- | --- |
| [README.md](../README.md) | UPDATED / RETAIN | Added documentation entry points; existing qualification summaries remain bounded claims, not new verification. |
| [apps/desktop/src-tauri/icons/README.md](../apps/desktop/src-tauri/icons/README.md) | UPDATED / RETAIN | Added documentation entry points; existing qualification summaries remain bounded claims, not new verification. |
| [docs/ARCHITECTURE.md](../docs/ARCHITECTURE.md) | UPDATED / RETAIN | Distinguished current three workspace members from proposed future layout; labeled PR1 foundation as historical. |
| [docs/BDD_CONFORMANCE.md](../docs/BDD_CONFORMANCE.md) | RETAIN / SCOPED REFERENCE | Retain original milestone requirements/reference material. Scope and completion claims are historical; current behavior needs its own verification. |
| [docs/BDD_PR2_ACCEPTANCE.md](../docs/BDD_PR2_ACCEPTANCE.md) | HISTORICAL / RETAIN | Retain original milestone requirements/reference material. Scope and completion claims are historical; current behavior needs its own verification. |
| [docs/BDD_PR2_MODELING_RULES.md](../docs/BDD_PR2_MODELING_RULES.md) | RETAIN / SCOPED REFERENCE | Retain original milestone requirements/reference material. Scope and completion claims are historical; current behavior needs its own verification. |
| [docs/IMPORTER_INPUT_SPECIFICATION.md](../docs/IMPORTER_INPUT_SPECIFICATION.md) | RETAIN / SCOPED REFERENCE | Keep as the input-authoring entry point; explicitly scoped to its PR67 baseline. Detailed schema/semantic verification remains a separate audit. |
| [docs/IMPORT_RULES_AND_QUALIFICATION.txt](../docs/IMPORT_RULES_AND_QUALIFICATION.txt) | UPDATE REQUIRED | UPDATE REQUIRED: PR55 header and older PLANNED rows conflict with later override sections and PR67 input specification. Consolidate field-by-field against adapters/tests before publishing a new qualification matrix. |
| [docs/PR11_IBD_STANDARDS.md](../docs/PR11_IBD_STANDARDS.md) | RETAIN / SCOPED REFERENCE | Retain original milestone requirements/reference material. Scope and completion claims are historical; current behavior needs its own verification. |
| [docs/PR12_BEHAVIOR_STANDARDS.md](../docs/PR12_BEHAVIOR_STANDARDS.md) | RETAIN / SCOPED REFERENCE | Retain original milestone requirements/reference material. Scope and completion claims are historical; current behavior needs its own verification. |
| [docs/PR13_ACTIVITY_STANDARDS.md](../docs/PR13_ACTIVITY_STANDARDS.md) | RETAIN / SCOPED REFERENCE | Retain original milestone requirements/reference material. Scope and completion claims are historical; current behavior needs its own verification. |
| [docs/PR13_IMPLEMENTATION_CHECKLIST.md](../docs/PR13_IMPLEMENTATION_CHECKLIST.md) | HISTORICAL / RETAIN | Unchecked tasks conflict with the implementation described in PR13_STATUS and later Activity code; preserve as original checklist, not current backlog. |
| [docs/PR13_SCOPE.md](../docs/PR13_SCOPE.md) | HISTORICAL / RETAIN | Retain original milestone requirements/reference material. Scope and completion claims are historical; current behavior needs its own verification. |
| [docs/PR13_STATUS.md](../docs/PR13_STATUS.md) | HISTORICAL / RETAIN | Branch and qualification-in-progress statements describe PR13, not current main; retain original evidence without inferring manual acceptance. |
| [docs/PR14_WORKSPACE_CONVERGENCE.md](../docs/PR14_WORKSPACE_CONVERGENCE.md) | HISTORICAL / RETAIN | Old routing-family limitations predate the shared family registry and later integration contracts. |
| [docs/PR21_REQUIREMENTS_MIGRATION.md](../docs/PR21_REQUIREMENTS_MIGRATION.md) | RETAIN / SCOPED REFERENCE | Retain original milestone requirements/reference material. Scope and completion claims are historical; current behavior needs its own verification. |
| [docs/PR31_ACTIVITY_EXECUTION.md](../docs/PR31_ACTIVITY_EXECUTION.md) | RETAIN / SCOPED REFERENCE | Retain runtime contract; PR34 operation integration supersedes the original default-operation limitation. Other limitations need current-code audit. |
| [docs/PR32_STATE_MACHINE_EXECUTION.md](../docs/PR32_STATE_MACHINE_EXECUTION.md) | RETAIN / SCOPED REFERENCE | Retain runtime contract; PR33-35 supersede future-work references. Revalidate pseudostate limitations separately. |
| [docs/PR33_STRUCTURAL_RUNTIME_CONFORMANCE.md](../docs/PR33_STRUCTURAL_RUNTIME_CONFORMANCE.md) | UPDATE REQUIRED | UPDATE REQUIRED: original metamodel limitations must be reconciled with later port/connector/feature imports; authored support does not prove runtime support. |
| [docs/PR34_OPERATION_SIGNAL_SEQUENCE_RUNTIME.md](../docs/PR34_OPERATION_SIGNAL_SEQUENCE_RUNTIME.md) | RETAIN / SCOPED REFERENCE | Retain bounded execution contract; PR35 now has implementation. Historical manual qualification is not a current-run result. |
| [docs/PR35_NATIVE_PARAMETRIC_RUNTIME.md](../docs/PR35_NATIVE_PARAMETRIC_RUNTIME.md) | UPDATED / RETAIN | Removed stale in-progress branch instruction; added links to existing runtime implementation and tests. |
| [docs/PR3_BDD_WORKFLOW.md](../docs/PR3_BDD_WORKFLOW.md) | HISTORICAL / RETAIN | Retain original milestone requirements/reference material. Scope and completion claims are historical; current behavior needs its own verification. |
| [docs/PR4_PROJECT_PERSISTENCE.md](../docs/PR4_PROJECT_PERSISTENCE.md) | HISTORICAL / RETAIN | Retain original milestone requirements/reference material. Scope and completion claims are historical; current behavior needs its own verification. |
| [docs/PR53_PROFILES_XMI.md](../docs/PR53_PROFILES_XMI.md) | RETAIN / SCOPED REFERENCE | Retain bounded semantic/profile contract; use PR55 for presentation extension and current adapter/tests for verification. |
| [docs/PR55_XMI_DI_INTEROPERABILITY.md](../docs/PR55_XMI_DI_INTEROPERABILITY.md) | RETAIN / SCOPED REFERENCE | Retain producer/native round-trip distinctions and vendor-fixture limitations; qualification claims are milestone-scoped. |
| [docs/PR8_ACCEPTANCE.md](../docs/PR8_ACCEPTANCE.md) | HISTORICAL / RETAIN | Retain original milestone requirements/reference material. Scope and completion claims are historical; current behavior needs its own verification. |
| [docs/PR8_BDD_STANDARDS_BASELINE.md](../docs/PR8_BDD_STANDARDS_BASELINE.md) | RETAIN / SCOPED REFERENCE | Retain original milestone requirements/reference material. Scope and completion claims are historical; current behavior needs its own verification. |
| [docs/PR8_IMPLEMENTATION_CHECKLIST.md](../docs/PR8_IMPLEMENTATION_CHECKLIST.md) | HISTORICAL / RETAIN | Unchecked migration tasks are a historical snapshot; derive remaining work from current code and tests. |
| [docs/PR8_SOURCES.md](../docs/PR8_SOURCES.md) | RETAIN / SCOPED REFERENCE | Retain original milestone requirements/reference material. Scope and completion claims are historical; current behavior needs its own verification. |
| [docs/ROADMAP.md](../docs/ROADMAP.md) | UPDATED / RETAIN | Marked as strategic themes, not current progress or an approved execution queue. |
| [docs/RUST_AUTHORITY_RECOVERY.md](../docs/RUST_AUTHORITY_RECOVERY.md) | HISTORICAL / RETAIN | Historical 7,279 JavaScript line ceiling is not enforced by the current script; current gate uses controller ceilings and Rust/frontend ratio. |

## Follow-up documentation tasks

1. Consolidate the import qualification contract against current model-script, spreadsheet, ReqIF, and XMI adapters/tests; eliminate overridden status rows without discarding unsupported boundaries.
2. Reconcile PR33 authored-model limitations with later model fields and independently verify runtime support.
3. During each functional/notation/behavior audit, replace milestone coverage claims with commit-specific evidence. Do not check old acceptance boxes merely because code exists.
4. Reprioritize ROADMAP themes after findings and feature requirements are agreed.

Evidence inspected includes root `Cargo.toml`, `scripts/validate_rust_authority.py`, `.github/workflows/ci.yml`, the core/desktop Parametric runtime, PR35 test fixtures, and the PR67 importer specification.
