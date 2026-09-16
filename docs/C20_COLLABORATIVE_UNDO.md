# C20.05 — Actor-scoped shared change reversal

Baseline: `2b590bfb605411d17c850b9f0d296345f505185d` (PR101 candidate).
One workflow: inspect recent changes made by the authenticated account and reverse
a selected change while preserving other users' unrelated committed work.

The server records a typed inverse delta for elements, relationships and BDD
diagrams in the same transaction as each operation and its retry receipt. Only the
authenticated author may reverse an operation. A reversal requires the affected
records to match their original after-state; edits touching those records cause
an explicit conflict. Unrelated changes remain intact. All core semantics and
shared diagram invariants are validated before commit, so new dependencies can
also block reversal. A failed reversal changes neither state nor history.

Reversal is a new revision and is itself recorded. Reversing that reversal restores
the prior change if the same conflict/dependency checks pass. The selected target
can be reversed only once; the usual exact operation ID supports safe retries.
History before inverse capture was introduced remains visible but unavailable for
reversal. History is limited to the most recent 50 operations by the current actor.
Diagram conflicts are deliberately conservative at whole-diagram granularity.

Allowed production paths: persistence collaboration authority and a dedicated typed
undo module; authenticated server history endpoint/diagnostic; desktop history
command and registration; Shared Projects history controls. Supporting paths:
persistence/server/client/frontend tests and collaboration documents. The existing
collaboration migration adds one history table; native project payloads remain
compatible. No dependencies, CI/agent configuration or offline history changes.

Acceptance: two actors make disjoint changes; one reverses their own earlier change
without losing the other's. Same-object modification or a new dependency blocks
reversal atomically. Another actor/viewer cannot reverse the operation. Reopen and
retry preserve identity, and semantic plus diagram changes reverse together. Native
CI, independent review and actual two-device acceptance remain required.
