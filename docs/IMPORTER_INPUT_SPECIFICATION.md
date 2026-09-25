# Systems Modeler Pro — Importer Input Specification

**Status:** Authoritative input-authoring contract  
**Audience:** Humans, automation agents, workbook generators, model-script generators, interchange adapters, and reviewers  
**Baseline:** `main` after PR67, 2026-09-08  
**Purpose:** Define exactly how import inputs must be structured so Systems Modeler Pro can preview and apply them without semantic, reference, transaction, or presentation errors.

> This document is the source of truth for **creating import inputs**. For implementation status, qualification history, compatibility boundaries, and test matrices, also see [`IMPORT_RULES_AND_QUALIFICATION.txt`](IMPORT_RULES_AND_QUALIFICATION.txt).

---

## 1. Normative language

The words **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** are normative.

A generator that claims an input is **import-ready** MUST satisfy this specification before handing the file to the importer.

The importer remains authoritative. Passing a generator-side preflight does not replace the native Rust preview and validation step.

---

## 2. Choose the correct import mechanism first

Do not force one source format to do a job owned by another import path.

| Goal | Correct mechanism | Important boundary |
| --- | --- | --- |
| Generate a complete new SysML model including semantic content and any/all of the 9 supported diagram families | **Groovy-compatible Model Script** (`.groovy`, `.gvy`, `.smscript`, or accepted JSON text) | Preferred mechanism for generated complete models and demos |
| Import bulk semantic data from business/engineering tables | **Mapped XLSX/CSV semantic import** | Creates/updates semantics through the qualified scope; mapped workbooks are not the all-nine diagram-authoring path |
| Round-trip an existing Systems Modeler authored workspace through XLSX | **Systems Modeler XLSX interchange** | Requires the exporter-owned `SystemsModelerState` embedded portable state; do not hand-build this format unless implementing the exact portable-state schema |
| Exchange Requirements/TestCases and supported traceability with requirements tools | **ReqIF / ReqIFZ** | Use ReqIF identifiers and the configured source namespace |
| Exchange supported UML/SysML XMI content | **XMI 2.x path** | Use only within the documented XMI qualification boundary |
| Open/save the native working project | **`.smproj`** | Native persistence, not an interchange authoring format |

### 2.1 Recommended generator rule

If an agent is asked to create a **complete importable model with diagrams**, it SHOULD generate a Model Script unless the task explicitly requires another interchange standard.

If an agent is asked to create **tabular engineering content** such as packages, blocks, requirements, properties, ports, relationships, behaviors, or parametric records without all-nine diagram presentation, it SHOULD generate a mapped XLSX/CSV plus its MapGroup configuration.

---

## 3. Universal import invariants

These rules apply to every generated source wherever the format can represent the concept.

### 3.1 Stable identity

1. Every import domain MUST have a nonblank **source namespace**.
2. Every authored semantic record that supports external identity MUST have a nonblank **External ID**.
3. `source namespace + External ID` is the canonical durable source identity.
4. External IDs MUST be unique within the relevant source identity domain.
5. A display name MUST NOT be treated as permanent identity.
6. A spreadsheet row number, worksheet position, diagram coordinate, or presentation ID MUST NOT be used as semantic identity.
7. A Requirement ID is human-facing requirement identity and is distinct from External ID and the internal UUID.
8. **Internal project UUIDs MUST NOT be copied from a prior project and used as source references.**
9. File names MUST NOT define semantic identity.

### 3.2 Namespace versioning rule

When revising and reimporting the **same logical source model**, keep the same source namespace and the same External IDs so the importer can classify records as `UPDATE` or `NO_CHANGE`.

Change the source namespace only when the content intentionally represents a different independent identity domain.

Do **not** change the namespace merely because a file is called `v2`, `v3`, etc. if the intended behavior is an update of the same model.

### 3.3 Ownership and containment

1. Every owned semantic record MUST have a legal owner.
2. A generator MUST NOT silently place a record at the project root because the intended owner could not be resolved.
3. Owners MUST exist before they are referenced by plan-local generated content.
4. Ownership MUST match the native SysML/UML semantic rule.
5. Presentation on a diagram MUST NOT be used as a substitute for semantic ownership.

Common examples:

- `Parameter` is owned by an `Operation`.
- `ConstraintParameter` is used for a `ConstraintBlock`; do not substitute an ordinary `Parameter`.
- Part/Reference/Value/Flow/Constraint properties and Ports require compatible classifier/constraint ownership.
- `Reception` requires a legal classifier owner and an accepted `Signal`.
- Activity owners must be a namespace or classifier; Activity context, when supplied, must be a classifier.
- State Machine and Interaction contexts must be valid classifier/context elements.
- Connector context must be a structured classifier capable of owning the referenced structural paths.

### 3.4 Reference integrity

Every required owner, type, source, target, Signal, Operation, Activity, Region, Lifeline, Connector, parameter, represented path, conveyed item, and diagram semantic reference MUST resolve to exactly one legal target.

A generator MUST produce:

- **0 unresolved references**;
- **0 ambiguous references**;
- **0 wrong-kind references**;
- **0 forward plan-local references** unless a specific importer operation explicitly supports them; generators SHOULD always define before use;
- **0 raw stale UUID references**.

The importer does not use fuzzy matching to repair a bad source.

### 3.5 Ordering

Generated records SHOULD follow dependency order:

1. packages/namespaces;
2. primitive/data/value/interface/classifier types;
3. blocks/constraint blocks/association blocks;
4. operations, signals, requirements, actors/use cases, test cases;
5. owned properties, ports, parameters, receptions, constraint parameters;
6. ordinary relationships and requirement traceability;
7. connectors;
8. item flows;
9. Activities, then partitions/structured nodes, then nodes/pins, then edges;
10. State Machines, then regions, then vertices, then transitions;
11. Interactions, then lifelines, then occurrences, then messages/executions/fragments/invariants;
12. parametric metadata and bindings;
13. diagram declarations after the semantic targets they display exist.

