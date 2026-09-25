# SysML workflow review and corrections — 25 September 2026

Work order: C22.workflow-evidence-2026-09-25. This documentation-only leaf updates
the capability record and index. Product baseline:
`2d1f51b0faef8abb840d7c848adca2f458bb92cc`. Application changes have separate bounded
work orders and PRs listed below. Direct primary-session work; independent review
and installed-desktop acceptance remain outstanding.

## Result

The tool has substantial SysML authoring and runtime implementation, but **all
SysML features are not complete or qualified**. This pass repairs concrete
relationship/history defects, adds reuse of an existing part on a BDD, and rejects
unsupported structured Activity execution before it can give misleading results.
Passing these tests does not establish complete SysML 1.6 conformance.

The New/Open development-entrypoint permission correction is already in the
baseline through PR175. The candidates below are published for review; they are
not automatically merged, released, or installed on the user's machine.

An integration discrepancy was found: PR172 and PR173 were merged into
`codex/reconnect-history-atomic` after that branch's earlier PR171 had merged to
main. Their fixes were consequently absent from the current main source. PR176
and PR177 restore those changes against the actual baseline. A merged PR badge
on a different base branch is not evidence that main contains its implementation.

## Corrected workflows

| PR | User-visible result | Regression evidence |
| --- | --- | --- |
| [176](https://github.com/rbrundage5/Systems-Modeler-Pro/pull/176) | Requirement reconnect stages endpoint validation, transitive Copy text updates and dependent presentations in one native transaction | Native rollback/history/reopen fixtures; actual frontend reconnect handler performs no extra pre-checkpoint |
| [177](https://github.com/rbrundage5/Systems-Modeler-Pro/pull/177) | Relationship deletion validates dependencies, retires affected presentations and preserves the linked part usage | Native deletion/history/rejection tests; frontend delete handler uses native history ownership |
| [178](https://github.com/rbrundage5/Systems-Modeler-Pro/pull/178) | Association-end edits update linked properties and validate dependent views in one transaction | Stable identity/owner/type, two BDDs, database reopen, no-op history and rejected-edit redo preservation |
| [179](https://github.com/rbrundage5/Systems-Modeler-Pro/pull/179) | Existing PartProperty can be shown as a composition on a BDD without creating another Block or part | Two parts sharing one type, two views, repeat no-op, persistence, undo/redo, busy-state and invalid-geometry rollback; production Properties handler tests |
| [180](https://github.com/rbrundage5/Systems-Modeler-Pro/pull/180) | Requirement Apply stops creating an extra frontend checkpoint before its existing native transaction | Both success and rejection tests failed before the label correction and passed afterward; rejection retains the draft |
| [181](https://github.com/rbrundage5/Systems-Modeler-Pro/pull/181) | Conditional, ExpansionRegion, Loop and Sequence structured nodes reject unsupported execution before session mutation | All four kinds, initialization/embedded initialization/reset, existing values, transitive CallBehavior, recursive graphs and unrelated Activities |

Each feature PR records exact allowed files, acceptance and negative cases. No
schema migration, agent configuration, CI-control or source-reference change is
included. The Activity fix exposes missing semantics; it does not implement them.

## How typing and BDD decomposition work

The reusable classifier is a Block, for example `Transmitter`. A usage owned by
another Block is a PartProperty, for example `primary : Transmitter`, owned by
`Station`. `backup : Transmitter` is another property with a different identity
that reuses the same type. Naming a Block `primary : Transmitter` does not create
that semantic typing relationship. Use the property's Type selector.

For the requested breakdown in a BDD:

1. Create the reusable `Station` and `Transmitter` Blocks.
2. Create or select `Station`'s PartProperty `primary`, select `Transmitter` as its
   Type and set its multiplicity. Apply the Properties edit.
3. With the target BDD active, select that PartProperty in the repository and choose
   **Show composition on BDD** (PR179).
4. The BDD shows the owning and type Blocks and their composition association,
   linked to the existing part identity. Repeat for `backup` to show the separate
   role using the same type. Repeating the same presentation command is a no-op.
5. Use an IBD of `Station` to show usage rectangles such as
   `primary : Transmitter`, their ports and internal connectors.

The BDD composition diamond belongs at the whole (`Station`) end. The part role
and multiplicity describe the linked usage at the type end. A displayed part
usage and its reusable Block type must remain distinct model elements.

Editing a type updates its shared definition. Editing a usage's name, multiplicity
or Type updates that usage. A plain Block definition does not itself have a
property-style Type field; Generalization is the separate specialization relation.
Composition, Generalization, ReferenceProperty and an IBD connector have different
semantics and should not be substituted for one another to obtain a visual shape.

Existing projects need not be rebuilt. These fixes retain stable identities and
the existing project format. Reopen them in a build containing the fixes. Existing
correctly typed/linked elements gain the corrected behavior. Old unlinked
composition relationships need the existing explicit property-link workflow;
labels alone cannot safely infer a missing owner/type/usage. The new command
cannot repair arbitrary previously malformed data or unsupported self-typed BDD
routing. The latter is rejected explicitly.

## All-nine-family coverage and remaining boundaries

These are capability/evidence categories, not a blanket VERIFIED stamp. Existing
core and persistence tests are run by native CI; all-nine rendered acceptance has
not been executed in this session. The earlier detailed finding register remains
applicable except where the specific corrections above supersede it.

| Family | Existing implementation/test evidence | Still missing or unqualified |
| --- | --- | --- |
| BDD | Blocks, typed features, association ends/property linkage, inheritance membership; `association_property_identity`, `association_end_integrity`, `block_specialization`, `typed_feature_atomicity`, BDD round trips | Redefinition/subsetting; full AssociationBlock model; self-association/routing breadth; complete instance/slot/notation acceptance |
| IBD | Part/ref occurrences, inherited features, ports, connectors, ItemFlows; `pr11_ibd`, `inherited_ibd_features`, `pr43_ports`, `pr45_item_flow`, persistence/runtime tests | Association-typed connectors/end multiplicities; port-to-nested-port paths and full interface contracts; shared authored/runtime ItemFlow contract qualification |
| Requirement | Text/ID/traceability/TestCase/allocation machinery; `pr21_requirements`, `copy_text_integrity`, requirements persistence | Owned requirement hierarchy and recursive Copy; trace matrices, coverage/suspect/evidence workflow; broader cross-repository endpoints |
| Use Case | Actors, use cases, include/extend and subject-boundary model; `pr24_use_cases`, use-case persistence | Complete scenario/extension-point editing and rendered relationship/subject acceptance not qualified here |
| Package | Ownership/namespace validation, imports and traversal; `pr26b_package_diagrams`, `package_import_traversal`, persistence | PackageMerge combination semantics; full visibility/alias/ambiguity/reparent UI acceptance |
| Activity | Actions/pins, flows, decisions, concurrency, call transfer, signals/time; `pr13_activity`, `pr31_activity_execution`, instance-context and persistence suites | Four structured kinds explicitly rejected by PR181; their authored data/execution still missing; complete interruption/cancellation and editor matrix unqualified |
| State Machine | Hierarchy/orthogonal/submachine/history/transition machinery; `pr32_state_machine_*`, Activity bridge and persistence suites | Complete priority/event/cancellation/connection-point conformance and rendered nested authoring unqualified |
| Sequence | Lifelines/messages/fragments and bounded operation/signal/reply execution; `pr12_behavior`, `pr34_operation_signal_sequence`, persistence | Other message-sort execution is explicitly limited; lifetime/gates/fragment breadth and occurrence-order acceptance remain |
| Parametric | Typed bindings, inherited Block roles, scalar evaluation and units metadata; `pr25_*`, `pr35_parametric_runtime`, persistence | Inherited ConstraintBlock definitions, arbitrary nested binding paths, Block-instance equality and unrestricted simultaneous solving |

Cross-cutting work also remains: native dirty/saved revisions and Save/Discard/
Cancel protection; execution-session retirement; every remaining generic command's
rejection/no-op/history ownership; all-nine ordinary-workspace collaboration;
genuine producer interchange; measured release-build scale; independent review;
and production release/update qualification. See the [24 September audit](TOOL_COMPLETION_AUDIT_2026_09_24.md)
and [implementation disposition](TOOL_COMPLETION_IMPLEMENTATION_2026_09_24.md).

## Verification and integration

Baseline: **168 frontend tests passed**, plus **21 distinct Python contract
validators**. The combined candidate: **182 frontend tests passed**, zero failures
or skips, plus all **21 validators** and `git diff --check` passed. These run
production handlers with mocked IPC and source/architecture guards; they do not
replace native behavior tests or prove rendered interaction.

The review/testing assembly is
[codex/sysml-candidate-2026-09-25](https://github.com/rbrundage5/Systems-Modeler-Pro/tree/codex/sysml-candidate-2026-09-25),
commit `cd2d3caab25d183d105b2ebb2cfc6c18f9852771`, tree
`8d271510fc646d2797bd38e4cc4a0fdcf3826533`. It contains exactly the six candidate
heads and conflict resolution retaining all native history labels. It has no
additional feature changes. It is not a merge to main. Leaf PRs remain the review
units; the final integrated tree still needs native CI and connected desktop
acceptance after sequential integration. No aggregate native result is claimed.

Native Rust build/tests/lint and packaging are checked using existing GitHub
workflows because this hosted workspace has no Rust toolchain or installed native
desktop session. CI found and prompted correction of formatting, a missing command
import and the association-end IPC lint annotation. These were fixed in the
candidate branches; an initial failing run is not reported as a pass.

CI status and exact heads are recorded below at publication and must be refreshed
before merge. Native success is not independent review or production release.

| PR | Candidate head | Native CI | Package checks |
| --- | --- | --- | --- |
| #176 | `9f1625f76b746da250a91185bc4b042b151405be` | [36124154586](https://github.com/rbrundage5/Systems-Modeler-Pro/actions/runs/36124154586): success | windows-installer: success; windows-release: success |
| #177 | `2a6d8b65e3a7cf5c1b62bd1d94c8d180e7a41f11` | [36124297982](https://github.com/rbrundage5/Systems-Modeler-Pro/actions/runs/36124297982): success | windows-installer: success; windows-release: success |
| #178 | `dd9860cabefa6c7c42f817ec64ac6f405c0f933b` | [36125892348](https://github.com/rbrundage5/Systems-Modeler-Pro/actions/runs/36125892348): in_progress | windows-installer: pending; windows-release: in_progress |
| #179 | `de070fddce31182b7225e219aebb4929b31f43ff` | [36125885020](https://github.com/rbrundage5/Systems-Modeler-Pro/actions/runs/36125885020): in_progress | windows-installer: pending; windows-release: in_progress |
| #180 | `ef958e6185cf98ff00327c65fee493b0b3553c3a` | [36125261806](https://github.com/rbrundage5/Systems-Modeler-Pro/actions/runs/36125261806): success | windows-installer: success; windows-release: success |
| #181 | `7fb4e943be0ecf76bd6c3c42c4b2865697649f9d` | [36125886632](https://github.com/rbrundage5/Systems-Modeler-Pro/actions/runs/36125886632): in_progress | windows-installer: pending; windows-release: in_progress |

## Dependency-ordered continuation

1. Review and integrate the bounded candidates into main, retaining all five added
   native-history labels; run native CI and the connected edit/undo/save/reopen
   journey on that exact integrated binary.
2. Complete native dirty/saved revision and execution-session lifecycle protection,
   then audit the remaining command transactions one workflow at a time.
3. Implement structural/requirements semantic gaps through the existing bounded
   design leaves, starting with core identity/validation/persistence before UI,
   runtime and adapters. The [connector typing design](C04_CONNECTOR_TYPING_DESIGN.md)
   already specifies its dependency sequence.
4. Specify and implement each missing behavior construct with deterministic traces,
   failure cases, persistence and editor support; keep unsupported execution clear.
5. Qualify every diagram family, shared editing, authentic interchange and measured
   scale, then independently review and qualify the installed release/update path.

Repository instructions require independent review and prohibit automatic merge;
worker isolation is unqualified, so no delegated reviewer was launched. Those
limits do not block publishing these concrete fixes. They do prevent calling this
pass a complete or independently accepted SysML application.
