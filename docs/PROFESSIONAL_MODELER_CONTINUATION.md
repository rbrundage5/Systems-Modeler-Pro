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

Recorded baseline (2026-09-23): main `7296098fa550ac2b79d4aacb994ab116bee6f2d7`,
after verified merges of PR144-147.
Refresh these statuses from GitHub before acting; this table is a checkpoint.

| Work | Evidence / disposition |
| --- | --- |
| Relationship validation scale | PR140 merged. Debug CI workload: 10,000 relationships 4.47s to 0.07s; 100,000 relationships 0.64s. |
| Typed unit-reference validation | PR142 merged. Debug CI workload: 9,001 elements 1.12s to 0.09s; 90,001 elements 0.88s. |
| Imported inheritance cycles | PR143 merged. Native cycle/diamond/deep-chain validation; 20,000 inheritance levels tested. |
| Atomic history transitions | PR144 merged (`a97bd87`). Candidate `3f3d5bb`: acquire all seven authored fields represented by HistorySnapshot and both history stacks before publication. Busy authored state rejects without holding partial locks; failure/contention and ownership-moving undo/redo regressions. This does not normalize every command lock order. |
| Inherited-feature query | PR145 merged (`516144c`). Candidate `582c066`: indexed iterative traversal, unique ancestor features, deterministic results and a 20,000-level query regression. All three native-foundation jobs passed (run 35770898479); four new query tests passed in 0.19s total. Current callers are tests/API consumers, not the structural runtime/UI. |
| Connected IBD port preview | PR146 merged (`9467a5a`). Candidate `90dce35`: Rust-routed attached connectors, names and ItemFlow adornments during port gestures; cancellation, hidden labels and stale-response protection. All 57 local frontend integration tests pass, including 20 gesture tests. Native and installed/rendered acceptance are distinct. |
| Activity authoring lock order | [PR148](https://github.com/rbrundage5/Systems-Modeler-Pro/pull/148), `8c6938e6e32db2af39d6f8848d45bcea28cff0fe`: acquire Activity repository before diagrams in nested creation/editing/route/layout/header paths. Three native failure/contention regressions passed. All three native-foundation jobs passed in run 35849283405. |
| PackageImport traversal scale | [PR149](https://github.com/rbrundage5/Systems-Modeler-Pro/pull/149), `726f5e5691c0de0570c50df7b65c83ea4096ebe6`: fresh per-query export/adjacency indexes and an iterative globally visited import walk. Four native tests cover a 20,000-link chain, 35 repeated diamond levels, cycles, aliases, visibility, ambiguity, rename freshness and malformed targets. All three native-foundation jobs passed in run 35849310247; four traversal tests took 0.28s total in debug CI. |
| Behavior authoring lock order | [PR150](https://github.com/rbrundage5/Systems-Modeler-Pro/pull/150), `1cf3d3b52af320ab99511399f852b6b4a2299127`: State vertex, Sequence lifeline/message and header paths acquire repository before presentations, with project first when needed. Missing/poisoned project cannot leave a rejected Message appended. Four native command-level regressions added. |

At this checkpoint PR148/149 are ready for review. PR150 remains a draft while the
latest fully validated fixture completes native CI (run 35850253232); its core job
has passed. Read each live PR for subsequent check results. Packaging checks are
separate from native CI. Review and inline-thread retrieval found no entries;
independent review and installed interactive acceptance remain outstanding. All
work was primary-session implementation; the worker environment gate is unchanged.

These timings include fixture construction, debug validation and destruction on
GitHub-hosted runners. They are not latency distributions, release benchmarks,
peak-memory measurements, renderer measurements or whole-application capacity.

## Dependency-ordered work queue

Each row is a workstream to split into concrete leaf work orders before editing.
Name allowed files, one user workflow, success evidence and a negative/rollback
case. Revalidate findings against current main; do not duplicate merged fixes.

| Priority / coverage | Next concrete outcome | Required acceptance |
| --- | --- | --- |
| P0 C01/C02: concurrent authoring locks | Land and recheck PR148/150, then audit structural and other remaining command guard lifetimes. PR144 rejects busy history operations but does not repair every existing command. | A held presentation guard cannot deadlock Save/history or another native command; rejected/contended operations publish nothing, and retry succeeds. Use bounded concurrency regressions rather than timing-only assertions or an unbounded stress test. |
| P0 C02: session replacement | Native New/Open must retire previous runtime/history only when the complete replacement commits. Inspect complete Open, all execution managers and the overwritten frontend Open wrappers. | Failed staging or any required lock leaves authored state, runtime and history intact; successful replacement cannot undo into or execute the previous project. |
| P0 C02: dirty revisions | Native session/revision tracking and coherent Save/Discard/Cancel for destructive session transitions. | Concurrent edits cannot be marked saved by an earlier save; Cancel preserves everything; failed Save keeps dirty state and the current file identity. |
| P1 C21: history memory | Measure retained memory for a large project plus repeated IBD geometry edits; introduce a smaller geometry history representation after atomic history is integrated. | Interleaved geometry/semantic edits undo and redo correctly, failed edits preserve stacks, redo invalidation is correct, and memory growth is measured. Do not disable undo to meet a budget. |
| P1 C07/C21: namespace queries | Land PR149 for PackageImport traversal, then replace repeated whole-model qualified-name computation in bulk lookup with an appropriately scoped fresh index. Do not present the import fix as a complete namespace performance solution. | Renames, moves, duplicate/ambiguous names, visibility and package/element imports retain correct resolution; record fixture sizes, timings and memory. |
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

The lock audit is now split into published Activity (PR148) and Behavior (PR150)
leaves. Their shared header changes affect separate match arms; a local three-way
merge of both branches succeeded. Each PR targets main directly. Refresh main and
mergeability before integrating; these checks cannot guarantee compatibility with
unknown future edits. The PackageImport PR modifies separate core paths.

Only actual overlapping guard lifetimes were changed. For example, Behavior
move/resize/routing paths often clone their first read and release its guard before
acquiring another; textual acquisition order alone is not a deadlock finding.
Activity edge creation/reconnection still need a separate late-routing-failure
rollback check. Existing State/Sequence create-staged commands and unrelated
rendering algorithms were outside these lock-order leaves.

## Next executable leaves after this batch

1. **C02.session.new-publication:** `workspace.rs::new_project` still writes the
   Project before acquiring later locks; the active New button calls it and later
   resets Activity in `frontend/project-reset-integrity.js`. Reproduce a poisoned
   late lock with a populated multi-repository session. Define a complete native
   New publication primitive using the established authored-state order; failed
   New must retain all previous repositories and path. Inspect active wrappers
   before selecting the exact IPC integration paths.
2. **C02.session.history-retirement:** `bdd_elements.rs::open_project_file_complete`
   atomically publishes authored repositories, while `frontend/undo-redo-ui.js`
   invokes `history_reset` afterward. The current complete Open command has no
   history/runtime guard parameters. Reproduce failure between native publication
   and reset, then integrate history retirement with session publication. Runtime
   managers need a separately specified generation/retirement step; do not claim
   complete runtime isolation from authored-state atomicity alone.
3. **C02.mutation.bdd-details:** inspect
   `bdd_elements.rs::update_bdd_element_details`, which applies type/documentation/
   unit fields before its final `validate_element` result. Reproduce a late-invalid
   unit/type combination and compare all prior fields; if confirmed, stage or roll
   back the complete edit using existing native validation. This is a candidate
   found by code inspection, not an executed failure regression yet.
4. **C07.namespace.qualified-lookup:** `namespace.rs::resolve_qualified_name` scans
   every element and recomputes ownership paths; `visible_members` recomputes names
   inside sorting comparisons. Preserve malformed-ownership diagnostics, ambiguity
   and deterministic ordering in a fresh per-query index. Diagnose malformed
   ownership explicitly rather than silently hiding it. Record both shallow-wide
   and deep-nested fixtures separately from PackageImport depth.
5. **C21.history.geometry-memory:** profile the seven-field history snapshot (up
   to 100 retained entries) under
   repeated geometry edits before choosing a delta representation. Retain existing
   interleaved semantic/presentation undo behavior, contention rejection and retry.
6. **C10.sequence.lifeline-discovery:** `collect_lifeline_candidates` stops at
   depth greater than six and scans direct children; the UI consumes this list in
   `behavior-sequence-input.js`. Qualify inherited and deeply nested property-path
   discovery before choosing lazy expansion, cycle handling and explicit limits.
   The current depth cap is code evidence, not proof that every omitted path is
   valid or that simply removing the cap is safe.

The UI revamp remains active in the work queue. It needs representative installed
or rendered acceptance and a bounded repository/canvas/properties journey; this
batch does not claim to provide that facelift. No source, workflow, agent gate or
application process/network policy was broadened by these fixes.

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
