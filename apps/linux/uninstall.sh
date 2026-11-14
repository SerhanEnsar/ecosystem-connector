#!/bin/sh
# Removes what install.sh installed. Pairings and received files are kept.
set -eu
prefix=${PREFIX:-$HOME/.local}
app_id=io.github.serhanensar.ecosystemconnector
data=${XDG_DATA_HOME:-$HOME/.local/share}
[ "$prefix" = "$HOME/.local" ] || data=$prefix/share
rm -f "$prefix/bin/ec" "$prefix/bin/econnector" \
  "$data/applications/$app_id.desktop" \
  "$data/icons/hicolor/scalable/apps/$app_id.svg"
rm -rf "$prefix/lib/econnector"
echo "Removed ECOnnector. Your pairings stay in ~/.local/share/EcosystemConnector."
