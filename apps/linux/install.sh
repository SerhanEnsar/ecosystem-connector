#!/bin/sh
# Installs ECOnnector for the current user (no root needed).
#   ./install.sh            installs into ~/.local
#   PREFIX=/opt/x ./install.sh   installs somewhere else
set -eu

here=$(cd "$(dirname "$0")" && pwd)
prefix=${PREFIX:-$HOME/.local}
app_id=io.github.serhanensar.ecosystemconnector
data=${XDG_DATA_HOME:-$HOME/.local/share}
[ "$prefix" = "$HOME/.local" ] || data=$prefix/share

if ! python3 - <<'PY' 2>/dev/null
import gi
gi.require_version("Gtk", "4.0")
gi.require_version("Adw", "1")
from gi.repository import Adw, Gtk
assert (Adw.get_major_version(), Adw.get_minor_version()) >= (1, 5)
PY
then
  echo "ECOnnector needs Python 3 with GTK 4 and libadwaita 1.5 or newer." >&2
  echo "  Debian/Ubuntu: sudo apt install python3-gi gir1.2-gtk-4.0 gir1.2-adw-1" >&2
  echo "  Fedora:        sudo dnf install python3-gobject gtk4 libadwaita" >&2
  echo "  Arch:          sudo pacman -S python-gobject gtk4 libadwaita" >&2
  exit 1
fi

install -Dm755 "$here/ec" "$prefix/bin/ec"
install -Dm644 "$here/econnector_gui.py" "$prefix/lib/econnector/econnector_gui.py"
install -Dm644 "$here/$app_id.svg" "$data/icons/hicolor/scalable/apps/$app_id.svg"

cat > "$prefix/bin/econnector" <<LAUNCHER
#!/bin/sh
EC_BIN="$prefix/bin/ec" exec python3 "$prefix/lib/econnector/econnector_gui.py" "\$@"
LAUNCHER
chmod 755 "$prefix/bin/econnector"

mkdir -p "$data/applications"
sed "s|@EXEC@|$prefix/bin/econnector|" "$here/$app_id.desktop.in" > "$data/applications/$app_id.desktop"

if command -v update-desktop-database >/dev/null; then
  update-desktop-database "$data/applications" 2>/dev/null || true
fi
if command -v gtk-update-icon-cache >/dev/null; then
  gtk-update-icon-cache -qtf "$data/icons/hicolor" 2>/dev/null || true
fi

echo "Installed ECOnnector. Open it from your app menu, or run: econnector"
case ":$PATH:" in
  *":$prefix/bin:"*) ;;
  *) echo "Note: $prefix/bin is not on your PATH; the app menu entry still works." ;;
esac
