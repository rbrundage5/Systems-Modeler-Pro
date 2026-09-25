# Relationship semantics: evidence and continuation

Date: 2026-09-23. Baseline: `7296098fa550ac2b79d4aacb994ab116bee6f2d7`.
Work order: C22.relationship-coverage-register. This file is the sole allowed
path for the documentation leaf. It inventories all native relationship kinds,
records bounded fixes, and specifies the remaining checks. It does **not** close
the C03-C15 parent audits or certify all relationships as correct.

The user requires relationships to change and constrain the semantic model, with
consistent editing, presentation, persistence and applicable execution. Rust is
the authority. A rendered line, enum variant, historical PR or passing integration
text check is insufficient evidence of this behavior.

## Source and execution boundary

The approved uploaded SysML 1.6 (`formal-19-11-01.pdf`) and UML 2.5.1
(`formal-17-12-05.pdf`) were inspected read-only. Relevant clauses are identified
below; reference documents or extracted passages are not republished. No vendor
parity or certification claim is made. Repository sources and current-head GitHub
Actions supply implementation evidence.

Primary-session implementation was used. Registered workers remain disabled by
the repository environment gate. Self-inspection is not independent review.
Independent review and installed/rendered acceptance remain outstanding. No local
Rust toolchain is installed; native compilation/tests/lint run in the existing
GitHub workflow. No new downloads, execution controls or workflow gates were added.

## Published native repair candidates

Each PR targets the recorded main baseline independently. No automatic merge or
force push is authorized by this work order. Recheck actual heads, checks and
mergeability before integration; a clean merge today is not a future guarantee.

