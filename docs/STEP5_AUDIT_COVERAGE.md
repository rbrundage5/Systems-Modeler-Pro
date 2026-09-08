# Step 5 — Audit coverage checklist

Planning coverage, not audit results. All rows begin UNVERIFIED. No audit has been
executed by creating this checklist. See [scope and acceptance](STEP5_SCOPE_AND_ACCEPTANCE.md)
for evidence fields, priorities, scenarios and completion gates.

Split every semicolon-separated topic into individually identified child checks
(e.g. C04.01), and apply every applicable evidence dimension. Track omissions as
new child checks. A parent cannot be VERIFIED merely because one representative
test passes. NOT APPLICABLE requires an explicit standards/product-scope rationale.

| ID | Area | Required subtopics | Status |
| --- | --- | --- | --- |
| C01 | Code/architecture | Rust/frontend authority; shared commands; duplicate controllers; error paths; locking/deadlocks; transaction boundaries; dependencies; CI/test quality | UNVERIFIED |
| C02 | Identity/storage | Stable IDs; ownership/reparenting; reference integrity; atomic rollback; schema migration; corruption diagnostics; save/open compatibility; backup/recovery; history | UNVERIFIED |
| C03 | BDD | Blocks/InterfaceBlocks/AssociationBlocks; primitive/data/value/enumeration types; units/quantities; part/ref/value/constraint properties; ports; operations/parameters/receptions; instances/slots; association ends; inheritance/redefinition/subsetting; stereotypes and compartments | UNVERIFIED |
| C04 | IBD | Block context and repository owner; part/ref occurrences; proxy/full/nested ports; conjugation; provided/required interface contracts; nested endpoint paths; assembly/delegation; connector typing; ItemFlows and conveyed direction/types | UNVERIFIED |
| C05 | Requirements | ID/text/metadata; hierarchy and ownership; derive/satisfy/verify/refine/trace/copy; TestCases; allocations; coverage/suspect/impact workflows; diagram reuse | UNVERIFIED |
| C06 | Use Case | Actors and generalization; use cases and subject boundaries; associations; include/extend and extension points; scenarios and traceability | UNVERIFIED |
| C07 | Package | Model/Package/ModelLibrary; containment/reparenting; qualified names; visibility/import/merge relationships where applicable; frames and navigation | UNVERIFIED |
| C08 | Activity authoring | Actions; pins; parameters/nodes; control/object flows; object states; initial/finals; decision/merge/fork/join; guards/weights; partitions; structured/conditional/loop/sequence/expansion/interruptible regions; operation/behavior/signal/time references | UNVERIFIED |
| C09 | State authoring | Regions; composite/orthogonal states; submachines; initial/final/choice/junction/fork/join/history/entry/exit/terminate; trigger/event/guard/effect; entry/do/exit; internal/local/external/completion transitions | UNVERIFIED |
| C10 | Sequence authoring | Lifelines/property paths; occurrence order; call/signal/reply/create/delete/found/lost; executions; combined fragments/operands/guards; gates; invariants; message editing independent of geometry | UNVERIFIED |
| C11 | Parametric authoring | Constraint blocks/properties/parameters; expressions; bindings; typed endpoints; value defaults; unit/quantity metadata; scope; diagnostics and property-editor access | UNVERIFIED |
| C12 | Activity execution | Token/object stores; pin multiplicity; decisions/concurrency/join; call transfer; signals/events/time; structured and expansion semantics; termination; bounded expressions; stale-model invalidation | UNVERIFIED |
| C13 | State execution | Run-to-completion; transition priority; orthogonal dispatch; history; connection points; entry/do/exit cancellation; time/change/call/signal events; submachines; ambiguity diagnostics | UNVERIFIED |
| C14 | Sequence/operation execution | Occurrence-specific operation arguments/results; signal/reception addressing; fragment control; create/delete/found/lost semantics; participant selection; shared event/time integration | UNVERIFIED |
| C15 | Structural/parametric runtime | Repeated occurrences; part/ref/multiplicity configuration; port/connector transport; inherited features/slots; deterministic evaluation; units/dimensions; cycles/unsupported expressions; runtime versus authored values | UNVERIFIED |
| C16 | Shared editing | Palette selection; click placement; repository drag/drop; relationships; select/multi-select; move/resize; delete semantic versus remove presentation; copy/paste/duplicate; ESC; marquee; Space-drag pan; zoom/fit; undo/redo; repository rename/reparent/delete; diagram switching/drill-down | UNVERIFIED |
| C17 | Notation/visuals | All nine families: symbols; line styles/arrows/diamonds; roles/multiplicity/stereotypes; compartments; frame/context/header; port anchors; label attachment; self/parallel routes; containment; crossings/overlaps; dense/nested Clean/Route; readable exported views where supported | UNVERIFIED |
| C18 | UI/UX | Workspace/ribbon/palette; repository search/navigation; properties and behavior configuration; dialogs/focus/keyboard; feedback/errors; runtime inspection; DPI/scaling/contrast; consistent terminology; empty/loading states | UNVERIFIED |
| C19 | Interchange/automation | Native project; portable JSON; mapped CSV/XLSX versus authored XLSX; bounded Groovy; ReqIF; profiles/XMI semantics and DI; stable source identity; preview/reimport/removal; rollback; malformed/untrusted inputs; round trips; vendor boundaries; future CLI/SDK/API scope | UNVERIFIED |
| C20 | Collaboration/governance | Reuse assessment; identity/authentication/authorization; operations/revisions/order/deduplication; atomic batches/imports; conflict and delete/edit policy; user-scoped undo; presence; disconnect/replay/snapshot; host restart; history/audit; locks/branch/merge; LAN/on-prem deployment and protocol compatibility | UNVERIFIED |
| C21 | Scale/reliability/security | Small/dense/large fixtures; startup/query/snapshot/render/routing/import/save timings; memory; working sets; cancellation; long operations; dependency/input safety; access revocation and recovery | UNVERIFIED |
| C22 | Documentation/release | PR69 unresolved import/PR33 contracts; reference provenance/version access; capability evidence; reproducible build and manual checks; feature docs; migration/release limitations; stale source cleanup | UNVERIFIED |

## Fixture plan

Prepare minimal positive/negative semantic cases; one connected all-nine-family
model; dense/nested routing scenes; old native-file compatibility cases; unchanged,
changed and invalid imports; deterministic runtime scenarios; and two-client
concurrency/reconnect cases. Existing fixtures are reused only after verifying
suitability. Keep generated demo models importable rather than embedding them in
application startup. Define sizes/hardware before recording performance budgets.

## Findings and fix handoff template

- ID / parent coverage / affected dimensions:
- Baseline commit and reference edition/clause:
- Expected behavior and actual behavior:
- Reproduction / code / test / rendered evidence:
- Status and severity; implemented / UI-exposed / tested separately:
- Acceptance scenario and negative/rollback case:
- Dependencies, affected shared modules and assigned owner:
- Fix branch / PR / reviewer:
- Verification command or manual procedure, result and exact commit:
- Remaining limitations and next action:

Initial finding register is empty. The historical documentation flags in PR69
are follow-up inputs (C15/C19/C22), not newly confirmed functional defects.
