# Pre-package engineering review — 21 September 2026

**Disposition: not ready to describe as a complete CATIA/Cameo replacement.**
Four concrete editing fixes are prepared as draft PRs. Broader code and source
review identifies additional authoring, execution, interchange and qualification
work. No application package is released by this review.

## Work order and evidence boundary

C22.02.01: reconcile the release-readiness register with current code, the approved
references, and executed qualification. This documentation-only work order permits
`docs/PREPACKAGE_AUDIT_2026_09_21.md` and one index entry in `docs/README.md`.
It does not authorize bundling independent product fixes into this PR.

Repository baseline: `2b9750e8b3ca6de76f86ec7f367f155513063862`. Lead-only work;
no workers dispatched. The user authorized continued improvements and draft PR
publication. Independent review and native desktop acceptance are outstanding.
Main is unchanged. No automatic merge, force push, publication of reference PDFs,
external reference retrieval, or new dependencies.

Approved evidence: uploaded project brief; CATIA Magic feature/deployment guide
sections 2, 3 and 5; SysML 1.6 and UML 2.5.1; current repository implementation and
test results. The CATIA guide is secondary and release-dependent. It supports
workflow comparisons, not release-specific compatibility certification. No approved
genuine vendor project/XMI fixture is present in the repository evidence reviewed.

## Prepared changes

Review order is **107 → 108 → 109 → 111**. Each PR is based on its predecessor,
so the combined candidate receives CI while each PR retains its own leaf diff.
Retarget each dependent PR to main after its predecessor is reviewed and merged.
The shared regression-entry conflict was resolved without dropping tests.

