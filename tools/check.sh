#!/usr/bin/env bash
# Runs the same checks as CI for one area, or for everything.
#
# Usage: tools/check.sh [core|ui|android|macos|windows|all]   (default: all)
set -euo pipefail
cd "$(dirname "$0")/.."

area="${1:-all}"

core() {
  echo "== core (Rust)"
  cargo fmt --all --check
  cargo clippy --workspace --all-targets --locked -- -D warnings
  cargo test --workspace --locked
  if command -v cargo-deny >/dev/null; then cargo deny check; else echo "skip: cargo-deny not installed"; fi
}

flutter_pkg() {
  echo "== $1"
  flutter analyze "$1"
  (cd "$1" && flutter test)
}


case "$area" in
  core) core ;;
  ui) flutter pub get >/dev/null; flutter_pkg packages/ec_ui ;;
  android|macos|windows) flutter pub get >/dev/null; flutter_pkg packages/ec_ui; flutter_pkg "apps/$area" ;;
  all)
    core
    flutter pub get >/dev/null
    for pkg in packages/ec_ui apps/android apps/macos apps/windows; do flutter_pkg "$pkg"; done
    ;;
  *) echo "unknown area: $area" >&2; exit 2 ;;
esac
echo "All checks passed for: $area"
