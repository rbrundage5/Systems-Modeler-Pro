# Startup updates (C22.startup-updater)

This candidate depends on the Windows installer PR. Automatic release publication
and production signing activation are separate dependent work. No agent delegation
is enabled. No keys are committed.

## User workflow

Signed Windows release builds check for updates in a separate startup window.
Choose **Install update and restart** or **Open application**. Offline/error/no-update
results never prevent opening the existing version. During downloading/verifying/
installation, opening the modeler is disabled. Updates are not installed during a
modeling or collaboration session: save your work and restart normally to update.

The modeler window starts hidden. Once opened, Rust permanently closes the update
gate for that process, including when a slow check finishes afterward. No save,
model, history or collaboration commands are issued by the update screen.
Development builds and builds without a configured public key open directly.

## Integrity and configuration

Rust owns update metadata and downloaded bytes. Only the local updater window can
call the three custom startup commands. The frontend supplies no URL, key, binary
or path. No plugin IPC permissions are granted. Checks use the fixed repository
HTTPS latest.json endpoint and accept only this repository's Windows release
installer URLs; the updater plugin validates signatures before installation.
Default newer-version comparison is retained.

The plugin is pinned to 2.11.0. A release builder supplies the public key through
SMP_UPDATER_PUBLIC_KEY at compile time; build.rs tracks changes to that variable.
The corresponding private key is exclusively a release-workflow secret. A build
without a public key is explicitly not an update-enabled production build.

The Windows plugin's passive installer handles process exit and restart. Download,
signature and pre-launch installation errors retain the existing application.
OS installer failure after process handoff remains an external failure requiring
the existing installer/release recovery route.

## Scope and evidence

Baseline: installer candidate 6e7ae953b33b40d42792e3cbd94bc077b678e90d.
Allowed application paths: src-tauri/src/app_updates.rs and app_updates/gate.rs,
src-tauri/src/main.rs registration, src-tauri/Cargo.toml, src-tauri/build.rs,
src-tauri/tauri.conf.json; frontend/updater.html and updater.js. Supporting changes:
Cargo.lock, scripts/test_startup_updater.cjs, desktop-update-checks workflow,
this document. No shared model, save, history, renderer or collaboration changes.

Focused tests cover open-vs-check races, exclusive install, missing update,
retry/failure transitions, caller-window rejection, release URL rejection and the
production UI controller. Controller tests simulate DOM controls and Rust results;
they do not claim cryptographic or rendered desktop execution.

Required before activation: exact-candidate independent review; native Windows/Linux
CI; signed release A-to-B installation with a real installed copy; offline and
invalid-signature behavior through the native updater; saved-model reopen after
upgrade. The updater must not be declared qualified from parsing or simulated tests.

Sources: official Tauri updater documentation and published
tauri-apps/plugins-workspace tag updater-v2.11.0, updater implementation/config.
