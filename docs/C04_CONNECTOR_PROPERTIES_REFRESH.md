# C04.08.01c — connector Properties after history refresh

Baseline: 5431a6b9580acd057ee5c0d518da6a280b369f63 (PR120).
Reproduction: keep a connector selected, update the saved snapshot through undo,
then rerender. The form retained its previous clean name/kind/endpoints because
its cached read was keyed only by selection. The new regression fails with
`original` instead of `Restored` before the repair.

Allowed paths: frontend/connector-properties.js;
scripts/test_connector_properties.cjs; this record. Invalidate a clean cached form
when its saved relationship or diagram changes; retain user drafts and pending
submissions. No semantic mutation or history changes.

Acceptance: restored name/kind/endpoints and a fresh Rust read; an unsaved draft
survives rerender; all original form regressions continue to pass. Independent
review and native visual acceptance remain open; lead direct implementation.
