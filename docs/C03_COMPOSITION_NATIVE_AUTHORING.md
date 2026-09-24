# REL-COMP-01B: native BDD composition authoring

Depends on REL-COMP-01A / PR157. Baseline main: 596e24f.
Direct primary-session work; independent review remains outstanding.

Allowed paths: desktop `workspace.rs`, `workspace/bdd_elements.rs`,
`workspace/relationship_editing.rs`, `workspace/ibd.rs`, and frontend `app.js`.
Native tests are colocated with these commands. The IBD helper becomes visible
inside the workspace solely to exercise the actual population workflow.

Both native BDD creation entry points share one implementation. Drawing a
composition creates a distinct whole-owned PartProperty typed by the target.
Names are generated from the target and made unique within the whole; users can
rename the same stable usage through Properties or the association-end editor.
Repeated usages are legal and receive separate route lanes. Routing is staged
before either semantic record is created. Native snapshots give the renderer the
diamond side; JS only applies that rendering instruction.

End edits update the authoritative property, including part/reference kind when
aggregation changes, before validating and publishing the candidate. Invalid
navigation, whole multiplicity, type or aggregation changes reject atomically.
Legacy unlinked associations retain their prior editing behavior.

Acceptance: create Vehicle-to-Wheel twice; the repository owns two independent
usages, the BDD labels/diamond match those usages, and IBD population uses the same
Property IDs. Rejected routing creates neither Property nor association. End
editing preserves identities and rejected edits preserve the exact prior model.
Existing property inspector/history and complete Save/Open commands are reused.

Remaining dependency leaves: cross-view endpoint mutation and copy/duplicate,
explicit legacy linking, workbook/XMI identity roundtrips, rendered interaction
and full runtime lifetime acceptance. This PR does not close those leaves.
