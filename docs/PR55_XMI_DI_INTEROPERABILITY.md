# PR55 — XMI diagram interchange closure

PR55 completes the current SysML 1.x interchange milestone by adding a diagram/presentation layer to the PR54 semantic XMI pipeline. The semantic repository remains authoritative; imported diagrams are lowered into the existing native diagram stores and validated as one complete authored workspace before a single atomic commit.

## Qualified paths

- Native XMI export embeds the full portable authored state and emits deterministic presentation records. Import restores semantics, profiles, stereotypes, tagged values, and all nine native diagram families without replacing native routing, layout, execution, persistence, history, or editing systems.
- Presentation records carry stable source-bound diagram, node, and edge identities. Valid finite bounds, label anchors, and clear waypoint routes are preserved. Missing geometry receives deterministic placement; invalid or obstructed routes use the existing native router.
- Preview reports presentation CREATE, UPDATE, NO_CHANGE, REMOVE, and BLOCKED outcomes alongside semantic outcomes. Missing presentation endpoints and semantic references block the candidate. No live workspace state changes until the complete candidate validates.
- Authoritative reimport removes only presentation records previously owned by the same source namespace. Manually authored and differently sourced diagrams remain protected.
- Export is deterministic for unchanged authored state. Native export/import/export preserves the complete authored workspace, including BDD, IBD, Requirement, Use Case, Package, Activity, State Machine, Sequence, and Parametric diagrams.

## External producer boundary

The checked-in `examples/xmi/generic-uml-di.xmi` fixture is synthetic and producer-neutral. It qualifies generic diagram, node/shape, edge, bounds, waypoint, label-anchor, semantic-reference, and owner/context normalization for the shared BDD-shaped diagram stores: BDD, Requirement, Use Case, Package, and Parametric.

External IBD, Activity, State Machine, and Sequence presentation records are accepted losslessly through the native authored-state payload. A standalone producer-specific presentation import for those specialized stores is blocked rather than guessed.

No authentic CATIA/No Magic or Cameo/MagicDraw XMI fixture is available in the repository. Consequently:

- no vendor release, XMI dialect, or native project format is certified;
- recognized namespace-independent/local-name shapes are best-effort normalization, not a compatibility claim;
- unknown extensions are preserved where safe and required unresolved references are blocking diagnostics;
- release-specific qualification requires a legally usable genuine fixture, expected-output assertions, and the same atomic/reimport regression suite.

## Regression contract

The Core job enforces formatting, non-desktop workspace tests, Clippy with warnings denied, frontend syntax, all existing integration contracts, the XMI contract, and Rust-authority ceilings. The Windows Desktop job enforces desktop compilation, the full desktop test suite, Clippy with warnings denied, the same integration contracts, and a clean lockfile. PR55 does not weaken or bypass either job.
