# Step 5 — Scope, priorities, and acceptance

Status: planning package prepared for user review; not execution authorization.
Baseline: `6f71f828b1308f113cbf9971ed6897bda420760f` (PR69 merged).
Date: 2026-09-08.

## Checkpoint and next steps

1. Documentation cleanup PR69: merged.
2. ChatGPT Project brief and instructions: installed per user confirmation.
3. Standards and supporting references: uploaded; standard versions verified.
4. Step 5: this concrete scope and acceptance package; acceptance pending review.
5. Step 6: separate agent-configuration PR after this plan is accepted.
6. Step 7: verify build/test/UI/reference access and isolated working environments.
7. Step 8: merge qualified setup and execute a bounded pilot under a work order.
8. Step 9: execute prioritized audit/fix/feature work packages.

No application changes, standards audit results, agent configuration, automatic
merge, or background execution are introduced by this document.

## Scope and authority

The objective is a professional SysML 1.6 tool with inherited UML 2.5.1 semantics,
complete user workflows, consistent notation/UI, and collaborative modeling.
Audit all nine diagram families and shared systems, including existing partial
capabilities. CATIA/Cameo is a workflow reference, not certification or a mandate
to clone every licensed product feature. SysML v2 remains a separately scoped
future capability; it is not silently included in this SysML 1.6 release.

Rust remains authoritative for semantics, validation, commands, routing/layout,
history, persistence and runtime. Reuse existing shared engines and transactions.
Keep offline desktop use, stable identity, SQLite compatibility and thin frontend
boundaries. No speculative module splits, arbitrary JavaScript line caps, or
replacement simulation engines. Source proportions alone do not prove performance.

## Audit coverage and evidence

Use [the coverage checklist](STEP5_AUDIT_COVERAGE.md). Every listed subtopic must
receive a child finding/check record before its parent coverage row can close.
For each applicable subtopic inspect these independent dimensions:

| Dimension | Required evidence |
| --- | --- |
| Semantic representation | Actual model fields, IDs, ownership and typing paths |
| Authoring and configuration | User can create, discover and configure the feature |
| Validation | Valid and invalid fixtures; diagnostic and rollback behavior |
| Notation/presentation | Applicable standard clause/figure and rendered evidence |
| Editing | Applicable shared interaction checks, history and cross-diagram reuse |
| Persistence/interchange | Save/reopen and applicable adapter round trips |
| Execution | Defined supported semantics, deterministic cases and explicit limits |
| Scale/accessibility | Representative workload or applicable keyboard/focus checks |

Each record contains: stable ID, parent coverage ID, baseline commit, scope,
reference edition/clause, expected result, actual result, code/test/UI evidence,
status, severity, dependency, proposed acceptance scenario, owner, PR and verified
commit. Allowed statuses: UNVERIFIED, VERIFIED, DEFECTIVE, MISSING, NOT APPLICABLE
(with rationale). Track implemented, UI-exposed and tested separately. Do not use
an enum, old checklist, screenshot alone or source-presence script as proof of
full capability. Known baseline defects remain visible, not silently waived.

Audit completion means all child checks have a disposition and evidence. A report
with unverified items is coverage-accounted but not fully qualified. Scope can grow
through an explicit new row when a reference/code path reveals an omission.

## Priority and dependency policy

| Order | Work | Exit condition |
| --- | --- | --- |
| P0 | Data loss, corruption, invalid atomic apply, serious access-control defects | Reproduced, corrected and independently verified |
| P1 | Broken existing workflows, semantic rejection/acceptance errors, execution errors | Relevant end-to-end and regression scenarios pass |
| P2 | Missing configuration and notation/visual correctness | Standards evidence and usable authoring/editing path verified |
| P3 | Complete missing features and UI/UX improvements | Agreed feature journey and integrated acceptance pass |
| P4 | Optional integrations, advanced governance and scale expansion | Explicit scope, workload and release allocation agreed |

