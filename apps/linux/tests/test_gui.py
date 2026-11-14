"""Tests for the parts of the Linux app that read `ec` output.

Run from the repository root:  python3 -m unittest discover -s apps/linux/tests
Needs python3-gi with GTK 4 and libadwaita, but no display.
"""

import os
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
os.environ["EC_BIN"] = os.path.join(HERE, "fake_ec.sh")
os.environ["LANGUAGE"] = "en"
sys.path.insert(0, os.path.dirname(HERE))

import econnector_gui as gui  # noqa: E402
from gi.repository import GLib  # noqa: E402


def run(args, answer=None, timeout=10):
    loop = GLib.MainLoop()
    seen = {"lines": [], "prompts": []}

    def on_prompt(p):
        seen["prompts"].append(p)
        proc.answer(answer)

    def on_exit(ok, _proc):
        seen["ok"] = ok
        loop.quit()

    proc = gui.ECProcess(args, on_line=seen["lines"].append, on_prompt=on_prompt, on_exit=on_exit)
    GLib.timeout_add_seconds(timeout, loop.quit)
    loop.run()
    seen["proc"] = proc
    return seen


class ParseTests(unittest.TestCase):
    def test_qr_half_blocks(self):
        m = gui.parse_qr(["    █▀▄ ", "    ▄ █ ", "plain text"])
        self.assertEqual(m, [
            [True, True, False, False],
            [True, False, True, False],
            [False, False, True, False],
            [True, False, True, False],
        ])

    def test_qr_absent(self):
        self.assertIsNone(gui.parse_qr(["no code here"]))

    def test_progress(self):
        m = gui.PROGRESS_RE.match("  100.0%  30.0 MB / 30.0 MB  10.0 MB/s, 2 path(s)    ")
        self.assertEqual(m.group(1), "100.0")
        self.assertEqual(m.group(2), "30.0 MB / 30.0 MB  10.0 MB/s, 2 path(s)")

    def test_device_id(self):
        self.assertTrue(gui.DEVICE_ID_RE.match("13E5-A973"))
        self.assertFalse(gui.DEVICE_ID_RE.match("No"))

    def test_friendly_errors(self):
        self.assertIn("receiving", gui.human_error("timed out"))
        self.assertEqual(gui.human_error("something else"), "something else")


class ProcessTests(unittest.TestCase):
    def test_pair_confirmed(self):
        seen = run(["pair"], answer=True)
        self.assertTrue(seen["ok"])
        self.assertEqual(len(seen["prompts"]), 1)
        self.assertIn("123 456", seen["prompts"][0])
        self.assertIn("Paired with Test (1234-ABCD).", seen["lines"])
        self.assertIsNotNone(gui.parse_qr(seen["proc"].lines))

    def test_pair_declined(self):
        seen = run(["pair"], answer=False)
        self.assertFalse(seen["ok"])
        self.assertIn("declined", seen["proc"].error_text())

    def test_send_progress_lines_split_on_carriage_return(self):
        seen = run(["send", "1234", "/tmp/x"])
        self.assertTrue(seen["ok"])
        progress = [line for line in seen["lines"] if gui.PROGRESS_RE.match(line)]
        self.assertEqual(len(progress), 2)
        self.assertTrue(seen["lines"][-1].startswith("Sent 1 file(s)"))


@unittest.skipUnless(os.environ.get("EC_REAL"), "set EC_REAL to a built `ec` to run against the real tool")
class RealEcTests(unittest.TestCase):
    """Guards the output wording this app parses, using the real `ec` on two local devices."""

    def setUp(self):
        import tempfile
        self.tmp = tempfile.mkdtemp()
        self.addCleanup(__import__("shutil").rmtree, self.tmp, True)
        self.old = gui.EC
        gui.EC = os.environ["EC_REAL"]
        self.addCleanup(setattr, gui, "EC", self.old)
        self.a = os.path.join(self.tmp, "a")
        self.b = os.path.join(self.tmp, "b")

    def test_pair_send_receive(self):
        import re
        loop = GLib.MainLoop()
        state = {}

        def a_line(line):
            m = re.search(r"(ecpair:[^'\s]+)", line)
            if m:
                state["qr"] = gui.parse_qr(pair.lines)
                state["join"] = gui.ECProcess(
                    ["--home", self.b, "join", m.group(1)],
                    on_prompt=lambda _p: state["join"].answer(True),
                    on_exit=lambda ok, _p: done("join_ok", ok))

        def done(key, ok):
            state[key] = ok
            if "pair_ok" in state and "join_ok" in state:
                loop.quit()

        pair = gui.ECProcess(["--home", self.a, "pair"], on_line=a_line,
                             on_prompt=lambda _p: pair.answer(True),
                             on_exit=lambda ok, _p: done("pair_ok", ok))
        GLib.timeout_add_seconds(30, loop.quit)
        loop.run()
        self.assertTrue(state.get("pair_ok"))
        self.assertTrue(state.get("join_ok"))
        self.assertTrue(state["qr"] and len(state["qr"]) == len(state["qr"][0]), "QR must be square")

        recv = os.path.join(self.tmp, "recv")
        payload = os.path.join(self.tmp, "hello.txt")
        with open(payload, "w") as f:
            f.write("hello")
        events = []
        loop = GLib.MainLoop()

        def serve_line(line):
            events.append(line.strip())
            if "is online on UDP port" in line:
                a_id = [ln for ln in self._ec(["--home", self.a, "id"]).splitlines() if ln.startswith("Id:")][0]
                short = a_id.split()[1]
                gui.ECProcess(["--home", self.b, "send", short, payload],
                              on_exit=lambda ok, p: (events.append(("sent", ok, p.lines[-1])), None))
            if line.strip().startswith("saved "):
                GLib.timeout_add(300, lambda: (loop.quit(), False)[1])

        serve = gui.ECProcess(["--home", self.a, "serve", "--port", "0", "--dir", recv], on_line=serve_line)
        GLib.timeout_add_seconds(30, loop.quit)
        loop.run()
        serve.stop()
        self.assertTrue(any(isinstance(e, str) and re.match(r"Receiving 1 file\(s\), .+ from \S+", e)
                            for e in events), events)
        self.assertTrue(os.path.exists(os.path.join(recv, "hello.txt")))
        sent = [e for e in events if isinstance(e, tuple)]
        self.assertTrue(sent and sent[0][1] and sent[0][2].startswith("Sent 1 file(s)"), sent)

    @staticmethod
    def _ec(args):
        import subprocess
        return subprocess.run([gui.EC] + args, capture_output=True, text=True, check=True).stdout


if __name__ == "__main__":
    unittest.main()
