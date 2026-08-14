# Professional Modeling Roadmap

The development target is not feature parity with the legacy prototype. The target is a production-grade systems-modeling platform with the depth, consistency, governance, and scale expected from professional tools such as CATIA/Cameo-class environments.

## Phase 1 — Native semantic foundation

- typed semantic identities
- transactional SQLite repository
- model/package/block/relationship foundation
- native Tauri application shell
- exact save/reload semantics

## Phase 2 — SysML structural metamodel

- classifiers, value/data/interface types
- properties, ports, operations, parameters, constraints
- association ends, multiplicity, navigability, aggregation/composition
- item flows and connector semantics
- inheritance, redefinition, subsetting, property paths
- BDD and IBD semantic/presentation separation

## Phase 3 — Requirements and traceability

- requirements, IDs, text, hierarchy, derivation, satisfaction, verification, refinement, allocation, copy relationships
- traceability matrices, tables, coverage, suspect links, impact analysis

## Phase 4 — Behavior

- activities, actions, pins, object/control flows, partitions, regions
- state machines and complete transition semantics
- sequence interactions, lifelines, messages, executions, fragments
- use cases, actors, include/extend/generalization

## Phase 5 — Parametrics and verification

- constraint blocks/properties and binding connectors
- value types, units, quantity kinds
- executable/evaluable constraint interfaces
- test cases, verification executions, evidence, coverage

## Phase 6 — Professional diagrams

- diagram type rules and ownership
- notation registry
- compartment/layout engine
- spatial indexing and viewport rendering
- obstacle-safe routing and deterministic clean layout
- drill-down/child diagram navigation

## Phase 7 — Import, interchange, and legacy migration

- migration from `modeler-proto`
- CATIA/Cameo-style workbook import
- transactional import staging and rollback
- stable-ID merge/reimport behavior
- JSON/interchange export
- future standards-based interchange where practical

## Phase 8 — Collaboration and governance

- operation journal
- revisions and commits
- branches and semantic merge
- locks and conflict resolution
- presence and live collaboration
- audit records and model reviews
- local peer/LAN host mode
- standalone on-premises server

## Phase 9 — Scale and automation

- working-set query engine
- lazy project loading
- million-record qualification
- parallel validation/import/routing
- CLI
- Python SDK
- REST/gRPC automation API

## Non-negotiable architecture rules

- GitHub is development infrastructure only, never a runtime dependency.
- Cloud services are optional, never required for local modeling.
- The frontend cannot directly mutate authoritative model storage.
- Diagrams cannot become the semantic source of truth.
- No feature is marked complete without automated semantic, persistence, and UI qualification appropriate to that feature.