Priorities are initial triage rules, not findings or estimates. User-designated
release priorities may reorder P2-P4. Dependencies override scheduling: shared
mutation/storage changes precede dependent clients. Collaboration requirements
and UI design may be investigated in parallel with audits; conflicting shared-file
implementation is serialized. Do not wait for the entire audit to finish before
fixing confirmed defects inside an authorized audit-and-fix work package.

## Feature inventory and staged outcomes

All entries below are requirements to assess/specify, not assertions of absence.

| ID | Capability | First complete outcome | Dependencies / further scope |
| --- | --- | --- | --- |
| F01 | Model authoring and engineering data | Create/configure/validate/reuse all scoped structural, requirement and behavioral concepts | Coverage findings determine missing fields/UI |
| F02 | Professional diagrams and shared editing | Correct notation, readable routes/labels and consistent editing in all nine families | Shared interaction and presentation contracts |
| F03 | Behavior and engineering analysis | Author, configure, initialize, run/step, inspect and reset each scoped execution workflow | Shared runtime; advanced semantics specified before implementation |
| F04 | Interchange and automation | Preview, atomic apply, stable reimport and scoped export with clear diagnostics | Current adapter contracts; vendor claims need genuine fixtures |
| F05 | UI/UX cleanup | Consistent workspace, navigation, properties, errors and runtime controls | Representative layouts accepted before broad visual changes |
| F06 | Collaboration | Two authenticated clients edit the same persistent project with consistent committed state and reconnect recovery | Reuse assessment, authoritative operations, permissions and revision contract |
| F07 | Verification and traceability | Navigate requirement-to-design-to-test links and inspect defined coverage/evidence | Agree coverage/evidence lifecycle; tables and reports scoped explicitly |
| F08 | Governance and scale | Assess revision history, branching/merge, locks, audit records and large-project behavior | Separate increments after collaboration foundation and measured baseline |
| F09 | Documentation reconciliation | Current import matrix and structural-runtime limitations match verified implementation | PR69 flagged import contract and PR33 records |

Every feature specification must include user journey, model/validation changes,
UI/configuration, transaction/undo, persistence/migration, applicable interchange,
runtime and collaboration interactions, negative cases, diagnostics, tests,
manual checks, documentation and explicit exclusions. Foundation PR completion
does not close the parent feature.

## Concrete acceptance scenarios

| ID | Scenario | Pass condition |
| --- | --- | --- |
| A01 | Create and reuse a typed element across applicable diagrams; rename and reparent | Stable semantic identity; references remain valid; presentation does not duplicate semantics |
| A02 | Edit, undo/redo, save, close and reopen a representative connected model | Authored semantics, ownership and presentation survive; failed changes are atomic |
| A03 | Run shared editing checks in every applicable family | Palette/place/drop/connect/select/move/resize/delete/remove/clipboard/ESC/marquee/pan/zoom/fit/history work; pan does not move elements |
| A04 | Render sparse and dense/nested diagrams before and after move, Route and Clean | Correct endpoint notation; labels attached/readable; routes avoid unrelated elements, ports and labels; no extreme off-canvas detours |
| A05 | Execute deterministic valid and invalid behavior fixtures | Expected values/events/order, readable failures, bounded execution, reset repeatability and authored-state isolation |
| A06 | Import valid source, reimport unchanged, modify source, and attempt invalid apply | Correct identity/update outcomes; no duplicates; invalid apply leaves project unchanged; applicable export/reimport preserves scoped data |
| A07 | Use properties, navigation and diagnostics without hidden required configuration | All required fields discoverable; keyboard/focus behavior works; errors name context and remedy |
| A08 | Two clients create/rename, conflict, disconnect/reconnect and reopen shared project | Same committed revision/state; retries do not duplicate operations; conflicts explicit; unauthorized mutation rejected |
| A09 | Collaboratively delete versus edit and undo a user change | Defined conflict outcome; no silent data loss or undo of unrelated user changes |
| A10 | Measure representative small, dense and large models | Record hardware, sizes, timings and memory; agree numeric budgets before claiming scale qualification |
| A11 | Trace a requirement to satisfying design and verifying TestCase | Links resolve by identity, coverage follows defined rules and survives relevant round trips |

