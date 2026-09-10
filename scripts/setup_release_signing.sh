#!/usr/bin/env bash
# One-time owner action in a browser-based GitHub Codespace only.
set -euo pipefail
if [ "${CODESPACES:-}" != true ]; then
  echo "Run this only in the repository's GitHub Codespace terminal." >&2
  exit 2
fi
repo=rbrundage5/Systems-Modeler-Pro
repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"
case "$(git remote get-url origin)" in
  https://github.com/rbrundage5/Systems-Modeler-Pro|https://github.com/rbrundage5/Systems-Modeler-Pro.git) ;;
  *) echo "Unexpected repository origin." >&2; exit 2 ;;
esac
command -v gh >/dev/null
command -v npm >/dev/null
# Do not rotate a signing identity that installed applications already trust.
existing="$(gh secret list --repo "$repo" --json name --jq '.[].name')"
if printf '%s\n' "$existing" | grep -Fxq TAURI_SIGNING_PRIVATE_KEY; then
  if ! gh variable get SMP_UPDATER_PUBLIC_KEY --repo "$repo" >/dev/null; then
    echo "Private key exists but its public variable is missing or inaccessible. Restore the original .pub file; do not generate a replacement." >&2
    exit 2
  fi
  echo "Signing entries already exist. No key was generated or replaced."
  exit 0
fi
if printf '%s\n' "$existing" | grep -Fxq TAURI_SIGNING_PRIVATE_KEY_PASSWORD; then
  echo "A signing-password secret exists without its private key. Resolve the existing signing setup before generating a new identity." >&2
  exit 2
fi
umask 077
key_dir="$(mktemp -d /tmp/smp-release-signing.XXXXXX)"
npm install --prefix "$key_dir/cli" --no-save --package-lock=false @tauri-apps/cli@2.8.4
"$key_dir/cli/node_modules/.bin/tauri" signer generate --ci --write-keys "$key_dir/updater.key"
echo "Signing backup directory in this Codespace: $key_dir"
# Values go directly from the remote file to GitHub; never paste keys into chat.
gh secret set TAURI_SIGNING_PRIVATE_KEY --repo "$repo" < "$key_dir/updater.key"
gh variable set SMP_UPDATER_PUBLIC_KEY --repo "$repo" --body "$(cat "$key_dir/updater.key.pub")"
echo "Signing configured. Release publication remains disabled."
echo "Private-key backup in this Codespace: $key_dir/updater.key"
echo "Preserve that backup in your approved credential store before deleting this Codespace."
echo "Do not commit, share in chat, or upload the private key as a build artifact."
