# Professional SysML modeler: continuing implementation

This is the product work queue for the user's authorized review, improvement and
revamp of Systems-Modeler-Pro. It is not a declaration of product completeness or
an environment/agent qualification. Read current AGENTS.md and the live repository
state. Preserve the repository and approved-source boundaries.

## Reusable Codex task

Continue the professional SysML modeler program in this document. Execute the
work, including real branch/PR publication and CI corrections, rather than only
writing a plan. Audit the current implementation, reproduce the highest-impact
issue, implement a coherent workflow improvement, test its positive and failure
paths, publish it, and immediately select the next independent ready task.
One reviewable leaf per PR is not one leaf per session. A PR awaiting checks,
review or merge is a checkpoint, not a reason to abandon other authorized work.

Prioritize data integrity, usable modeling workflows, native performance and
SysML correctness. Include substantial UI/UX and notation improvements where the
workflow needs them. Rust owns semantic state, validation, transactions, history,
routing, persistence and execution. HTML/CSS/JavaScript render native results and
capture input. Do not replace native work with superficial frontend changes or
rewrite established engines without a demonstrated need.

Use current main as the integration baseline and target main directly. Inspect
open work before editing shared modules. Resolve current conflicts and retest
affected behavior yourself; preserve the user's and other writers' changes.
Publish through an available authenticated GitHub capability and verify the real
URL, head, base, checks and review threads. Never describe a metadata-only PR
record as published or merged. Do not force-push or merge automatically.

Keep checking for newly exposed gaps after each completed task. Remove obsolete
or redundant product code only after proving the replacement covers its callers
and workflows. Review executable/process/network boundaries for their actual
modeling or application-delivery purpose; preserve legitimate import/update
functions and existing access controls. Do not change worker gates to make
delegation convenient. Use registered workers only when genuinely qualified;
otherwise continue authorized primary-session work and report independent review
as outstanding.

Continue until the selected work is completed, the execution budget requires a
checkpoint, or every dependency-ready task has a concrete blocker. At a real
boundary, save exact heads, evidence, unfinished work and the next executable
step. Do not invent an unlimited background process or claim work continues after
the session ends. A new session should resume this queue without repeating the
completed audit or requesting the same routine implementation authorization.

## Current evidence checkpoint

Recorded baseline: main `26c0d4ed13ab49cde4dd1ec864063be9d231dd00`, after PR142/143.
Refresh these statuses from GitHub before acting; this table is a checkpoint.

| Work | Evidence / disposition |
| --- | --- |
| Relationship validation scale | PR140 merged. Debug CI workload: 10,000 relationships 4.47s to 0.07s; 100,000 relationships 0.64s. |
| Typed unit-reference validation | PR142 merged. Debug CI workload: 9,001 elements 1.12s to 0.09s; 90,001 elements 0.88s. |
| Imported inheritance cycles | PR143 merged. Native cycle/diamond/deep-chain validation; 20,000 inheritance levels tested. |
| Atomic history transitions | PR144, `3f3d5bb`: acquire all seven authored fields represented by HistorySnapshot and both history stacks before publication. Busy authored state rejects without holding partial locks; failure/contention and ownership-moving undo/redo regressions. Verify current native CI. |
| Inherited-feature query | PR145, `582c066`: indexed iterative traversal, unique ancestor features, deterministic results and a 20,000-level query regression. All three native-foundation jobs passed (run 35770898479); four new query tests passed in 0.19s total. Current callers are tests/API consumers, not the structural runtime/UI. |
| Connected IBD port preview | PR146, `90dce35`: Rust-routed attached connectors, names and ItemFlow adornments during port gestures; cancellation, hidden labels and stale-response protection. All 57 local frontend integration tests pass, including 20 gesture tests. Native and installed/rendered acceptance are distinct. |

These timings include fixture construction, debug validation and destruction on
GitHub-hosted runners. They are not latency distributions, release benchmarks,
peak-memory measurements, renderer measurements or whole-application capacity.

## Dependency-ordered work queue

Each row is a workstream to split into concrete leaf work orders before editing.
Name allowed files, one user workflow, success evidence and a negative/rollback
case. Revalidate findings against current main; do not duplicate merged fixes.

