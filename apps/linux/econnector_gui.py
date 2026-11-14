#!/usr/bin/env python3
# Copyright 2026 SerhanEnsar
# SPDX-License-Identifier: Apache-2.0
"""ECOnnector for Linux: a GTK 4 / libadwaita window around the `ec` command line tool.

Every network action is done by `ec`; this file only starts it, reads its output and
answers its pairing prompt. Output lines are parsed with the patterns `ec` prints today
(see crates/ec-cli), so a change to that wording needs a matching change here.
"""

import codecs
import json
import locale
import os
import re
import shutil
import signal
import sys

import gi

gi.require_version("Gtk", "4.0")
gi.require_version("Adw", "1")
from gi.repository import Adw, Gdk, Gio, GLib, Gtk  # noqa: E402

try:
    gi.require_version("GLibUnix", "2.0")
    from gi.repository import GLibUnix  # noqa: E402
except (ValueError, ImportError):
    GLibUnix = None

APP_ID = "io.github.serhanensar.ecosystemconnector"
HERE = os.path.dirname(os.path.abspath(__file__))


def find_ec():
    for candidate in (os.environ.get("EC_BIN"), os.path.join(HERE, "ec"), shutil.which("ec"),
                      os.path.expanduser("~/.local/bin/ec")):
        if candidate and os.path.isfile(candidate) and os.access(candidate, os.X_OK):
            return candidate
    return None


EC = find_ec()
# If this window dies without cleaning up, `ec serve` must not be left running.
SETPRIV = shutil.which("setpriv")
WRAP = [SETPRIV, "--pdeathsig", "INT"] if SETPRIV else []
CONFIG_DIR = os.path.join(GLib.get_user_config_dir(), "econnector")
CONFIG_FILE = os.path.join(CONFIG_DIR, "linux-app.json")

# ---------- translations ----------
# English is the source language. Add a language by adding a dict keyed by the English text.
TR = {
    "Send": "Gönder", "Receive": "Al", "Devices": "Cihazlar",
    "Target device": "Hedef cihaz", "Send to": "Gönderilecek cihaz",
    "Files": "Dosyalar", "Drop files or folders here": "Dosya veya klasörleri buraya sürükleyin",
    "Choose files": "Dosya seç", "Choose folders": "Klasör seç", "Selected": "Seçilenler",
    "Clear": "Temizle", "Remove": "Kaldır", "Cancel": "İptal",
    "Send a web page": "Web sayfası gönder",
    "The other device offers to open it in its browser": "Sayfa diğer cihazda tarayıcıda açılmak üzere önerilir",
    "Address (https://…)": "Adres (https://…)",
    "Pair a device first": "Önce bir cihaz eşleştirin",
    "Pair a device in the Devices tab first": "Önce “Cihazlar” sekmesinden bir cihaz eşleştirin",
    "Looking for {name}…": "{name} aranıyor…", "Preparing files…": "Dosyalar hazırlanıyor…",
    "Sending…": "Gönderiliyor…", "Sent": "Gönderildi",
    "Sent {n} file(s) ({size} in {time}, {speed}/s)": "{n} dosya gönderildi ({size}, {time}, {speed}/sn)",
    "Could not send: {err}": "Gönderilemedi: {err}",
    "Page sent to {name}": "Sayfa {name} cihazına gönderildi",
    "Files to send": "Gönderilecek dosyalar", "Folders to send": "Gönderilecek klasörler",
    "Receiving files": "Dosya alma",
    "While on, this device is visible on the network and accepts files from paired devices":
        "Açıkken bu cihaz ağda görünür ve eşleşmiş cihazlardan dosya alır",
    "Receive files": "Dosya almayı aç", "Off": "Kapalı", "Starting…": "Başlatılıyor…",
    "On · listening on UDP port {port}": "Açık · UDP {port} portu dinleniyor",
    "Stopped: {err}": "Durdu: {err}", "Receiving stopped: {err}": "Dosya alma durdu: {err}",
    "Save to": "Kayıt klasörü", "Change folder": "Klasörü değiştir", "Open folder": "Klasörü aç",
    "Start when the app opens": "Uygulama açılınca otomatik başlat",
    "Recently received": "Son alınanlar", "Nothing received yet": "Henüz bir şey alınmadı",
    "Receiving {n} file(s) ({size}) from {name}": "{name} cihazından {n} dosya alınıyor ({size})",
    "Received {n} file(s) from {name}": "{name} cihazından {n} dosya alındı",
    "File received": "Dosya alındı", "Show in folder": "Klasörde göster",
    "{name} sent a page: {title}": "{name} bir sayfa gönderdi: {title}",
    "{name} sent a page": "{name} bir sayfa gönderdi", "Open": "Aç",
    "This device": "Bu cihaz", "Rename": "Yeniden adlandır",
    "Rename this device": "Cihazı yeniden adlandır",
    "Other devices see this device by this name.": "Diğer cihazlar bu cihazı bu adla görür.",
    "Save": "Kaydet", "Renamed": "Ad değiştirildi",
    "Paired devices": "Eşleşmiş cihazlar", "Pair": "Eşleştir", "Join with invite": "Davetle katıl",
    "No paired devices yet": "Henüz eşleşmiş cihaz yok",
    "Use “Pair” to show a QR code, or join with the other device's invite link":
        "“Eşleştir” ile QR kod gösterin ya da diğer cihazın davet linkiyle katılın",
    "Test connection": "Bağlantıyı test et", "Forget": "Unut",
    "Connecting to {name}…": "{name} cihazına bağlanılıyor…",
    "{name} is connected · average {ms} ms": "{name} bağlı · ortalama gecikme {ms} ms",
    "Forget {name}?": "{name} unutulsun mu?",
    "You will need to pair again to send or receive files.":
        "Tekrar dosya alıp göndermek için yeniden eşleştirmeniz gerekir.",
    "{name} forgotten": "{name} unutuldu",
    "Devices on this network": "Yerel ağdaki cihazlar",
    "ECOnnector devices that are receiving files": "Dosya alması açık olan ECOnnector cihazları",
    "Refresh": "Yenile", "Paired": "Eşleşmiş", "Not paired": "Eşleşmemiş",
    "No devices found": "Ağda cihaz bulunamadı",
    "The other device needs ECOnnector open with receiving on":
        "Diğer cihazda ECOnnector açık ve dosya alması etkin olmalı",
    "Pair a device": "Cihaz eşleştir", "Preparing the QR code…": "QR kod hazırlanıyor…",
    "Copy invite link": "Davet linkini kopyala", "Invite link copied": "Davet linki kopyalandı",
    "Reject": "Reddet", "Codes match, confirm": "Kodlar aynı, onayla",
    "Paste the invite link the other device shows": "Diğer cihazın gösterdiği davet linkini yapıştırın",
    "ecpair:… link": "ecpair:… linki", "Join": "Katıl",
    "The link must start with “ecpair:”": "Link “ecpair:” ile başlamalı",
    "Connecting…": "Bağlanılıyor…", "Scan this QR code with the other device": "Diğer cihazla bu QR kodu okutun",
    "The invite works once, for 2 minutes.": "Davet 2 dakika geçerli ve tek kullanımlık.",
    "A device": "Bir cihaz", "{peer} wants to pair": "{peer} eşleşmek istiyor",
    "Does the other device show the same code?": "Diğer cihazda da aynı kod görünüyor mu?",
    "Waiting for the other device to confirm…": "Diğer cihazın onayı bekleniyor…",
    "Paired with {name}": "{name} ile eşleşildi", "Pairing complete": "Eşleştirme tamamlandı",
    "You can close this window.": "Bu pencereyi kapatabilirsiniz.",
    "Could not reach the device. Make sure receiving is on there.":
        "Cihaza ulaşılamadı. Diğer cihazda dosya almanın açık olduğundan emin olun.",
    "No paired device matches this id.": "Bu kimlikle eşleşmiş cihaz bulunamadı.",
    "Pairing was declined.": "Eşleştirme reddedildi.", "Unknown error": "Bilinmeyen hata",
    "ec not found": "ec bulunamadı",
    "The ECOnnector command line tool (ec) is not installed. Run install.sh from the release archive.":
        "ECOnnector komut satırı aracı (ec) kurulu değil. Release arşivindeki install.sh'ı çalıştırın.",
    "OK": "Tamam",
}


