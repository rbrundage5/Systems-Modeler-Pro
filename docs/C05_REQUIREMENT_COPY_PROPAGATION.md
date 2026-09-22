# C05.04.02 — transitive requirement Copy text propagation

Baseline: PR123, b8045aa775ed19ffd133ad2c6d84f3d7fad34e8d. Main remains
2b9750e8b3ca6de76f86ec7f367f155513063862. Lead direct implementation; no workers.

Approved evidence: SysML 1.6, formal-19-11-01, 16.3.2.2 (PDF page 217),
defines client text as a read-only copy of its supplier's text. Current
`update_requirement` updates only immediate clients. Creating a Copy relationship
likewise updates only its direct client, leaving existing downstream copies stale.

Allowed paths: crates/model-core/src/model.rs;
crates/model-core/tests/pr61_copy_reimport.rs; this record. One leaf covers core
Copy-text propagation for master updates and new Copy relationships. Existing
requirements command checkpoint/rejection handling is a separate next leaf.

Plan the affected set before mutation using bounded graph traversal; validate every
affected requirement and check suppliers outside that set for conflicting text.
Apply text to all copies without changing their IDs, names or documentation. Keep
identical copied-row reimport legal and genuine copied edits read-only. No new
cycle policy or automatic restructuring of the Copy graph is introduced.

Acceptance: chains and branches, late attachment of an existing copy subtree,
stable metadata, master-then-copy reimport, conflicting supplier rejection without
mutation, and malformed downstream reference rejection before changing the master.
Global validation of historical Copy graphs and traceability reconnection remain
separate follow-ups; no full requirement lifecycle qualification is claimed.
