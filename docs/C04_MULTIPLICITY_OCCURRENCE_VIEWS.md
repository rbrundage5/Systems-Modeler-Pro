# C04.OCCURRENCE — compact and allocated part population views

User work order: 25 September 2026, multiplicity to separate IBD symbols.
Baseline: structural/connector candidate `e4866c066205d470939d92aff658718ead0edf88`.
The Vehicle example remains a fixture only. Rust remains authoritative.

Semantic contract: a Block owns one typed part property. The IBD owns its
presentations, not another property definition. A symbol may represent the whole
property population or a named, numbered subset of that population. An occurrence
name is a display alias, not a new UML Property name or classifier. This is an
explicit occurrence-allocation view, not an implicit creation of InstanceSpecifications
or runtime instances. Ordinary classifier-level IBD notation remains available.
Reference: supplied SysML 1.6 Blocks/Internal Blocks and UML 2.5.1 Property,
MultiplicityElement and Connector semantics; no certification guide authority.

Dependency leaves:

1. C04.OCCURRENCE.01: additive occurrence ranges on each semantic path segment,
   names and coverage, native partition/compact editing, occurrence-aware nesting,
   movement and connector projection. Allowed: workspace ibd.rs, ibd_structure.rs,
   ibd_projection.rs, new ibd_occurrences.rs, connector_editing.rs, standard_editing.rs,
   main.rs registrations and mechanical constructors, focused tests and this record.
2. C04.OCCURRENCE.02: native labels/coverage snapshots, explicit occurrence editor,
   repository repeat-drop and compact/expanded controls. Allowed: those native
   snapshot/command paths, ibd-ui.js, focused frontend tests, and this record.

Preserve stable semantic IDs and definition ownership. Occurrence identity consists
of property path plus selected population ranges at every nesting level. Counts
apply per enclosing instance. Repeated presentations of the same selected range
do not allocate additional instances. Distinct allocations must not overlap;
coverage is counted once per selected range and per containing occurrence.

Count/range rejection and unsupported endpoint editing must be atomic. Existing
type-owned connectors can project within each separately displayed containing
occurrence. A classifier-level connector to an aggregate population must not be
silently attached to its first subset; keep the semantic connector and hide that
presentation until an aggregate endpoint exists. Creating per-individual wiring
requires an explicit instance-connection model, not a cloned definition.

Acceptance: generic property [4] -> four named [1] groups or two [2] groups;
partial [3]/[4] coverage; invalid [5]/[4], duplicate aliases and overlap rejection;
multi-level repeated groups expand/move/collapse independently; type definitions
unchanged; return to compact; remove-only; save/reopen and undo/redo; inherited
properties and bounded ranges. Existing metadata defaults to compact.
Independent review and packaged/rendered UI verification remain required.
