#!/usr/bin/env bash
# Only for the configured Ubuntu hosted setup phase, never the user's computer.
set -euo pipefail
cd /workspace/Systems-Modeler-Pro
test "$(git rev-parse --show-toplevel)" = /workspace/Systems-Modeler-Pro
case "$(git remote get-url origin)" in
  https://github.com/rbrundage5/Systems-Modeler-Pro|https://github.com/rbrundage5/Systems-Modeler-Pro.git) ;;
  *) echo "Unexpected repository origin; stop setup." >&2; exit 2 ;;
esac
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
