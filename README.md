# Systems Modeler Pro

Systems Modeler Pro is the clean native rewrite of the Systems Modeler prototype.

## Product direction

The target is a professional systems-modeling platform with CATIA/Cameo-class modeling workflows while remaining an independent implementation. The application is being designed for:

- SysML-oriented semantic modeling with strict ownership, typing, relationships, requirements, behavior, parametrics, verification, and traceability
- BDD, IBD, requirements, use case, activity, state, sequence, parametric, package, and traceability diagram workflows
- very large repositories with working-set-driven performance
- deterministic validation, transactions, undo/redo, branching, revision history, and auditability
- local/offline desktop use without GitHub or cloud services at runtime
- peer/LAN collaboration and optional on-premises collaboration server
- stable project storage suitable for long-lived engineering programs
- compatibility migration from the legacy `modeler-proto` project format
- future automation through CLI, REST/gRPC, and Python SDK surfaces

## Architecture direction

The native product uses a Rust semantic/model engine, SQLite persistence, and a Tauri desktop shell with a thin web renderer. Rust owns application behavior and authoritative workspace state; frontend JavaScript is limited to snapshot rendering, input capture, temporary gesture previews, and typed Tauri invocation. The semantic model, command rules, routing, layout, history, validation, or persistence must never live in the UI layer.

Frontend authority debt is measured and prevented from growing by `scripts/validate_rust_authority.py`. New work must reduce or preserve those ceilings while keeping Rust at least the enforced majority of application source.

`modeler-proto` remains the legacy/reference implementation and is intentionally not copied into this repository.

## Import / Interchange

Bulk, scripting, and interchange adapters converge on the Rust-owned `ModelBuildPlan` construction path. CSV/XLSX mapping, bounded Groovy-compatible model scripting, and ReqIF all construct ordinary native model content through the existing semantic authority. ReqIF uses stable source namespace + ReqIF `IDENTIFIER` identity, non-mutating CREATE/UPDATE/NO_CHANGE/REMOVE/BLOCKED preview, atomic apply, preserved exchange metadata/XHTML, and deterministic `.reqif`/`.reqifz` export.

Groovy/model scripts are normal **file imports** in the desktop product. Open **File → Import Model Script**, select a `.groovy`, `.gvy`, `.smscript`, or supported JSON model-script file, review the non-mutating preview, and import atomically. The professional demonstration fixture is [`examples/model-script/professional-ev-demo.groovy`](examples/model-script/professional-ev-demo.groovy); it imports one connected EV SysML model with 12 curated diagrams covering all nine supported diagram families, requirements/verification, multi-level parts breakdown and interfaces, Activity/State/Sequence behavior, drill-down, and executable Parametric analysis. The presentation walkthrough is in [`docs/PROFESSIONAL_EV_DEMO.md`](docs/PROFESSIONAL_EV_DEMO.md).

Semantic import and diagram presentation are separate contracts. Current CSV/XLSX import constructs qualified semantic content through Activity, State Machine, Sequence, and Parametric scopes, but does **not** construct/populate all-nine-family diagram presentations from workbooks. Portable JSON v1 separately preserves the current authored semantic and presentation state through the same complete-build authority.

| Format / mechanism | Current status |
| --- | --- |
| Native `.smproj` | QUALIFIED native working-project persistence; not an interchange format |
| Portable JSON v1 | QUALIFIED authored-project interchange |
| CSV mapped semantic import | QUALIFIED through current semantic scope |
| XLSX mapped semantic import | QUALIFIED through current semantic scope |
| Legacy `.xls` | NOT IMPLEMENTED / PLANNED |
| Groovy / model-script file import | QUALIFIED bounded native model construction / automation with preview + atomic apply |
| ReqIF | QUALIFIED Requirement/TestCase + supported traceability import, stable reimport, `.reqif`/`.reqifz` export |
| XMI 2.x semantic interchange | QUALIFIED for the documented native subset, preview/reimport, profiles, and deterministic export |
| XMI diagram interchange | QUALIFIED for lossless native all-nine-family round trip; producer-neutral shared diagram import is bounded as documented |
| SysML v2 interchange | PLANNED / NOT IMPLEMENTED |
| Native CATIA / 3DEXPERIENCE project files | NOT SUPPORTED |

"CATIA-style" and "Cameo-style" describe configurable mapping approaches. No authentic CATIA/No Magic or Cameo/MagicDraw fixture is currently available in this repository, so no vendor release-specific compatibility or certification claim is made.

For the authoritative import architecture, supported semantic coverage, reimport rules, qualification matrices, runtime boundaries, diagram/presentation status, and planned adapter contracts, see [`docs/IMPORT_RULES_AND_QUALIFICATION.txt`](docs/IMPORT_RULES_AND_QUALIFICATION.txt).

## Repository status

The native migration is active and includes qualified semantic construction/import, portable interchange, native project persistence, and execution foundations. Additional diagram-construction, synchronization/export, and external-adapter work remains explicitly staged and is not implied by the current qualified scope.
