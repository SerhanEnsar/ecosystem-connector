#!/usr/bin/env bash
# Runs the same checks as CI for one area, or for everything.
#
# Usage: tools/check.sh [core|ui|android|macos|windows|linux|all]   (default: all)
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

# The Linux app is Python + GTK 4. Set EC_REAL to a built `ec` to also test against it.
linux() {
  echo "== apps/linux"
  python3 -m py_compile apps/linux/econnector_gui.py
  python3 -W ignore::DeprecationWarning -m unittest discover -s apps/linux/tests -v
  if command -v shellcheck >/dev/null; then shellcheck apps/linux/*.sh apps/linux/tests/*.sh; else echo "skip: shellcheck not installed"; fi
  if command -v desktop-file-validate >/dev/null; then
    tmp=$(mktemp --suffix=.desktop)
    sed 's|@EXEC@|/usr/bin/econnector|' apps/linux/*.desktop.in > "$tmp"
    desktop-file-validate "$tmp"
    rm -f "$tmp"
  fi
}

case "$area" in
  core) core ;;
  ui) flutter pub get >/dev/null; flutter_pkg packages/ec_ui ;;
  linux) linux ;;
  android|macos|windows) flutter pub get >/dev/null; flutter_pkg packages/ec_ui; flutter_pkg "apps/$area" ;;
  all)
    core
    flutter pub get >/dev/null
    for pkg in packages/ec_ui apps/android apps/macos apps/windows; do flutter_pkg "$pkg"; done
    linux
    ;;
  *) echo "unknown area: $area" >&2; exit 2 ;;
esac
echo "All checks passed for: $area"
