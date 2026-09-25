# Native application command boundary

Leaf `C21.EXEC.03a`, baseline `848b9e9650627ff819cea3d2e2cff382fa77d3af`.
Scope: `src/native_access.rs` and wiring at the existing desktop custom IPC
dispatcher in `src/main.rs`, plus this evidence record. No alternate command
registry, model engine, frontend semantic authority or worker is introduced.

Before dispatching a custom application command, Rust checks the native webview
label and current top-level URL. Only bundled main/index documents can reach the
existing modeling command registry. Only the bundled updater document in the
updater window can request Check, Install or Open Application. Unknown windows,
unavailable URLs, remote/file/data/blob pages and lookalike origins fail closed.
The updater cannot invoke project/file/import/collaboration commands. Main cannot
invoke updater operations. Existing startup-phase checks remain authoritative
inside update handlers. The generated Tauri handler continues to reject unknown
application commands; this policy does not grant plugin permissions.

Four Rust policy tests cover legitimate platform URLs and command groups,
cross-window denial, unavailable/unknown contexts, and hostile document URLs.
The existing native CI compiles the real dispatcher wiring. No compiler is
available locally; native execution and installed-window navigation/IPC attempts
must be qualified in CI/desktop acceptance. Independent review remains open.

This complements the separate content-policy change that denies frames and
untrusted executable content. It checks the current native top-level document;
it is not an iframe-origin attestation, a sandbox for a compromised approved
page, or user-selection authorization for every file path. A future file-grant
leaf must replace untrusted raw path authority with native picker/session grants
without breaking native Open/Save/import/export. No claim of completing that
larger file workflow is made by this boundary.
