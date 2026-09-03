# PR53 — Profiles and semantic XMI

PR53 adds native profile semantics and a bounded, namespace-aware XMI 2.x interchange adapter. XMI import is a staged parse → neutral IR → preview → validated candidate → atomic commit pipeline. XMI never mutates presentation geometry.

## Native profile model

The core owns strongly identified profiles, stereotypes, tag definitions, profile applications, and stereotype applications. Tags are typed as String, Boolean, Integer, Real, Enumeration, or semantic reference and enforce multiplicity. Applicability is explicit for element and relationship kinds. Unresolved legacy display labels remain intact while native applications keep labels synchronized for existing diagram rendering.

Profile state is part of repository transactions, portable JSON, spreadsheet full-fidelity state, workspace snapshots, and the semantic payload used by model-script reconstruction. The desktop profile editor can create/apply/remove definitions and applications and inspect/edit typed values through Rust commands.

## Supported XMI subset

- Prefix-independent XMI/UML namespace recognition and exact `xmi:id` resolution, including forward references.
- UML models, packages, classifiers, datatypes, enumerations, signals, instances, properties, ports, operations, parameters, receptions, actors, use cases, and the listed relationship kinds mapped by the existing native model.
- SysML stereotype applications for blocks, properties, ports, requirements, test cases, allocation, requirement traceability, binding connectors, and item flows where a native kind exists.
- UML Profile/Stereotype/Property definitions and typed stereotype attributes for the native tag subset.
- Activity, StateMachine, and Interaction semantic shells plus a lossless Systems Modeler semantic extension for native-authored behavior detail.
- Deterministic identity-based export and additive/update or authoritative-source reimport.

Unsupported required references, duplicate identities, wrong-kind collisions, invalid tag values, and protected authoritative removals block commit with structured diagnostics. Producer extensions are retained as uninterpreted interchange data where possible.

## Qualification safeguards

XMI metaclass attributes are recognized only through the XMI namespace, so ordinary UML/SysML attributes such as semantic `type` references and stereotype tag values cannot be confused with `xmi:type` or `xmi:id`. Native authored-state preview releases the project validation guard before taking the portable semantic snapshot, preventing recursive locking on the desktop runtime. The Windows desktop qualification includes regression coverage for both behaviors, authoritative CRLF/LF reimport handling, full desktop tests, Clippy with warnings denied, and the existing cross-family integration contracts.

## Explicit boundaries

This is semantic XMI, not UML Diagram Interchange: diagram positions, routing, viewport state, and presentation preferences are excluded. Import does not promise universal compatibility with every UML/SysML producer. Unknown semantic constructs are diagnosed and preserved when safe; they are not guessed into native concepts. The native semantic extension enables exact Systems Modeler round trips but is not presented as a vendor-neutral UML feature.
