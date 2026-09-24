# REL-COMP-01C / REL-EDIT-01: linked property endpoint editing

Depends on PR159 and PR157. Main baseline: 596e24f.
Direct primary-session implementation; independent review remains outstanding.
Allowed paths: desktop `workspace/relationship_editing.rs`,
`workspace/history.rs`, `workspace/standard_editing.rs`. Tests in the same modules.

One semantic workflow: preserve linked Property identity when changing and
copying a composition's classifier endpoints. Reconnect changes the existing
Property's owner/type, validates the candidate, and prepares every affected BDD
edge before publication. The selected BDD still requires presented endpoints;
other affected views add a missing endpoint presentation to the right of their
existing nodes. Existing nodes and unrelated edges are preserved. Routes and
attached label anchors are recomputed. Any route or dependent IBD validation
failure rejects the whole edit. Reconnect no longer updates only one view.

The established structural Properties transaction stages the same affected-view
updates for retype operations and includes them in its single undo/redo snapshot.
Duplicate creates or maps a separate Property identity and remaps its owner/type;
it never reuses the original composition's Property in a second association.

Acceptance: reconnect across two diagrams, including an absent replacement in
the second view; verify property/end/relationship IDs remain stable and every
edge matches semantics. Reject invalid secondary-view geometry without changing
any state. Retype a linked property through Properties, then undo/redo the full
model and views. Duplicate the association and verify distinct property identity.

Remaining: full association/connector semantic deletion cascade, explicit legacy
linking and workbook/XMI identity adapter roundtrips, rendered acceptance and
runtime lifetime qualification. No general feature-completeness claim.
