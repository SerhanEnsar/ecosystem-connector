# ECOnnector for Linux

A GTK 4 / libadwaita window around the `ec` command line tool. It pairs with your other
devices (QR code or invite link), sends files, folders and web pages to them, and
receives files while it is open.

## Install

From a release archive (`ECOnnector-<version>-linux-x64.tar.gz`):

```sh
tar xzf ECOnnector-*-linux-x64.tar.gz
cd ECOnnector-*-linux-x64
./install.sh
```

This installs for your user only, into `~/.local` (set `PREFIX` to change that), and
adds ECOnnector to the app menu. `./uninstall.sh` removes it again; your pairings stay
in `~/.local/share/EcosystemConnector`.

It needs Python 3 with GTK 4 and libadwaita 1.5 or newer, which Ubuntu 24.04,
Fedora 40 and current Arch already have:

| Distribution | Command |
|---|---|
| Debian, Ubuntu | `sudo apt install python3-gi gir1.2-gtk-4.0 gir1.2-adw-1` |
| Fedora | `sudo dnf install python3-gobject gtk4 libadwaita` |
| Arch | `sudo pacman -S python-gobject gtk4 libadwaita` |

## Use

- **Devices**: *Pair* shows a QR code to scan with the phone app; *Join with invite*
  takes the `ecpair:` link another device shows. Both devices then show the same
  six-digit code; confirm only if they match.
- **Send**: pick a paired device, drop files or folders onto the window (or choose them),
  then *Send*. A web page address can be sent too; the other device offers to open it.
- **Receive**: while *Receive files* is on, this device is visible on the local network
  and saves what paired devices send into the folder shown (default
  `Downloads/ECOnnector`). It starts with the app unless you turn that off, and stops
  when the window closes.

If another device cannot reach this one, allow UDP port 47820 in your firewall, for
example `sudo ufw allow 47820/udp`.

## Development

See [docs/DEVELOPMENT.md](../../docs/DEVELOPMENT.md#linux). The window speaks English, and
Turkish when the system language is Turkish; translations live in `econnector_gui.py`.