### 3.6 Validation is not optional

Generated content MUST satisfy the same native Rust semantics as manually authored content. Do not remove, bypass, or weaken validation to make a file import.

### 3.7 Atomicity

Import preview MUST be treated as non-mutating. Apply MUST be treated as one logical transaction.

If any blocking semantic error exists, the correct outcome is **zero commit**, not a partially imported model.

---

## 4. Reference syntax for Model Script

Use these reference forms deliberately.

| Form | Meaning | Use |
| --- | --- | --- |
| `$root` | Current Project root Element | Legal where the field accepts an ordinary Element reference; useful as owner for top-level generated content |
| `handle:EXT_ID` | External ID in the script/import source domain | Preferred plan-local generated reference |
| `ext:EXT_ID` | External ID in the script/import source domain | Equivalent external-ID token form |
| `qname:Exact::Qualified::Name` | Exact qualified-name lookup in the existing Project | Existing ordinary Project elements only; must resolve uniquely |

### 4.1 Specialized semantic references

Specialized Activity/State Machine/Sequence references do **not** accept `$root` or `qname:` where the field expects an Activity/Behavior semantic identity. They require an External ID / plan-local handle.

Examples:

- `activity` on `activity_node` -> `handle:ACT_MAIN`
- `state_machine` on `region` -> `handle:SM_MAIN`
- `region` on `vertex` -> `handle:REGION_MAIN`
- `interaction` on `lifeline` -> `handle:INT_MAIN`
- `lifeline` on `occurrence` -> `handle:LL_DRIVER`

### 4.2 Raw UUID prohibition

Do not write a UUID such as:

```text
9d554bc0-....
```

as an owner/source/target expecting it to resolve as a prior Project element. Model-script non-prefixed tokens are treated as source External IDs, not trusted prior-project UUID identity.

---

## 5. Model Script file contract

The current Groovy-compatible host is intentionally bounded. It does **not** execute arbitrary Groovy or a JVM.

The safe canonical file is a strict JSON construction program wrapped in `modelScript(''' ... ''')`.

### 5.1 Minimal skeleton

```groovy
modelScript('''
{
  "source_namespace": "program:system:model",
  "operations": [
    {"op":"element","external_id":"PKG","kind":"Package","name":"System","owner":"$root"},
    {"op":"element","external_id":"SYS","kind":"Block","name":"System","owner":"handle:PKG"}
  ],
  "diagrams": [
    {"external_id":"D_BDD","family":"BDD","name":"System Definition","owner":"handle:PKG"}
  ]
}
''')
```

### 5.2 Top-level fields

| Field | Required | Rule |
| --- | --- | --- |
| `source_namespace` | YES | Nonblank and stable across revisions of the same logical source |
| `operations` | NO, defaults empty | Ordered semantic construction program |
| `diagrams` | NO, defaults empty | Diagram declarations; declare after their semantic dependencies exist |

### 5.3 Script JSON rules

- JSON inside the wrapper MUST be valid JSON.
- Do not put comments inside the JSON payload.
- Strings MUST be escaped correctly.
- Enum spellings are case-sensitive unless the specific tagged type uses the documented snake-case form.
- Each operation External ID MUST be stable and unique in its source domain.
- Define referenced generated records before use.

---

## 6. Model Script operation schemas

The following operation names are the current construction surface.

### 6.1 Ordinary semantic element

```json
{
  "op": "element",
  "external_id": "REQ_RANGE",
  "kind": "Requirement",
  "name": "Driving Range",
  "owner": "handle:PKG_REQ",
  "documentation": "Engineering rationale...",
  "requirement_id": "SYS-001",
  "requirement_text": "The vehicle shall provide at least 400 km of range."
}
```

Required: `op`, `external_id`, `kind`, `name`, `owner`.

Optional fields when semantically legal:

- `type_ref`
- `documentation`
- `visibility` (`public` or `private`)
- `requirement_id`
- `requirement_text`
- `multiplicity` as `{ "lower": 0, "upper": 1 }`; `upper: null` means `*`
- `default_value`
- `parameter_direction` (`In`, `Out`, `InOut`, `Return`)
- `flow_direction` (`In`, `Out`, `InOut`)
- `is_conjugated`
- `extension_points` (array of strings; Use Case only)

### 6.2 Ordinary relationship

```json
{
  "op": "relationship",
  "external_id": "SAT_RANGE",
  "kind": "Satisfy",
  "source": "handle:VEHICLE",
  "target": "handle:REQ_RANGE",
  "owner": "handle:PKG_REQ"
}
```

Required: `external_id`, `kind`, `source`, `target`.

Optional: `owner`, `name`, `documentation`, `visibility`, `source_end`, `target_end`, `alias`, `extension_condition`, `extension_location`.

Association end example:

```json
"source_end": {
  "role_name": "vehicle",
  "multiplicity": {"lower": 1, "upper": 1},
  "navigable": true,
  "aggregation": "None"
}
```

Aggregation values: `None`, `Shared`, `Composite`.

For SysML composition, prefer an `Association` with a composite association end rather than the legacy generic `Composition` relationship kind.

### 6.3 Connector

```json
{
  "op": "connector",
  "external_id": "CONN_POWER",
  "context": "handle:VEHICLE",
  "kind": "Assembly",
  "source_path": ["BATTERY_PART", "BATTERY_POWER_PORT"],
  "target_path": ["INVERTER_PART", "INVERTER_POWER_PORT"],
  "name": "powerLink"
}
```

Connector kind: `Assembly` or `Delegation`.

Rules:

- `context` MUST resolve to the structured classifier owning the internal structure.
- Every structural-path segment MUST already exist and be legal in that context.
- Paths MUST use semantic structure, not diagram nodes.
- Port/property types MUST be compatible with the connector topology.