Routing must respect legitimate containment/frame-boundary transitions and internal
state transitions; classify obstacles by semantics rather than prohibit all boundary
crossings. Preserve independent frame/context/diagram identity and drill-down.

## Definition of done and review gates

- Audit finding: reproducible evidence plus expected result and scope; coordinator
  checks validity and duplicates before routing a fix.
- Implementation task: bounded acceptance passes and no unexplained new regressions;
  relevant failure/rollback cases included. No broad refactor for convenience.
- PR: independent review addressed, required CI checks pass on final head, relevant
  desktop/visual acceptance performed or explicitly still outstanding. Report exact
  commit and known limitations. Never claim an unexecuted test passed.
- Feature: full integrated user journey verified across all constituent PRs, with
  current documentation. Manual gaps prevent complete qualification.
- Release: scoped feature/defect register reconciled; unresolved items explicitly
  dispositioned. No blanket CATIA compatibility or complete SysML certification.

Run focused verification during development; required integration/CI gates at
qualification. Repeat checks when subsequent edits invalidate their evidence.
Documentation-only planning uses link, coverage and diff checks, not application tests.

## Proposed decisions and when they must be resolved

| ID | Proposal | Decision deadline |
| --- | --- | --- |
| D01 | First pilot: bounded cross-family frame/header and interaction audit, with one verified fix if found; do not manufacture a defect | Before pilot work order |
| D02 | Initial collaboration slice: authenticated LAN session with an authoritative host, two clients, persistent operations/revisions and reconnect; optional on-prem server afterward | Before collaboration architecture implementation; reuse audit may alter mechanism |
| D03 | Preserve desktop workspace familiarity; approve representative repository/canvas/properties/runtime layouts before broad UI edits | Before UI implementation |
| D04 | Measure current scale first, then agree target project sizes, latency and memory budgets | Before performance qualification |
| D05 | SysML v2, full external vendor certification, cloud service operation, and arbitrary code execution are separate future scope | Revisit only with an explicit feature work order |

These proposals do not block agent configuration. Approval of this planning PR
accepts audit scope and gates, not all future product choices or implementation.
The coordinator brings deadline-specific choices with evidence and a recommendation.
Feature ordering after the pilot follows severity, dependencies and user priorities.

## Agent setup handoff (Step 6 requirements only)

Configure lead, domain auditors, implementers and independent reviewers; choose
available models by ambiguity/risk and escalate bounded failed attempts. Keep one
shared-module owner, isolated worktrees, compact evidence handoffs, finite retry
budgets and resumable records. Start with a small team; no guaranteed usage savings.
Publish configuration separately. Verify reference access, supported configuration,
build/test commands and desktop access before the pilot. ChatGPT uploads are not
assumed accessible to standalone Codex; do not commit uploaded commercial books.

## References

- [SysML 1.6](https://www.omg.org/spec/SysML/1.6): uploaded `formal-19-11-01.pdf`, normative baseline.
- [UML 2.5.1](https://www.omg.org/spec/UML/2.5.1): uploaded `formal-17-12-05.pdf`, applicable inherited semantics.
- Uploaded SysML Distilled and A Practical Guide to SysML: explanatory references;
  older editions cannot override the chosen normative version.
- Uploaded CATIA Magic guide: secondary feature/workflow source; verify actual
  vendor release documentation for product claims.
- [Architecture](ARCHITECTURE.md), [documentation review](DOCUMENTATION_REVIEW.md),
  and [importer specification](IMPORTER_INPUT_SPECIFICATION.md): scoped repository references.

Standards audit records must cite actual relevant clauses, including applicable
UML subset/version constraints, rather than treat these high-level URLs as proof.
