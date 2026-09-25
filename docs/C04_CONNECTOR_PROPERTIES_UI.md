# C04.08.01b — usable connector Properties form

Dependency: PR116, the atomic connector specification API. Main remains
2b9750e8b3ca6de76f86ec7f367f155513063862 until user-reviewed merges.

Expose name, assembly/delegation kind and qualified endpoint choices in the normal
IBD Properties panel. One Apply invokes the Rust transaction. Retain rejected
values and actionable errors; disable duplicate submission; ignore stale endpoint
responses after selection changes. Reload Saved discards only the draft. Preserve
Route IBD and delegation to existing element Properties.

Allowed paths: apps/desktop/frontend/connector-properties.js, ibd-ui.js and
index.html; scripts/test_connector_properties.cjs and its existing regression
entry point; this record. Lead direct implementation; no agents or independent
review are claimed. No frontend semantic validation, routing or checkpoint is added.

Acceptance: exact one-command Apply, qualified/stable choices, rejected draft,
loading failure and retry, late-response isolation, repeated Apply suppression.
Native Windows pointer/keyboard/visual acceptance and independent review remain
required. Existing ItemFlow editing and association typing are separate work.