def _lang():
    for var in ("LANGUAGE", "LC_ALL", "LC_MESSAGES", "LANG"):
        value = os.environ.get(var)
        if value:
            return value.split(":")[0][:2].lower()
    try:
        return (locale.getlocale()[0] or "en")[:2].lower()
    except ValueError:
        return "en"


STRINGS = {"tr": TR}.get(_lang(), {})


def T(text, **kw):
    s = STRINGS.get(text, text)
    return s.format(**kw) if kw else s


# ---------- settings ----------
def default_receive_dir():
    dl = GLib.get_user_special_dir(GLib.UserDirectory.DIRECTORY_DOWNLOAD) or os.path.expanduser("~/Downloads")
    return os.path.join(dl, "ECOnnector")


def load_settings():
    s = {"receive_dir": default_receive_dir(), "auto_serve": True}
    try:
        with open(CONFIG_FILE, encoding="utf-8") as f:
            s.update(json.load(f))
    except (OSError, ValueError):
        pass
    return s


def save_settings(s):
    os.makedirs(CONFIG_DIR, exist_ok=True)
    with open(CONFIG_FILE, "w", encoding="utf-8") as f:
        json.dump(s, f, ensure_ascii=False, indent=2)


# ---------- talking to `ec` ----------
PROGRESS_RE = re.compile(r"^\s*(\d+(?:\.\d+)?)%\s+(.*?)\s*$")
DEVICE_ID_RE = re.compile(r"^[0-9A-F]{4}-[0-9A-F]{4}$")


def make_spinner():
    """Adw.Spinner needs libadwaita 1.6; Ubuntu 24.04 ships 1.5, so fall back to Gtk.Spinner."""
    if hasattr(Adw, "Spinner"):
        return Adw.Spinner()
    return Gtk.Spinner(spinning=True)


class ECProcess:
    """Runs a long-lived `ec` command and reports its output line by line.

    Progress lines end in \\r, so both \\r and \\n split lines. A pending `[y/N]` prompt has
    no line ending and is reported through on_prompt.
    """

    def __init__(self, args, on_line=None, on_prompt=None, on_exit=None):
        self.on_line = on_line
        self.on_prompt = on_prompt
        self.on_exit = on_exit
        self.lines = []
        self._buf = ""
        self._dec = codecs.getincrementaldecoder("utf-8")(errors="replace")
        self._eof = False
        self._exited = False
        self._status = None
        flags = (Gio.SubprocessFlags.STDOUT_PIPE | Gio.SubprocessFlags.STDERR_MERGE
                 | Gio.SubprocessFlags.STDIN_PIPE)
        self.proc = Gio.Subprocess.new(WRAP + [EC] + args, flags)
        self._out = self.proc.get_stdout_pipe()
        self._in = self.proc.get_stdin_pipe()
        self._read()
        self.proc.wait_async(None, self._on_wait)

    @property
    def running(self):
        return not self._exited

    def _read(self):
        self._out.read_bytes_async(4096, GLib.PRIORITY_DEFAULT, None, self._on_read)

    def _on_read(self, stream, res):
        try:
            data = stream.read_bytes_finish(res).get_data()
        except GLib.Error:
            data = b""
        if not data:
            self._buf += self._dec.decode(b"", final=True)
            if self._buf.strip():
                self._emit(self._buf)
            self._buf = ""
            self._eof = True
            self._maybe_finish()
            return
        self._buf += self._dec.decode(data)
        parts = re.split(r"[\r\n]", self._buf)
        self._buf = parts.pop()
        for p in parts:
            if p.strip():
                self._emit(p)
        if self._buf.rstrip().endswith("[y/N]"):
            prompt, self._buf = self._buf.strip(), ""
            self.lines.append(prompt)
            if self.on_prompt:
                self.on_prompt(prompt)
        self._read()

    def _emit(self, line):
        line = line.rstrip()
        self.lines.append(line)
        if self.on_line:
            try:
                self.on_line(line)
            except Exception:  # a UI bug must not stop reading the output
                import traceback
                traceback.print_exc()

    def _on_wait(self, proc, res):
        try:
            proc.wait_finish(res)
        except GLib.Error:
            pass
        self._status = proc.get_exit_status() if proc.get_if_exited() else -1
        self._exited = True
        self._maybe_finish()

    def _maybe_finish(self):
        if self._eof and self._exited and self.on_exit:
            cb, self.on_exit = self.on_exit, None
            cb(self._status == 0, self)

    def answer(self, yes):
        try:
            self._in.write_bytes(GLib.Bytes.new(b"y\n" if yes else b"n\n"), None)
            self._in.flush(None)
        except GLib.Error:
            pass

    def stop(self):
        """Ctrl-C first, so `ec serve` can say goodbye; kill it if it does not exit."""
        if self._exited:
            return
        self.proc.send_signal(signal.SIGINT)

        def force():
            if not self._exited:
                self.proc.force_exit()
            return False
        GLib.timeout_add(2500, force)

    def error_text(self):
        for line in reversed(self.lines):
            if line.startswith("Error:"):
                return line[len("Error:"):].strip()
        return self.lines[-1] if self.lines else T("Unknown error")


