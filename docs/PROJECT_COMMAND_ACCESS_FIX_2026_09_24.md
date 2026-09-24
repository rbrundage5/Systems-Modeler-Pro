# Development project command access regression

Leaf: C21.IPC.DEV.01, P1 broken application startup/project workflows.
Baseline: `35b326b1e6da2ac9a98f8faf00198750e7cbcc60` (main after PR174).
Work order: restore New/Open and related project commands after the reported
"This document is not permitted to invoke that application command" rejection.
The user confirmed launching from a PowerShell/source checkout.

## Cause and scope

`native_access::guard` checked the current native webview URL against packaged
Tauri entrypoints only. A main window at `http://localhost:1430/` or the CLI's
configured loopback address therefore rejected every application command before
dispatch, including `new_project` and `open_project_file_complete`. This is a
command-access failure; it occurs before a selected project file is loaded.

The repository config specifies `frontendDist` without `devUrl`. Tauri's CLI can
serve that directory from its built-in development HTTP server and inject the
effective `build.devUrl`. The guard omitted that launch mode.

Production scope is only `apps/desktop/src-tauri/src/native_access.rs`; its tests,
this record and the documentation index record the same leaf. No model schema,
project contents, semantic validation, persistence, dependencies, CI controls or
agent restrictions change. Existing open historical PRs do not write this module.
Work is primary-session implementation; worker dispatch remains disabled and
independent review is outstanding.

## Correction

The guard reads the effective development URL from native Tauri configuration and
uses `tauri::is_dev()` to distinguish development from packaged execution. The
main entrypoint at that exact configured loopback HTTP(S) URL, including its
`index.html` form, may invoke existing modeling commands. The port, host, scheme
and configured path must match; no arbitrary localhost origin is trusted.

Packaged entrypoint behavior and main/updater command separation are preserved.
Packaged execution never gains development-origin access, even if a development
URL remains in configuration. Unknown windows, different ports/hosts/schemes,
other pages, query/fragment variants, remote origins, credential-bearing URLs and
file/data documents remain denied. The development URL cannot be supplied in an
IPC argument. Updater commands remain restricted to their bundled entrypoint.

## Verification and acceptance

Executed locally:

- `node --test scripts/test_*.cjs`: 168 passed, zero failed, including New/Open/
  Save As success, cancellation and rejection outcomes. These mock native IPC.
- Rust-authority, history integration and workspace convergence checks: passed.
- `git diff --check`: passed.

Five additional Rust regressions exercise configured IPv4/IPv6/localhost
development entrypoints, project command access, build/config requirements,
origin/document rejection, unsafe configuration, and main/updater isolation.
Existing packaged-main tests now explicitly cover New, complete Open/Save and
snapshot commands. Native CI results and the exact candidate commit are recorded
in the PR; Rust is not installed in the primary hosted scratch runtime.

Native acceptance still required on the user's launch environment:

1. Update to the reviewed candidate and restart with the existing development
   launch command. Confirm the workspace initializes without permission errors.
2. Create a disposable project, author a Block and diagram, Save As, create a
   second project, and reopen the first. Confirm identity/content survive.
3. Open a copy of a previous native project, edit and save/reopen it. Cancel New,
   Open and Save As; attempt a malformed fixture. Confirm existing state survives.
4. Verify an unrelated local server, another page and an unknown window cannot
   invoke project commands; main cannot invoke startup updater commands.

CI and source tests do not establish a rendered Windows acceptance result or
whole-tool completion. This candidate is not automatically merged. Next step:
review the exact PR after native checks pass, merge when approved, update the
source checkout and restart. Existing running processes do not load changed Rust
code until rebuilt/restarted.

Official framework references used to resolve this launch-mode behavior:

- [Tauri BuildConfig](https://docs.rs/tauri-utils/latest/tauri_utils/config/struct.BuildConfig.html):
  built-in dev server when devUrl is absent and frontendDist is a directory.
- [Tauri is_dev](https://docs.rs/tauri/latest/tauri/fn.is_dev.html): framework
  development-mode selection, rather than treating debug assertions as origin
  authority.
