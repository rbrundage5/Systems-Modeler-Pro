# Frontend content policy

Work leaf `C21.EXEC.02`: restrict executable and network content in the two
desktop webviews. Baseline `848b9e9650627ff819cea3d2e2cff382fa77d3af`.
Allowed production paths: `apps/desktop/src-tauri/tauri.conf.json` and the updater
HTML/CSS needed to preserve its display. Tests/documentation support this same
contract. No workers or independent reviewer were used; merge is not automatic.

The Tauri content-security policy now defaults to deny. Scripts are restricted
to bundled same-origin assets; no inline/eval/data/blob/remote script permission
is granted. Browser connections are restricted to the Tauri IPC transports.
Objects, frames, workers, forms and base-URL changes are denied. Images are local
or generated data/blob images, and fonts are local. Rust still owns the intended
updater and authenticated collaboration HTTP requests.

Dynamic inline styles remain permitted because the renderer sets diagram
geometry and injects model presentation styles. That is a style permission, not
permission to execute script. Updater CSS is moved to a local asset so static
inline-style hashing does not interfere with the dynamic-style contract. Tauri's
automatic IPC script hash/nonce handling remains enabled.

Three desktop-crate regression tests check policy boundaries and both entrypoint
asset sets. Local verification checks JSON, asset paths, unchanged updater CSS,
the existing six updater controller tests and whitespace. Rust compilation/tests
are delegated to the existing CI toolchain; no local compiler is available.
Native Windows/Linux rendering and attempted blocked script/network loads still
require installed-webview acceptance; config/source checks alone are not that
evidence. Preserve palette, move/resize, labels, SVG markers, dialogs, upload/
download and startup Update/Open while confirming hostile inline/remote content
is rejected. This leaf is not a substitute for native command/file authorization.