def run_ec(args, callback):
    """Runs a short `ec` command and calls callback(ok, output)."""
    flags = Gio.SubprocessFlags.STDOUT_PIPE | Gio.SubprocessFlags.STDERR_MERGE
    proc = Gio.Subprocess.new(WRAP + [EC] + args, flags)

    def done(p, res):
        try:
            _, out, _ = p.communicate_utf8_finish(res)
        except GLib.Error as e:
            callback(False, str(e))
            return
        callback(p.get_if_exited() and p.get_exit_status() == 0, out or "")
    proc.communicate_utf8_async(None, None, done)


def parse_qr(lines):
    """Turns the half-block QR code `ec pair` prints into a matrix of dark modules.

    Each character holds two modules: █ both dark, ▀ top dark, ▄ bottom dark.
    """
    rows = [line for line in lines if any(c in line for c in "█▀▄")]
    if not rows:
        return None
    margin = min(len(r) - len(r.lstrip(" ")) for r in rows)
    width = max(len(r) for r in rows) - margin
    matrix = []
    for r in rows:
        r = r[margin:].ljust(width)
        matrix.append([c in "█▀" for c in r])
        matrix.append([c in "█▄" for c in r])
    while matrix and not any(matrix[-1]):
        matrix.pop()
    return matrix


def human_error(msg):
    m = msg.lower()
    if "timed out" in m:
        return T("Could not reach the device. Make sure receiving is on there.")
    if "no single paired device" in m:
        return T("No paired device matches this id.")
    if "declined" in m:
        return T("Pairing was declined.")
    return msg


def esc(text):
    return GLib.markup_escape_text(text)


