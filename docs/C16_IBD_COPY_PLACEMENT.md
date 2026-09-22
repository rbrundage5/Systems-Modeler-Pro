# C16.08.04 — collision-aware IBD part copy placement

Main baseline: 2b9750e8b3ca6de76f86ec7f367f155513063862.
Dependency: PR114, a08e41af6a060bbbf2ddcc2768cc18a5ac663132.

## Reproduction and bounded work order

CI run 35634321182 reproduced a connected-copy failure: translating a 180x100
part and its left port by 28x28 placed the new endpoint inside the original part.
The router correctly rejected the path. This defect is separate from child ID
mapping. The identity tests now use exposed right-side ports to isolate that rule;
this leaf retains the original left-port geometry as a routing regression.

Choose one deterministic Rust translation for all selected part presentations in
Paste or Duplicate. Preserve the group's internal geometry, port attachment and
semantics. Try the conventional offset, then shift past conflicting existing
parts with routing clearance. If necessary, try a row below existing parts. Keep
placements inside an authored context frame and the finite canvas range. If no
candidate fits, reject with an enlarge-frame remedy before state/history changes.
This is bounded placement, not a new general layout engine or optimum packing.

Allowed paths: apps/desktop/src-tauri/src/workspace/standard_editing.rs and this
record. Lead works directly; no workers or independent review are claimed. No
frontend, persistence schema, routing rules or agent controls change.

Acceptance: copy and duplicate a connected left-port part; route to the new
endpoint without crossing the original; preserve selection-group displacement;
repeated pastes avoid existing parts; a crowded/small authored frame rejects
atomically. Existing routes/labels and dense-layout visual qualification remain
part of the wider routing audit; this leaf only chooses new part placements.

Qualification pending native CI. Native Windows and independent review remain
open. Next professional workflow gap: editing connector and ItemFlow specifications.
