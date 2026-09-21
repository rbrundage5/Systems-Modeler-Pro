# C04.09.01b — faithful ItemFlow direction and refresh

Dependency: the ItemFlow specification API in PR118. Reproductions in the existing
renderer: all arrows use connector point order regardless of semantic direction;
same-classifier flows are deduplicated even when opposite; authoritative reads are
merged into old rows so deleted/undone flows survive in the cache.

This leaf adds Rust-derived direction to the existing notation query and renders
every relationship identity in that direction. Refresh replaces notation and
rejects stale responses after a newer refresh or project switch. Labels remain
presentation preferences; no routing or semantic validation moves to JavaScript.

Allowed paths: workspace/item_flow_notation.rs; frontend/item-flow-ui.js;
scripts/test_item_flow_notation.cjs and the existing frontend regression entry
point; this record. Lead direct implementation with no worker/review claim.

Acceptance: opposite flows with the same conveyed classifier both appear; arrow
orientation follows the Rust direction; an empty server result removes old flows;
late reads cannot revive them; Rust rejects inconsistent endpoints. Native CI,
visual acceptance and independent review are recorded separately. UI editing is
the next leaf. Runtime flow execution remains separate.
