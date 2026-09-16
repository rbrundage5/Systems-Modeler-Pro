# C20.06b.02 — shared BDD element creation

Baseline: `82e7fecdecca2f172758d6197a4295f15634daa3` (PR105).
One workflow: two authenticated editors create the existing top-level BDD element
kinds and observe stable semantic identity, place those elements on the shared BDD,
recover an uncertain creation and reverse their own creation safely.

Allowed production paths: model-core structural_presentation.rs and the new
creation module; desktop workspace/bdd_elements.rs; persistence collaboration.rs;
frontend collaboration-ui.js. Supporting paths: core/persistence creation tests,
existing HTTP client integration tests, frontend regression script and this document.
No other active writer; worker delegation remains disabled.

Reuse Project::create_element through a typed core command for native and shared
creation. Requirement authoring retains its existing ID/text command. Owned features,
ports and constraints on usages retain their specialized creation contracts and are
not accepted by this classifier command. Shared validation, role checks, revision,
exact retry receipt and inverse capture remain in the existing transaction.

Advertise shared-bdd-elements-v1 so the current client fails clearly on older
servers. Keep the existing Block/TestCase wire variants valid. New UI selections
capture kind/owner/name and preserve the draft on rejection; restored outbox records
restore the selected kind and name. No protocol-version change or schema migration.

Acceptance: every enum kind creates with the native semantic kind and valid owner;
invalid owner/kind requests preserve state/revision/history; two real HTTP sessions
converge; viewer and stale writes reject; exact retry after reopen stays single;
reversal removes the created record when no later dependency exists. Specialized
features remain rejected. Existing requirements and all-nine offline tests remain
required. Independent review and rendered two-device acceptance are outstanding.

This supplies creation only. Complete feature/property/relationship editing,
ordinary-workspace remote session integration and all-nine collaboration remain
open. Do not merge automatically.