### 6.4 Item Flow

```json
{
  "op": "item_flow",
  "external_id": "FLOW_POWER",
  "connector": "handle:CONN_POWER",
  "source_path": ["BATTERY_PART", "BATTERY_POWER_PORT"],
  "target_path": ["INVERTER_PART", "INVERTER_POWER_PORT"],
  "conveyed_items": ["handle:POWER_SIGNAL"],
  "name": "PowerFlow"
}
```

Rules:

- Referenced Connector MUST already exist.
- Source/target paths MUST be valid Connector-relative structural paths.
- Conveyed items MUST resolve to valid semantic classifiers/items.
- Item Flow topology MUST agree with the Connector rather than inventing a separate unrelated path.

### 6.5 Activity

```json
{"op":"activity","external_id":"ACT_DRIVE","name":"Drive Vehicle","owner":"handle:PKG_BEHAV","context":"handle:VEHICLE"}
```

`context` is optional; if supplied it must be a classifier.

#### Activity partitions

```json
{"op":"activity_partition","external_id":"PART_DRIVER","activity":"handle:ACT_DRIVE","name":"Driver","represented_element":"handle:DRIVER","is_dimension":false,"is_external":false}
```

#### Structured activity nodes

```json
{"op":"structured_activity_node","external_id":"STRUCT_LOOP","activity":"handle:ACT_DRIVE","name":"Control Loop","kind":"Loop"}
```

Kinds: `Structured`, `Conditional`, `Loop`, `Sequence`, `ExpansionRegion`, `InterruptibleRegion`.

#### Activity nodes

```json
{"op":"activity_node","external_id":"ACT_INIT","activity":"handle:ACT_DRIVE","name":"Initial","node":{"kind":"initial"}}
```

Supported `node.kind` values:

- `initial`
- `activity_final`
- `flow_final`
- `decision` (`decision_input` optional)
- `merge`
- `fork`
- `join` (`join_specification` optional)
- `opaque_action` (`body` optional)
- `call_behavior` (`activity` required)
- `call_operation` (`operation` required)
- `send_signal` (`signal` required)
- `accept_event` (`signal` optional)
- `accept_time_event` (`expression` required)
- `object` (`object_kind`, optional type/multiplicity/ordering/selection)
- `activity_parameter` (`parameter` required)

Optional node containment: `partition`, `structured_node`.

#### Pins

```json
{"op":"pin","external_id":"PIN_CMD","owner_action":"handle:ACT_CALL","name":"command","direction":"Input","type_ref":"handle:CMD_TYPE"}
```

Pin directions: `Input`, `Output`, `Value`.

#### Activity edges

```json
{"op":"activity_edge","external_id":"ACT_E1","activity":"handle:ACT_DRIVE","name":"","kind":"ControlFlow","source":"handle:ACT_INIT","target":"handle:ACT_STEP"}
```

Kinds: `ControlFlow`, `ObjectFlow`.

Optional: `source_is_pin`, `target_is_pin`, `guard`, `weight`, `selection`, `transformation`, `interrupting_region`.

Native topology still applies. Examples:

- Initial: no incoming; exactly one outgoing ControlFlow.
- Activity/Flow Final: no outgoing.
- Fork: exactly one incoming and at least two outgoing ControlFlows.
- Join: at least two incoming and exactly one outgoing ControlFlow.
- Decision: one incoming and at least one outgoing.
- Merge: at least one incoming and exactly one outgoing.
- ControlFlow cannot connect directly to Pins.
- ObjectFlow endpoints and types must be compatible.

### 6.6 State Machine

```json
{"op":"state_machine","external_id":"SM_MODES","name":"Vehicle Modes","context":"handle:VEHICLE"}
{"op":"region","external_id":"SM_REGION","name":"Modes","state_machine":"handle:SM_MODES"}
{"op":"vertex","external_id":"SM_INIT","region":"handle:SM_REGION","name":"Initial","vertex":{"kind":"pseudostate","pseudostate":"Initial"}}
{"op":"vertex","external_id":"SM_READY","region":"handle:SM_REGION","name":"Ready","vertex":{"kind":"state"}}
{"op":"transition","external_id":"SM_T1","region":"handle:SM_REGION","source":"handle:SM_INIT","target":"handle:SM_READY"}
```

Vertex forms:

- `{ "kind":"state", "entry":..., "do_activity":..., "exit":..., "submachine":... }`
- `{ "kind":"final_state" }`
- `{ "kind":"pseudostate", "pseudostate":"Initial" }`

Pseudostates: `Initial`, `Choice`, `Junction`, `Fork`, `Join`, `ShallowHistory`, `DeepHistory`, `EntryPoint`, `ExitPoint`, `Terminate`.

Transition kinds: `External`, `Internal`, `Local`.

Trigger forms:

- `{ "kind":"signal", "signal":"handle:SIG" }`
- `{ "kind":"call", "operation":"handle:OP" }`
- `{ "kind":"time", "expression":"5 s", "is_relative":true }`
- `{ "kind":"change", "expression":"speed > 0" }`
- `{ "kind":"any_receive" }`

Optional transition fields: `guard`, `effect`.

Regions MUST be created before their vertices/transitions. Vertex endpoints MUST belong to the legal region/state-machine structure.

### 6.7 Sequence / Interaction

```json
{"op":"interaction","external_id":"INT_START","name":"Startup","context":"handle:VEHICLE"}
{"op":"lifeline","external_id":"LL_CTRL","interaction":"handle:INT_START","name":"controller","represented_path":["handle:CONTROLLER_PART"]}
{"op":"occurrence","external_id":"OCC_1","interaction":"handle:INT_START","lifeline":"handle:LL_CTRL","order":1}
```

Message:

```json
{
  "op":"message",
  "external_id":"MSG_START",
  "interaction":"handle:INT_START",
  "name":"start",
  "sort":"SynchCall",
  "send":"handle:OCC_SEND",
  "receive":"handle:OCC_RECV",
  "signature":{"kind":"operation","operation":"handle:OP_START"},
  "arguments":["mode=1"]
}
```

Message sorts: `SynchCall`, `AsynchCall`, `AsynchSignal`, `Reply`, `Create`, `Delete`, `Lost`, `Found`.

Execution:

```json
{"op":"execution","external_id":"EXEC_1","interaction":"handle:INT_START","lifeline":"handle:LL_CTRL","start":"handle:OCC_RECV","finish":"handle:OCC_FINISH","behavior":"handle:OP_START"}
```

Combined Fragment:

```json
{"op":"combined_fragment","external_id":"FRAG_ALT","interaction":"handle:INT_START","operator":"Alt","covered_lifelines":["handle:LL_CTRL"]}
```

Operators: `Alt`, `Opt`, `Loop`, `Break`, `Par`, `Critical`, `Neg`, `Assert`, `Strict`, `Seq`, `Ignore`, `Consider`.

Operand:

```json
{"op":"operand","external_id":"OPERAND_OK","fragment":"handle:FRAG_ALT","guard":"ok","start_order":10,"end_order":20}
```

State invariant:

```json
{"op":"state_invariant","external_id":"INV_READY","interaction":"handle:INT_START","lifeline":"handle:LL_CTRL","order":15,"constraint":"ready"}
```

Sequence ordering is semantic and MUST be internally consistent. Do not use pixel Y coordinates as occurrence identity/order.

### 6.8 Parametric metadata

```json
{
  "op":"parametric_metadata",
  "element":"handle:CB_FORCE",
  "constraint_expression":"F = m * a",
  "quantity_dimension":"M L T^-2",
  "unit_symbol":"N",
  "unit_scale_to_base":1.0
}
```

Optional metadata fields:

- `constraint_expression`
- `quantity_kind_external_id`
- `unit_external_id`
- `quantity_dimension`
- `unit_symbol`
- `unit_scale_to_base` (finite numeric value)

### 6.9 Binding

```json
{
  "op":"binding",
  "external_id":"BIND_MASS",
  "name":"massBinding",
  "owner":"handle:VEHICLE",
  "source":{"role":"handle:MASS"},
  "target":{"role":"handle:FORCE_CONSTRAINT","parameter":"handle:CP_M"}
}
```

Binding endpoint roles/parameters MUST be legal and type-compatible. Do not create a Binding merely because two elements are visually near each other.

---

## 7. Model Script ElementKind values

The model-core enum currently contains:

```text
Model
Package
ModelLibrary
Block
AssociationBlock
InterfaceBlock
ConstraintBlock
ValueType
DataType
PrimitiveType
Enumeration
EnumerationLiteral
Signal
Unit
QuantityKind
InstanceSpecification
Slot
PartProperty
ReferenceProperty
ValueProperty
FlowProperty
ConstraintProperty
ConstraintParameter
ProxyPort
FullPort
Operation
Parameter
Reception
Requirement
TestCase
Actor
UseCase
Comment
```

Parsing a kind does not make every owner/type combination legal. Native validation still controls ownership and semantics.

A generator SHOULD NOT create a second ordinary `Model` element as a substitute for the existing Project root.

---

## 8. Relationship direction and intent

Use source/target consistently.

| Relationship | Source -> Target generator convention |
| --- | --- |
| `Generalization` | specific/child -> general/parent |
| `Dependency` | dependent/client -> supplier |
| `Realization` | realizing element -> specification |
| `Allocate` | allocated-from -> allocated-to |
| `DeriveRequirement` | derived Requirement -> source Requirement |
| `Satisfy` | satisfying design/behavior element -> Requirement |
| `Verify` | TestCase -> Requirement |
| `Refine` | refining element -> refined Requirement/model element |
| `Trace` | tracing source -> traced target |
| `Copy` | copied/view Requirement -> authoritative/source Requirement |
| `Include` | including Use Case -> included Use Case |
| `Extend` | extending Use Case -> extended/base Use Case |

For `Association`, semantic end fields determine role names, multiplicity, navigability, and aggregation.

---

## 9. Diagram declaration contract for Model Script

Diagram declarations are separate from semantics.

```json
{
  "external_id":"D_REQ",
  "family":"Requirement",
  "name":"Vehicle Requirements",
  "owner":"handle:PKG_REQ",
  "context":"handle:VEHICLE",
  "populate":true,
  "clean_layout":true,
  "route":true
}
```

Supported families:

1. `Package`
2. `Requirement`
3. `Use Case`
4. `BDD`
5. `IBD`
6. `Activity`
7. `State Machine`
8. `Sequence`
9. `Parametric`

Rules:

- Every diagram MUST have a unique stable `external_id`.
- `owner` MUST resolve to a legal semantic owner.
- `context` is used only where applicable and MUST resolve legally.
- Activity, State Machine, and Sequence diagrams MUST use `semantic` to reference the existing Activity, State Machine, or Interaction they represent.
- Do not declare a specialized diagram before its `semantic` record exists.
- `populate`, `clean_layout`, and `route` default to `true` if omitted.
- For complex generated models, a generator MAY set `clean_layout:false` or `route:false` where automatic layout/routing is intentionally deferred, but this does not waive semantic validity.

Specialized examples:

```json
{"external_id":"D_ACT","family":"Activity","name":"Operate Vehicle","owner":"handle:PKG_BEHAV","semantic":"handle:ACT_OPERATE"}
{"external_id":"D_SM","family":"State Machine","name":"Vehicle Modes","owner":"handle:PKG_BEHAV","semantic":"handle:SM_MODES"}
{"external_id":"D_SEQ","family":"Sequence","name":"Startup Sequence","owner":"handle:PKG_BEHAV","semantic":"handle:INT_START"}
```

