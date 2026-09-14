# Windows installation and update delivery

Status: installer build candidate. No published release or in-app updater is
established by this change. Agent delegation remains disabled.

## Install without PowerShell

After this change is reviewed and merged, successful native-foundation-ci runs
on main trigger the windows-installer workflow for the exact tested commit.
The workflow can also be started manually from Actions. Pull requests affecting
desktop/build inputs produce development installers for review.

1. Open this repository's Actions tab and select **windows-installer**.
2. Open a successful run for the intended main commit.
3. Download the **systems-modeler-pro-windows-x64-...** artifact and extract it.
4. Check BUILD_INFO.json identifies the intended commit, version and installer.
5. Run the included **-setup.exe**, then open Systems Modeler Pro from Windows.

The app runs locally. Rust, Node, PowerShell and a source checkout are not needed
for ordinary use. The installer uses Tauri's normal WebView2 prerequisite handling;
first installation may need Internet access. This initial artifact is not
Authenticode-signed: Windows or organizational policy may warn or block it.
Do not disable security policy to install it. Code signing is a separate release
credential decision. Never treat a SHA-256 digest alone as publisher authentication.

CI installs and launches the package only in a disposable GitHub Windows runner,
using a runner-temporary installation directory. It checks successful installation
and that the process remains alive for 15 seconds. This does not verify rendering,
editing, persistence or a complete native user journey. No user computer is scanned
or modified by the workflow.

## Current build contract (C22.windows-installer)

Baseline: 9383c9e3f8dd6f925cc4706f172606ca6dac97a3.

Allowed changes: .github/workflows/windows-installer.yml,
apps/desktop/src-tauri/src/main.rs (release Windows console suppression only),
this document, and the docs/README.md index entry.

The Tauri CLI is pinned to 2.8.4. It builds x86_64-pc-windows-msvc NSIS with
Cargo --locked and automatically enables tauri/custom-protocol to embed and serve
the existing frontend. Existing development behavior, app identifier, model schema,
dependencies, Cargo.lock and native-foundation-ci are unchanged. Release Windows
builds use the Windows GUI subsystem so ordinary launch does not open a console.

The workflow has contents:read only, does not persist checkout credentials, uses
no signing secrets, and creates no Git tags, releases, commits or merges.
A workflow_run build accepts successful main push CI from this repository only
and checks out that run's head SHA. PR/manual builds are development artifacts,
not evidence that all regression checks passed.

Acceptance: CI produces exactly one nonempty Windows executable installer;
Cargo.lock remains clean; recorded SHA and SHA-256 match the artifact; installation
succeeds and the installed executable stays running during the smoke check.
Failure: build, changed lockfile, missing/extra installer, failed install or early
application exit fails the job and prevents artifact upload.

Before distributing a release, independently review the exact candidate and verify
the installed application opens, edits, saves and reopens a disposable test model.
Compare all nine diagram families with existing qualified behavior. Packaging
success does not establish full product or Step 7 qualification.

## Dependent work: signed in-app updates

The user's requested final workflow remains: reviewed merge, successful CI,
versioned signed release, then an in-app **Update and restart** action.
Installer artifacts alone do not update previously installed copies.

Prepare this as a separate dependent implementation after installer validation:

- Rust-backed update checking and installation using Tauri's updater plugin.
- A stable public update endpoint for this repository's published Windows releases.
- Monotonically increasing application versions; exact tested-commit provenance;
  serialized publication that cannot replace a newer release with an older build.
- Signed updater artifacts. Embed the owner's public key in the application;
  private signing material belongs only in GitHub Actions secrets, never Git,
  browser frontend code, agent prompts, artifacts or logs.
- No secret-bearing release job on pull-request code. Failed CI or signing must
  prevent publication. Re-running a release must not silently overwrite assets.
- A small update UI with available/no-update/offline/error states. Installation
  requires explicit user action and an integrated save/cancel boundary so unsaved
  models are not lost. Do not force a restart during editing or shared sessions.
- Tests for no update, invalid signature, download failure, canceled installation,
  newer-version ordering, and save failure; qualify update from installed version A
  to version B with a saved project preserved.
- Configure release publication separately from automatic merging; agents never
  gain permission to merge because the release workflow exists.

Updater signing and Windows Authenticode signing serve different purposes.
The first is required by Tauri's updater; the second identifies the Windows
publisher. Do not use a disposable test key as the production updater identity.

Until the updater is qualified, installing a newer development artifact is manual.
Back up models before testing upgrades; data compatibility must be verified rather
than inferred from unchanged installation paths.

## Evidence and sources

The initial GitHub inspection found no releases, no distribution workflow, no
updater plugin dependency, and no updater configuration at the recorded baseline.
Open PR79, PR81 and PR83 touch agent setup paths and do not overlap this change.
All four main checks were successful when the work started.

Official implementation references consulted for this user-authorized packaging work:
- https://v2.tauri.app/distribute/windows-installer/
- https://v2.tauri.app/distribute/pipelines/github/
- https://v2.tauri.app/plugin/updater/
- Tauri CLI tag tauri-cli-v2.8.4, crates/tauri-cli/src/interface/rust.rs:
  build_options enables tauri/custom-protocol.

Independent review, actual Windows build and installation results must be recorded
on the candidate PR. A locally parsed workflow is not a passing Windows build.
