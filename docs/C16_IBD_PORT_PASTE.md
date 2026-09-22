# C16.08.01 — IBD parent/port paste identity

Baseline main: `2b9750e8b3ca6de76f86ec7f367f155513063862`.
Dependency baseline: `55e6a635c06f0bd42f168f96194c713f73c40c90` (#111).

## Work order and finding

Authorized pre-package editing audit. Direct lead implementation; workers remain
disabled by AGENTS.md. No independent review is claimed.
Allowed production/test path: apps/desktop/src-tauri/src/workspace/standard_editing.rs.
This record is the only documentation change. No workflow or control changes.

Copying a property carries all its port presentations. When the same selection also
explicitly contains one of those ports, paste previously copied it again onto the
original property and overwrote the connector endpoint mapping. The diagram could
remain semantically valid while visually connecting the wrong occurrence.

Preserve the child presentation mapping created while copying the parent. Skip the
redundant explicit child copy. Properties are processed before ports regardless of
selection order. Paste reuses semantic identity; it does not create a new model port.

Acceptance: select a connected property, its port and connector in either order,
paste, retain one child on the copied property, leave the original unchanged, and
connect the copied connector to the copied child. Semantic state must be unchanged.
The regression covers both orders and validates the resulting snapshot.

## Qualification and remaining scope

Rust is unavailable locally; new regression execution requires native CI. Independent
review and native Windows clipboard interaction remain required. No CATIA parity or
release-complete assertion. Vendor-specific clipboard behavior is not established
by this fix. Existing snapshot validation/commit and history paths are unchanged.

Separate findings remain: standalone port paste boundary projection, semantic
Duplicate child identity/path handling, connector/ItemFlow configuration, sequence
execution, interchange fidelity and all-nine-family rendered acceptance. These are
not waived by this narrow correction. Next: native CI and independent review, then
standalone port paste with the actual visible context-frame contract.
