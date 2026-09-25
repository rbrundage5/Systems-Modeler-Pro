# C03.REUSE.02 — explicit composition authoring

Depends on PR179. User requires relationship creation and property authoring to
share stable usage identity. Baseline: PR179 head 24054938bddcd54fd95d69470d94bbf276951d07.
Reference: supplied SysML 1.6 §8.3.2.3, UML 2.5.1 Property/association ends.

Allowed: property_presentation.rs, bdd_elements.rs public command boundary,
main.rs, app.js/bdd-completion-ui.js/bdd-extended-ui.js dispatch, new
bdd-composition-authoring.js, index.html, undo-redo-ui.js and focused tests/docs.
Drawing a composition asks explicitly for an existing part ID or a named new part
with multiplicity. The selected target classifier is its type. No generated suffix
or name matching establishes identity. One native transaction creates/reuses the
part and association, stages diagram geometry and commits one history entry.
Invalid name/type/owner/multiplicity/routing leaves the original workspace intact.
Self-typed definitions reuse the existing Rust self-route and stay classifier-level.

Acceptance: two distinct usages, deliberate reuse, duplicate-name rejection,
missing/invalid type rollback, native history, presentation-only repeat no-op,
self-route, all production BDD dispatch paths, cancel/retry retains input. Native,
rendered and independent review gates are recorded separately.
