# Pre-package engineering review — 21 September 2026

Verification refreshed 22 September 2026.

**Disposition: not ready to describe as a complete CATIA/Cameo replacement.**
Fourteen bounded improvements are prepared as draft PRs. Broader code and source
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

Review order is **107 → 108 → 109 → 111 → 112 → 113 → 114 → 115 → 116 →
117 → 118 → 119 → 120 → 121**. Each PR is based on its predecessor,
so the combined candidate receives CI while each PR retains its own leaf diff.
Retarget each dependent PR to main after its predecessor is reviewed and merged.
The shared regression-entry conflict was resolved without dropping tests.

| Leaf | Candidate | Behavior changed | Explicit limits |
| --- | --- | --- | --- |
| C18.03.01 | [PR107](https://github.com/rbrundage5/Systems-Modeler-Pro/pull/107) | Apply structural name, documentation, type, default, multiplicity and supported feature flags in one Rust transaction; compatible types are searchable by qualified name/identity; rejected edits retain drafts; one undo step | This changes a typed element's classifier reference, not arbitrary metaclass conversion. Requirement/TestCase and specialized behavior editors remain distinct. |
| C16.06.01 | [PR108](https://github.com/rbrundage5/Systems-Modeler-Pro/pull/108) | Cancelled HTML symbol movement/resize restores original geometry across seven renderer families without discarding a Properties draft | Native pointer capture, touch, high-DPI and visual behavior still need installed-desktop acceptance. |
| C04.03.01 | [PR109](https://github.com/rbrundage5/Systems-Modeler-Pro/pull/109) | One Rust boundary projection for context/nested port preview and commit; square port resize; coalesced preview; connected routes and labels update together; context frame participates in authored save/history; proportional attachment on owner move/resize | Legacy diagrams adopt the visible frame on their first actual context-boundary edit. Clipboard and the separate selection-move command are not closed by this leaf. Live connector preview and dense port packing remain unqualified. |
| C16.06.02 | [PR111](https://github.com/rbrundage5/Systems-Modeler-Pro/pull/111) | Selection moves process parents before ports, use actual clamped movement, project independent ports, update connected routes and labels, and commit through atomic history | Corrects the registered command; no general arrow-key binding or new group-drag UI is introduced. Clipboard remains a separate finding. Native selection UX and independent review remain open. |
| C16.08.01 | [PR112](https://github.com/rbrundage5/Systems-Modeler-Pro/pull/112) | Parent-plus-port presentation paste creates one copied child and preserves connector mapping in either selection order | Keeps semantic identity; individual port placement is the following leaf. |
| C16.08.02 | [PR113](https://github.com/rbrundage5/Systems-Modeler-Pro/pull/113) | Standalone nested/context port paste retains its owner side, clamps along that boundary, and adopts the visible legacy frame atomically | Rejects incompatible contexts/missing owners; dense port packing remains open. |
| C16.08.03 | [PR114](https://github.com/rbrundage5/Systems-Modeler-Pro/pull/114) | Semantic Duplicate remaps occurrence paths and child presentation IDs; repeated views share one copied semantic property; selected parent/child does not alter the reusable type | Standalone duplicated ports intentionally create a new feature. Legacy frame adoption precedes copy insertion. |
| C16.08.04 | [PR115](https://github.com/rbrundage5/Systems-Modeler-Pro/pull/115) | Copy/Duplicate places part groups clear of existing parts with one translation; rejects a crowded frame before changing state/history | Bounded placement, not optimal packing or a complete dense routing/layout qualification. |
| C04.08.01a | [PR116](https://github.com/rbrundage5/Systems-Modeler-Pro/pull/116) | Atomic connector name/kind/endpoints transaction updates every presented view and remaps dependent ItemFlows | Every affected diagram must already present a replacement endpoint; invalid topology/routing rejects atomically. Association typing is separate. |
| C04.08.01b | [PR117](https://github.com/rbrundage5/Systems-Modeler-Pro/pull/117) | Connector Properties exposes qualified endpoint choices and one Apply with draft/error retention and stale-response guards | Native UI acceptance remains open. |
| Existing ItemFlow API | [PR118](https://github.com/rbrundage5/Systems-Modeler-Pro/pull/118) | Atomic name, forward/reverse direction and conveyed-classifier editing, with qualified choices and no-op/history handling | Retains realizing connector and relationship identity; does not implement flow execution. |
| ItemFlow notation | [PR119](https://github.com/rbrundage5/Systems-Modeler-Pro/pull/119) | Arrows follow Rust-derived direction; opposite same-classifier flows remain distinct; authoritative refresh removes deleted/undone flows and rejects stale reads | Visual/high-DPI/native interaction acceptance remains open. |
| ItemFlow Properties | [PR120](https://github.com/rbrundage5/Systems-Modeler-Pro/pull/120) | Select existing flow identity; edit name, direction and multiple conveyed classifiers through one transaction | Separate creation/deletion commands retain their existing scope; independent review remains open. |
| C04.08.01c | [PR121](https://github.com/rbrundage5/Systems-Modeler-Pro/pull/121) | Clean connector forms reload saved name/kind/endpoints after undo; unsaved drafts survive rerender | No collaborative conflict-resolution claim. |

| Candidate | Published head | Native CI run | Result |
| --- | --- | --- | --- |
| PR107 | `2038552d6544b5b0ad7c4cdf2d91d4bdc741b7de` | [35616250437](https://github.com/rbrundage5/Systems-Modeler-Pro/actions/runs/35616250437) | All three jobs passed |
| PR108 | `975ee323bf440269cfe85d043d3f08bba1b2b9a2` | [35616500069](https://github.com/rbrundage5/Systems-Modeler-Pro/actions/runs/35616500069) | All three jobs passed |
| PR109 | `1e35a8aaaa47c0f534120e5e0f52bf8c042960e0` | [35617108299](https://github.com/rbrundage5/Systems-Modeler-Pro/actions/runs/35617108299) | All three jobs passed |
| PR111 | `55e6a635c06f0bd42f168f96194c713f73c40c90` | [35619809423](https://github.com/rbrundage5/Systems-Modeler-Pro/actions/runs/35619809423) | All three jobs passed |
| PR112 | `651622f0d3e63a489bc7455547f87d84bcc96562` | [35635498699](https://github.com/rbrundage5/Systems-Modeler-Pro/actions/runs/35635498699) | All three jobs passed |
| PR113 | `1fc7cb04d94e953f98a595b4fa7faa256a1f6903` | [35637207141](https://github.com/rbrundage5/Systems-Modeler-Pro/actions/runs/35637207141) | All three jobs passed |
| PR114 | `1a7bd1f3359725454e4159557d12e775a00ded4a` | [35638562926](https://github.com/rbrundage5/Systems-Modeler-Pro/actions/runs/35638562926) | All three jobs passed |
| PR115 | `07938dc10d46c816523257891aff3e7162d49ead` | [35638566737](https://github.com/rbrundage5/Systems-Modeler-Pro/actions/runs/35638566737) | All three jobs passed |
| PR116 | `9af949da2ff58d7e355ee14f0fbf2508829a2faf` | [35638569181](https://github.com/rbrundage5/Systems-Modeler-Pro/actions/runs/35638569181) | All three jobs passed |
| PR117 | `5e8aa0922660b21d1877971dbf5eaa3692064319` | [35638572341](https://github.com/rbrundage5/Systems-Modeler-Pro/actions/runs/35638572341) | All three jobs passed |
| PR118 | `48e9a17d37dc21fafdb3ef108c4b796b8f736e5f` | [35638578340](https://github.com/rbrundage5/Systems-Modeler-Pro/actions/runs/35638578340) | All three jobs passed; 222 desktop tests |
| PR119 | `2e3f7d7cdde7dc1ddd654c1a7777696b11b4390c` | [35638961742](https://github.com/rbrundage5/Systems-Modeler-Pro/actions/runs/35638961742) | All three jobs passed; 223 desktop tests |
| PR120 | `5431a6b9580acd057ee5c0d518da6a280b369f63` | [35639502344](https://github.com/rbrundage5/Systems-Modeler-Pro/actions/runs/35639502344) | All three jobs passed |
| PR121 | `28bf2bed5c91e2634e16ea1e75ecf5adef1dc459` | [35639637040](https://github.com/rbrundage5/Systems-Modeler-Pro/actions/runs/35639637040) | All three jobs passed |

Local qualification of the combined PR121 tree: **53/53 frontend regression tests**,
all frontend JavaScript syntax checks, all 21 static integration validators and
diff hygiene pass. No installed-desktop visual claim is made. All candidates
remain draft and unmerged; dependency heads and final CI evidence above supersede
initial "pending CI" notes in the individual implementation work orders.
The combined core job also passes 229 Rust non-desktop tests with zero failures
or ignored tests. PR119 passes 223 Windows desktop tests, including the semantic
ItemFlow direction/identity and invalid-endpoint notation regression.

CI caught and drove additional fixes: near-corner port copies now preserve their
original side instead of switching to the nearest side; legacy frame adoption
occurs before inserting duplicated parts so its reroute sees valid original
geometry. The original left-port copy failure remains covered by PR115. A crowded
frame fixture was corrected from an invalid 220-pixel height to a valid 240-pixel
height so it exercises capacity rejection, without relaxing the geometry contract.
The connector undo-refresh regression failed before PR121 and passes afterward.

The combined candidate through PR111 passes **229 non-desktop tests, 206 desktop tests and
32 Node regressions**, plus format/lint, Linux compilation and existing integration
gates, with no failures or ignored tests. PR109's six Rust port-geometry tests cover four-side projection, preview/commit
and route-label consistency, frame/legacy attachment, atomic history/native/portable
round trip, invalid edits/routing failure, and unrelated stale routes. The frontend
suite includes five frame cases: pointercancel, lost capture, delayed commit plus
legacy undo, rejected commit, and stationary/sub-threshold clicks. PR108's original
gesture fixture reproduced eight failures before restoration and ten passes after.

All 21 local static integration validators pass. Static checks are structural
guards, not visual tests. No Rust compiler or installed browser binary is available
locally; Rust compilation and execution use the existing CI. PR111's earlier run
passed core/Linux and 205 of 206 Windows tests; its rollback test broke the wrong
endpoint and therefore exercised an unrelated stale route. The corrected fixture
preserves connection to the moved port, with no production or assertion relaxation.
All four selection regressions passed on the final head, including rollback,
no-op redo preservation and legacy-frame requirements.

## Wider workflow review

The supplied professional-tool guide describes a model repository with multiple
presentations, specification editing, contextual reuse, relationships, traceability,
and configured execution. That is the comparison target, rather than visual
similarity alone. This pass keeps implementation evidence separate from complete
end-to-end qualification.

| Area | Evidence inspected or exercised | Current conclusion |
| --- | --- | --- |
| BDD and typed features | `element_specification`, feature editor, `bdd_conformance`, `pr8_bdd`, `pr43_ports`, `pr46_operation_parameter_reception`; PR107 transaction tests | Stronger atomic editing candidate; retype rejection protects existing IBD/binding references. No all-properties or full metamodel certification. |
| IBD and interfaces | `ibd.rs`, `ibd_geometry`, current IBD palette/Properties renderer, `pr11_ibd`, `pr43_ports`, `pr45_item_flow`, PR109/111–121 | Port movement/clipboard/duplicate and connector/ItemFlow editing candidates are prepared. Association typing, dense layout and native acceptance remain open. |
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

**Baseline defect; candidates PR112–115 prepared.** In the main-baseline
`workspace/standard_editing.rs::paste_clipboard`, the `ClipboardItem::IbdPort` path
adds `PASTE_OFFSET` to both coordinates and appends the copy to its parent or context.
It does not project the copy onto that owner's boundary. Copying a left-edge port
therefore produces an interior port; a copied property with its entire port set is
a different path and must remain valid.

The separate `duplicate_selection_items` IBD path also offsets an individual port
without boundary projection. Duplicating a property clones its port presentations
without regenerating their IDs, translating their coordinates, or updating their
property paths. In a diagram retaining the source property, the repeated port IDs
hit `validate_ibd_diagrams`' duplicate-ID rejection. This is a source-confirmed
duplicate-path finding; no native UI reproduction is claimed. Repair presentation
paste and semantic Duplicate as separately reviewable increments, preserving their
different identity rules.

Acceptance: copy/paste one nested or context port on every side; preserve semantic
identity for presentation paste; show the copied symbol on the correct owner; test
cross-diagram context compatibility, cancellation, duplicate semantics, undo and
native/portable round trip. Reuse PR109's Rust projection. Pass the visible legacy
frame explicitly when needed. Missing/incompatible owners must reject atomically.
For Duplicate, require new semantic identities only where requested, fresh
presentation IDs, correctly remapped property paths, attached geometry and
connectors targeting the duplicated occurrences. Include parent-plus-port selections
in both orders and late validation failure without partial semantic creation.

Candidate evidence now covers those alternate identity/attachment paths, all four
port sides, connected copy placement, rollback and history. The final-head CI
table records passing runs. Native installed-desktop acceptance and independent
review still prevent declaring the whole clipboard workflow release-qualified.

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
`framePreference` command field for legacy context ports. Final-head CI passed in
run `35619809423`; independent review and native acceptance remain open. The
clipboard finding is addressed by the separate PR112–115 candidates.

The command is registered through `standard_editing_bridge`; no general arrow-key
binding was found in the inspected frontend. Therefore the code defect is confirmed,
while a claim that keyboard nudging is already UI-exposed would be unsupported.

Acceptance: move a selected parent and child once, independent of selection order;
use actual clamped parent displacement; boundary-constrain independently selected
ports; reroute only affected connectors with labels; invalid geometry/router failure
preserves state and history. UI exposure of keyboard nudging is a separate shared
workflow increment with focus ownership and repeat/queue acceptance.

### C04.08.01 — IBD connector specification editing is incomplete

**Main-baseline UI gap; PR116–121 candidates prepared.** The baseline IBD Properties override
in `frontend/ibd-ui.js` exposes a stable ID and Route IBD action for a selected
Connector, with Item Flow creation elsewhere in the palette. It does not provide
an Apply form for connector name/kind/endpoints or an existing ItemFlow editor.
The IBD script loads after the structural feature editor in `index.html`.

Split delivery: (1) a Rust transactional connector specification API and qualified
endpoint candidates, (2) a draft-preserving form using that API, (3) existing
ItemFlow direction/conveyed-type editing. Preserve semantic and presentation IDs,
all dependent diagrams, undo/redo and persistence. Reject incompatible endpoints
and invalid assembly/delegation topology without partially renaming the relationship.

The delivered candidates implement this split, including flow direction rendering
and removal of stale notation after undo/delete. The Rust transaction tests verify
multi-view endpoint remapping, dependent flows, stable identity, invalid topology,
undo/redo/no-op behavior and serialization. Frontend tests cover complete payloads,
draft retention, read failures, late responses and repeated Apply. Manual/native
acceptance and independent review are still outstanding. The ItemFlow work-order
tags C04.09.01a–c do not close the separate association-typing finding below.

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

1. Review and integrate the fourteen leaf PRs in order; require all CI checks on
   the final heads and requalify the actual integrated revision.
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