# ---------- pairing ----------
class PairDialog(Adw.Dialog):
    """Pair by showing a QR code (mode "pair") or by pasting an invite link (mode "join")."""

    def __init__(self, win, mode):
        super().__init__(title=T("Pair a device") if mode == "pair" else T("Join with invite"))
        self.win = win
        self.proc = None
        self.qr = None
        self.invite = None
        self.peer = None
        self.done = False
        self.set_content_width(460)

        tv = Adw.ToolbarView()
        tv.add_top_bar(Adw.HeaderBar())
        self.box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=14,
                           margin_top=12, margin_bottom=24, margin_start=24, margin_end=24)
        tv.set_content(self.box)
        self.set_child(tv)
        self.connect("closed", self._on_closed)

        self.status = Gtk.Label(wrap=True, justify=Gtk.Justification.CENTER)
        self.status.add_css_class("title-4")
        self.spinner = make_spinner()
        self.spinner.set_size_request(32, 32)

        self.qr_area = Gtk.DrawingArea(content_width=260, content_height=260, halign=Gtk.Align.CENTER,
                                       visible=False)
        self.qr_area.set_draw_func(self._draw_qr)

        self.invite_row = Gtk.Box(spacing=6, halign=Gtk.Align.CENTER, visible=False)
        copy = Gtk.Button(child=Adw.ButtonContent(icon_name="edit-copy-symbolic", label=T("Copy invite link")))
        copy.connect("clicked", self._copy_invite)
        self.invite_row.append(copy)

        self.code = Gtk.Label(visible=False, selectable=True)
        self.code.add_css_class("title-1")
        self.hint = Gtk.Label(wrap=True, justify=Gtk.Justification.CENTER, visible=False)
        self.hint.add_css_class("dim-label")

        self.buttons = Gtk.Box(spacing=12, halign=Gtk.Align.CENTER, visible=False)
        no = Gtk.Button(label=T("Reject"))
        no.add_css_class("pill")
        no.connect("clicked", lambda *_: self._answer(False))
        yes = Gtk.Button(label=T("Codes match, confirm"))
        yes.add_css_class("pill")
        yes.add_css_class("suggested-action")
        yes.connect("clicked", lambda *_: self._answer(True))
        self.buttons.append(no)
        self.buttons.append(yes)

        for w in (self.status, self.spinner, self.qr_area, self.invite_row, self.code, self.hint, self.buttons):
            self.box.append(w)

        if mode == "pair":
            self.status.set_label(T("Preparing the QR code…"))
            self._start(["pair"])
        else:
            self._ask_invite()

    def _ask_invite(self):
        self.status.set_label(T("Paste the invite link the other device shows"))
        self.spinner.set_visible(False)
        self.entry_group = Adw.PreferencesGroup()
        self.entry = Adw.EntryRow(title=T("ecpair:… link"))
        self.entry.connect("entry-activated", lambda *_: self._join())
        self.entry_group.add(self.entry)
        self.join_btn = Gtk.Button(label=T("Join"), halign=Gtk.Align.CENTER)
        self.join_btn.add_css_class("pill")
        self.join_btn.add_css_class("suggested-action")
        self.join_btn.connect("clicked", lambda *_: self._join())
        self.box.insert_child_after(self.entry_group, self.status)
        self.box.insert_child_after(self.join_btn, self.entry_group)
        self.win.get_clipboard().read_text_async(None, self._on_clipboard)

    def _on_clipboard(self, clipboard, res):
        try:
            text = clipboard.read_text_finish(res)
        except GLib.Error:
            return
        if text and text.strip().startswith("ecpair:") and not self.entry.get_text():
            self.entry.set_text(text.strip())

    def _join(self):
        link = self.entry.get_text().strip().strip("'\"")
        if link.startswith("ec join "):
            link = link[len("ec join "):].strip().strip("'\"")
        if not link.startswith("ecpair:"):
            self.status.set_label(T("The link must start with “ecpair:”"))
            return
        self.entry_group.set_visible(False)
        self.join_btn.set_visible(False)
        self.spinner.set_visible(True)
        self.status.set_label(T("Connecting…"))
        self._start(["join", link])

    def _start(self, args):
        self.proc = ECProcess(args, on_line=self._on_line, on_prompt=self._on_prompt, on_exit=self._on_exit)

    def _on_line(self, line):
        s = line.strip()
        if s.startswith("Scan the code"):
            self.qr = parse_qr(self.proc.lines)
            self.qr_area.set_visible(self.qr is not None)
            self.qr_area.queue_draw()
            self.spinner.set_visible(False)
            self.status.set_label(T("Scan this QR code with the other device"))
        elif s.startswith("ec join "):
            m = re.search(r"(ecpair:[^'\s]+)", s)
            if m:
                self.invite = m.group(1)
                self.invite_row.set_visible(True)
        elif s.startswith("Waiting up to"):
            self.hint.set_label(T("The invite works once, for 2 minutes."))
            self.hint.set_visible(True)
        elif s.endswith(" wants to pair."):
            self.peer = s[: -len(" wants to pair.")]
        elif s.startswith("Paired with "):
            self._finish(True, T("Paired with {name}", name=s[len("Paired with "):].rstrip(".")))

    def _on_prompt(self, prompt):
        m = re.search(r"code (\d{3} \d{3})", prompt)
        self.qr_area.set_visible(False)
        self.invite_row.set_visible(False)
        self.spinner.set_visible(False)
        self.status.set_label(T("{peer} wants to pair", peer=self.peer or T("A device")))
        self.code.set_label(m.group(1) if m else "")
        self.code.set_visible(True)
        self.hint.set_label(T("Does the other device show the same code?"))
        self.hint.set_visible(True)
        self.buttons.set_visible(True)

    def _answer(self, yes):
        self.buttons.set_visible(False)
        self.hint.set_visible(False)
        if yes:
            self.spinner.set_visible(True)
            self.status.set_label(T("Waiting for the other device to confirm…"))
        self.proc.answer(yes)

    def _on_exit(self, ok, proc):
        if self.done:
            return
        if ok:
            self._finish(True, T("Pairing complete"))
        else:
            self._finish(False, human_error(proc.error_text()))

    def _finish(self, ok, text):
        self.done = True
        for w in (self.spinner, self.qr_area, self.invite_row, self.buttons, self.code):
            w.set_visible(False)
        self.status.set_label(text)
        self.hint.set_label(T("You can close this window.") if ok else "")
        self.hint.set_visible(ok)
        icon = Gtk.Image(icon_name="emblem-ok-symbolic" if ok else "dialog-error-symbolic", pixel_size=64)
        icon.add_css_class("success" if ok else "error")
        self.box.prepend(icon)
        if ok:
            self.win.on_paired()

    def _copy_invite(self, *_):
        if self.invite:
            self.win.get_clipboard().set_content(
                Gdk.ContentProvider.new_for_bytes("text/plain;charset=utf-8", GLib.Bytes.new(self.invite.encode())))
            self.win.toast(T("Invite link copied"))

    def _draw_qr(self, area, cr, w, h):
        if not self.qr:
            return
        n_rows, n_cols = len(self.qr), len(self.qr[0])
        quiet = 4
        size = min(w / (n_cols + 2 * quiet), h / (n_rows + 2 * quiet))
        ox = (w - size * n_cols) / 2
        oy = (h - size * n_rows) / 2
        cr.set_source_rgb(1, 1, 1)
        cr.rectangle(ox - quiet * size, oy - quiet * size, size * (n_cols + 2 * quiet), size * (n_rows + 2 * quiet))
        cr.fill()
        cr.set_source_rgb(0, 0, 0)
        for y, row in enumerate(self.qr):
            for x, dark in enumerate(row):
                if dark:
                    cr.rectangle(ox + x * size, oy + y * size, size + 0.3, size + 0.3)
        cr.fill()

    def _on_closed(self, *_):
        if self.proc and self.proc.running:
            self.proc.stop()


