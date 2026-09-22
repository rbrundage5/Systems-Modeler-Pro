# Remove superseded behavior patch assets

Leaf `C01.EXEC.06`, baseline `848b9e9650627ff819cea3d2e2cff382fa77d3af`.
Allowed production changes are deletion of these three unused assets only:

- `apps/desktop/frontend/behavior-runtime-hardening.js`
- `apps/desktop/frontend/behavior-safe-transition.js`
- `apps/desktop/frontend/behavior-nested-transition-notation.js`

Neither application HTML entrypoint loads these scripts. The existing Behavior
integration validator explicitly forbids loading them. They contain superseded
JavaScript behavior/routing patches while the active application uses Rust
commands, the authoritative renderer and transition-presentation adapter. Tauri
bundles the frontend directory, so leaving unused patch files there unnecessarily
retains executable assets even though no active caller was found.

This candidate deletes the assets; it does not reactivate them, remove supported
behavior semantics, or replace existing controllers. The source-marker guard
against loading the old paths remains unchanged. No dependency, agent control,
HTML entrypoint or active frontend file is edited.

Companion setup-maintenance scope `C01.EXEC.06-CI`: remove only the deleted
`behavior-runtime-hardening.js` entry from the explicit syntax-check list in
`.github/workflows/ci.yml`. CI run `35749063286` reproduced the stale-file failure
after removal. No job, permission, trigger, active-file check or integration guard
is removed or relaxed. This maintenance is necessary for the same asset retirement;
workers remain disabled. The negative case is a syntax error in any retained file:
the unchanged `node --check` invocation must still fail.

Verification: both entrypoint asset graphs resolve after deletion; all retained
frontend files are byte-identical to baseline; no executable loader refers to the
removed files. Existing Behavior integration and Rust-authority validators pass,
and diff checks pass. These prove the bounded source/bundle cleanup, not complete
native behavior or SysML qualification. Independent review is outstanding; no
workers were dispatched and merge is not automatic.
