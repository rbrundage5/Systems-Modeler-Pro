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

The native product will use a Rust semantic/model engine, SQLite persistence, a Tauri desktop shell, and a TypeScript/web frontend. The authoritative semantic model must not live in the UI layer.

`modeler-proto` remains the legacy/reference implementation and is intentionally not copied into this repository.

## Repository status

Foundation only. Feature migration will occur incrementally behind explicit model-engine APIs and conformance tests.
