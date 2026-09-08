#!/usr/bin/env bash
# Only for the configured Ubuntu hosted setup phase, never the user's computer.
set -euo pipefail
cd /workspace/Systems-Modeler-Pro
test "$(git rev-parse --show-toplevel)" = /workspace/Systems-Modeler-Pro
# Hosted setup may omit origin. Verify the approved tracked planning baseline
# independently of remotes; this is a checkout check, not a security sandbox.
expected_plan_blob=cc3331fc9af7bf4701b94f87ee6c4f4e946432c1
actual_plan_blob="$(git rev-parse HEAD:docs/STEP5_SCOPE_AND_ACCEPTANCE.md)"
if [ "$actual_plan_blob" != "$expected_plan_blob" ]; then
  echo "Checkout does not match the approved planning document; review baseline." >&2
  exit 2
fi
git ls-files --error-unmatch Cargo.toml Cargo.lock crates/model-core/Cargo.toml apps/desktop/src-tauri/Cargo.toml > /dev/null
if git remote | grep -Fxq origin; then
  case "$(git remote get-url origin)" in
    https://github.com/rbrundage5/Systems-Modeler-Pro|https://github.com/rbrundage5/Systems-Modeler-Pro.git) ;;
    *) echo "Unexpected repository origin; stop setup." >&2; exit 2 ;;
  esac
else
  echo "No origin remote: approved tracked baseline and checkout path verified."
fi
command -v rustup
command -v cargo
command -v node
command -v python3
# Uses only the preconfigured Ubuntu package sources and Cargo lockfile.
# Network is required here in setup, not during agent work.
if [ "$(id -u)" = 0 ]; then
  apt-get update
  DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends build-essential pkg-config libssl-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev patchelf
else
  sudo -n apt-get update
  sudo -n env DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends build-essential pkg-config libssl-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev patchelf
fi
rustup show active-toolchain
rustup component add rustfmt clippy
cargo fetch --locked
cargo metadata --offline --locked --format-version 1 > /dev/null
git diff --exit-code -- Cargo.lock
echo "Dependencies prepared. Agents remain disabled; isolation is not qualified."
