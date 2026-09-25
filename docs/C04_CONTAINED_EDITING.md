# C04.NESTED.02 — contained presentation editing

Depends on C04.NESTED.01 / PR186. Bounded continuation of the authorized nested
structure workflow. Supplied SysML 1.6 §8 nesting notation; diagram geometry is
presentation state, not semantic ownership.

Allowed paths: ibd_structure.rs, ibd.rs, presentation_interaction.rs,
standard_editing.rs, item-flow-ui.js and focused tests/docs. Moving a container
moves its displayed descendants and attached ports once. Resizing cannot clip
children. Removing a container symbol removes its descendant symbols and incident
edge symbols, never definitions. Clean Workspace moves top-level groups as units.
Routes exclude containing rectangles as obstacles, retaining their headers.
Collapsed endpoint routes/ItemFlows are retained but not substituted or drawn.

Tests: nested movement, resize rejection, subtree removal, repeated path isolation,
Clean relative geometry, route/label containment and no semantic mutation.
Connector projection is a separate dependent leaf C04.NESTED.03; complete actual
application acceptance and independent review remain required.