| Leaf | PR / candidate | Behavior and acceptance evidence |
| --- | --- | --- |
| C03.generalization.ibd-membership | [152](https://github.com/rbrundage5/Systems-Modeler-Pro/pull/152), `d3d53d5b85ffc5eb1e4b73f7b9064f6cfd59a143` | Inherited non-private parts/ports become valid IBD members and connector paths. Original IDs/owners remain unchanged; diamond inheritance deduplicates. Three core tests and a native Populate IBD/SQLite reopen/idempotence/rollback test. Native CI run 35866877279 passed. |
| C03.association.end-integrity | [153](https://github.com/rbrundage5/Systems-Modeler-Pro/pull/153), `880738d702e0423d0f0adf859d349bbcfb1df7c5` | Creation and project validation share checks for classifier ends, count, identity, multiplicity, endpoint summaries and aggregation. All-Block associations must be binary. Five core tests cover invalid authoring/reopen without mutation and valid parallel/reflexive/non-Block n-ary cases. Current native gates must be checked. |
| C03.relationship.reconnect-rollback | [154](https://github.com/rbrundage5/Systems-Modeler-Pro/pull/154), `b132430f228c65c25f69064e2cb3907ab9466b9b` | BDD reconnect stages fallible diagram work before semantic publication, holds both guards through commit, and rolls back failed full-project validation. Four native command tests cover both endpoint edits, missing presentations, invalid route geometry, poisoned lock and a generalization cycle. Native CI run 35868161948 passed. |
| C05.copy.imported-text-integrity | [155](https://github.com/rbrundage5/Systems-Modeler-Pro/pull/155), `42207196d75572cf28bf6c005772d7ce34bc71e5` | Whole-project validation rejects unequal Copy client/supplier text with the failing relationship ID. Three core tests cover malformed roundtrips, late chain and second-supplier errors, valid propagation and local IDs. Current native gates must be checked. |

Allowed production paths were limited to `crates/model-core/src/model.rs`,
`crates/model-core/src/ibd.rs`, and the desktop `workspace/ibd.rs` or
`workspace/relationship_editing.rs` needed by the respective leaf. Their PRs name
the exact tests and work orders. Shared-module writing was sequential.

Local checks passed where applicable: `git diff --check`,
`scripts/validate_structural_runtime_integration.py`,
`scripts/validate_presentation_interaction.py`, and
`scripts/validate_requirements_integration.py`. These checks do not replace
native tests or visual acceptance.

## Complete inventory and closure obligations

The first twenty rows cover every `RelationshipKind` variant in `model.rs`.
Aggregation is end metadata on Association, not an additional enum variant.
Later rows cover relationships stored in other repositories or feature fields.
"Pending" means the stated dimension has not been qualified, even if existing
tests cover other dimensions. Existing fixture names below identify starting
evidence, not complete conformance.

| Relationship | Required semantic effect / direction | Current evidence and remaining qualification |
| --- | --- | --- |
| Dependency | Client depends on supplier; preserve endpoints and reference integrity. | `model.rs` checks endpoints/duplicates. Pending: permitted endpoint scope, edit/delete/import parity, direction and cross-diagram notation. It does not imply feature inheritance. |
| PackageImport | Importing namespace resolves visible members from imported package, with visibility and ambiguity rules. | `namespace.rs`; existing namespace resolution tests. PR149 separately addresses indexed traversal. Pending: cyclic/public/private/re-export resolution and scale across all adapters. |
| ElementImport | Import one packageable element with alias/visibility while preserving semantic owner. | `model.rs`, `namespace.rs`, `pr47_core_namespace_relationships.rs`. Pending: alias collision/visibility mutation and adapter parity. |
| PackageMerge | Receiving package combines compatible definitions from the merged package. | Endpoints/ownership are represented; `namespace.rs` resolution handles imports but not merge combination. Full merge semantics are a confirmed implementation gap, not established by drawing the arrow. |
| Association | Stable Property ends describe links, types, roles, multiplicities and navigation. | PR153 validates explicit end payloads. Property/end identity and ownership remain absent; self/parallel BDD authoring is narrower than native association support. |
| Composition | Composite Block-typed property belongs to the whole and is typed by the part definition. | **Confirmed gap:** BDD creation writes composite AssociationEnd metadata but creates no PartProperty. Legacy Composition remains readable. The repair sequence below is required. |
| Generalization | Specific classifier inherits applicable features by identity from general classifier. | PR145 indexed traversal and PR152 IBD membership. Pending: all selectors/compartments, Block-specialization constraints, explicit redefinition/subsetting, runtime and type-conformance parity. |
| Realization | Implementation/client fulfills specification/supplier. | Stored by native model with notation mapping. Pending: applicable endpoint constraints, editing/deletion and trace queries. It must not silently behave as Generalization. |
| Allocate | Directional assignment between modeled elements across abstractions. | `pr42_allocation.rs` and persistence fixture. Core Element endpoints exclude some NamedElements and cannot directly address separate Activity/Behavior IDs; cross-repository allocation needs an explicit contract. |
| Connector | Connect roles/ports within a structured context with assembly/delegation topology. | `ibd.rs`, `pr11_ibd.rs`, connector persistence tests; inherited membership addressed by PR152. Pending: inherited context across all commands, interface contracts/conjugation, connector typing and delete/reference closure. |
| ItemFlow | Convey classifiers in a specified direction over a realizing connection. | `validate_item_flow` checks conveyed classifiers and connector endpoints. Pending: detailed direction/type compatibility, reconnect/delete propagation, inherited/qualified paths and runtime delivery. |
| DeriveRequirement | Derived client requirement points to its source requirement. | `pr21_requirements.rs` endpoint/owner fixtures. Pending: cross-view edits, import/export direction, dependency impact and suspect propagation. |
| Satisfy | Satisfying client points to requirement supplier. | Core endpoint/owner validation. Pending: profile endpoint scope and coverage/impact views; existence of a link must not be reported as verified satisfaction. |
| Verify | Verifying client points to requirement supplier. | Core currently accepts only ElementKind::TestCase as client. Pending: supported verification-element contract and result linkage. A Verify link is not a passing test result. |
| Refine | Client provides a more detailed description of supplier. | Core currently requires exactly one Requirement endpoint. This is narrower than general refinement; pending a separately scoped endpoint/property-path correction. |
| Trace | Directed trace between named elements with explicit source/target interpretation. | Core binary IDs and requirement fixtures. Pending: property paths, cross-repository endpoints, queries and change impact. |
| Copy | Client text follows supplier text; client's local ID and semantic identity remain separate. | `pr61_copy_reimport.rs`, `pr21_requirements.rs`, PR155 whole-project text invariant. Pending: recursive subrequirement structure, imported graph policy and supplier deletion/edit impact. |
| Include | Including Use Case points to included Use Case for required reuse. | `pr24_use_cases.rs` checks kinds and self-reference. Pending: edit/import parity, nested include graphs and supported execution boundaries. |
| Extend | Extending Use Case points to extended Use Case with condition/location. | Native extension location resolves on target Use Case. Pending: extension-point rename/delete, multiple points, import/edit rollback and presentation. |
| BindingConnector | Equality constraint between compatible property values; no signal-flow direction. | `parametrics.rs`, `pr25_parametrics.rs` type/quantity/cycle fixtures. Direct-owner checks currently exclude inherited roles; inherited binding scope is a confirmed gap. Unit conversion and evaluator limitations remain separately qualified. |
| Association shared aggregation | Reference-style association with hollow diamond; retain defined model-specific meaning. | PR153 validates one aggregated binary end. SysML/UML give shared aggregation no universal additional lifecycle behavior; do not invent composite deletion semantics. Property linkage remains pending. |
| Activity ControlFlow | Transfer control tokens under guards and activity topology. | `activity.rs`, `pr13_activity.rs`, execution and persistence fixtures. Pending: create/edit/reconnect rollback, fork/join/decision/merge semantics, region boundaries and concurrent execution. |
| Activity ObjectFlow | Transfer typed objects through compatible nodes/pins with multiplicity/state constraints. | Native object-flow fixtures exist. Pending: polymorphic types, pin direction/multiplicity, buffering/weight, ordering and import/runtime parity. |
| State External transition | Exit/enter appropriate configuration on trigger/guard/effect. | `behavior.rs`, `pr12_behavior.rs`, state execution fixtures. Pending: cross-region endpoints, priorities, hierarchy, source/target edits and rollback. |
| State Internal transition | Execute internal transition semantics without external exit/re-entry. | Stored TransitionKind::Internal. Pending: allowed source/target form, entry/exit traces and authoring/runtime consistency. |
| State Local transition | Apply local composite-state transition semantics. | Stored TransitionKind::Local. Pending: descendant targets, active-state changes, entry/exit traces and invalid topology. |
| Sequence SynchCall / AsynchCall | Ordered calls with valid operation/arguments and appropriate reply/wait behavior. | `behavior.rs` validates message/occurrence/signature data. Pending: lifeline addressing, inherited operations, execution pairing and deterministic runtime traces. |
| Sequence AsynchSignal | Ordered signal send/receive with compatible signature and reception. | Existing operation/signal integration tests. Pending: receiver contract, inherited ports/receptions, arguments, queue and runtime addressing. |
| Sequence Reply | Reply correlates to the applicable call/execution. | MessageSort represented. Pending: native correlation, output parameters, invalid/out-of-order replies and roundtrip. |
| Sequence Create / Delete | Establish/end a participant lifetime with correct occurrence ordering. | MessageSort represented. Pending: lifetime constraints, subsequent messages, authoring edits and runtime behavior. |
| Sequence Lost / Found | Explicitly absent receiver/sender with consistent occurrences. | Optional occurrence ends represented. Pending: sort-specific absence rules, gates and runtime/export boundaries. |
| Typing / ownership / containment | Owner and type are distinct; typed properties refer to reusable definitions. | Core fields and ownership validation. Pending: every relationship-sensitive reparent, retype, duplicate and delete command, including behaviors and imported IDs. |
| Redefinition / subsetting | Explicit feature links constrain inherited members and property sets. | No dedicated native fields/effective-member implementation found. Confirmed feature gap; equal names are not sufficient evidence of redefinition. |

Reference anchors inspected: UML 2.5.1 §§7.4.3.3 (imports), 7.7.3 (dependencies),
9.2.3.2-3 and 9.9.4 (inheritance/redefinition), 11.5.3.1 (association),
12.2.3.2 onward (package merge), 18.1.3.2-3 (extend/include); SysML 1.6
§§8.3.2.3-4 (binding and Block), 15.3.2.1 (allocation), 16.3.2.2-9
(requirement relationships). Behavior closure additionally needs the exact
applicable Activity/State/Interaction clauses and execution traces recorded per
child work order; this inventory does not assert those audits are complete.

## Confirmed remaining defects and bounded next leaves

1. **REL-COMP-01, composition/property identity (C03/C04).**
   `apps/desktop/src-tauri/src/workspace.rs::create_bdd_relationship` calls
   `create_association` with composite end metadata. `AssociationEnd` has no link
   to a semantic Property. Creating Whole--Part therefore does not create an IBD
   part usage. Current end aggregation follows the diagram diamond side; UML
   property aggregation belongs to the part end, with the diamond displayed at
   the whole. A naive extra PartProperty would leave two disconnected records.
   Use the dependency-ordered repair below.
2. **REL-GEN-01, incomplete feature consumers (C03/C04/C10/C15).**
   PR152 covers IBD population/paths/ports only. Sequence
   `collect_lifeline_candidates` uses direct children and a depth-six cutoff.
   `structural_runtime.rs::effective_features` has its own recursive traversal
   and explicit unsupported-redefinition/name-conflict behavior. Audit each
   consumer separately; do not strip private ancestor instance storage merely
   because private members are not publicly inherited.
3. **REL-GEN-02, Block specialization constraint (C03).**
   `create_relationship`/`validate` accept general classifier pairs except special
   Actor/UseCase restrictions. SysML 1.6 §8.3.2.4 constraint 8 requires a
   specialization of a Block to also be a Block specialization. Add a native
   create/reopen/reconnect invariant with positive built-in Block subtypes and
   rejected non-Block classifiers; keep redefinition as a separate feature.
4. **REL-BIND-01, inherited binding context (C11).**
   `validate_binding_connector` requires direct owner equality. Specify inherited
   accessible value/constraint roles and definition parameter paths, then reuse
   authoritative feature membership with rejection/rollback and evaluation tests.
5. **REL-EDIT-01, cross-diagram reconnect (C16/C17).**
   PR154 fixes rejected-edit atomicity for the selected BDD only. Other diagram
   edges referencing the same relationship are not updated. Specify all-view
   endpoint presentation policy, stage affected routes, preserve identity and
   undo/redo the entire semantic edit atomically. Recompute attached labels too.
6. **REL-EDIT-02, BDD delete lock order (C01/C16).**
   `delete_bdd_relationship` acquires diagrams before project, opposite reconnect
   and other project-first commands. Separate native lock-order fix with bounded
   concurrency/failure tests; do not conflate it with semantic cascade deletion.
7. **REL-ASSOC-02, self/parallel authoring (C03/C17).**
   Core associations support distinct role usages between identical classifier
   pairs and reflexive associations. BDD create/reconnect rejects self-links and
   equivalent kind/pairs. Qualify stable member-end identity and routing, then
   remove only unjustified restrictions; do not relax other relationship rules.
8. **REL-API-01, incomplete generic construction (C02/C03/C04).**
   Generic `create_relationship` can construct payload-requiring kinds before
   Connector/ItemFlow/Binding data exists. Specialized constructors fill payloads
   afterward. Specify a staged internal primitive and safe public commands so no
   supported public API publishes an incomplete relation. Preserve adapter parity.

## Composition repair sequence and acceptance

This is a feature specification and dependency split, not implemented behavior.
Each increment needs its own exact-path work order and negative case. Continue
from real main/PR state, not an assumed merge of earlier candidates.

1. **Canonical identity and compatibility.** Define explicit association member
   Property identity, classifier owner versus type, and anonymous inverse ends.
   Choose one authoritative source for name/multiplicity/navigation/aggregation.
   Specify legacy empty-end and notation-oriented-end migration before changing
   serialization. Do not infer identity from matching names or Block pairs.
2. **Native atomic authoring.** Drawing composition Whole-to-Part creates/reuses a
   Whole-owned PartProperty typed by Part and links the correct association end.
   Associate reference properties likewise. Failed validation publishes neither
   half. Two differently named usages of the same Part definition remain distinct.
3. **Native mutation and deletion.** End/property editors, retype/reconnect,
   aggregation changes, ownership changes and semantic deletion use the same
   authority. Removing a diagram shape preserves semantics; deleting a usage must
   not delete its reusable Block type. Detect external references before commit.
4. **Persistence and interchange.** Save/reopen and legacy migration preserve IDs
   and semantics; corrupt/mixed linkage rejects before session publication.
   Copy/paste, portable import, spreadsheet/XMI/ReqIF where applicable must remap
   both sides consistently. `standard_editing.rs` already regenerates member-end
   IDs; new links must participate in that mapping.
5. **Presentation and execution.** BDD properties and diamonds, IBD occurrences,
   inspector, runtime instances/multiplicity and history show the same Property.
   Generalized whole Blocks can use the inherited property without copying it.
   Add rendered dense/self/parallel cases and instance ownership/lifetime tests.

Minimum connected acceptance model: Vehicle owns `front:Wheel` and `rear:Wheel`;
ElectricVehicle specializes Vehicle and inherits both by identity; ports connect
through these usages with an ItemFlow; a bound value participates in a constraint;
requirements derive/copy/trace/satisfy/verify against the intended elements.
Rename/retype/reconnect/delete/undo/redo, save/reopen and supported import/export
must preserve the corresponding relationships across every affected view. Verify
each assertion separately; one successful demo cannot close all families.

## Common verification contract

For every row, record positive and invalid endpoint/type/owner/direction cases,
stable relationship and end identity, multiplicity/visibility semantics, and
negative edits with unchanged model, presentations and history. Verify reference
closure for rename/retype/reparent/delete and cross-diagram reuse. Test malformed
load/import before publication and supported roundtrips. Exercise actual native
commands, not just serialization or frontend source-string checks.

Only relationships with execution semantics require execution traces; Dependency,
Trace or Verify must not invent runtime effects. Verify keyboard creation/editing,
attached labels, routing, self/parallel edges, cancellation and undo/redo with
rendered evidence when the desktop facility is available.

Scale qualification remains open. Existing 10k/100k relationship validation and
20k inheritance-depth fixtures cover particular algorithms, not all interaction
performance. Measure mixed dense models and repeated shared-type occurrences,
batch import, relationship queries, affected-view updates and save/open with
recorded hardware, counts, latency and memory. Avoid per-edge full repository
scans and persistent caches without revision invalidation. Do not claim that
large-model editing is qualified from a core validation benchmark alone.

Before reporting closure: update exact commit/check evidence, resolve CI/review
findings, verify compatibility against current main and concurrent candidates,
and retain unverified dimensions explicitly. Composition linkage, all-relationship
correctness and professional-tool parity remain open until their acceptance
evidence exists.