For a generated all-nine-family model, the generator MUST verify that all 9 declared families are committed exactly once and become visible/openable after apply.

---

## 10. Mapped XLSX/CSV import contract

### 10.1 A mapped workbook import is two things

A mapped spreadsheet import consists of:

1. the `.xlsx` or `.csv` data file; and
2. a `SpreadsheetImportMapGroup` configuration describing how source columns map to semantic properties.

The workbook alone is not required to use Rust field names. Business-facing headers are allowed.

### 10.2 File rules

- `.xlsx` is supported.
- `.csv` is supported.
- legacy `.xls` is not supported.
- XLSX mappings MUST specify an explicit worksheet name.
- `header_row` is **one-based** and MUST be at least `1`.
- The configured header row MUST exist.
- Every mapped source header MUST exist exactly once.
- Blank rows after the header are ignored.
- A single semantic property MUST NOT be mapped from more than one source column in the same map.

### 10.3 MapGroup JSON shape

Canonical structure:

```json
{
  "mappings": [
    {
      "name": "Requirements",
      "source": "C:/path/vehicle.xlsx",
      "worksheet": "Requirements",
      "header_row": 1,
      "element_kind": "Requirement",
      "relationship_kind": null,
      "relationship_identity": "ExternalId",
      "target_scope": "<CURRENT TARGET ELEMENT UUID>",
      "identification_property": "ExternalId",
      "search_scope": "TargetRecursive",
      "source_namespace": "program:vehicle:requirements",
      "mapping_version": "1",
      "column_mappings": [
        {"source_column":"External ID","property":"ExternalId"},
        {"source_column":"Name","property":"Name"},
        {"source_column":"Requirement ID","property":"RequirementId"},
        {"source_column":"Requirement Text","property":"RequirementText"},
        {"source_column":"Owner","property":"Owner"}
      ]
    }
  ]
}
```

### 10.4 Map configuration fields

| Field | Required | Rule |
| --- | --- | --- |
| `name` | YES | Nonblank import-map name |
| `source` | YES | Runtime file path; file name/path is not semantic identity |
| `worksheet` | XLSX YES | Exact worksheet name; CSV may omit |
| `header_row` | YES | One-based physical header row |
| `element_kind` | YES | Fixed element kind for an element map; legacy field remains present for relationship/behavior maps |
| `relationship_kind` | Relationship map: fixed kind or mapped `RelationshipKind` column | Use fixed kind for homogeneous relationship sheets |
| `relationship_identity` | Defaults to `ExternalId` | `ExternalId`, `KindSourceTarget`, or `KindSourceTargetAssociationEnds` |
| `target_scope` | YES | Existing stable Project `ElementId` of the namespace into which the mapping is evaluated |
| `identification_property` | YES | `ExternalId`, `Name`, or `RequirementId` |
| `search_scope` | YES | `TargetOnly` or `TargetRecursive` |
| `source_namespace` | YES | Nonblank; not derived from file name |
| `mapping_version` | YES | Nonblank mapping-profile version |
| `column_mappings` | YES | Physical header -> controlled semantic property mappings |

`target_scope` is runtime Project identity, not workbook source identity. A workbook generator that does not know the current Project UUID SHOULD leave target-scope injection to the importing UI/adapter rather than inventing a UUID.

### 10.5 Preferred identification policy

For generated engineering workbooks, use:

```text
identification_property = ExternalId
relationship_identity = ExternalId
```

Use Name identification only for a controlled legacy source where names are guaranteed unique. Use `RequirementId` only for Requirement maps.

### 10.6 Map execution order

MapGroup mappings are executed in array order so later maps can resolve content planned by earlier maps without early commits.

Recommended workbook mapping order:

1. Packages / ModelLibraries
2. types and classifiers
3. Blocks / InterfaceBlocks / ConstraintBlocks / Signals
4. Operations / Requirements / TestCases / Actors / UseCases
5. properties / ports / Parameters / Receptions
6. ordinary relationships
7. Connectors
8. ItemFlows
9. Activity records
10. State Machine records
11. Sequence records
12. Parametric metadata / bindings

---

## 11. Direct spreadsheet ElementKind support

The mapped element path currently accepts these fixed `element_kind` values:

```text
Package
ModelLibrary
Block
AssociationBlock
InterfaceBlock
ConstraintBlock
ValueType
DataType
PrimitiveType
Enumeration
Signal
Actor
UseCase
Requirement
TestCase
PartProperty
ReferenceProperty
ValueProperty
FlowProperty
ConstraintProperty
ConstraintParameter
ProxyPort
FullPort
Operation
Parameter
Reception
```

Do not assume every model-core ElementKind is a direct spreadsheet element-map kind.

Additional specialized behavior/parametric records use the `BehaviorKind` path described below.

---

## 12. Spreadsheet element-column rules that commonly block import

### 12.1 Identification

The column corresponding to `identification_property` MUST be mapped.

- `ExternalId` identification -> map `ExternalId`.
- `Name` identification -> map `Name`.
- `RequirementId` identification -> map `RequirementId` and the fixed element kind MUST be `Requirement`.

### 12.2 Type requirements

The following element maps MUST map `Type`:

```text
PartProperty
ReferenceProperty
ValueProperty
FlowProperty
ConstraintProperty
ConstraintParameter
ProxyPort
FullPort
Parameter
```

The Type reference MUST resolve to a compatible semantic type.

### 12.3 Requirement-only fields

`RequirementId` and `RequirementText` are valid only for `Requirement` element maps.

Engineering requirement generators SHOULD always provide both a stable Requirement ID and nonblank Requirement Text.

### 12.4 Use Case-only field

`ExtensionPoints` is valid only for `UseCase`.

### 12.5 Parameter

`Parameter` requires:

- mapped `Type`;
- mapped `ParameterDirection`.