| Leaf | Candidate | Behavior changed | Explicit limits |
| --- | --- | --- | --- |
| C18.03.01 | [PR107](https://github.com/rbrundage5/Systems-Modeler-Pro/pull/107) | Apply structural name, documentation, type, default, multiplicity and supported feature flags in one Rust transaction; compatible types are searchable by qualified name/identity; rejected edits retain drafts; one undo step | This changes a typed element's classifier reference, not arbitrary metaclass conversion. Requirement/TestCase and specialized behavior editors remain distinct. |
| C16.06.01 | [PR108](https://github.com/rbrundage5/Systems-Modeler-Pro/pull/108) | Cancelled HTML symbol movement/resize restores original geometry across seven renderer families without discarding a Properties draft | Native pointer capture, touch, high-DPI and visual behavior still need installed-desktop acceptance. |
| C04.03.01 | [PR109](https://github.com/rbrundage5/Systems-Modeler-Pro/pull/109) | One Rust boundary projection for context/nested port preview and commit; square port resize; coalesced preview; connected routes and labels update together; context frame participates in authored save/history; proportional attachment on owner move/resize | Legacy diagrams adopt the visible frame on their first actual context-boundary edit. Clipboard and the separate selection-move command are not closed by this leaf. Live connector preview and dense port packing remain unqualified. |

PR107 standalone head `7f177038d42c64085179f6bdbfb7145e308db57c` passed all
native CI jobs in run 35613445846: **229 non-desktop tests, 196 desktop tests**,
format/lint, Linux compilation and existing frontend/integration gates. A subsequent
fixture-only change permits optional presentation metadata when combined with PR109.

PR108 standalone head `cd5ef99e50cb738b5377ccc6effbe2a0b8f0f976` passed all
native CI jobs in run 35612994914. Its gesture fixture demonstrates eight failures
on the original code and ten passes with restoration applied.

PR109 head `8a27f8f43406f64f2900216a2f4b6f2af2c5064f` passed all native CI jobs
in run 35615732982, including **199 desktop tests**. All six new Rust tests passed:
four-side projection; preview/commit and route-label consistency; frame/legacy
attachment; atomic history/native/portable round trip; invalid edits/routing failure;
and independence from unrelated stale routes.

The later combined frontend candidate passes **32 Node regressions**, including
five frame cases: pointercancel, lost capture, delayed commit plus legacy undo,
rejected commit, and stationary/sub-threshold clicks. All 21 existing static
integration validators pass. Static checks are structural guards, not visual tests.
Final integrated-head CI is recorded in the PRs; earlier success does not substitute
for that gate. PR107 final head `2038552d6544b5b0ad7c4cdf2d91d4bdc741b7de`
also passed every native CI job in run 35616250437. No Rust compiler or installed browser binary is available locally.

Final geometry stack: PR108 head `975ee323bf440269cfe85d043d3f08bba1b2b9a2`
passed all CI in run 35616500069. PR109 head
`1e35a8aaaa47c0f534120e5e0f52bf8c042960e0` passed all CI in run 35617108299,
including the combined 32-test frontend entry point. PR111 is the separate
selection-command follow-up; its latest qualification is recorded in that PR.

## Wider workflow review

The supplied professional-tool guide describes a model repository with multiple
presentations, specification editing, contextual reuse, relationships, traceability,
and configured execution. That is the comparison target, rather than visual
similarity alone. This pass keeps implementation evidence separate from complete
end-to-end qualification.

| Area | Evidence inspected or exercised | Current conclusion |
| --- | --- | --- |
| BDD and typed features | `element_specification`, feature editor, `bdd_conformance`, `pr8_bdd`, `pr43_ports`, `pr46_operation_parameter_reception`; PR107 transaction tests | Stronger atomic editing candidate; retype rejection protects existing IBD/binding references. No all-properties or full metamodel certification. |
| IBD and interfaces | `ibd.rs`, `ibd_geometry`, current IBD palette/Properties renderer, `pr11_ibd`, `pr43_ports`, `pr45_item_flow` | Direct port geometry addressed; connector specification, selection movement and port clipboard gaps remain below. |
| Requirements and traceability | Existing requirement editor, `pr21_requirements`, `pr42_allocation`, native traceability persistence tests | Existing authored semantics and round trips have automated evidence. Coverage/suspect/impact dashboards and a complete stakeholder workflow are not qualified here. |
| Use Case | `pr24_use_cases`; subject-boundary/actor notation and shared movement tests | Existing semantics and saved notation have automated evidence. Native context menus, editing, and dense layouts need visual acceptance. |
| Package/repository | `pr22_repository_editing`, `pr26b_package_diagrams`, package persistence and drill-down contracts | Identity/reparenting/navigation foundations have regression coverage. Discoverability and full keyboard navigation are not established by those checks. |
| Activity | `pr13_activity`, `pr31_activity_execution`, runtime bridge/reset/budget suites | Authoring and deterministic execution cases are tested. Each modeled action/region/configuration still needs an explicit supported-workflow record. |
| State Machine | `pr12_behavior`, `pr32_state_machine_*`, behavior metadata tests | Composition, event, fork/join and reset tests exist and ran in the non-desktop suite. Complete native authoring, animation and visual notation remain unverified. |
| Sequence | Behavior/sequence integration and `pr34_operation_signal_sequence`; runtime initialization switch | Execution supports a bounded set of message sorts; create/delete/found/lost execution is rejected, not equivalent to authoring support. |
| Parametric | `pr25_parametrics`, binding diagnostics, `pr35_parametric_runtime`, SQLite tests | Bounded evaluator, binding and authored/runtime isolation have automated evidence. This is not a general multiphysics or unrestricted equation solver. |
| Interchange | Native all-nine-family portable/XLSX/XMI round trips; XMI preview/lowering code | Native round trips pass. Specialized external diagram records require native payload; genuine CATIA/Cameo interoperability remains unverified. |
| Shared projects | `COLLABORATION_DESKTOP.md`, shared-project boundary and dedicated CI workflow | Current shared BDD/semantic operations are separate from the offline editor. Full multi-family shared authoring and collaborative undo are not delivered by these fixes. Dedicated two-client/native qualification remains separate. |
| Packaging/scale | CI jobs, existing scope/acceptance gates, available local tooling | No installed-desktop smoke pass, high-DPI visual pass, agreed scale measurements, or independent review in this environment. No final release qualification. |

## Open findings and ordered follow-up work

These are retained findings, not silently accepted limitations or claims that entire
coverage parents are audited. Each implementation needs a separate bounded work order.

### C16.08.01 — pasted individual ports can leave their owning boundary

**Code-confirmed defect; high priority.** In
`workspace/standard_editing.rs::paste_clipboard`, the `ClipboardItem::IbdPort` path
adds `PASTE_OFFSET` to both coordinates and appends the copy to its parent or context.
It does not project the copy onto that owner's boundary. Copying a left-edge port
therefore produces an interior port; a copied property with its entire port set is
a different path and must remain valid.

Acceptance: copy/paste one nested or context port on every side; preserve semantic
identity for presentation paste; show the copied symbol on the correct owner; test
cross-diagram context compatibility, cancellation, duplicate semantics, undo and
native/portable round trip. Reuse PR109's Rust projection. Pass the visible legacy
frame explicitly when needed. Missing/incompatible owners must reject atomically.

### C16.06.02 — the selection-move command can detach ports

**Code-confirmed defect; candidate [PR111](https://github.com/rbrundage5/Systems-Modeler-Pro/pull/111) prepared.** In
the baseline `workspace/standard_editing.rs::move_selection_items`, an IBD property is clamped
to the canvas but its ports receive the unclamped delta. For example, property and
left port at x=200 moved by dx=-300 produce property x=0 and port x=-100. Selecting
both a property and one of its ports can translate the port twice. Individual port
selection receives an unconstrained translation. The command also reroutes every
connector and replaces points without updating attached label anchors.

PR111 reuses the Rust boundary and atomic history helpers, processes parents before
children, and reroutes only affected connectors with labels. Four added Rust tests
cover the failure and no-op paths as well as attachment. It adds an optional
`framePreference` command field for legacy context ports. CI and independent review
must qualify its final head; the clipboard finding remains separate.

The command is registered through `standard_editing_bridge`; no general arrow-key
binding was found in the inspected frontend. Therefore the code defect is confirmed,
while a claim that keyboard nudging is already UI-exposed would be unsupported.

Acceptance: move a selected parent and child once, independent of selection order;
use actual clamped parent displacement; boundary-constrain independently selected
ports; reroute only affected connectors with labels; invalid geometry/router failure
preserves state and history. UI exposure of keyboard nudging is a separate shared
workflow increment with focus ownership and repeat/queue acceptance.

### C04.08.01 — IBD connector specification editing is incomplete

**UI gap confirmed in current source; high priority.** The IBD Properties override
in `frontend/ibd-ui.js` exposes a stable ID and Route IBD action for a selected
Connector, with Item Flow creation elsewhere in the palette. It does not provide
an Apply form for connector name/kind/endpoints or an existing ItemFlow editor.
The IBD script loads after the structural feature editor in `index.html`.

Split delivery: (1) a Rust transactional connector specification API and qualified
endpoint candidates, (2) a draft-preserving form using that API, (3) existing
ItemFlow direction/conveyed-type editing. Preserve semantic and presentation IDs,
all dependent diagrams, undo/redo and persistence. Reject incompatible endpoints
and invalid assembly/delegation topology without partially renaming the relationship.

### C04.09.01 — explicit association typing of connectors is absent

**Metamodel gap confirmed; specification required.** `model-core/ibd.rs::Connector`
contains context, kind, source and target but no association classifier reference.
The older structural-runtime document identifies the same limit. Do not advertise
complete AssociationBlock-typed connector behavior from endpoint type validation.

Dependency sequence: normative SysML/UML clause audit; model/validation and migration;
typing UI; applicable interchange; runtime semantics and negative cases. A raw
dropdown alone cannot close this capability.

### C14.01.01 — sequence authoring exceeds executable message sorts

**Current execution boundary confirmed.** Initialization in
`operation_signal_sequence_execution.rs` accepts synchronous call, asynchronous
call, asynchronous signal and reply. Other sorts return an explicit error.
Create/delete/found/lost semantics need separate lifetime/occurrence-specific work
with deterministic runtime tests; do not infer execution support from diagram icons.

### C19.06.01 — external vendor diagrams are not certified

**Evidence and implementation boundary confirmed.** `xmi_runtime.rs` blocks external
IBD/Activity/State Machine/Sequence presentations without native authored payload.
The producer-neutral synthetic fixture does not establish vendor interoperability.
Native all-nine-family round trips are a distinct, passing capability.

Next input: an approved, legally usable fixture from a named CATIA/Cameo version,
with expected semantic identities and diagrams. Then qualify preview, unchanged
reimport, changed-source reimport, source removals, rejection rollback and export.
No guessed dialect mapping or compatibility claim.

### C17.04.01 / C21.01.01 — native visual and scale qualification missing

**Unverified, not an invented defect.** Measure pointer responsiveness, render and
routing latency, memory, dense port/label overlap, and high-DPI behavior on named
hardware and model sizes. Agree budgets before claiming scale readiness. Native
WebView gestures, touch and visual appearance have not been exercised here.

## Release acceptance still required

1. Review and integrate the four leaf PRs; require all CI checks on the final heads.
2. Close or explicitly disposition the open editing findings above. Do not label
   general editing complete while those alternate paths remain inconsistent.
3. On an installed desktop, exercise create/reuse/rename/retype and connected
   move/resize/cancel/undo/save/reopen in all nine families. Include duplicate names,
   nested/repeated occurrences, multiple diagrams of the same semantic element,
   negative edits and recovery after failed commands.
4. Visually check all four port sides and corners at 50%, 100% and 200% zoom,
   moving/resizing parts and frames, connected labels, Escape/lost capture, and
   dense/nested diagrams. Verify no-op clicks do not create undo entries.
5. Qualify imports and execution against their stated boundaries. Preserve explicit
   diagnostics for unsupported cases; retain authored/runtime state isolation.
6. Perform independent review and a clean install/open/save/reopen smoke test of
   the actual packaged build. Only then mark that specific release scope ready.

SysML v2, unrestricted execution, proprietary native-file compatibility, complete
collaboration/governance and multiphysics integrations are larger capabilities;
they are not implied by this editing-hardening work.
