# C20.02 — Actionable shared requirement rejection

Baseline: `7f548b82af7d8f6cf9ade780b4224de2a5185278` (PR97 candidate).
Finding: the HTTP adapter reduced every semantic rejection to the same generic
message. An editor could not distinguish a duplicate Requirement ID from a copied
requirement's protected text. Oversized edits incorrectly suggested retrying a
pending operation even though HTTP 413 was treated as a definite rejection.

This leaf adds stable, bounded diagnostic codes for blank/duplicate Requirement
IDs, copied requirement protection, and invalid names. The Rust client maps only
known codes to clear remedies. Arbitrary server messages, storage errors, internal
paths and credentials are never displayed. Error response decoding is capped at
1024 bytes; malformed, oversized, unknown or legacy responses use the existing
safe message. HTTP 413 names the existing 16 KiB body limit and the remedy.

Allowed paths: server `src/lib.rs` and `tests/server.rs`; desktop
`src/collaboration.rs`; this record. This increment changes no semantic validation,
authorization, transaction, retry identity, UI authority, dependency or CI gate.
Rejected operations remain definite failures: pending state is cleared, refresh
is required, and no duplicate POST or automatic rebase is introduced.

Acceptance: actual HTTP duplicate/blank ID requests return the expected code and
leave the snapshot unchanged. Rust client transport tests verify remedies, pending
state, unchanged revision, single-request behavior and fallback for untrusted
diagnostics. Native CI must qualify the exact candidate. Independent review and
native two-device acceptance remain outstanding; this does not complete broader
collaboration scope.
