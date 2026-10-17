#!/usr/bin/env bash
# Formats the project's own code. Generated bindings (packages/ec_bridge/lib/src/rust)
# and vendored code (packages/ec_bridge/cargokit) are left alone on purpose.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo fmt --all
dart format packages/ec_ui packages/ec_app packages/ec_bridge/lib/ec_bridge.dart \
  apps/android/lib apps/android/test apps/macos/lib apps/macos/test \
  apps/windows/lib apps/windows/test