| Priority / coverage | Next concrete outcome | Required acceptance |
| --- | --- | --- |
| P0 C02: concurrent authoring locks | Normalize the existing repository/presentation lock inversions in separate Activity, Behavior and structural-command leaves. PR144 rejects busy history operations but does not repair every existing command. | A held presentation guard cannot deadlock Save/history or another native command; rejected/contended operations publish nothing, and retry succeeds. Use bounded concurrency regressions rather than timing-only assertions or an unbounded stress test. |
| P0 C02: session replacement | Native New/Open must retire previous runtime/history only when the complete replacement commits. Inspect complete Open, all execution managers and the overwritten frontend Open wrappers. | Failed staging or any required lock leaves authored state, runtime and history intact; successful replacement cannot undo into or execute the previous project. |
| P0 C02: dirty revisions | Native session/revision tracking and coherent Save/Discard/Cancel for destructive session transitions. | Concurrent edits cannot be marked saved by an earlier save; Cancel preserves everything; failed Save keeps dirty state and the current file identity. |
| P1 C21: history memory | Measure retained memory for a large project plus repeated IBD geometry edits; introduce a smaller geometry history representation after atomic history is integrated. | Interleaved geometry/semantic edits undo and redo correctly, failed edits preserve stacks, redo invalidation is correct, and memory growth is measured. Do not disable undo to meet a budget. |
| P1 C07/C21: namespace queries | Replace repeated whole-model qualified-name computation in bulk lookup with an appropriately scoped fresh index. | Renames, moves, duplicate/ambiguous names, visibility and package/element imports retain correct resolution; record fixture sizes, timings and memory. |
| P1 C02/C19: recovery and compatibility | Reopen representative old project files and inject filesystem/late-metadata failures across complete Save/Open. | Stable IDs and all authored repositories survive round trips; failures retain the previous complete revision; recovery messages give a usable next action. |
| P1 C03-C11: semantic integrity | Continue explicit association-end, ownership, Copy-graph, endpoint mutation and cross-repository reference checks. | Valid workflows remain accepted; invalid imported and edited models fail before publication with contextual diagnostics and no partial mutation. |
| P2 C18: modeling workspace revamp | Build a coherent repository/canvas/properties workflow with readable density, consistent selection, keyboard/focus behavior and useful empty/error states. | Capture representative small and large-model layouts, verify native selection/edit/undo/save/reopen paths, and test accessibility/DPI behavior. No generic dashboard or cosmetic-only completion claim. |
| P2 C16/C17: diagram editing and notation | Qualify port movement, attached labels/routes, multi-select, resize, frames, drill-down and Route/Clean across all nine families. | Sparse and dense/nested diagrams retain endpoints, labels, containment and valid routes; cancelled/rejected gestures restore the original view and create no history entry. |
| P2 C05: requirements workbench | Make requirement hierarchy, ID/text editing and requirement-to-design-to-test navigation usable at scale. | Reuse existing native Requirement/Copy/traceability rules; filtering/navigation preserve identity; a Satisfy/Verify link alone must not imply a successful verification result. |
| P2 C19: import workflow | Qualify preview/apply/reimport and usable source diagnostics using approved real fixtures. | No duplicate stable IDs, missing owners/endpoints or silent unsupported content; invalid apply rolls back; genuine CATIA compatibility remains unverified without source fixtures. |
| P2 C20: ordinary-workspace collaboration | Audit which ordinary authoring commands and diagram families participate in the established shared protocol; extend bounded gaps. | Two-client convergence, actor-scoped permissions/undo, conflict handling, reconnect/retry and durable recovery with no parallel collaboration engine. |
| P2 C22: application acceptance | Exercise installed application workflows and the existing delivery/update path after native gates pass. | Representative connected-model create/edit/undo/save/reopen/import/runtime and two-device checks; distinguish packaging smoke tests from product acceptance. |

The workspace revamp must retain a familiar modeling layout: searchable
containment/navigation, discoverable family-specific authoring tools, a clear
diagram canvas, a coherent specification editor and contextual diagnostics/runtime
inspection. Start with representative layouts and implement their entire
interaction journey. A color change does not satisfy this workstream.

The lock audit identified Activity node/edge creation, Activity editing and
route/layout paths acquiring diagrams before the Activity repository, while
complete Save and structural history acquire the repository first. Behavior
editing and shared header rename contain analogous inverse acquisitions. Confirm
guard lifetimes in each function before changing it: some chained reads clone
their result and already release the first guard. Do not treat a textual order
scan as proof that every match deadlocks. Activity edge creation/reconnection
also warrant a separate late-routing-failure rollback check before declaring
their mutation workflow atomic.

## Evidence and continuation record

For each leaf, record the baseline and final commit, changed paths, live PR,
native/renderer tests actually executed, review findings, current mergeability,
measurements with configuration, and manual gaps. Track implementation, UI exposure,
automated verification and rendered/native acceptance separately. Use approved
SysML 1.6/UML 2.5.1 references for standards claims; vendor behavior is comparative
evidence, not the normative specification.

Run representative small, dense and large workloads through queries, editing,
history, save/open, import and rendering. Include cancellation and failure paths.
Measure native release builds and peak memory before setting production budgets.
No claim of zero defects, unlimited size, complete SysML certification or
CATIA/Cameo parity follows from a few successful tests.

At the next checkpoint, update this record and the affected PRs. Keep actionable
work visible instead of replacing the queue with a generic “done” summary.