Direction values: `In`, `Out`, `InOut`, `Return`.

### 12.6 Reception

`Reception` requires mapped `AcceptedSignal`.

Do not use generic `Type`, `Multiplicity`, or `DefaultValue` as a substitute for `AcceptedSignal`.

### 12.7 FlowProperty

`FlowProperty` requires mapped `FlowDirection`.

Values: `In`, `Out`, `InOut`.

### 12.8 Conjugated

`Conjugated` is valid only for `ProxyPort` and `FullPort`.

### 12.9 Default Value

`DefaultValue` is valid only for `ValueProperty` and `Parameter` in the direct mapped element path.

### 12.10 Multiplicity

Multiplicity is valid on typed feature kinds and `Parameter`. Use valid SysML multiplicity syntax/data as expected by the mapping adapter; do not provide a lower bound greater than the upper bound.

---

## 13. Spreadsheet relationship-map rules

Supported direct mapped relationship kinds:

```text
Association
Generalization
Dependency
Realization
Allocate
DeriveRequirement
Satisfy
Verify
Refine
Trace
Copy
Connector
ItemFlow
Include
Extend
PackageImport
ElementImport
PackageMerge
```

### 13.1 Required relationship columns

Every relationship map MUST map:

- `Source`
- `Target`

It MUST also have either:

- a fixed `relationship_kind`; or
- a mapped `RelationshipKind` column.

If `relationship_identity` is `ExternalId`, the map MUST map `ExternalId`.

### 13.2 Generator rule for endpoints

For error-resistant generated workbooks, Source/Target cells SHOULD contain stable source External IDs that resolve in the same source namespace / ordered MapGroup, not display names.

### 13.3 Association-only fields

These are valid only for Association rows:

- `SourceEndRole`
- `TargetEndRole`
- `SourceMultiplicity`
- `TargetMultiplicity`
- `SourceNavigable`
- `TargetNavigable`
- `SourceAggregation`
- `TargetAggregation`

### 13.4 Connector relationship map

A fixed native Connector map MUST also map:

- `ConnectorContext`
- `ConnectorKind`

Do not map Connector-only fields on another relationship kind.

### 13.5 ItemFlow relationship map

A fixed native ItemFlow map MUST also map:

- `Connector`
- `ConveyedItems`

Do not map ItemFlow-only fields on another relationship kind.

### 13.6 ElementImport alias

`Alias` is valid only for `ElementImport`.

### 13.7 Extend fields

`ExtensionCondition` and `ExtensionLocation` are valid only for `Extend`.

---

## 14. Spreadsheet BehaviorKind values

Specialized Activity / State Machine / Sequence / Parametric spreadsheet rows use a mapped `BehaviorKind` plus a mapped `ExternalId`.

Current controlled row kinds are:

```text
Activity
ActivityPartition
StructuredActivityNode
ActivityInitial
ActivityFinal
FlowFinal
Decision
Merge
ActivityFork
ActivityJoin
OpaqueAction
CallBehaviorAction
CallOperationAction
SendSignalAction
AcceptEventAction
AcceptTimeEventAction
ObjectNode
CentralBufferNode
DataStoreNode
ActivityParameterNode
Pin
ControlFlow
ObjectFlow
StateMachine
Region
State
FinalState
StateInitial
Choice
Junction
StateFork
StateJoin
ShallowHistory
DeepHistory
EntryPoint
ExitPoint
Terminate
Transition
Interaction
Lifeline
Occurrence
Message
ExecutionSpecification
CombinedFragment
InteractionOperand
StateInvariant
ParametricElement
BindingConnector
```

Behavior mappings MUST include at least:

- mapped `BehaviorKind`;
- mapped `ExternalId`;
- nonblank `source_namespace`;
- nonblank `mapping_version`;
- a valid namespace `target_scope`.

Other required columns depend on the BehaviorKind and its semantic dependencies. Use the same dependency ordering described in Sections 3.5 and 10.6.

---

## 15. Complete list of mapped spreadsheet semantic properties

These are the controlled semantic property names accepted by the current mapping schema. A generator SHOULD use these exact enum spellings in MapGroup JSON.

```text
Name
Documentation
Owner
Type
Multiplicity
DefaultValue
ParameterDirection
AcceptedSignal
FlowDirection
Conjugated
ExternalId
Visibility
RequirementId
RequirementText
ExtensionPoints
RelationshipKind
Alias
ExtensionCondition
ExtensionLocation
ConnectorContext
ConnectorKind
Connector
ConveyedItems
Source
Target
SourceEndRole
TargetEndRole
SourceMultiplicity
TargetMultiplicity
SourceNavigable
TargetNavigable
SourceAggregation
TargetAggregation
BehaviorKind
Context
Activity
StateMachine
Region
ParentState
Partition
StructuredNode
StructuredKind
ParentStructuredNode
RepresentedElement
IsDimension
IsExternal
Body
CalledActivity
Operation
Signal
Expression
DecisionInput
JoinSpecification
ObjectOrdering
Selection
Parameter
PinDirection
Ordered
Unique
Value
Guard
Weight
Transformation
InterruptingRegion
OwnerAction
Entry
DoActivity
Exit
Submachine
TransitionKind
TriggerKind
TriggerReference
TriggerExpression
TriggerRelative
Effect
Interaction
Lifeline
RepresentedPath
Order
MessageSort
SendOccurrence
ReceiveOccurrence
Signature
Arguments
StartOccurrence
FinishOccurrence
Behavior
CombinedFragment
Operator
CoveredLifelines
StartOrder
EndOrder
Constraint
ConstraintExpression
QuantityKind
Unit
QuantityDimension
UnitSymbol
UnitScaleToBase
BindingSourceRole
BindingSourceParameter
BindingTargetRole
BindingTargetParameter
```

Do not map a semantic property merely because it exists in this list. It must also be legal for the selected element/relationship/BehaviorKind.

