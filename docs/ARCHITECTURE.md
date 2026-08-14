# Systems Modeler Pro Architecture

## Product objective

Systems Modeler Pro is being built as a professional systems-modeling environment comparable in workflow depth and engineering rigor to established desktop modeling tools while remaining an independent implementation.

The product must eventually support complete SysML-oriented structure, behavior, requirements, parametrics, verification, traceability, configuration/version governance, collaboration, automation, and very large engineering repositories.

## Architectural principles

1. **The semantic model is authoritative.** Diagrams are presentations of semantic content, not the source of truth.
2. **The UI never owns the model.** Frontend code issues typed commands/queries against the Rust engine.
3. **SQLite is the local authoritative store.** JSON remains an interchange/compatibility format, not the primary database.
4. **Stable IDs never depend on names or presentation IDs.** Semantic identity survives rename, reparent, diagram changes, import, and collaboration.
5. **Relationships are first-class semantic records.** Endpoints, ownership, typing, roles, multiplicities, direction, and derived presentation state are stored independently.
6. **Transactions protect model integrity.** Imports, bulk edits, merges, and migrations commit atomically or roll back.
7. **Performance follows the working set.** Repository size must not force full-model loads or scans for ordinary operations.
8. **Collaboration is operation-based.** Revisions, branches, locks, audit records, and conflicts are semantic operations, not full-project replacement.
9. **Offline use is first-class.** No GitHub, Cloudflare, or public internet service is required to create, edit, validate, save, or reopen a model.
10. **Server deployment is optional.** The same Rust model/collaboration logic should support embedded LAN hosting and a standalone on-premises server.

## Long-term repository layout

```text
apps/
  desktop/          Tauri desktop application
  server/           optional collaboration/model server
crates/
  model-core/       semantic identities, elements, relationships, transactions
  model-query/      indexed model queries and working-set loading
  validation/       SysML/UML/profile validation
  persistence/      SQLite repositories, migrations, transactions
  import/           CATIA/Cameo-style workbook and interchange migration
  collaboration/    operations, revisions, branches, locks, merge/conflicts
  diagrams/         diagram/presentation semantics
  routing/          routing, layout, spatial indexes
  verification/     test cases, evidence, coverage, analysis
frontend/           shared TypeScript UI when extracted from the desktop app
sdk/                future CLI and Python SDK
compatibility/      legacy modeler-proto import/migration fixtures
```

## Foundation scope

PR #1 intentionally implements only enough semantics to prove the architecture:

- strongly typed project/element/relationship IDs
- Model, Package, and Block semantic elements
- Dependency, Association, Composition, and Generalization relationship records
- ownership and endpoint integrity checks
- indexed SQLite tables for project, element, and relationship storage
- transactional save and exact semantic reload
- native Tauri application shell

This is not the final SysML metamodel. It is the stable platform on which the complete metamodel will be built.

## Migration rule

`modeler-proto` is a behavioral and compatibility reference only. Source code is not wholesale copied into this repository. Features are reimplemented against explicit conformance cases so old architectural constraints are not inherited accidentally.
