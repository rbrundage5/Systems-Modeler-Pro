# C04.NESTED.03 — existing connectors in expanded type usages

Depends on PR186 and PR187. Supplied SysML 1.6 §§8.3.2.12 and 9: nested
connector ends identify roles through ordered property paths. This repository's
ConnectorEnd path includes the terminal property, unlike the stereotype's external
encoding; preserve that established internal representation.

Allowed paths: ibd.rs, ibd_structure.rs, new ibd_projection.rs, connector_editing.rs,
main.rs module declaration, IbdConnectorPresentation constructor sites,
ibd-ui.js, focused regressions and this record.

An edge occurrence retains the semantic relationship ID plus its contextual
prefix. Existing type-owned connector ends are projected through that prefix;
expansion never creates semantic connectors. Missing/hidden endpoint symbols
never receive substitute endpoints. Reconnecting in a type-level IBD updates all
projected occurrences transactionally or rejects before publication if a required
replacement endpoint is absent. The contextual view directs editing to the type
level, with explicit scope text. ItemFlow identity/direction remains semantic.

Acceptance: repeated type usages display distinct edge occurrences, delegation
at the nested owning boundary, no duplicates on repeat expansion, saved paths,
shared reconnect propagation, collapse visibility, no semantic mutation. Native,
rendered and independent review gates remain separately recorded.
