# C04.07.02 — reject inconsistent nested connector roles

Baseline: PR121, 28bf2bed5c91e2634e16ea1e75ecf5adef1dc459. Main remains
2b9750e8b3ca6de76f86ec7f367f155513063862. Lead direct work; no workers.

Finding: `Project::validate_connector_end` verifies a nested port's owner against
the resolved property path but does not verify that `role_id` is the path's final
property. A forged end can resolve through part A while claiming part B as its
role. This contradicts the existing `ConnectorEnd::nested_port` representation
and permits inconsistent authored occurrence identity.

Allowed paths: crates/model-core/src/ibd.rs;
crates/model-core/tests/pr11_ibd.rs; this record. Reuse the existing
InvalidConnectorPath diagnostic; no new schema, frontend or runtime subsystem.

Acceptance: valid nested and boundary ends still pass; mismatched roles
reject; create/edit reject without modifying the project; deserialized corrupted
projects fail validation. Existing connector/ItemFlow and desktop regressions
remain required. Independent review/native acceptance are still outstanding.

This P1 validation repair precedes additional connector association-typing work.
It does not claim AssociationBlock-typed connectors or complete port compatibility.
