# C22.running-app-installer-guard

Baseline: PR86 candidate `be21d55e9a5b76c16223017a625d52fa026f3469`.
This is one installer-safety finding, separate from release publication.

## Finding

The pinned Tauri CLI 2.8.4 NSIS `CheckIfAppIsRunning` macro force-closes an
application in passive or silent installation mode. A startup update gate in one
process does not protect a different process with unsaved modeling work. Its
existence must not authorize that second process to be closed.

Evidence: upstream `crates/tauri-bundler/src/bundle/windows/nsis/utils.nsh`,
CheckIfAppIsRunning macro; installer.nsi includes that macro for installation and
uninstallation. It includes utils.nsh before the configured installerHooks file.

## Change

The small configured NSIS include replaces that same macro with a refusal. It
never calls a process-kill function. A detected running copy results in nonzero
exit status and, in interactive/passive mode, an instruction to save and close all
copies before retrying. A compile-time error prevents silently losing this
protection if the pinned bundler's macro contract changes.

The macro keeps the bundler's current-user versus all-user process-detection
scope. It applies to generated installers and uninstallers. It cannot retrofit
an old uninstaller already installed without this protection: save and close all
old development copies before first migrating to the signed release channel.
An app starting after the process check remains subject to Windows executable
file locking; this macro does not intentionally terminate that new process.

## Validation boundary

The signed-package job installs and launches a copy, submits a second installer
with `/S /UPDATE`, requires refusal, checks the original process is still alive
and its executable unchanged, then closes only its own test process and requires
the update retry to succeed. This exercises the actual generated NSIS installer.

The test does not simulate authored models or qualify native UI save/reopen,
updater download/handoff, racing app launches, or old-version uninstall migration.
Production publication remains disabled pending the independent reviews and
native installed upgrade qualification described in WINDOWS_RELEASES.md.

Scope: Tauri NSIS hook registration, this macro, the existing signed-package
regression, and this evidence record. No modeling or agent configuration changes.

Source: [pinned Tauri NSIS utility](https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.8.4/crates/tauri-bundler/src/bundle/windows/nsis/utils.nsh).