---

## 16. Full-state Systems Modeler XLSX interchange

The `systems-modeler` spreadsheet interchange profile is a **round-trip format**, not the preferred hand-authored workbook template.

It contains an embedded portable authored-state payload in a sheet named:

```text
SystemsModelerState
```

The direct importer requires that sheet and reconstructs the portable JSON chunks before candidate validation/apply.

The workbook also contains normalized inspectable sheets such as Elements, Requirements, Relationships, Activities, behaviors, diagrams, and other semantic/presentation views. Those visible sheets do **not** replace the authoritative embedded authored state for full-fidelity direct round trip.

### Generator rule

Do not manually fabricate a `SystemsModelerState` workbook unless the generator implements the exact current portable authored-state schema and its invariants. For ordinary generated spreadsheet input, use the mapped XLSX route. For a complete model including diagrams, use Model Script.

---

## 17. Requirements modeling rules for generated imports

A proper engineering requirements model SHOULD contain a real hierarchy and traceability, not isolated Requirement boxes.

### 17.1 Requirement records

Each authoritative Requirement SHOULD have:

- stable External ID;
- stable human Requirement ID;
- clear `name`;
- nonblank normative `requirement_text`;
- legal owner package;
- optional rationale/documentation separated from the normative requirement statement.

### 17.2 Recommended hierarchy

```text
Stakeholder / Mission
    -> System
        -> Performance / Interface
            -> Subsystem / Component
                -> Verification
```

### 17.3 Traceability semantics

Use the actual relationship kinds rather than generic Dependency when the SysML semantic is known:

- `DeriveRequirement`
- `Satisfy`
- `Verify`
- `Refine`
- `Trace`
- `Copy`

Verification TestCases SHOULD `Verify` authoritative Requirement elements.

Architecture/behavior SHOULD `Satisfy` or `Refine` the requirements they implement or elaborate.

---

## 18. No-error preflight checklist

Before handing a workbook/script to a user, CI job, Work session, Codex task, or another agent, the generator MUST check the following.

### 18.1 Identity

- [ ] source namespace is nonblank
- [ ] source namespace is intentionally stable for reimport
- [ ] every required External ID is nonblank
- [ ] duplicate External IDs = 0
- [ ] no stale internal UUID is used as source identity/reference
- [ ] Requirement IDs are unique where the project governance requires uniqueness

### 18.2 References

- [ ] missing required references = 0
- [ ] ambiguous references = 0
- [ ] wrong-kind references = 0
- [ ] generated dependencies are defined before use
- [ ] every owner resolves
- [ ] every type resolves
- [ ] every relationship source/target resolves
- [ ] every behavior parent/context resolves
- [ ] every specialized semantic reference uses an External ID/handle

### 18.3 Structural semantics

- [ ] property/port owners are legal
- [ ] property/port types are compatible
- [ ] Parameter owner/type/direction are legal
- [ ] Reception accepted Signal resolves
- [ ] Connector context is legal
- [ ] Connector source/target paths are legal
- [ ] ItemFlow refers to a real Connector
- [ ] ItemFlow topology matches the Connector
- [ ] conveyed items resolve

### 18.4 Requirements / relationships

- [ ] authoritative Requirements have nonblank Requirement ID and text
- [ ] relationship kind is the intended SysML semantic kind
- [ ] duplicate semantic relationship keys = 0
- [ ] Satisfy/Verify/Derive/Refine/Trace/Copy direction is correct

### 18.5 Activity

- [ ] Activity owner is namespace/classifier
- [ ] Activity context, if present, is classifier
- [ ] partitions/structured parents resolve
- [ ] node/pin IDs are unique
- [ ] Activity edge endpoints resolve
- [ ] initial/final/fork/join/decision/merge topology is valid
- [ ] ControlFlow does not directly connect Pins
- [ ] ObjectFlow endpoint types/directions are compatible

### 18.6 State Machine

- [ ] State Machine context resolves
- [ ] regions exist before contained vertices/transitions
- [ ] transition endpoints belong to legal regions
- [ ] triggers reference legal Signals/Operations or valid event expressions
- [ ] submachine references resolve

### 18.7 Sequence

- [ ] Interaction context resolves
- [ ] represented lifeline paths resolve
- [ ] occurrences reference valid lifelines
- [ ] occurrence ordering is coherent
- [ ] message send/receive occurrences resolve
- [ ] message signature kind matches referenced Operation/Signal
- [ ] execution start/finish occurrences resolve and are ordered
- [ ] fragment/operand coverage/order is coherent

### 18.8 Parametrics

- [ ] ConstraintBlock / ConstraintProperty / ConstraintParameter ownership is legal
- [ ] constraint expression metadata is valid for the selected element
- [ ] unit scale is finite when supplied
- [ ] binding roles/parameters resolve
- [ ] binding endpoint types are compatible

### 18.9 Diagrams for Model Script

- [ ] every diagram has a stable unique External ID
- [ ] every owner/context resolves
- [ ] specialized Activity/State Machine/Sequence `semantic` target resolves
- [ ] expected family count matches intended model
- [ ] for all-nine model: exactly one each of Package, Requirement, Use Case, BDD, IBD, Activity, State Machine, Sequence, Parametric

### 18.10 Workbook physical checks

- [ ] file extension is `.xlsx` or `.csv`
- [ ] XLSX worksheet names in the map exist exactly
- [ ] header row is one-based and exists
- [ ] all mapped headers exist exactly once
- [ ] no semantic property is mapped twice in one map
- [ ] mapping version is nonblank
- [ ] target scope is a real current Project namespace

### 18.11 Native acceptance

The generator MUST NOT call an input successful until native preview reports:

```text
BLOCKED = 0
```

and no error diagnostics.

After apply:

