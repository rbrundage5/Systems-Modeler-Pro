# Systems-Modeler-Pro: completion audit

Date: 24 September 2026. Product baseline: **`f3e45cc0d1bf5ff40001afe75ac03fc1e4bfa8c7`**.
Work order: **C22.completion-audit-2026-09-24**. Scope: current implementation,
qualification gaps, release evidence and a dependency-ordered completion backlog.
Allowed changes: this report, its read-only witness under
`docs/reviews/2026-09-24/`, and the documentation index. No application, schema,
workflow, agent configuration or reference-file changes.

## Assessment

**The application is buildable and has substantial implemented modeling capability,
but it is not complete against the professional SysML 1.6 product goal.** Remaining
work includes confirmed command/session defects, missing semantic and engineering
workflows, incomplete ordinary-workspace collaboration, and unexecuted desktop,
large-model and production-update acceptance. This is more than a packaging task.

All three cited PRs are now merged into main:

| PR | Integrated behavior | Live evidence inspected |
| --- | --- | --- |
| [164](https://github.com/rbrundage5/Systems-Modeler-Pro/pull/164) | Composition creates linked PartProperties; linked editing, cross-view endpoint staging, retype history, duplication and legacy linking are integrated | Head `95449198d1629abcce137e2c05258b24c5aefad6`; merged as `4eab83c2bc19edba5171297590347f7e3745e9ea`; five reported workflows successful |
| [165](https://github.com/rbrundage5/Systems-Modeler-Pro/pull/165) | Inherited visible Parametric value/constraint roles, context-specific binding validation/evaluation, placement and reopen/history checks | Head `c6774522bc574695e2e0185b4ecab09ab9b190a3`; merged as `ce42b34aeaee08652ac335725b38aa3f03bda56f`; five reported workflows successful, including both packaging workflows |
| [166](https://github.com/rbrundage5/Systems-Modeler-Pro/pull/166) | Failed typed-feature creation removes the newly inserted element; multiplicity bounds are checked | Head `891289fa78567e93de632b69b8ab52ba37901891`; merged as the audit baseline; five reported workflows successful, including both packaging workflows |

These are delivered fixes, not remaining tasks. Their GitHub review lists were
empty when inspected. Merge and green checks do not supply independent review.
An overall completion percentage would be misleading: the 22 coverage parents
are not a weighted, fully dispositioned set of child acceptance checks.

## Method and evidence limits

Read current `AGENTS.md`, the documentation index, Step 5 scope/coverage, agent
workflow and publication rules before auditing current source. Cloned only the
authoritative repository into the hosted task workspace and pinned the baseline.
Inspected core semantics, native commands, frontend assembly, history/persistence,
runtime, interchange, collaboration, tests and release controls. Historical reports
were leads, not current findings. No personal files or other repositories were used.

The supplied SysML 1.6 and UML 2.5.1 PDFs were read for Copy, ItemFlow,
connector typing and PackageMerge requirements. The supplied CATIA guide was used
only as secondary workflow context. No source text or diagrams are republished;
no current vendor-version parity claim is made. The other uploaded textbooks were
not needed to decide the findings below.

Worker execution is disabled by current repository instructions; no delegated
workers were launched. This direct audit is not independent review. There is no
local Rust toolchain or installed Windows desktop session in this audit environment.
Native results below are retrieved GitHub CI evidence at the exact integrated SHA.
The two new witnesses execute actual frontend source with mocked native IPC; they
do not constitute native fault injection or rendered acceptance.

This report accounts for all 22 parent areas and all nine diagram families. It is
not a completed clause-by-clause conformance audit of every child or every line of
code. Untested dimensions remain UNVERIFIED; missing implementation and unexecuted
acceptance are separate categories.

### Executed and retrieved verification

| Evidence | Result | Boundary |
| --- | --- | --- |
| [Integrated native CI 35993864931](https://github.com/rbrundage5/Systems-Modeler-Pro/actions/runs/35993864931) | `core`, Windows `desktop-check`, Linux `desktop-linux-check` successful | Retrieved job/step results include format, core/persistence tests/lint, desktop tests/lint and integration contracts; not a fresh local Rust run |
| [Main release checks 35993865003](https://github.com/rbrundage5/Systems-Modeler-Pro/actions/runs/35993865003) | `release-contract` and `signed-package` successful | Signing uses a disposable test identity; `plan` and `publish` skipped |
| [Main installer 35994903047](https://github.com/rbrundage5/Systems-Modeler-Pro/actions/runs/35994903047) | Installer build, provenance and disposable-host install/launch smoke successful | Does not demonstrate interactive modeling or an installed A-to-B update |
| Main updater/setup workflows | `35993864934` and `35993864900` successful | Configuration/controller checks are not product or worker-environment qualification |
| Twelve existing frontend test files | **153 tests passed, zero failures/skips** | Production handler/controller tests with mocked environments; command below |
| [Completion witness](reviews/2026-09-24/completion-evidence.cjs) | **Two remaining gaps reproduced** | Expected-defect assertions; not a passing-product regression suite or CI addition |
| `validate_rust_authority.py`, `validate_history_integration.py` | Passed | Source/architecture guards; not proof of every mutation's atomicity |
| GitHub PR reviews #164-166 | Empty | Independent review remains outstanding |
| GitHub Releases collection | Empty | No release was returned to the connected account; no production update delivery established |

Frontend test command executed at the baseline:

```sh
node --test scripts/test_new_project_outcome.cjs scripts/test_open_project_outcome.cjs scripts/test_save_as_outcome.cjs scripts/test_element_specification.cjs scripts/test_dialog_escape_ownership.cjs scripts/test_cancelled_presentation_gesture.cjs scripts/test_ibd_port_gesture.cjs scripts/test_standard_editing_clipboard.cjs scripts/test_connector_properties.cjs scripts/test_item_flow_properties.cjs scripts/test_item_flow_notation.cjs scripts/test_collaboration_ui.cjs
node docs/reviews/2026-09-24/completion-evidence.cjs
python3 scripts/validate_rust_authority.py
python3 scripts/validate_history_integration.py
```

## Confirmed remaining defects and reliability gaps

All findings below use the product baseline above. P0 denotes potential loss or
mixing of authored session state; P1 denotes broken model/workflow invariants.
Each row is an individual audit finding. Implementations must receive their own
bounded work order and relevant native regression before closure.

| ID / priority | Finding and current evidence | Required completion evidence |
| --- | --- | --- |
| **TC-01 / P0** | **New Project publication is not atomic.** [`workspace.rs::new_project`](../apps/desktop/src-tauri/src/workspace.rs) replaces Project before acquiring later diagram/Behavior/ReqIF/path locks. Activity and execution resets occur later in [`project-reset-integrity.js`](../apps/desktop/frontend/project-reset-integrity.js) and other wrappers. Source-confirmed failure window; native reproduction not executed here. | Native late-lock/failure fixtures preserve all old repositories, path, history and runtime on failure. Successful New retires the old session through an explicit native contract. Split authored publication and runtime retirement if necessary. |
| **TC-02 / P0** | **Open can publish the new model while retaining the old history.** [`open_project_file_complete`](../apps/desktop/src-tauri/src/workspace/bdd_elements.rs) atomically replaces authored repositories but does not include history/runtime. [`project-open-compat.js`](../apps/desktop/frontend/project-open-compat.js) performs fallible snapshot/refresh work before returning `committed`; [`undo-redo-ui.js`](../apps/desktop/frontend/undo-redo-ui.js) resets history only afterward. New witness reproduces this frontend gap. | Inject failure after native publication and at history/runtime locks. A successful replacement cannot undo into the preceding project; failure before commit preserves it entirely. UI refresh failure must not leave history associated with a different session. Qualify same-project reopen too. |
| **TC-03 / P1** | **Generic rejected edits still consume history/redo.** `undo-redo-ui.js::checkpointIfNeeded` checkpoints before many commands based on display labels. [`history.rs::commit_snapshot`](../apps/desktop/src-tauri/src/workspace/history.rs) pushes undo and clears redo immediately. The production-source witness reproduces a rejected reconnect requesting this checkpoint. Dedicated specification transactions are fixed; the generic path is not. | Through actual create/reconnect/delete/edit handlers, rejection and no-op preserve both stacks; a committed change creates exactly one native history entry. Replace one command contract at a time, preserving existing Rust-owned transactions. |
| **TC-04 / P1** | **Local unsaved-work protection is incomplete.** New/Open handlers replace the model without a native authored-revision Save/Discard/Cancel policy. The inspected status uses file identity rather than a complete saved-revision contract. Form-local `dirty` flags do not protect the project. | Native current/saved revisions cover every authored repository. New/Open/window close and failed/concurrent Save preserve work consistently. Native file pickers, explicit cancellation and recovery feedback complete the user journey. |
| **TC-05 / P1** | **Legacy detail editing mutates before validation.** [`bdd_elements.rs::update_bdd_element_details`](../apps/desktop/src-tauri/src/workspace/bdd_elements.rs) changes type/documentation/default/unit fields before its final `validate_element`. For a ValueType, an invalid unit reference can return an error with earlier edits retained. The command remains registered; the newer element specification path is separately staged. | A native negative test sets documentation plus an unresolved unit on a valid ValueType and compares complete before/after state and history. Stage the entire legacy edit or retire it after proving every caller uses the qualified replacement. |
| **TC-06 / P1** | **Requirement reconnect remains outside the complete transaction contract.** [`requirements.rs::reconnect_traceability_relationship`](../apps/desktop/src-tauri/src/workspace/requirements.rs) checkpoints, holds diagrams before Project, mutates endpoints, validates, and later resolves/routes presentations. Copy validates before updating text; reconnecting to a different-text supplier is rejected by the now-enforced Copy invariant before propagation. Other views and transitive Copy clients are not staged. | Separate rollback/lock-order and Copy-reconnect leaves. Test different supplier text, chain/branch propagation, conflicting suppliers, missing/invalid presentations, reused views, labels, undo and reopen. Invalid edits publish nothing; valid reconnect propagates through the shared Copy authority. |
| **TC-07 / P1** | **Generic relationship deletion bypasses dependency checks.** [`relationship_editing.rs::delete_bdd_relationship_in_state`](../apps/desktop/src-tauri/src/workspace/relationship_editing.rs) directly removes any supplied relationship ID and BDD edges without kind restriction or dependent connector/ItemFlow/profile validation. Lock order was repaired; semantic deletion remains separate. This concerns the exposed native command, not proof that every invalid ID is offered by the visible UI. | Delete a connector used by ItemFlow through the exposed boundary: reject or consistently update dependencies and every view in one transaction. Distinguish symbol removal, relationship deletion and deletion of a linked usage; reusable Block types must survive usage deletion. |
| **TC-08 / P1** | **Authored ItemFlow validation is weaker than runtime flow validation.** [`ibd.rs::validate_connector_compatibility/validate_item_flow`](../crates/model-core/src/ibd.rs) checks endpoint types/topology and conveyed classifiers but not effective flow-feature direction and compatibility. [`structural_runtime.rs::validate_item_flow_contract`](../crates/model-core/src/structural_runtime.rs) has additional effective-direction/conveyed-type checks. | Share an appropriately scoped normative contract across authoring, load/import and runtime; preserve the distinction between legal connectors and legal conveyed flows. Positive/negative cases cover conjugation, in/out/inout, subtype flows, inherited flow properties and reversed direction. Reference: SysML 1.6 §9.3.2.12. |

Two further boundaries require audit without overstating a demonstrated defect:
legacy `save_project_file/open_project_file` commands remain registered beside the
repaired complete paths; and other mutation handlers, including Activity edge
creation/reconnect, need late-routing failure tests. Do not describe the active
complete Save/Open authored-state transaction as still missing. Review remaining
overlapping lock lifetimes rather than infer deadlocks from textual lock order alone.

## Missing capability and workflow completion register

P2 below means a substantial completion dependency, not permission to omit it from
the requested professional product. Rows that span subsystems must be split before
implementation. A full runtime solver or vendor-specific integration is a product
scope choice; it is not automatically mandated by the SysML language.

| ID / coverage | Current disposition and source | Completion contract |
| --- | --- | --- |
| **TC-09 / C05 / P2** | **Requirement containment and recursive Copy: MISSING.** `model.rs::validate_owner_kind` permits Requirement owners only in package namespaces. Copy text propagation does not implement owned subrequirement correspondence. SysML 1.6 §16.3.2.2 defines recursive Copy constraints. | Three-level owned requirement tree; valid reparent/rename; stable correspondence when copying; leaf changes propagate without changing local IDs; invalid cycles/conflicts rollback; tree/UI/native/import/export preservation. Implement ownership before recursive Copy/adapters. |
| **TC-10 / C05,C18 / P2** | **Requirements engineering workbench: MISSING/UNVERIFIED depth.** Element and relationship editors exist; inspected frontend/core paths do not establish a full requirement table, trace matrix, coverage rules, suspect-change lifecycle or persisted verification-result/evidence workflow. | Navigate requirement -> satisfying design -> verifying TestCase -> dated/configured result/evidence. Filter by missing links, failures and suspect impacts. Define coverage explicitly; a Satisfy or Verify edge is never equivalent to passing verification. Reuse semantic IDs and the existing repository. |
| **TC-11 / C03,C15 / P2** | **Explicit redefinition/subsetting: MISSING.** `Element` lacks these semantic links; `structural_runtime.rs` explicitly diagnoses conflicting inherited feature names without them. Current inherited-membership work is real but does not implement feature override semantics. | Stable feature references, legal type/multiplicity constraints, effective-member resolution, selectors/compartments, instance storage, persistence and adapters. Test diamonds, visibility, duplicate names and redefinition conflicts. |
| **TC-12 / C04 / P2** | **Association-typed connectors and full AssociationBlock workflow: MISSING.** `Connector` has context/kind/two ends but no Association type; `ConnectorEnd` has path/role/port without independent end multiplicity. Existing [design](C04_CONNECTOR_TYPING_DESIGN.md) remains applicable. UML 2.5.1 §§11.8.10-11.8.11. | Implement the design's ordinary typed-connector representation, lifecycle and property-editor leaves first; then coherent AssociationBlock identity, participants, connector properties and internal decomposition. Preserve untyped connectors and old files. |
| **TC-13 / C03,C04 / P2** | **Nested port paths and complete interface contracts: PARTIAL.** `resolve_structural_path` traverses Part/Reference properties and a terminal Port, including inherited membership. It does not traverse a Port-to-nested-Port chain. Effective provided/required contracts need full authoring/load/runtime qualification. | Repeated occurrences, nested/conjugated ports, inherited contracts, assembly/delegation and invalid path/type cases work through the same native authority and survive edit/undo/import. Do not reimplement the already-merged inherited part/terminal-port support. |
| **TC-14 / C03,C16,C17 / P2** | **Association breadth: PARTIAL.** Core supports explicit ends; composition linkage now exists. BDD create/reconnect still rejects self endpoints even where a reflexive association can be meaningful. Generic association Property identity/ownership and shared aggregation are not closed by composition alone. | Two named usages of the same type, legitimate reflexive/parallel associations, editable end multiplicity/navigation and stable IDs work in BDD/IBD, routing, duplication and round trips. Do not relax unrelated self-link prohibitions. |
| **TC-15 / C07 / P2** | **PackageMerge semantics: MISSING.** Relationship endpoints exist; `namespace.rs` implements PackageImport/ElementImport resolution, not PackageMerge combination. UML 2.5.1 §12.2.3.2. | Receiving/merged package semantics, compatible member combination, conflicts/visibility and deterministic namespace results; tests must show semantic effects beyond drawing a merge arrow. |
| **TC-16 / C05,C08-C10,C19 / P2** | **Cross-repository semantic references: PARTIAL/UNVERIFIED.** Core relationships use core Element IDs; Activity and Behavior have separate repositories. The existing relationship register identifies unresolved allocation/trace/refinement endpoint coverage. | Explicitly qualify each supported NamedElement endpoint and property path across repositories, including design/behavior/TestCase links; show navigation, reference-safe deletion, history and interchange. Recheck exact stereotype applicability rather than infer all endpoint restrictions are valid. |
| **TC-17 / C08,C12 / P2** | **Conditional and Expansion Activity semantics: MISSING.** `activity_execution.rs::warn_for_structured_semantics` explicitly states clauses and expansion-node/mode semantics are not represented. Contained flows running with warnings do not establish full node semantics. | Add accurate preflight limits first; then separately author/validate/persist/execute conditional clauses and expansion semantics. Supported token concurrency, pins, events, structured loops and cancellation require deterministic positive/negative traces. |
| **TC-18 / C09,C10,C13,C14 / P2** | **Behavior execution breadth: PARTIAL/UNVERIFIED.** `operation_signal_sequence_execution.rs` supports calls, signals and replies and explicitly rejects other message sorts. State hierarchy/history/submachine machinery exists; complete conformance is not established here. | Keep unsupported execution explicit. Qualify message ordering/reply correlation, fragment control, participant lifetime and gates; implement missing lifecycle cases only under scoped requirements. Separately qualify State priorities, orthogonal events, entry/do/exit cancellation, connection points and history. |
| **TC-19 / C11,C15 / P2** | **Parametric depth: PARTIAL.** PR165 covers inherited Block roles. Inherited ConstraintBlock parameter/expression definitions, arbitrary nested binding paths and Block-instance equality execution remain excluded. The scalar evaluator uses directed expressions/topological dependencies, not unrestricted simultaneous solving. | Complete inherited definitions and path-aware bindings as separate leaves. Expose supported expression, unit/dimension and solver limits before Run; deterministic values/errors and authored-state isolation. General nonlinear/offset-unit solving needs an explicit additional product scope. |
| **TC-20 / C20 / P2** | **Ordinary-workspace all-nine collaboration: MISSING.** [`SharedEdit`](../crates/persistence/src/collaboration.rs) covers bounded semantic/Requirement/BDD operations. Normal modeling commands are not all routed through it; a separate Shared Projects UI is not the full ordinary-workspace journey. | Reuse existing Rust authority and [C20 extraction plan](C20_COLLABORATION_COMPLETION.md): typed BDD dispatch/session first, then IBD, Requirement/Use Case/Package and each behavior/Parametric family. Cover bulk imports, permissions, revisions, conflicts, inverse operations and reconnect without whole-snapshot overwrites. |
| **TC-21 / C20,C21 / P2** | **Deployed collaboration/governance: UNVERIFIED complete journey.** Authentication, durable retries, presence and own-change reversal exist. [Native acceptance](C20_COLLABORATION_ACCEPTANCE.md) still records unexecuted two-device scenarios. Branch/merge, administration and lock policy require explicit scope. | Two installed clients on different devices, matching server, HTTPS, editor/viewer/revocation, conflicting edit/delete, offline/restart recovery and actor-scoped undo. Add ordinary-workspace per-family acceptance; the older bounded Shared Projects checklist alone cannot close C20. |
| **TC-22 / C16-C18 / P2** | **Rendered editing and notation: UNVERIFIED.** Source and handler tests support many interactions; no native sparse/dense/nested all-nine visual record was produced here. | Per-family create/place/connect/edit/move/resize/clipboard/delete-vs-remove/ESC/pan/zoom/fit/drill-down/undo/save/reopen. Inspect arrow/diamond ends, multiplicities, labels, frames, repeated ports, self/parallel routes and Route/Clean results. Include keyboard/focus and supported DPI/display sizes. |
| **TC-23 / C21 / P1** | **Whole-application scale: UNVERIFIED; costs identified.** `history.rs` retains up to 100 full seven-field snapshots, including geometry history. `namespace.rs::resolve_qualified_name` recomputes paths across elements and sorting also recomputes names. Several staged edits clone the model. These are profiling targets, not measured user-facing failures. | Measure release builds, real rendering/IPC, p50/p95, memory, cancellation and long-session history. Optimize evidenced bottlenecks while preserving undo, stable IDs and all-nine behavior. Existing core scale tests are insufficient to claim a 100,000-element interactive workspace. |
| **TC-24 / C19,C21 / P2** | **Interchange and hostile-input qualification: PARTIAL/UNVERIFIED.** Native/portable/mapped spreadsheet/ReqIF/bounded model-script/XMI paths exist. Native XMI round trips may embed authored state; genuine producer DI is a different contract. Existing records do not establish CATIA/Cameo release-specific interchange. | Current capability matrix per adapter/field/family; authentic approved producer fixtures; unchanged/changed reimport and removals; invalid-owner/type/endpoint rollback; linked composition/inheritance/profile preservation. Exercise decompression, parser depth, expanded size/count budgets and cancellation with benign synthetic adverse fixtures. |
| **TC-25 / C01,C22 / P2** | **Command ownership and capability ledger: PARTIAL.** Override chains and label-based mutation policy remain. Documentation indexes old unmerged statuses and an UPDATE REQUIRED import contract. Several old draft PRs remain open although related implementation has been integrated elsewhere. | One explicit owner per lifecycle/interaction/semantic contract; retire demonstrated duplicate paths without broad rewrites. Maintain current implemented/UI-exposed/tested/native-accepted evidence per child. Mark superseded PR records after comparing actual trees; do not merge stale feature branches to chase badges. |
| **TC-26 / C22 / P1 for update-delivery goal** | **Production release/update channel: NOT ESTABLISHED.** GitHub returned no releases. Release jobs use disposable signing for validation; observed publication jobs skipped. The workflow requires production activation/signing policy. | After release qualification and owner-controlled activation, publish an exact-tested-commit signed release. On an installed machine, update A -> B, reopen the saved model, verify offline/failure behavior and running-instance protection. Private signing material stays outside source/chat; updater signing is distinct from Authenticode. |
| **TC-27 / C01,C22 / P1 release gate** | **Independent review and end-to-end qualification: OUTSTANDING.** #164-166 review lists are empty; source-marker guards can pass while the new witness reproduces history gaps. | Independent review of candidate changes plus a connected all-nine project journey covering lifecycle, failure/undo, shared reuse, import/reimport, supported runtime and two-device operation. Record exact binary/source SHA and unresolved defects. |

## Reconciliation with earlier audits

Do not reopen fixed work solely because an older report says it is absent:

| Historical finding | Current disposition |
| --- | --- |
| Cancel New/Open/Save As lifecycle defects | Existing outcome tests passed in this audit. Remaining TC-01/02 concern native publication and later failure boundaries. |
| Complete authored Save/Open missing | Complete paths now stage all authored repositories and save metadata transactionally; native failure fixtures exist in `bdd_elements.rs`. Retain legacy-command and history/runtime boundary audits. |
| Copied requirement local ID treated as read-only | `update_requirement` now separates editable local ID from supplier-controlled text. Recursive hierarchy remains TC-09. |
| Root/ownership and inheritance graph validation absent | Current `validate_ownership_tree` and `validate_generalization_graph` exist. Broad load/edit/adapter parity still needs its own evidence. |
| Composition creates no part | Fixed/integrated by #164 and preceding work. Current `AssociationEnd::property_id` and composition/linked-edit tests establish real semantic linkage. |
| BDD reconnect updates only the selected view | Current `stage_relationship_presentations` stages affected views and labels, including adding a missing endpoint presentation in another view. Do not carry forward REL-EDIT-01 unchanged. Generic history remains TC-03. |
| Inherited IBD membership / Sequence lifeline discovery / Parametric Block roles absent | Native inherited feature consumers have been added. General inheritance/redefinition and remaining definition/path cases are separately scoped above. |
| BDD delete lock order reversed | Current handler is Project-first. Dependency-integrity deletion is still TC-07. |
| CSP is null / retired behavior scripts still loaded | A restrictive CSP is now configured; legacy retirement is recorded. Continue input/native qualification without repeating the obsolete null-CSP claim. |
| #165/#166 packaging running | Completed successfully at both recorded PR heads. |

The 22 September review and 23 September relationship register remain historical
evidence. Their unverified child checks are not automatically closed by this
reconciliation.

## Coverage and completion gates

| Coverage | Completion disposition at this baseline |
| --- | --- |
| C01 Code/architecture | TC-03/05/06/25/27; Rust authority retained, command and independent-review gaps remain |
| C02 Identity/storage | TC-01-07; complete authored Save/Open improved, full session lifecycle remains open |
| C03 BDD | Composition delivered; TC-11-14/22 plus unqualified instances/slots/compartments and consumer parity |
| C04 IBD | Inherited membership delivered; TC-08/12/13/22 |
| C05 Requirements | Local-ID/Copy text and atomic specification improvements delivered; TC-06/09/10/16 |
| C06 Use Case | Existing authoring/specification paths; extension-point rename/delete, subject moves, reuse and round-trip/rendered acceptance UNVERIFIED |
| C07 Package | Import traversal improvements delivered; TC-15/23 and visibility/alias/ambiguity/reparent acceptance |
| C08 Activity authoring | TC-17/22; structured-node representation and failure-atomic edge edits need closure |
| C09 State authoring | TC-18/22; full nested/orthogonal/connection-point authoring acceptance UNVERIFIED |
| C10 Sequence authoring | Inherited selector improvements delivered; TC-18/22, order/lifetime/fragment/gate acceptance remains |
| C11 Parametric authoring | #165 delivered; TC-19/22 |
| C12 Activity execution | TC-17; supported execution plus unsupported structured semantics require explicit qualification |
| C13 State execution | TC-18; deterministic full child matrix UNVERIFIED |
| C14 Sequence/operation execution | TC-18; bounded execution implemented, broader sorts/controls incomplete or unverified |
| C15 Structural/Parametric runtime | TC-08/11/13/19/23; preserve existing runtime checks and authored-state isolation |
| C16 Shared editing | TC-03/06/07/14/22; all-family full journey not qualified |
| C17 Notation/visuals | TC-14/22; no new rendered native acceptance |
| C18 UI/UX | TC-04/10/22/25; discoverability, focus, DPI and high-density workflow remain |
| C19 Interchange/automation | TC-09/11-16/24; current adapter and genuine producer matrices remain |
| C20 Collaboration/governance | TC-20/21; bounded shared foundation is not all-nine ordinary-workspace collaboration |
| C21 Scale/reliability/security | TC-21/23/24; measured release workload and adverse-input/recovery evidence remain |
| C22 Documentation/release | TC-25-27; build/package green, production update delivery and independent/native acceptance open |

All nine families require a real connected fixture, not nine unrelated empty
diagrams. Reuse the same elements across BDD/IBD/Requirement/Use Case/Package;
connect Activity/State/Sequence behaviors and Parametric properties to their true
owners, types and contexts. Verify identity and semantics after cross-view edits,
undo, save/reopen and applicable interchange. A small success does not close dense
or deeply nested interaction checks.

### Scale qualification that is still needed

The current `project_relationship_scale_100k.rs` fixture validates 100,000
relationships over **1,101 elements**. `project_unit_scale_30k.rs` validates
**90,001 elements** focused on unit references. Both are useful core regressions;
neither renders or interactively edits a complete large engineering model.

Recommended workload design, not measured or accepted production limits:

- Connected projects near 1,000 / 10,000 / 100,000 semantic records, with realistic
  ownership, inheritance, ports, requirements and behavior mixtures.
- Separate sparse/dense/nested views near 50 / 250 / 1,000 visible symbols, so total
  repository size is not confused with the number of symbols on one diagram.
- On recorded Windows hardware, measure startup/open, repository search and
  navigation, properties, pan/zoom, move/resize, Route/Clean, import, save/reopen,
  undo/redo and collaboration sync. Include 100 retained history operations and
  repeated diagram/session switching; record peak and retained memory.
- Establish numeric p95 interaction, completion-time, memory, progress and cancel
  budgets before declaring a supported capacity. Report both native work and the
  renderer/IPC portion. Do not choose a delta/caching design until the profile
  identifies the actual bottleneck; caches require revision-safe invalidation.

## Dependency-ordered completion plan

| Order | Work package | Exit condition |
| --- | --- | --- |
| 1 | Native New publication; Open/history retirement; runtime session retirement; dirty-revision guard | TC-01/02/04 failure fixtures pass; no prior-session undo/execution or silent unsaved replacement |
| 2 | Generic command/history contract, legacy detail rollback, Requirement reconnect and safe deletion | TC-03/05/06/07 close independently; rejected/no-op edits preserve state/history, valid edits update every affected view |
| 3 | Requirements containment/recursive Copy and engineering workbench | TC-09/10 close through usable requirement-to-design-to-test workflows; adapter changes follow the core model |
| 4 | Structural semantics and relationship closure | TC-08/11-16 close in dependency order; start with core references/contracts before UI, runtime and adapters |
| 5 | Accurate runtime preflight and remaining authored/executable semantics | TC-17-19 supported cases have deterministic traces; missing capabilities are implemented or explicitly excluded from a named release scope |
| 6 | Shared Rust command extraction and ordinary-workspace collaboration | TC-20, then TC-21; one family at a time, existing offline journeys preserved |
| 7 | Native all-nine editing/notation, real interchange and measured scale | TC-22-24; fix reproduced problems and rerun affected gates; qualification begins early and repeats on the integrated candidate |
| 8 | Current capability ledger, independent review and installed release/update qualification | TC-25-27; exact tested release, production publication and successful installed A-to-B update |

This is an ordering of dependencies, not one giant implementation PR. Rendered
and performance checks should begin alongside the first fixes so they expose
problems early. The parent's completion claim requires every applicable child to
be VERIFIED or explicitly dispositioned outside a named release scope; known
defects cannot be relabeled as optional polish.

**Next executable leaf:** TC-01, atomic native New Project publication. Start from
the current main SHA after refreshing GitHub. Scope authored-state staging and
publication in `workspace.rs` plus the existing native state helpers and focused
tests. Reproduce a poisoned late lock with populated structural/Activity/Behavior
repositories, current path and history; specify which runtime/history retirement
work belongs to dependent leaves. Preserve the now-passing Cancel New behavior.
Independently review the bounded fix and verify its real PR/head/CI. Do not merge
automatically. This audit implements none of the repair work described above.
