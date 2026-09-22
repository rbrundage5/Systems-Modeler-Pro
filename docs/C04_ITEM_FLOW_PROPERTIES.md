# C04.09.01c — existing ItemFlow Properties editor

Baseline: 2e3f7d7cdde7dc1ddd654c1a7777696b11b4390c (PR119), depending
on the atomic API in PR118 and direction/refresh repair in PR119.

One connector's Properties exposes its existing flow identities and a form for
name, direction, and multiple conveyed classifiers. Rust supplies qualified
choices and endpoint labels. One Apply submits the entire specification.
Rejected drafts persist, pending writes prevent duplicate submission, Reload
Saved retries reads, and stale responses cannot change a different selection.

Allowed paths: frontend/item-flow-properties.js, item-flow-ui.js, index.html;
scripts/test_item_flow_properties.cjs and frontend regression entry point; this
record. Lead direct implementation, workers disabled, independent review open.

Acceptance: exact atomic payload, named endpoints/multiple types, rejected draft,
selection-race guard, duplicate submission guard, loading failure/retry, and
continued normal connector/element Properties. Native pointer/keyboard/rendered
acceptance is still required. Flow creation, deletion, runtime and connector
association typing remain existing/separate commands rather than this leaf.