- apply must report success;
- the complete candidate must pass native semantic and presentation validation;
- an intended fresh all-nine model must commit exactly the declared 9 diagrams;
- reimport of the same unchanged source namespace + External IDs SHOULD classify as `NO_CHANGE` rather than duplicate content.

---

## 19. Known bad patterns — do not generate these

```text
BAD: owner/source/target is a UUID copied from another Project
BAD: source namespace changes on every filename revision even though the model is the same identity domain
BAD: External IDs are generated from row numbers and change when rows are reordered
BAD: owner is omitted and the generator assumes the importer will place the record at root
BAD: a Type or Signal is referenced before it exists in the generated plan
BAD: relationship Source/Target cells contain ambiguous display names
BAD: Parameter is placed under ConstraintBlock instead of using the correct constraint semantic
BAD: ProxyPort/FullPort is generated without a compatible type
BAD: Connector or ItemFlow is represented only as a graphical line
BAD: ItemFlow uses structural paths unrelated to its Connector
BAD: Binding endpoints are type-incompatible
BAD: Activity final node has outgoing flow
BAD: ControlFlow connects directly to Pins
BAD: transition/message endpoints are invented to satisfy a diagram
BAD: specialized Activity/Behavior reference uses $root or qname instead of External ID
BAD: a diagram references an Activity/State Machine/Interaction that has not been created
BAD: hand-authored workbook assumes visible Systems Modeler semantic sheets replace SystemsModelerState
BAD: .xls is supplied to the mapped importer
BAD: preview diagnostics are ignored and Apply is attempted anyway
```

---

## 20. Recommended deliverables from an AI/automation generator

When an agent is instructed to generate an import artifact, it SHOULD return:

### For Model Script

1. the `.groovy`/`.smscript` file;
2. a short validation report containing:
   - source namespace;
   - operation count;
   - diagram count by family;
   - duplicate External ID count;
   - unresolved/missing reference count;
   - forward-reference count;
   - requirement completeness count;
   - connector/item-flow validation summary;
   - Activity/State Machine/Sequence/Parametric validation summary;
3. explicit statement that static preflight is not a substitute for native preview/apply.

### For mapped workbook

1. the `.xlsx` or `.csv` file;
2. the MapGroup JSON/profile required to import it;
3. a worksheet/header inventory;
4. a stable External-ID manifest;
5. the same reference/semantic preflight summary above;
6. a note identifying which `target_scope` must be selected/injected at import time.

---

## 21. Copy/paste instruction for another agent

The following block may be fed directly to another generator/reviewer:

```text
Create an import artifact for Systems Modeler Pro that complies with
`docs/IMPORTER_INPUT_SPECIFICATION.md`.

Hard requirements:
- use the correct import mechanism for the requested outcome;
- preserve one stable nonblank source namespace for one logical source model;
- use stable unique External IDs and never stale Project UUIDs as source references;
- define generated dependencies before use;
- resolve every owner/type/source/target/behavior/connector/binding/diagram reference;
- use legal native SysML ownership and typing;
- use dedicated Connector, ItemFlow, Activity, State Machine, Sequence, and Binding semantics rather than graphical placeholders;
- do not weaken native validation;
- produce zero duplicate External IDs, zero missing references, zero ambiguous references, and zero forward plan-local references;
- for a mapped workbook, provide the MapGroup JSON and use only legal property mappings for each semantic kind;
- for a complete model with diagrams, prefer Model Script and declare only diagrams whose semantic targets already exist;
- run a static preflight and report the results;
- do not call the artifact qualified until Systems Modeler Pro native preview reports BLOCKED 0 and no error diagnostics.
```

---

## 22. Repository references for implementers

When this specification and code disagree, the current Rust schema/validator is authoritative and this document must be updated in the same change.

Primary implementation references:

- [`apps/desktop/src-tauri/src/workspace/model_script.rs`](../apps/desktop/src-tauri/src/workspace/model_script.rs) — Model Script JSON schema/reference parsing/preview/apply
- [`examples/model-script/vehicle-model.groovy`](../examples/model-script/vehicle-model.groovy) — bounded all-nine-family example
- [`examples/model-script/complete-vehicle-demo.groovy`](../examples/model-script/complete-vehicle-demo.groovy) — larger complete example
- [`apps/desktop/src-tauri/src/workspace/spreadsheet_import.rs`](../apps/desktop/src-tauri/src/workspace/spreadsheet_import.rs) — mapped XLSX/CSV schema, supported properties/kinds, validation
- [`apps/desktop/src-tauri/src/workspace/spreadsheet_import/pr48_behavior.rs`](../apps/desktop/src-tauri/src/workspace/spreadsheet_import/pr48_behavior.rs) — Activity/State Machine/Sequence/Parametric spreadsheet row kinds
- [`apps/desktop/src-tauri/src/workspace/spreadsheet_interchange.rs`](../apps/desktop/src-tauri/src/workspace/spreadsheet_interchange.rs) — full-state XLSX interchange
- [`crates/model-core/src/model.rs`](../crates/model-core/src/model.rs) — core element/relationship kinds
- [`crates/model-core/src/activity.rs`](../crates/model-core/src/activity.rs) — Activity semantic rules
- [`crates/model-core/src/behavior.rs`](../crates/model-core/src/behavior.rs) — State Machine/Sequence semantic rules
- [`docs/IMPORT_RULES_AND_QUALIFICATION.txt`](IMPORT_RULES_AND_QUALIFICATION.txt) — qualification/status/developer contract

---

## 23. Maintenance rule

Any PR that changes:

- accepted Model Script operations or fields;
- reference syntax;
- supported spreadsheet semantic properties;
- supported element/relationship/BehaviorKind scope;
- required spreadsheet columns;
- diagram family import behavior;
- identity/reimport semantics; or
- preview/apply qualification rules

**MUST update this document in the same PR.**

That requirement exists so future humans and agents do not have to reverse-engineer importer behavior from implementation history before generating a valid source file.
