# Windows installed application delivery (C22.signed-release)

This pipeline follows the installer and startup-updater changes. It builds an
updater-signed Windows x64 installer from tested main commits and publishes its
manifest on GitHub Releases. Automatic agent delegation remains disabled; no
Step 7 qualification or product-completeness claim follows from packaging.

## Everyday use after activation

1. Download the `*-setup.exe` from the latest published GitHub Release and install
   it once. Use the installed Start-menu application thereafter.
2. Codex prepares bounded changes in GitHub PRs. An independent review and the
   owner's merge remain required. This workflow never merges a PR.
3. Successful main validation produces a newer signed release automatically.
4. On the next application launch, choose **Install update and restart**, or
   **Open application** to use the current version. Offline use remains available.

No local source checkout, Rust, Node, Git pull, or PowerShell command is needed for
ordinary installed use. The installed desktop program still runs on Windows.
Save and close modeling sessions before updating; installation happens before the
modeler becomes accessible. Development workflow artifacts are not the production
update channel and may have no embedded public key.

## One-time owner activation, after independent review and merge

The release flag is initially absent/disabled. PR validation uses a disposable
signing identity, never production secrets, and never uploads its test package.

In the repository's **browser GitHub Codespace terminal**, on the reviewed main
checkout, run:

```bash
bash scripts/setup_release_signing.sh
```

The script refuses non-Codespace use, checks the approved repository, generates a
signing identity in a private temporary directory, and sends its private key
directly to the repository Actions secret `TAURI_SIGNING_PRIVATE_KEY`. Its public
half becomes the repository Actions variable `SMP_UPDATER_PUBLIC_KEY`. Existing
signing entries are not replaced. GitHub CLI must already have repository secret
and variable management access. An access error is a blocker, not a reason to put
credentials into chat or source control. The generated identity has no password;
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD` must be absent/empty for this identity.

Preserve the private-key backup from the printed remote directory in your approved
credential store before deleting the Codespace. Future installed versions trust
this key; generating a different key would break their upgrade path. Never publish
the key, include it in an artifact, or paste it into a task/chat.

After preserving the backup, enable release publication in that same terminal:

```bash
gh variable set SMP_RELEASES_ENABLED --repo rbrundage5/Systems-Modeler-Pro --body true
```

For the first release, open GitHub **Actions → windows-release → Run workflow**,
select `main`, and enter the numeric run ID of the successful
`native-foundation-ci` **main push** run for the current main SHA. The run ID is
the number after `/actions/runs/` in that run's URL. PR runs are rejected. A
superseded SHA is skipped. Subsequent successful main pushes trigger this
automatically while the flag remains true.

The owner's credential setup, merges and first installed upgrade are not
completed merely by adding this workflow. Do not discard an old checkout or local
model files until saved models have been reopened successfully in the installed
version and any unpushed work has been preserved.

## Publication and failure behavior

- The native workflow must succeed on this repository's main push. The planner
  also requires `core`, `desktop-check`, `desktop-linux-check`, `configuration`,
  `startup-update`, `release-contract` and `signed-package` to succeed. Missing,
  pending, failed or untrusted check results cannot authorize publication.
- Versions derive monotonically from the native workflow run number, starting
  in `0.2.*`, with Windows component limits. A later/equal published version or
  newer main SHA skips obsolete work. An unrelated latest-release tag blocks the
  channel rather than silently replacing it.
- The Windows builder uses pinned Tauri CLI 2.8.4, the committed Cargo lockfile,
  a temporary version override, and the public key compiled into the application.
  Tauri signs the generated NSIS installer. A Rust helper using the updater's
  verifier validates the signature and rejects modified bytes and another key.
  The installer is then silently installed and its application process launched.
- Publication rechecks the source and CI. It creates a draft at the exact tested
  SHA, uploads the installer, signature, BUILD_INFO.json and latest.json, then
  rechecks before publishing. An interrupted upload leaves an unpublished draft;
  existing tags and partial releases are never overwritten. Diagnose a partial
  draft explicitly before a retry; no automatic deletion or replacement occurs.
- `latest.json` names only the signed Windows x64 installer in this repository's
  release channel. BUILD_INFO records source SHA, version, SHA-256 and signature
  verification. The updater verifies downloaded bytes before installation.
- Setting `SMP_RELEASES_ENABLED=false` stops new publication. Existing installed
  versions and already-published downloads continue to work. It does not revoke
  an existing release or uninstall an application.

Updater signatures establish the update trust relationship. They are separate
from Windows Authenticode publisher signing. SmartScreen reputation and an
Authenticode signing service have not been qualified by this work.

## Required evidence and scope

Baseline: PR85 candidate `af7f26694520281e5cb874ad2c7a610d0aad7c1f`.
Scope: windows-release workflow; release policy, policy tests, Windows build and
owner signing scripts; signature verifier example and its Cargo dev dependencies;
this guide. No modeling, persistence, history, renderer or agent changes.

CI tests actual package signing, positive/negative signature verification,
installation and process startup. Policy tests use a fake GitHub API. These do
not establish rendered startup UX, an installed native A-to-B update, offline or
bad-signature behavior through the native UI, saved-model reopen after upgrade,
or installer recovery after OS handoff. Those checks and independent review are
still required before calling the installed update path fully qualified.

Sources: [Tauri Windows installer](https://v2.tauri.app/distribute/windows-installer/),
[Tauri updater](https://v2.tauri.app/plugin/updater/), and
[GitHub release API](https://docs.github.com/en/rest/releases/releases).