# ---------- main window ----------
class Window(Adw.ApplicationWindow):
    def __init__(self, app):
        super().__init__(application=app, title="ECOnnector", default_width=760, default_height=720)
        self.app = app
        self.settings = load_settings()
        self.devices = []  # [(id, name)]
        self.my_name = ""
        self.device_rows = []
        self.net_rows = []
        self.recv_rows = []
        self.send_paths = []
        self.path_rows = []
        self.send_proc = None
        self.serve_proc = None
        self.serve_restart = False
        self.recv_batch = None

        self.toasts = Adw.ToastOverlay()
        tv = Adw.ToolbarView()
        header = Adw.HeaderBar()
        self.stack = Adw.ViewStack()
        header.set_title_widget(Adw.ViewSwitcher(stack=self.stack, policy=Adw.ViewSwitcherPolicy.WIDE))
        tv.add_top_bar(header)
        tv.set_content(self.stack)
        self.toasts.set_child(tv)
        self.set_content(self.toasts)

        self.stack.add_titled_with_icon(self._build_send_page(), "send", T("Send"), "document-send-symbolic")
        self.stack.add_titled_with_icon(self._build_receive_page(), "receive", T("Receive"),
                                        "folder-download-symbolic")
        self.stack.add_titled_with_icon(self._build_devices_page(), "devices", T("Devices"), "computer-symbolic")

        self.connect("close-request", self.on_close)
        self.refresh_identity()
        self.refresh_devices()
        self.refresh_network()
        if self.settings.get("auto_serve", True):
            self.serve_switch.set_active(True)

    # ----- helpers -----
    def toast(self, text, button=None, callback=None):
        t = Adw.Toast(title=esc(text), timeout=4)
        if button:
            t.set_button_label(button)
            t.connect("button-clicked", lambda *_: callback())
        self.toasts.add_toast(t)

    def notify(self, nid, title, body, action=None, target=None):
        if self.is_active():
            return
        n = Gio.Notification.new(title)
        n.set_body(body)
        if action:
            n.set_default_action_and_target(action, target)
        self.app.send_notification(nid, n)

    def open_uri(self, uri):
        Gtk.UriLauncher.new(uri).launch(self, None, None)

    def open_path(self, path, folder=False):
        launcher = Gtk.FileLauncher.new(Gio.File.new_for_path(path))
        if folder:
            launcher.open_containing_folder(self, None, None)
        else:
            launcher.launch(self, None, None)

    @staticmethod
    def _clear_rows(group, rows):
        for r in rows:
            group.remove(r)
        rows.clear()

    @staticmethod
    def _flat_button(icon, tooltip, callback):
        b = Gtk.Button(icon_name=icon, valign=Gtk.Align.CENTER, tooltip_text=tooltip)
        b.add_css_class("flat")
        b.connect("clicked", callback)
        return b

    @staticmethod
    def _pill(label, suggested=False):
        b = Gtk.Button(label=label, halign=Gtk.Align.CENTER)
        b.add_css_class("pill")
        if suggested:
            b.add_css_class("suggested-action")
        return b

    # ----- Send -----
    def _build_send_page(self):
        page = Adw.PreferencesPage()

        g = Adw.PreferencesGroup(title=T("Target device"))
        self.target_model = Gtk.StringList()
        self.target_row = Adw.ComboRow(title=T("Send to"), model=self.target_model)
        g.add(self.target_row)
        page.add(g)

        g = Adw.PreferencesGroup(title=T("Files"))
        drop = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, margin_top=6, margin_bottom=6)
        drop.add_css_class("card")
        inner = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=10, margin_top=28, margin_bottom=28,
                        margin_start=12, margin_end=12)
        icon = Gtk.Image(icon_name="folder-documents-symbolic", pixel_size=48)
        icon.add_css_class("dim-label")
        label = Gtk.Label(label=T("Drop files or folders here"), wrap=True)
        label.add_css_class("title-4")
        btns = Gtk.Box(spacing=8, halign=Gtk.Align.CENTER)
        bf = self._pill(T("Choose files"))
        bf.connect("clicked", self._pick_files)
        bd = self._pill(T("Choose folders"))
        bd.connect("clicked", self._pick_folders)
        btns.append(bf)
        btns.append(bd)
        for w in (icon, label, btns):
            inner.append(w)
        drop.append(inner)
        target = Gtk.DropTarget.new(Gdk.FileList, Gdk.DragAction.COPY)
        target.connect("drop", self._on_drop)
        target.connect("enter", lambda *_: (drop.add_css_class("accent"), Gdk.DragAction.COPY)[1])
        target.connect("leave", lambda *_: drop.remove_css_class("accent"))
        drop.add_controller(target)
        g.add(drop)
        page.add(g)

        self.paths_group = Adw.PreferencesGroup(title=T("Selected"), visible=False)
        clear = Gtk.Button(label=T("Clear"), valign=Gtk.Align.CENTER)
        clear.add_css_class("flat")
        clear.connect("clicked", lambda *_: self._set_paths([]))
        self.paths_group.set_header_suffix(clear)
        page.add(self.paths_group)

        g = Adw.PreferencesGroup()
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=10)
        self.send_btn = self._pill(T("Send"), suggested=True)
        self.send_btn.set_sensitive(False)
        self.send_btn.connect("clicked", self._send)
        self.cancel_btn = self._pill(T("Cancel"))
        self.cancel_btn.set_visible(False)
        self.cancel_btn.connect("clicked", lambda *_: self.send_proc and self.send_proc.stop())
        self.send_progress = Gtk.ProgressBar(visible=False, show_text=True)
        self.send_status = Gtk.Label(wrap=True, visible=False)
        self.send_status.add_css_class("dim-label")
        for w in (self.send_btn, self.cancel_btn, self.send_progress, self.send_status):
            box.append(w)
        g.add(box)
        page.add(g)

        g = Adw.PreferencesGroup(title=T("Send a web page"),
                                 description=T("The other device offers to open it in its browser"))
        self.url_row = Adw.EntryRow(title=T("Address (https://…)"), show_apply_button=True)
        self.url_row.connect("apply", self._fling)
        g.add(self.url_row)
        page.add(g)
        return page

    def _selected_device(self):
        i = self.target_row.get_selected()
        if i == Gtk.INVALID_LIST_POSITION or i >= len(self.devices):
            return None
        return self.devices[i]

    def _pick_files(self, *_):
        d = Gtk.FileDialog(title=T("Files to send"))
        d.open_multiple(self, None, lambda d, r: self._picked(d.open_multiple_finish, r))

    def _pick_folders(self, *_):
        d = Gtk.FileDialog(title=T("Folders to send"))
        d.select_multiple_folders(self, None, lambda d, r: self._picked(d.select_multiple_folders_finish, r))

    def _picked(self, finish, res):
        try:
            files = finish(res)
        except GLib.Error:
            return
        self._add_paths([files.get_item(i).get_path() for i in range(files.get_n_items())])

    def _on_drop(self, target, value, x, y):
        target.get_widget().remove_css_class("accent")
        self._add_paths([f.get_path() for f in value.get_files()])
        return True

    def _add_paths(self, paths):
        new = list(self.send_paths)
        for p in paths:
            if p and p not in new:
                new.append(p)
        self._set_paths(new)

    def _set_paths(self, paths):
        self.send_paths = paths
        self._clear_rows(self.paths_group, self.path_rows)
        for p in paths:
            row = Adw.ActionRow(title=esc(os.path.basename(p) or p), subtitle=esc(os.path.dirname(p)))
            row.add_prefix(Gtk.Image(icon_name="folder-symbolic" if os.path.isdir(p) else "text-x-generic-symbolic"))
            row.add_suffix(self._flat_button(
                "list-remove-symbolic", T("Remove"),
                lambda _b, p=p: self._set_paths([x for x in self.send_paths if x != p])))
            self.paths_group.add(row)
            self.path_rows.append(row)
        self.paths_group.set_visible(bool(paths))
        self._update_send_sensitive()

    def _update_send_sensitive(self):
        busy = self.send_proc is not None and self.send_proc.running
        self.send_btn.set_sensitive(bool(self.send_paths) and bool(self.devices) and not busy)

    def _send(self, *_):
        dev = self._selected_device()
        if not dev or not self.send_paths:
            return
        self.send_btn.set_visible(False)
        self.cancel_btn.set_visible(True)
        self.send_progress.set_visible(True)
        self.send_progress.set_fraction(0)
        self.send_progress.set_text("")
        self.send_status.set_visible(True)
        self.send_status.set_label(T("Looking for {name}…", name=dev[1]))
        self.send_proc = ECProcess(["send", dev[0]] + self.send_paths,
                                   on_line=self._send_line, on_exit=self._send_done)

    def _send_line(self, line):
        s = line.strip()
        m = PROGRESS_RE.match(s)
        if m:
            self.send_progress.set_fraction(float(m.group(1)) / 100)
            self.send_progress.set_text(f"{m.group(1)}%  ·  {m.group(2)}")
            self.send_status.set_label(T("Sending…"))
        elif s.startswith("Hashing"):
            self.send_status.set_label(T("Preparing files…"))
            self.send_progress.pulse()

    def _send_done(self, ok, proc):
        self.send_btn.set_visible(True)
        self.cancel_btn.set_visible(False)
        self._update_send_sensitive()
        if ok:
            self.send_progress.set_fraction(1)
            last = next((line for line in reversed(proc.lines) if line.startswith("Sent ")), "")
            m = re.match(r"Sent (\d+) file\(s\), (.+?) in (.+?) \((.+?)/s", last)
            text = (T("Sent {n} file(s) ({size} in {time}, {speed}/s)",
                      n=m.group(1), size=m.group(2), time=m.group(3), speed=m.group(4)) if m else T("Sent"))
            self.send_status.set_label(text)
            self.toast(text)
            self._set_paths([])
        else:
            self.send_progress.set_visible(False)
            self.send_status.set_label(T("Could not send: {err}", err=human_error(proc.error_text())))

    def _fling(self, row):
        dev = self._selected_device()
        url = row.get_text().strip()
        if not dev:
            self.toast(T("Pair a device first"))
            return
        if not re.match(r"^https?://", url):
            url = "https://" + url
        row.set_sensitive(False)

        def done(ok, out):
            row.set_sensitive(True)
            if ok:
                row.set_text("")
                self.toast(T("Page sent to {name}", name=dev[1]))
            else:
                err = next((line[6:].strip() for line in out.splitlines() if line.startswith("Error:")), out.strip())
                self.toast(T("Could not send: {err}", err=human_error(err)))
        run_ec(["fling", dev[0], url], done)

    # ----- Receive -----
    def _build_receive_page(self):
        page = Adw.PreferencesPage()
        g = Adw.PreferencesGroup(
            title=T("Receiving files"),
            description=T("While on, this device is visible on the network and accepts files from paired devices"))
        self.serve_switch = Adw.SwitchRow(title=T("Receive files"), subtitle=T("Off"))
        self.serve_switch.connect("notify::active", self._on_serve_toggled)
        g.add(self.serve_switch)

        self.dir_row = Adw.ActionRow(title=T("Save to"), subtitle=esc(self.settings["receive_dir"]))
        self.dir_row.add_suffix(self._flat_button("document-open-symbolic", T("Change folder"), self._pick_receive_dir))
        self.dir_row.add_suffix(self._flat_button("folder-open-symbolic", T("Open folder"), self._open_receive_dir))
        g.add(self.dir_row)

        auto = Adw.SwitchRow(title=T("Start when the app opens"), active=self.settings.get("auto_serve", True))
        auto.connect("notify::active", self._on_auto_toggled)
        g.add(auto)
        page.add(g)

        g = Adw.PreferencesGroup()
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
        self.recv_label = Gtk.Label(visible=False, xalign=0, wrap=True)
        self.recv_progress = Gtk.ProgressBar(visible=False, show_text=True)
        box.append(self.recv_label)
        box.append(self.recv_progress)
        g.add(box)
        page.add(g)

        self.recv_group = Adw.PreferencesGroup(title=T("Recently received"))
        self.recv_empty = Adw.ActionRow(title=T("Nothing received yet"))
        self.recv_empty.add_css_class("dim-label")
        self.recv_group.add(self.recv_empty)
        page.add(self.recv_group)
        return page

    def _open_receive_dir(self, *_):
        os.makedirs(self.settings["receive_dir"], exist_ok=True)
        self.open_uri(Gio.File.new_for_path(self.settings["receive_dir"]).get_uri())

    def _on_auto_toggled(self, row, _pspec):
        self.settings["auto_serve"] = row.get_active()
        save_settings(self.settings)

    def _pick_receive_dir(self, *_):
        d = Gtk.FileDialog(title=T("Save to"))
        os.makedirs(self.settings["receive_dir"], exist_ok=True)
        d.set_initial_folder(Gio.File.new_for_path(self.settings["receive_dir"]))

        def done(d, res):
            try:
                f = d.select_folder_finish(res)
            except GLib.Error:
                return
            self.settings["receive_dir"] = f.get_path()
            save_settings(self.settings)
            self.dir_row.set_subtitle(esc(f.get_path()))
            self.restart_serve()
        d.select_folder(self, None, done)

    def _on_serve_toggled(self, row, _pspec):
        if row.get_active():
            self._start_serve()
        else:
            self.serve_restart = False
            if self.serve_proc and self.serve_proc.running:
                self.serve_proc.stop()
            row.set_subtitle(T("Off"))

    def _start_serve(self):
        if self.serve_proc and self.serve_proc.running:
            return
        os.makedirs(self.settings["receive_dir"], exist_ok=True)
        self.serve_switch.set_subtitle(T("Starting…"))
        self.serve_proc = ECProcess(["serve", "--dir", self.settings["receive_dir"]],
                                    on_line=self._serve_line, on_exit=self._serve_exit)

    def restart_serve(self):
        """`ec serve` reads pairings, its name and its folder only at start."""
        if self.serve_proc and self.serve_proc.running:
            self.serve_restart = True
            self.serve_proc.stop()

    def _serve_exit(self, ok, proc):
        self.recv_progress.set_visible(False)
        self.recv_label.set_visible(False)
        if self.serve_restart:
            self.serve_restart = False
            if self.serve_switch.get_active():
                self._start_serve()
            return
        if self.serve_switch.get_active():
            err = human_error(proc.error_text())
            self.serve_switch.set_active(False)
            self.serve_switch.set_subtitle(T("Stopped: {err}", err=err))
            self.toast(T("Receiving stopped: {err}", err=err))

    def _serve_line(self, line):
        s = line.strip()
        m = PROGRESS_RE.match(s)
        if m:
            self.recv_progress.set_visible(True)
            self.recv_progress.set_fraction(float(m.group(1)) / 100)
            self.recv_progress.set_text(f"{m.group(1)}%  ·  {m.group(2)}")
            return
        m = re.search(r"is online on UDP port (\d+)", s)
        if m:
            self.serve_switch.set_subtitle(T("On · listening on UDP port {port}", port=m.group(1)))
            return
        m = re.match(r"Receiving (\d+) file\(s\), (.+?) from (\S+)", s)
        if m:
            name = self._device_name(m.group(3))
            self.recv_label.set_label(T("Receiving {n} file(s) ({size}) from {name}",
                                        n=m.group(1), size=m.group(2), name=name))
            self.recv_label.set_visible(True)
            self.recv_progress.set_visible(True)
            self.recv_progress.set_fraction(0)
            self.recv_batch = [name, int(m.group(1)), 0]
            return
        if s.startswith("saved "):
            path = s[len("saved "):]
            self._add_recv_row(os.path.basename(path), path, file_path=path)
            if self.recv_batch:
                self.recv_batch[2] += 1
                name, total, got = self.recv_batch
                if got >= total:
                    self.recv_batch = None
                    self.recv_progress.set_visible(False)
                    self.recv_label.set_visible(False)
                    text = T("Received {n} file(s) from {name}", n=total, name=name)
                    self.toast(text, T("Show in folder"), lambda p=path: self.open_path(p, folder=True))
                    self.notify("received", T("File received"), text, "app.show-file", GLib.Variant("s", path))
            return
        m = re.match(r"(\S+) flung a page: (.*?)\s*(https?://\S+)$", s)
        if m:
            name = self._device_name(m.group(1))
            url = m.group(3)
            title = m.group(2) or url
            self._add_recv_row(title, url, url=url)
            self.toast(T("{name} sent a page: {title}", name=name, title=title), T("Open"),
                       lambda: self.open_uri(url))
            self.notify("fling", T("{name} sent a page", name=name), title, "app.open-url", GLib.Variant("s", url))
            return
        if s.startswith("Error:"):
            self.toast(human_error(s[6:].strip()))

    def _device_name(self, device_id):
        for i, n in self.devices:
            if i == device_id or device_id.startswith(i) or i.startswith(device_id):
                return n
        return device_id

    def _add_recv_row(self, title, subtitle, file_path=None, url=None):
        if self.recv_empty.get_parent():
            self.recv_group.remove(self.recv_empty)
        row = Adw.ActionRow(title=esc(title), subtitle=esc(subtitle), activatable=True)
        row.add_prefix(Gtk.Image(icon_name="web-browser-symbolic" if url else "text-x-generic-symbolic"))
        if file_path:
            row.connect("activated", lambda *_: self.open_path(file_path))
            row.add_suffix(self._flat_button("folder-open-symbolic", T("Show in folder"),
                                             lambda *_: self.open_path(file_path, folder=True)))
        else:
            row.connect("activated", lambda *_: self.open_uri(url))
        # Newest first, keep the last 30.
        for r in self.recv_rows:
            self.recv_group.remove(r)
        self.recv_rows.insert(0, row)
        del self.recv_rows[30:]
        for r in self.recv_rows:
            self.recv_group.add(r)

    # ----- Devices -----
    def _build_devices_page(self):
        page = Adw.PreferencesPage()

        g = Adw.PreferencesGroup(title=T("This device"))
        self.me_row = Adw.ActionRow(title="…")
        self.me_row.add_prefix(Gtk.Image(icon_name="computer-symbolic"))
        self.me_row.add_suffix(self._flat_button("document-edit-symbolic", T("Rename"), self._rename))
        g.add(self.me_row)
        page.add(g)

        self.dev_group = Adw.PreferencesGroup(title=T("Paired devices"))
        hb = Gtk.Box(spacing=6)
        join = Gtk.Button(valign=Gtk.Align.CENTER,
                          child=Adw.ButtonContent(icon_name="insert-link-symbolic", label=T("Join with invite")))
        join.connect("clicked", lambda *_: PairDialog(self, "join").present(self))
        pair = Gtk.Button(valign=Gtk.Align.CENTER,
                          child=Adw.ButtonContent(icon_name="list-add-symbolic", label=T("Pair")))
        pair.add_css_class("suggested-action")
        pair.connect("clicked", lambda *_: PairDialog(self, "pair").present(self))
        hb.append(join)
        hb.append(pair)
        self.dev_group.set_header_suffix(hb)
        page.add(self.dev_group)

        self.net_group = Adw.PreferencesGroup(title=T("Devices on this network"),
                                              description=T("ECOnnector devices that are receiving files"))
        nb = Gtk.Box(spacing=6)
        self.net_spinner = make_spinner()
        self.net_spinner.set_visible(False)
        nb.append(self.net_spinner)
        nb.append(self._flat_button("view-refresh-symbolic", T("Refresh"), lambda *_: self.refresh_network()))
        self.net_group.set_header_suffix(nb)
        page.add(self.net_group)
        return page

    def refresh_identity(self):
        def done(ok, out):
            name = re.search(r"^Name:\s*(.+)$", out, re.M)
            device_id = re.search(r"^Id:\s*(\S+)", out, re.M)
            self.my_name = name.group(1).strip() if name else "?"
            self.me_row.set_title(esc(self.my_name))
            self.me_row.set_subtitle(device_id.group(1) if device_id else esc(out.strip()))
        run_ec(["id"], done)

    def _rename(self, *_):
        d = Adw.AlertDialog(heading=T("Rename this device"), body=T("Other devices see this device by this name."))
        entry = Gtk.Entry(text=self.my_name, activates_default=True)
        d.set_extra_child(entry)
        d.add_response("cancel", T("Cancel"))
        d.add_response("ok", T("Save"))
        d.set_response_appearance("ok", Adw.ResponseAppearance.SUGGESTED)
        d.set_default_response("ok")

        def response(_d, r):
            name = entry.get_text().strip()
            if r != "ok" or not name:
                return

            def done(ok, out):
                self.refresh_identity()
                self.restart_serve()
                self.toast(T("Renamed") if ok else out.strip())
            run_ec(["rename", name], done)
        d.connect("response", response)
        d.present(self)

    def refresh_devices(self):
        def done(ok, out):
            devices = []
            for line in out.splitlines():
                parts = line.strip().split(None, 1)
                if len(parts) == 2 and DEVICE_ID_RE.match(parts[0]):
                    devices.append((parts[0], parts[1].strip()))
            self._set_devices(devices)
        run_ec(["devices"], done)

    def _set_devices(self, devices):
        previous = self._selected_device()
        self.devices = devices
        self._clear_rows(self.dev_group, self.device_rows)
        if not devices:
            row = Adw.ActionRow(
                title=T("No paired devices yet"),
                subtitle=T("Use “Pair” to show a QR code, or join with the other device's invite link"))
            row.add_css_class("dim-label")
            self.dev_group.add(row)
            self.device_rows.append(row)
        for device_id, name in devices:
            row = Adw.ActionRow(title=esc(name), subtitle=device_id)
            row.add_prefix(Gtk.Image(icon_name="phone-symbolic"))
            row.add_suffix(self._flat_button("network-wireless-symbolic", T("Test connection"),
                                             lambda b, d=device_id, n=name: self._ping(b, d, n)))
            row.add_suffix(self._flat_button("user-trash-symbolic", T("Forget"),
                                             lambda _b, d=device_id, n=name: self._forget(d, n)))
            self.dev_group.add(row)
            self.device_rows.append(row)
        self.target_model.splice(0, self.target_model.get_n_items(), [f"{n}  ({i})" for i, n in devices])
        if previous in devices:
            self.target_row.set_selected(devices.index(previous))
        self.target_row.set_sensitive(bool(devices))
        self.target_row.set_subtitle("" if devices else T("Pair a device in the Devices tab first"))
        self._update_send_sensitive()

    def _ping(self, button, device_id, name):
        button.set_sensitive(False)
        self.toast(T("Connecting to {name}…", name=name))

        def done(ok, proc):
            button.set_sensitive(True)
            if ok:
                times = [float(x) for x in re.findall(r"ping \d+: ([\d.]+) ms", "\n".join(proc.lines))]
                avg = sum(times) / len(times) if times else 0
                self.toast(T("{name} is connected · average {ms} ms", name=name, ms=f"{avg:.2f}"))
            else:
                self.toast(f"{name}: " + human_error(proc.error_text()))
        ECProcess(["ping", device_id], on_exit=done)

    def _forget(self, device_id, name):
        d = Adw.AlertDialog(heading=T("Forget {name}?", name=name),
                            body=T("You will need to pair again to send or receive files."))
        d.add_response("cancel", T("Cancel"))
        d.add_response("forget", T("Forget"))
        d.set_response_appearance("forget", Adw.ResponseAppearance.DESTRUCTIVE)

        def response(_d, r):
            if r != "forget":
                return

            def done(ok, out):
                self.refresh_devices()
                self.restart_serve()
                self.toast(T("{name} forgotten", name=name) if ok else out.strip())
            run_ec(["forget", device_id], done)
        d.connect("response", response)
        d.present(self)

    def refresh_network(self):
        self.net_spinner.set_visible(True)

        def done(ok, out):
            self.net_spinner.set_visible(False)
            self._clear_rows(self.net_group, self.net_rows)
            for line in out.splitlines():
                cols = re.split(r"\s{2,}", line.strip())
                if len(cols) >= 3 and DEVICE_ID_RE.match(cols[0]):
                    paired = cols[2] == "paired"
                    sub = T("Paired") if paired else T("Not paired")
                    if len(cols) > 3:
                        sub += f" · {cols[3]}"
                    row = Adw.ActionRow(title=esc(cols[1]), subtitle=f"{cols[0]} · {sub}")
                    row.add_prefix(Gtk.Image(
                        icon_name="emblem-ok-symbolic" if paired else "network-workgroup-symbolic"))
                    self.net_group.add(row)
                    self.net_rows.append(row)
            if not self.net_rows:
                row = Adw.ActionRow(title=T("No devices found"),
                                    subtitle=T("The other device needs ECOnnector open with receiving on"))
                row.add_css_class("dim-label")
                self.net_group.add(row)
                self.net_rows.append(row)
        run_ec(["discover"], done)

    def on_paired(self):
        self.refresh_devices()
        self.restart_serve()

    def on_close(self, *_):
        self.serve_restart = False
        for proc in (self.serve_proc, self.send_proc):
            if proc and proc.running:
                proc.stop()
        return False


