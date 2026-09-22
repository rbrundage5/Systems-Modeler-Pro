# Systems-Modeler-Pro: tool, SysML and code review

Date: 22 September 2026. Reviewed product baseline: **`848b9e9650627ff819cea3d2e2cff382fa77d3af`**, current main after [PR127](https://github.com/rbrundage5/Systems-Modeler-Pro/pull/127).

## Assessment

The application has substantial SysML authoring, persistence, execution and collaboration foundations. It is **not yet qualified as a complete professional SysML 1.6 modeling tool**. The most urgent work is data integrity and consistent command behavior, followed by semantic completeness and integrated engineering workflows. Adding more palette entries without addressing those foundations would increase the number of inconsistent paths.

This review identifies four reproducible frontend lifecycle/history defects, additional source-confirmed transaction and validation defects, missing SysML capabilities, and maintenance/qualification work. These are different evidence categories; the register does not present every recommendation as a reproduced bug. Several connector, collaboration and reconnect limitations were already recorded before this review and remain open on current main.

There is no basis in the inspected inventory to label the server, importers, installer, updater or tests as unrelated junk. They support modeling, exchange, collaboration and delivery. There are specific obsolete frontend files and duplicated controllers worth removing or consolidating.

## Scope, method and limits

- Request: review the whole product for tool usability, SysML correctness, wrong/redundant code and code with no useful product purpose. This change is a report and a read-only evidence probe; it changes no production behavior.
- Scope record: `REV-2026-09-22`; baseline above; branch `codex/full-tool-sysml-review`; allowed changes are this report, its evidence under `docs/reviews/2026-09-22/`, and the documentation entry point.
- Review performed directly by the lead. No workers were dispatched; repository worker execution remains disabled. No independent review is claimed.
- Inventoried 394 tracked files: 163 Rust files / 102,176 lines; 51 JavaScript files / 13,852 lines; 76 Markdown files / 6,152 lines. Counts include tests and documentation, not just production code. They are inventory measurements, not quality or performance scores.
- Inspected semantic models, native command handlers, persistence, active HTML/script entry points, execution boundaries, shared edits, tests and release records. All nine families and all 22 parent coverage areas receive a disposition below. This is a broad engineering review, not a claim that every line or every child of the Step 5 checklist was independently qualified.
- Executed the four-case Node probe against actual frontend functions and handler registration order. Native IPC is mocked. A successful probe means the defects were reproduced; it does **not** mean the application passed acceptance.
- No Rust compiler or native desktop/browser session was available for this review. Source-confirmed Rust failure paths still need native regression fixtures and installed-desktop acceptance. No visual, DPI, accessibility, performance, penetration-test or full vendor-interchange qualification is claimed.
- All findings below share the baseline above. Fix PR and independently verified fix commit: **none in this review**. Suggested owners are component responsibilities for later bounded work, not dispatched agents.

### References used

Approved uploaded references were read without modifying or publishing them:

| Reference | Use in this review |
| --- | --- |
| OMG SysML 1.6, `formal-19-11-01.pdf` | Requirements/Copy: 16.3.2.2; flow properties and ItemFlows: 9.3.2.8 and 9.3.2.12; AssociationBlock/ConnectorProperty: 8.3.2.7 and 8.3.2.13 |
| OMG UML 2.5.1, `formal-17-12-05.pdf` | Connector and ConnectorEnd: 11.8.10–11.8.11, including optional Association typing and end constraints |
| Uploaded CATIA Magic feature/deployment guide | Sections 2, 3 and 5: repository/specification workflows, diagrams, traceability and execution. Secondary workflow guidance; not proof of a particular current vendor release |
| Uploaded *A Practical Guide to SysML* | Sections 13.7–13.9: requirements tables/matrices and distinction between package organization and requirement containment |
| Repository scope, coverage, architecture, import and collaboration records | Product intent, previous acceptance and remaining work; current code takes precedence over stale milestone prose |

SysML Distilled did not load in this session and was not used. No claim of CATIA/Cameo certification, SysML v2 implementation or complete commercial-tool parity is made. Simulation depth and collaboration are product capabilities; their absence is not automatically a violation of the SysML modeling language.

## Prioritized findings

P0 means potential loss or partial publication of authored state. P1 means broken existing workflow, semantic acceptance/rejection error or misleading execution. P2 means important completeness, architecture or qualification work. Priorities describe urgency, not the strength of evidence.

### R01 — Cancel New Project still clears Activity state

**P0 · C02/C16/C18 · DEFECTIVE · frontend reproduction executed.**

In [activity-ui.js](../apps/desktop/frontend/activity-ui.js), lines 397–407, the New handler awaits the previous handler and then resets Activity state whenever a project exists. [app.js](../apps/desktop/frontend/app.js) returns normally when the name prompt is cancelled. The later [project-reset-integrity.js](../apps/desktop/frontend/project-reset-integrity.js) guard runs after that inner handler has already reset the Activity workspace. The probe observed `reset_activity_workspace`, `clear_activity_executions`, and an emptied frontend Activity snapshot without any `new_project` call.

**Fix/acceptance:** project lifecycle commands must return an explicit committed/cancelled outcome; reset all dependent state only after successful project replacement. With an existing project containing unsaved Activity work, Cancel must preserve every repository, diagram, selection and undo/redo stack. Verify the actual assembled UI as well as the handler test. Owner: project lifecycle. Negative case: cancellation and native New failure both leave the session unchanged.

### R02 — Save is not one complete authored-state transaction

**P0 · C02/C19 · DEFECTIVE · source-confirmed, native fault injection pending.**

[workspace.rs](../apps/desktop/src-tauri/src/workspace.rs), `save_project_file` lines 323–344, commits the core project before writing BDD, IBD, Behavior and ReqIF metadata. [repository.rs](../crates/persistence/src/repository.rs), lines 103–109, commits `save_project` independently; `save_metadata` performs separate writes. Behavior validation/locks occur after earlier writes. Activity is saved through another frontend-triggered command, and its repository and diagram metadata are also separate writes in [activity_workspace.rs](../apps/desktop/src-tauri/src/workspace/activity_workspace.rs). The `_complete` save wrapper forwards to this same implementation.

Consequently, a later validation, serialization or I/O failure can leave a partially updated project file while Save reports failure. This is a source-established failure path, not a claim that a user's file has already been corrupted.

**Fix/acceptance:** prepare and validate a complete authored snapshot, then persist it with one database transaction covering every repository and relevant metadata. Publish the saved path/clean revision only after commit. Inject failures at each write stage; the old file must reopen as the old complete revision. Verify all nine families, Activity/Behavior cross-references, exchange identity and Save As. Owner: persistence; foundational dependency for R05 and R26.

### R03 — Open can replace the main project before Activity loading fails

**P0 · C02/C19 · DEFECTIVE · source-confirmed, native fixture pending.**

[workspace.rs](../apps/desktop/src-tauri/src/workspace.rs), `open_project_file` lines 354–381, publishes the core project and its repositories/path. The active [project-open-compat.js](../apps/desktop/frontend/project-open-compat.js) then separately calls `load_activity_workspace`. That loader validates Activity metadata before replacing the old Activity repository. Malformed Activity metadata can therefore fail after the main project has changed, leaving repositories from different sessions and no successful Open outcome.

**Fix/acceptance:** stage and validate the complete file before a single in-memory session replacement. Open a valid core file with invalid Activity metadata while another unsaved project is open; every old repository, current path and history entry must remain intact. Also preserve normal old-file compatibility. Owner: project lifecycle; share the complete snapshot contract with R02, but implement Open as a separate leaf.

### R04 — Cancel Open erases current history

**P1 · C02/C16 · DEFECTIVE · frontend reproduction executed.**

`project-open-compat.js` wraps even the prompt in `runCommand('Opening project…', ...)`. [undo-redo-ui.js](../apps/desktop/frontend/undo-redo-ui.js), lines 30–35, resets history after any fulfilled operation with that label. A cancelled prompt returns normally. The probe recorded `history_reset` with no `open_project_file` call.

**Fix/acceptance:** reset history only on a committed session change. Cancel Open after creating both undo and redo entries; both stacks must be identical afterward. Owner: lifecycle/history; use R01's explicit outcome rather than another label exception.

### R05 — Cancel Save As still requests an Activity write

**P1 · C02/C18 · DEFECTIVE · frontend reproduction executed.**

The Activity Save As wrapper in `activity-ui.js`, lines 418–433, unconditionally calls `saveActivityAfterBase()` after the base handler returns. The active base is `saveProjectAsComplete` from `bdd-completion-ui.js`. Cancel returns normally; the old `current_file` remains set. The probe recorded `save_activity_workspace` without either core save command. Thus a cancelled dialog can persist one repository to the previous file.

**Fix/acceptance:** only one complete Save command may persist authored state. Cancel Save As must issue no persistence command and leave the previous file unchanged. Owner: lifecycle/persistence; depends on the complete-save contract, with a focused frontend cancellation fix independently reviewable.

### R06 — The UI defeats the new atomic requirement-history guarantee

**P1 · C05/C16 · DEFECTIVE · frontend reproduction executed.**

The actual requirement Apply path in `app.js:633` uses `Updating Requirement…`. `undo-redo-ui.js:19–27` recognizes only two labels as already checkpointed by Rust, neither of which matches that operation. It calls `history_checkpoint` before invoking the now-transactional Rust requirement update. The probe reproduced an early checkpoint on a rejected operation. Rust `commit_snapshot` clears redo. Successful edits can also acquire an extra checkpoint, and no-op applies can change history; those consequences follow from the source path and still need integrated regression cases.

**Fix/acceptance:** make history ownership explicit in the command contract and move mutation/history authority into Rust. Do not derive transaction behavior from user-facing status strings. Through the real Apply button, test rejection, no-op, success, Undo and Redo; rejection/no-op preserve stacks and success creates exactly one entry. Owner: history/requirement command adapter. PR125's native tests remain useful but did not cover this assembled frontend path.

### R07 — BDD reconnect/create can mutate semantics before presentation failure

**P1 · C03/C16 · DEFECTIVE · source-confirmed.**

[relationship_editing.rs](../apps/desktop/src-tauri/src/workspace/relationship_editing.rs), `reconnect_bdd_relationship` lines 145–241, updates semantic endpoints before locating the diagram, endpoint presentations and edge or routing. Only an immediate `project.validate()` error restores the original relationship. A missing presentation leaves a semantic edit despite a returned error. Successful reconnect updates only the requested view. [bdd_elements.rs](../apps/desktop/src-tauri/src/workspace/bdd_elements.rs), `create_bdd_relationship_complete`, also creates semantics before later routing. Activity edge creation has a similar mutation-before-route pattern and needs a separate family-specific leaf.

**Fix/acceptance:** stage semantics and all affected presentations/routes/labels, validate, then commit once. Test missing edge/node and route failure with byte-equivalent state/history afterward; test a reused relationship on two diagrams. Owner: structural editing. Split BDD reconnect, BDD create and Activity edge rollback into separate implementations.

### R08 — Requirement reconnect bypasses complete Copy and presentation updates

**P1 · C05/C16 · DEFECTIVE · source-confirmed; carried forward.**

[requirements.rs](../apps/desktop/src-tauri/src/workspace/requirements.rs), `reconnect_traceability_relationship` lines 222–327, checkpoints first, modifies semantic endpoints, then resolves remaining presentations and routes. Copy reconnect assigns only the direct client's text rather than using the transitive propagation authority. Other views and label anchoring are not updated by this handler. This remained open in the 21 September report; PR124/125 did not fix reconnect.

**Fix/acceptance:** reconnect through a staged Copy/traceability operation that updates every affected client and view and rejects conflicting suppliers before publication. Test missing presentations, a Copy chain/branch, conflicting suppliers, reuse across diagrams, labels and save/reopen. Failure preserves model, geometry and undo/redo. Owner: requirements; depends on the shared transaction contract, then the hierarchy work in R12 for full nested Copy support.

### R09 — Generic relationship deletion bypasses dependency integrity

**P1 · C02/C03/C05 · DEFECTIVE · source-confirmed.**

`relationship_editing.rs`, `delete_bdd_relationship` lines 247–276, accepts an existing diagram and relationship ID, removes any matching relationship from the project map and removes BDD-family presentations. It does not restrict the relationship kind or validate dependent ItemFlows, profile references or other repositories. The command is registered and the generic property form invokes it. Direct native invocation with a connector referenced by an ItemFlow can remove its realization without dependent cleanup/rejection; this is not a claim that every such invocation is currently exposed by a visible IBD button.

**Fix/acceptance:** one semantic relationship-deletion authority must check dependencies, update all affected presentations and commit atomically. Keep removal of a symbol distinct from deletion of semantics. Test connector/ItemFlow and profile-reference cases, shared presentation reuse, Undo and failure rollback. Owner: semantic lifecycle.

### R10 — Whole-project validation is weaker than creation/edit validation

**P1 · C02/C03/C07/C19 · DEFECTIVE · source-confirmed.**

[model.rs](../crates/model-core/src/model.rs), `validate_element`/`validate` lines 1300–1580, checks many local constraints but does not enforce root existence/kind, a coherent ownership tree or generalization acyclicity. Creation/reparent/reconnect paths have checks that loaded state does not receive. For example, a self-owned Package satisfies the local owner-kind check; an injected two-Block generalization cycle is not traversed by `validate()`. Association-end validation here only verifies referenced elements exist; creation-time structural invariants need parity checks. Copy text agreement is also not checked globally.

**Fix/acceptance:** define invariants once and enforce them at every trust boundary, including load and import. Add negative serialized fixtures for missing/wrong root, ownership cycle, inheritance cycle, malformed association ends and inconsistent Copy graphs. Reject with contextual diagnostics before replacing the session. Reconcile existing Copy lifecycle paths before tightening validation so legitimate edits/reimports remain usable. Owner: model-core; split independent invariant families into separate leaves.

### R11 — Copied requirement IDs are incorrectly treated as read-only

**P1 · C05 · DEFECTIVE · source/standard comparison.**

`model.rs`, `update_requirement` lines 1099–1113, permits a Copy client only when both the requirement ID and text are unchanged. SysML 1.6 §16.3.2.2 makes the copied **text** read-only; it does not require the client's local requirement ID to equal its previous value. The current rule rejects valid renumbering even when text is unchanged.

**Fix/acceptance:** compare copied text separately from editable local metadata; continue enforcing ID validity/uniqueness. A copied requirement can receive a new unique ID while retaining text and semantic UUID; altered copied text and duplicate IDs still reject atomically. Owner: requirements core; verify UI, import/reimport and undo paths.

### R12 — Requirement containment and recursive Copy are incomplete

**P2 · C05/C19 · MISSING capability · source/standard comparison.**

`model.rs`, `validate_owner_kind` around lines 1740–1770, permits Requirement ownership by package namespaces but not by another Requirement. Package organization and `deriveReqt` do not replace a requirements containment hierarchy. SysML §16.3.2.2's recursive `isCopy` constraint includes owned subrequirements and their structure. Propagating text through chains of Copy dependencies, as PR124 does, is useful but does not implement that nested hierarchy rule.

**Fix/acceptance:** specify contained requirement ownership first, then tree navigation/reparenting, native/interchange preservation, and recursive Copy with explicit stable correspondence. Create a three-level hierarchy, reuse/copy it, update a leaf, and verify structure/text agreement and rollback of conflicting changes. Owner: requirements metamodel; split schema/ownership, UI, Copy and adapters into dependent leaves.

### R13 — Interface and ItemFlow validation does not enforce full flow contracts

**P1 · C04/C15 · DEFECTIVE/incomplete semantics · source/standard comparison.**

[ibd.rs](../crates/model-core/src/ibd.rs), lines 247–317, treats equal or inheritance-related endpoint types as compatible in either direction. It rejects conjugated FullPorts, but this check does not inspect effective ProxyPort flow directions, conjugation, or required/provided feature contracts. `validate_item_flow` validates classifier existence, uniqueness and connector endpoint matching, but not whether the conveyed classifier/direction agrees with applicable flow properties. SysML §§9.3.2.8 and 9.3.2.12 define those constraints when flow properties are present.

**Fix/acceptance:** centralize effective interface-feature and directional conformance resolution; use it in ItemFlow validation and applicable runtime transport. Test conjugated/non-conjugated ProxyPorts, subtype flow compatibility, incompatible conveyed items and reversed direction. A connector's mere existence must not be mistaken for a claim that every item can traverse it. Owner: IBD semantics/runtime; separate model validation from transport implementation.

### R14 — Endpoint paths do not fully support inherited and nested ports

**P2 · C03/C04 · MISSING capability · source-confirmed.**

`ibd.rs`, `resolve_structural_path` and `validate_connector_end` around lines 146–216, traverse part/reference properties with exact owner matching; the terminal port must be owned directly by the reached classifier. The resolver does not traverse nested port paths or accept a port inherited from a general classifier through effective membership. The model has an `inherited_features` helper, but this endpoint path does not provide complete feature-resolution behavior.

**Fix/acceptance:** define a typed occurrence/feature path contract and resolve inherited membership consistently. Test an inherited port on a specialized Block, a nested ProxyPort path, repeated part occurrences, invalid ownership and save/reopen. Add redefinition/subsetting semantics only with an explicit identity and compatibility design. Owner: structural semantics; prerequisite for corresponding palette and endpoint picker completeness.

### R15 — Association-typed connectors remain a design, not an implementation

**P2 · C03/C04 · MISSING capability · carried forward.**

Current `Connector`/`ConnectorEnd`/`ItemFlow` fields in `ibd.rs` do not provide the full Association type, ordered end correspondence/multiplicity, AssociationBlock identity, association-realized ItemFlow and optional itemProperty capability. [C04_CONNECTOR_TYPING_DESIGN.md](C04_CONNECTOR_TYPING_DESIGN.md) is a useful implementation specification; merging PR123 did not add those model fields or workflows. UML §§11.8.10–11.8.11 and SysML §§8.3.2.7, 8.3.2.13 are the relevant existing design references. Untyped connectors are legal; the missing feature is support for the typed cases.

**Fix/acceptance:** follow that design's dependency leaves: core representation/validation, editing, persistence/interchange, lifecycle, then runtime interpretation where applicable. Positive/negative typed-end and multiplicity cases must survive reopen without changing connector identity. Owner: structural semantics.

### R16 — Execution capabilities need an explicit supported-semantics contract

**P1 for misleading execution; P2 for feature expansion · C08/C10/C12/C14/C15 · mixed DEFECTIVE risk/MISSING capability.**

[activity_execution.rs](../crates/model-core/src/activity_execution.rs), `warn_for_structured_semantics` lines 428–454, explicitly says ConditionalNode clauses and ExpansionRegion modes/nodes are not represented; it warns and executes available contained control flows. A warning does not make that execution equivalent to the authored structured notation. Conversely, [operation_signal_sequence_execution.rs](../crates/model-core/src/operation_signal_sequence_execution.rs), lines 450–465, explicitly rejects message sorts outside synchronous/asynchronous calls, signals and replies. That rejection is a sound bounded limitation, not a newly discovered execution bug.

[parametrics.rs](../crates/model-core/src/parametrics.rs) represents a single output expression per parsed constraint, topologically orders producers, and rejects dependency cycles/multiple producers. It is a bounded directed scalar evaluator, not a general simultaneous-equation solver. Unit conversion records multiplicative scale, not offset conversions. These limitations should be visible before Run, and must not be advertised as unrestricted engineering analysis.

**Fix/acceptance:** publish a machine-readable capability/preflight result with element-linked diagnostics. Unsupported semantic constructs must block a claimed full execution or require an explicitly labelled partial analysis mode. Preserve deterministic supported runs. Expand Conditional/Expansion, Sequence object lifecycle and parametric solving as separate specified features. Owner: execution engines/UI. No simulation completeness is inferred from metamodel enum presence.

### R17 — Collaboration does not cover the ordinary all-nine-family workspace

**P2 · C20 · MISSING capability · carried forward, rechecked against current source.**

[collaboration.rs](../crates/persistence/src/collaboration.rs), `SharedEdit` lines 153–237, exposes a bounded semantic/Requirement/BDD command set. Authentication, revisions, retry identity, presence and user-scoped undo have meaningful implementations and tests. The shared BDD geometry/creation extraction from PR104–106 is now in main. That does not provide the missing IBD, Activity, State, Sequence and Parametric edit protocols or ordinary-workspace integration. `shared-workspace.js` is a common local workspace adapter; its filename is not proof of collaborative editing.

**Fix/acceptance:** continue the existing C20 dependency plan with one authoritative command family at a time. Qualify two authenticated native clients, conflicting edits/deletes, reconnect/restart, actor-scoped undo and viewer rejection before expanding. Keep local presentation preferences distinct from shared authored state. Owner: collaboration/command core. Branching, governance and deployment UX remain separate capabilities.

### R18 — Traceability needs a usable engineering workbench

**P2 · C05/C18 · MISSING/UNVERIFIED workflow depth.**

Requirement IDs/text, TestCases and traceability relationships exist, but the inspected main frontend/requirement command paths do not provide a complete requirements table/matrix, defined coverage calculation, suspect-change lifecycle, impact traversal and persisted verification-evidence workflow. A `verify` edge alone does not prove a test passed or that a requirement is satisfied. The approved CATIA guide and Practical Guide §§13.7–13.9 support these as useful professional workflows, not additional mandatory SysML diagram families.

**Fix/acceptance:** first specify coverage meaning and evidence ownership; then add a filterable requirements table and trace matrix backed by existing semantic IDs. Navigate a requirement to design and TestCase, record evidence/result provenance, change the requirement, and show the defined impact/suspect outcome without silently rewriting relationships. Owner: requirements/engineering workflow. Split views, queries and evidence lifecycle; do not add a second requirements database.

### R19 — Global frontend override chains make behavior depend on script order

**P2 · C01/C16/C18 · architecture defect contributor.**

[index.html](../apps/desktop/frontend/index.html), lines 48–94, loads many scripts that replace `render`, `renderProperties`, `requireInvoke`, `runCommand` or button handlers. Individual adapters are legitimate, but ownership is implicit. R01/R04/R05/R06 demonstrate actual cross-layer consequences, not merely a style preference. Additional wrappers are likely to keep masking, rather than removing, inconsistent ownership.

**Fix/acceptance:** after fixing data-loss paths, extract explicit lifecycle and command adapters, a property-panel registry, and one event owner per interaction. Preserve the current Rust semantic/render-command authority and replace one override chain per PR. Test the assembled production load order and native invocation, not only each script in isolation. Owner: frontend shell. Avoid a wholesale UI rewrite.

### R20 — Three legacy behavior scripts are removal candidates

**P2 · C01/C22 · source-confirmed unreachable application scripts.**

`behavior-runtime-hardening.js`, `behavior-safe-transition.js` and `behavior-nested-transition-notation.js` remain in the frontend directory but are not loaded by either application HTML entry point. [validate_behavior_integration.py](../scripts/validate_behavior_integration.py), lines 141–146, explicitly forbids loading them. Their old behavior/routing patches are superseded by the Rust-authoritative command/renderer path.

**Fix/acceptance:** remove the dead files in a small cleanup after recording their replacements and confirming no packaging/runtime references. Keep the guard against reintroducing the obsolete approach. Do not make them active to justify their presence. Owner: frontend maintenance. No production files are deleted by this review.

### R21 — Repeated parsers, forwarding APIs and oversized adapters need consolidation

**P2 · C01 · maintenance finding.**

`workspace/behavior_completion/state.rs` and `transition.rs` duplicate trigger parsing; `message.rs` and `sequence.rs` duplicate message-sort parsing. Save/open `_complete` forwarding commands coexist with the base commands, increasing ambiguity about which path actually owns validation. Some other `_complete` commands add real behavior, so blanket deletion is unsafe.

`spreadsheet_import.rs` is 8,975 lines including tests; `model_script.rs` 4,524; `standard_editing.rs` 3,071. These adapters serve valid needs. Extract shared typed builders and family-specific parsing only when it removes a demonstrated duplicate rule or isolates a bounded testable contract. Broad `allow(dead_code)` for bulk modeling, `rustfmt::skip` on standard editing and `include!`-assembled large modules deserve targeted cleanup, not arbitrary file-size targets.

**Fix/acceptance:** one parser per semantic contract; compatibility wrappers with documented callers and removal conditions; importer parity fixtures before/after extraction. Unchanged inputs must produce the same stable identities and errors. Owner: command/import architecture. Keep semantic rules in model-core rather than multiplying adapter-specific validation.

### R22 — Passing checks do not yet qualify complete user journeys

**P1 · C01/C16/C17/C22 · UNVERIFIED acceptance with demonstrated test gap.**

The integration CI passed, yet the assembled lifecycle/history probe reproduces four defects. Source-marker scripts are useful architectural guards, but checking that a function name/string exists cannot prove rollback, a reachable button path, correct geometry or cancellation. Native Rust tests bypass the frontend checkpoint in R06; isolated frontend tests can miss handler composition.

**Fix/acceptance:** add a small high-value journey suite using production assembly and realistic native command outcomes, then native desktop tests. Required journeys: complete project lifecycle; failed/no-op edits with history; reused relationships across diagrams; all-nine save/reopen; import preview/apply/reimport/rollback; supported execution/reset; and two-client collaboration. Keep meaningful existing tests. One green workflow is not whole-product qualification. Owner: verification/release.

### R23 — Scale claims need measured workloads

**P2 · C21 · UNVERIFIED performance.**

[history.rs](../apps/desktop/src-tauri/src/workspace/history.rs) stores up to 100 complete snapshots of project, structural diagrams, Behavior and Activity repositories. Some staged edits compare serialized JSON. This is a concrete scaling cost to measure, not proof that current performance is unacceptable. Large imports, full refresh/render and routing also need representative measurements.

**Fix/acceptance:** define small/dense/large connected fixtures and record hardware, latency distributions, peak memory and file size for editing, snapshots, route, import and save/open. Agree budgets before optimizing. Consider bounded deltas/structural sharing or indexed lookup only after profiling, preserving reliable undo. Verify cancellation/progress for long operations. Owner: performance; no invented CATIA-equivalent capacity claims.

### R24 — Input and desktop hardening need bounded qualification

**P2 · C21 · UNVERIFIED security/reliability; configuration observation confirmed.**

[tauri.conf.json](../apps/desktop/src-tauri/tauri.conf.json) has `csp: null`. This is a hardening gap, not evidence of a demonstrated injection exploit. XMI/ReqIF handlers contain 64 MiB input limits, and ReqIF checks archive entry size; those checks should be retained. Compressed workbook expansion, parser depth, row/element/relationship counts and actual streamed read budgets still need adversarial fixtures and memory measurements. Review dependency advisories in an authorized environment rather than assuming safety from the language used.

**Fix/acceptance:** separately add a compatible CSP and resource-budget tests; verify ordinary imports, previews, rendering and dialogs still work. Invalid/oversized input must stop with a contextual diagnostic and no state change. Cover collaboration access revocation and retry recovery on deployed native clients. Owner: desktop/import/release security; this review is not a penetration test.

### R25 — Current status documentation contradicts merged reality

**P2 · C22 · DEFECTIVE documentation.**

The 21 September report still says candidates are unmerged, and the collaboration completion record says main is unchanged from an older baseline. PR127 integrated that batch. The import qualification document exists, but the entry point still marks it UPDATE REQUIRED; its contract needs reconciliation. Historical evidence should remain historical, with a clear successor rather than rewritten test claims.

**Fix/acceptance:** the entry point now directs readers to this current-baseline review. Follow up with a capability ledger tying implemented/UI-exposed/tested/native-accepted status to exact commits; distinguish merged foundations from completed features. Update migration/import and structural-runtime limitations against actual fixtures. Owner: documentation/release. No all-nine parent becomes VERIFIED solely because a PR merged.

### R26 — Local project lifecycle needs dirty-state protection and usable file dialogs

**P1 for unsaved-work protection; P2 for dialog UX · C02/C18 · source-confirmed workflow gap.**

The inspected local New/Open handlers prompt for a name/path and replace the project without an authored-revision Save/Discard/Cancel guard. The status bar's “unsaved” indication denotes absence of a file path, not later unsaved edits to an existing file. The inspected native close handler manages the startup update window, not preservation of main-window authored work. Path entry through `prompt()` and relative-path retries also give users a fragile file workflow. The separate collaboration dialog has draft guards, but those do not protect the local project session.

**Fix/acceptance:** track a Rust-authored revision and last fully saved revision; use native file pickers and a consistent Save/Discard/Cancel outcome for New/Open/close. Save failure or Cancel must preserve the current project. Verify edits after an initial save, all repositories, keyboard-triggered close, Save As and session recovery. Owner: project lifecycle; implement after R01–R05's transaction/outcome foundations.

## All nine diagram families

“Present” below means inspected model/command/UI paths exist. It does not mean the full family is qualified. All nine still require native rendered and integrated lifecycle acceptance.

| Family | Present foundations | Main improvement/qualification work |
| --- | --- | --- |
| Block Definition | Classifier/features, associations, inheritance, profiles, typed values, operations and presentation commands | R07/R09/R10; complete AssociationBlock/connector identity; qualify inherited features, redefinition/subsetting, instances/slots and all compartments. Avoid treating an enum as a full specification editor |
| Internal Block | Context, parts/references, ports, connectors, ItemFlows, improved boundary geometry and clipboard | R13–R15. Preserve recent boundary-drag/undo fixes. Native tests must cover zoom, pan, corners, frame/part resize, repeated ports, crowded sides, route/label attachment and save/reopen |
| Requirement | IDs/text, relationships, TestCases, Copy-chain propagation and staged specification edit | R06/R08/R11/R12/R18: hierarchy, correct local metadata editing, atomic lifecycle, recursive Copy, tables/matrices and defined verification evidence |
| Use Case | Actors, subject boundaries, include/extend, extension points, specification editing | Existing `update_use_case_specification` stages a candidate and validates Extend locations: retain that pattern. Qualify extension-point rename/removal, subject moves, reuse, scenarios and presentation/native round trips; no new semantic bug asserted without a failing case |
| Package | Model/Package/ModelLibrary, containment, namespace resolution, aliases/import relationships | R10. `namespace.rs` implements precedence and ambiguity handling. Qualify visibility, cyclic imports, merge semantics beyond storing a PackageMerge edge, repository navigation and context-correct diagram reuse |
| Activity | Actions/pins, control/object flows, partitions, structured-node representations and token execution | R01–R03/R07/R16. Complete Conditional clauses/Expansion representation and semantics before claiming full execution. Qualify loop/interrupt/exception/action coverage individually rather than infer it from structured-node names |
| State Machine | Composite/orthogonal structure, history/submachines, trigger/guard/effect and run-to-completion machinery | Preserve existing execution tests. Qualify priorities, connection points, cancellation of do behavior, time/event configuration, native nested-state editing and persistence. Consolidate duplicate trigger parsing; no all-state-semantics completeness claim |
| Sequence | Lifelines, message sorts, fragments/operands, gates, invariants and occurrence-aware editing | Explicit runtime subset in R16; keep unsupported message execution fail-closed. Qualify occurrence order independently of position, complex fragment nesting, create/delete lifecycle, reply linkage and two representations of the same interaction |
| Parametric | Constraint blocks/properties/parameters, typed bindings, values/units, directed evaluation and runtime integration | R16/R23: explicit language/solver contract, useful diagnostics, scope/instance selection, unit dimensions and supported conversions. Simultaneous/nonlinear equations are a separate feature, not an assumed property of existing bindings |

## Professional workflow direction

Use the approved CATIA guide as evidence for useful modeling journeys, while retaining this tool's architecture and product identity:

1. **One semantic repository, multiple presentations.** Creation, placement of an existing element, duplication, removal of a symbol and deletion of semantics need distinct commands and visible meanings. Properties, tree, diagrams, tables and imports must address the same stable IDs.
2. **One complete specification workflow.** Required type, ownership, multiplicity, port/interface, relationship and behavior fields should be discoverable, validated together and applied atomically. Preserve drafts on errors; offer navigation to referenced elements and usages.
3. **Reliable diagram editing.** Make boundary-port movement, reconnect, route/clean, labels, nested containment, multi-selection, keyboard/focus and undo consistent across families. Layout must respect context boundaries and maintain semantic endpoints. Rendered sparse/dense fixtures are essential.
4. **Engineering traceability beyond drawing.** Requirements tables, matrices, impact traversal and defined test/evidence records should make it possible to answer what satisfies a requirement, what verifies it and what changed.
5. **Trustworthy execution.** Show supported semantics and setup requirements before Run; provide step/reset, event/value inspection and diagnostics linked to model elements. Runtime state must remain separate from authored state.
6. **Repeatable interchange.** Users need preview, contextual diagnostics, stable source identity, explicit removal policy, atomic apply, unchanged/changed reimport and export round trips. Existing CSV/XLSX, JSON, ReqIF, XMI and bounded scripting are useful foundations; real vendor fixtures are required for vendor-specific claims.
7. **A complete collaboration journey.** Bring existing server authority into ordinary editing incrementally, preserving revision/conflict/undo semantics. Do not equate a separate shared-project window with completion of the all-nine-family workflow.

These are product improvements, not a proposal to copy every CATIA licensed feature, introduce SysML v2 implicitly, or add an unrelated application.

## Code relevance and cleanup decisions

| Area | Disposition | Reason and boundary |
| --- | --- | --- |
| Rust semantic, routing, persistence and execution crates | Keep and strengthen | Core modeling authority. Fix transaction/validation reuse before adding more adapter rules |
| Desktop frontend/renderers | Keep; consolidate explicit ownership | Necessary UI. Replace the demonstrated override hazards in bounded slices |
| Three unused legacy behavior scripts in R20 | Remove candidates | Intentionally excluded from active HTML and superseded; do not reactivate |
| Trigger/message parsers and pure forwarding commands | Consolidate after caller audit | Repeated semantic parsing and confusing API names; some similarly named commands add real behavior |
| Large spreadsheet/model-script/bulk-model modules | Keep; extract demonstrated duplication | Modeling input and automation are directly useful. Size alone is not a reason to delete or rewrite |
| Collaboration server/authentication/history | Keep | Directly supports multi-user systems modeling; bounded command coverage still needs expansion |
| Installer/updater/release workflows | Keep | Necessary professional distribution and updates; not unrelated modeling clutter |
| `.github`, repository agent controls and engineering scripts | Keep as development support | They are not end-user modeling features. Worker controls remain disabled and are outside this review's modification scope |
| Tests, examples and qualification documents | Keep; reconcile stale claims | They protect behavior and demonstrate import formats. Historical records must not be presented as current acceptance |
| Optional broad integrations, external lifecycle connectors, advanced governance | Explicit future scope | Useful only with a defined engineering journey and interoperability evidence; no speculative integrations are added here |

No obviously unrelated production application was identified in the inspected inventory. That conclusion is limited to this review and is not a malicious-code or supply-chain attestation.

## Coverage disposition

The [Step 5 checklist](STEP5_AUDIT_COVERAGE.md) remains the acceptance authority. The following accounts for every parent without falsely closing its unexecuted child checks.

| Coverage | Review disposition | Evidence / next qualification |
| --- | --- | --- |
| C01 Code/architecture | DEFECTIVE + UNVERIFIED remainder | R06/R19–R22; locking/deadlock and full dependency review remain open |
| C02 Identity/storage | DEFECTIVE | R01–R05/R09/R10/R26; native rollback, old-file migration and recovery fixtures needed |
| C03 BDD | DEFECTIVE/MISSING + UNVERIFIED remainder | R07/R09/R10/R14/R15 and family table |
| C04 IBD | DEFECTIVE/MISSING + UNVERIFIED remainder | R13–R15; recent geometry fixes retained, native dense scenes pending |
| C05 Requirements | DEFECTIVE/MISSING | R06/R08/R11/R12/R18 |
| C06 Use Case | UNVERIFIED full acceptance | Candidate validation inspected; extension/scenario/native cases in family table |
| C07 Package | DEFECTIVE + UNVERIFIED remainder | R10; namespace implementation inspected, merge/visibility/native scope pending |
| C08 Activity authoring | MISSING + UNVERIFIED remainder | Structured clause/expansion representation in R16; action/pin/region child checks remain |
| C09 State authoring | UNVERIFIED full acceptance | Existing composite/submachine/configuration paths inspected; rendered nested cases pending |
| C10 Sequence authoring | UNVERIFIED full acceptance | Message/fragment/gate paths inspected; occurrence-order/native cases pending |
| C11 Parametric authoring | UNVERIFIED full acceptance | Binding/constraint/type/unit paths inspected; integrated configuration and diagnostics pending |
| C12 Activity execution | MISSING + UNVERIFIED remainder | R16; deterministic supported semantics and authored-state isolation still need complete child matrix |
| C13 State execution | UNVERIFIED full acceptance | Existing runtime/history/submachine coverage retained; not re-executed locally |
| C14 Sequence/operation execution | MISSING supported breadth + UNVERIFIED remainder | Explicit message-sort rejection in R16; supported calls/signals need native scenario qualification |
| C15 Structural/parametric runtime | MISSING + UNVERIFIED remainder | R13/R14/R16/R23; occurrence/configuration and solver limits must be recorded |
| C16 Shared editing | DEFECTIVE | Four executed frontend reproductions plus reconnect/deletion findings; all-family gestures pending |
| C17 Notation/visuals | UNVERIFIED rendered acceptance | Source paths and existing routing/label work inspected; no new screenshot/desktop evidence |
| C18 UI/UX | DEFECTIVE/MISSING + UNVERIFIED remainder | R01/R05/R18/R19/R26; focus/DPI/accessibility require native acceptance |
| C19 Interchange/automation | DEFECTIVE + UNVERIFIED full adapter matrix | R02/R03/R10/R12/R24; retain source identity/preview tests, add connected all-nine fixtures |
| C20 Collaboration/governance | MISSING/UNVERIFIED complete journey | R17; foundations in main, ordinary workspace/two-device/deployment acceptance outstanding |
| C21 Scale/reliability/security | UNVERIFIED qualification | R23/R24; concrete observations are not performance or security certification |
| C22 Documentation/release | DEFECTIVE + UNVERIFIED release acceptance | R22/R25; integration CI is green, current capability/native acceptance ledger incomplete |

## Verification evidence

### Executed for this review

```sh
node docs/reviews/2026-09-22/frontend-evidence.cjs
```

Result: exit 0, four expected defects reproduced at the baseline. The [probe](reviews/2026-09-22/frontend-evidence.cjs) extracts actual application functions, BDD lifecycle replacements, Behavior and Activity lifecycle registration, and the requirement Apply button handler. It loads the later Open/history/reset overrides in production order. It uses a VM and mocked DOM/IPC; it does not simulate Rust persistence, native rendering or all frontend scripts. The New case also records the earlier Behavior wrapper clearing State, Sequence and Parametric runtime sessions on Cancel. It is an audit witness, not a regression test whose current expectations should be added to normal CI. Once fixed, turn each case into a normal positive invariant test.

Documentation relative-link/path checks and `git diff --check` were also run for this report. No production implementation or broad new test suite was added.

### Existing integration evidence retained

The integrated candidate `7f8f7e1d31699a61ec94f81bcd7944e925df651e` has the same tree as reviewed main. Its recorded CI evidence remains relevant:

| Check | Recorded result and limits |
| --- | --- |
| [Native foundation run 35739096050](https://github.com/rbrundage5/Systems-Modeler-Pro/actions/runs/35739096050) | 261 non-desktop Rust tests, 240 Windows desktop Rust tests, 53 modeling Node tests; Linux job passed. Tests, not complete native UI acceptance |
| [Desktop client qualification 35739096154](https://github.com/rbrundage5/Systems-Modeler-Pro/actions/runs/35739096154) | 23 focused client Rust tests and 27 collaboration Node tests. The 23 are a subset of desktop coverage, not extra unique tests |
| [Server qualification 35739096013](https://github.com/rbrundage5/Systems-Modeler-Pro/actions/runs/35739096013) | Passed; overlaps the non-desktop suite |
| [Windows installer 35739096272](https://github.com/rbrundage5/Systems-Modeler-Pro/actions/runs/35739096272) | Build/install and 15-second process smoke passed; does not prove a modeling journey |
| [Desktop update checks 35739096078](https://github.com/rbrundage5/Systems-Modeler-Pro/actions/runs/35739096078) | Passed |
| [Agent setup checks 35739096009](https://github.com/rbrundage5/Systems-Modeler-Pro/actions/runs/35739096009) | Passed configuration checks; does not enable workers or prove environment isolation |
| [Windows release 35739096007](https://github.com/rbrundage5/Systems-Modeler-Pro/actions/runs/35739096007) | Contract and signed-package checks passed; production plan/publish intentionally skipped |

The 80 integrated frontend checks (53 modeling + 27 collaboration) were also run during the preceding integration task. They were not rerun merely to inflate this documentation review's evidence. Green checks do not invalidate the new four-case reproduction or the source-confirmed missing transactions.

## Recommended implementation sequence

Every item below is a work package to split into individual validated leaves, not authorization to combine unrelated fixes into one PR. Preserve existing identity, file compatibility, Rust authority and all-nine behavior.

| Order | Work package | First bounded deliverables / exit evidence |
| --- | --- | --- |
| 1 | Stop cancellation/data-loss paths | R01 first; R04 and R05 separate fixes using explicit lifecycle outcomes. Actual UI cancellation must leave state/history/files untouched |
| 2 | Complete project transactions | R02 snapshot/persistence contract, then R03 staged Open; failpoint tests and all-nine reopen. Add R26 dirty-revision guard as a separate dependent leaf |
| 3 | Make edit/history authority consistent | R06 real requirement Apply path, then R07/R08/R09 independently. Rejection/no-op preserve redo; success changes all affected views once |
| 4 | Close core invariant gaps | R10 ownership/root, generalization, association and Copy validation as separate leaves; compatible legacy fixtures and contextual errors |
| 5 | Complete requirement semantics | R11 local ID editing, then R12 containment and recursive Copy, then adapters. Build R18 views/evidence on that model |
| 6 | Complete structural contracts | R13 directional ItemFlow/interface validation, R14 endpoint resolution, R15 typed connectors. Use the existing connector specification and preserve port interaction improvements |
| 7 | Make runtime limits trustworthy | R16 capability/preflight first; then independently specified Activity, Sequence and parametric expansion with deterministic fixtures |
| 8 | Consolidate demonstrated duplication | R19 one override chain at a time; R20 dead-script removal; R21 parsers/forwarders/import builders. Preserve production assembly and parity tests |
| 9 | Finish professional journeys | R17 ordinary collaboration command families; R18 engineering tables/evidence; native all-nine interaction and interchange qualification from R22 |
| 10 | Qualify scale, hardening and release | R23 measured budgets, R24 bounded security/input work, R25 current capability ledger and complete installer-to-model acceptance |

The immediate first fix should be **R01 (Cancel New Project clears Activity state)**. The first architectural investment should be **R02/R03's complete authored-session transaction contract**. Those directly protect engineering work and give the remaining feature work a reliable foundation.