class App(Adw.Application):
    def __init__(self):
        super().__init__(application_id=APP_ID, flags=Gio.ApplicationFlags.DEFAULT_FLAGS)
        self.win = None
        for name, handler in (("open-url", self._open_url), ("show-file", self._show_file)):
            action = Gio.SimpleAction.new(name, GLib.VariantType.new("s"))
            action.connect("activate", handler)
            self.add_action(action)

    def do_startup(self):
        Adw.Application.do_startup(self)
        for sig in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
            if GLibUnix:
                GLibUnix.signal_add(GLib.PRIORITY_DEFAULT, sig, self._on_signal)
            else:
                GLib.unix_signal_add(GLib.PRIORITY_DEFAULT, sig, self._on_signal)

    def _on_signal(self):
        if self.win:
            self.win.on_close()
        GLib.timeout_add(600, lambda: (self.quit(), False)[1])
        return False

    def do_activate(self):
        if self.win:
            self.win.present()
            return
        if not EC:
            w = Adw.ApplicationWindow(application=self, title="ECOnnector")
            w.present()
            d = Adw.AlertDialog(heading=T("ec not found"), body=T(
                "The ECOnnector command line tool (ec) is not installed. Run install.sh from the release archive."))
            d.add_response("ok", T("OK"))
            d.connect("response", lambda *_: self.quit())
            d.present(w)
            return
        self.win = Window(self)
        self.win.present()

    def _open_url(self, _action, value):
        self.activate()
        if self.win:
            self.win.open_uri(value.get_string())

    def _show_file(self, _action, value):
        self.activate()
        if self.win:
            self.win.open_path(value.get_string(), folder=True)


def main():
    return App().run(sys.argv)


if __name__ == "__main__":
    sys.exit(main())
