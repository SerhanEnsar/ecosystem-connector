#!/usr/bin/env python3
"""Turns bench/out/loopback.csv into docs/media/bench-loopback.svg.

No dependencies beyond the standard library. The chart shows the median transfer
speed per stream count, with a thin line from the slowest to the fastest run.

Usage: python3 bench/plot_loopback.py "<machine description>"
"""

import csv
import statistics
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CSV = ROOT / "bench" / "out" / "loopback.csv"
SVG = ROOT / "docs" / "media" / "bench-loopback.svg"

W, H = 640, 360
LEFT, RIGHT, TOP, BOTTOM = 64, 24, 72, 56
BAR = 24


def main() -> None:
    machine = sys.argv[1] if len(sys.argv) > 1 else "unknown machine"
    commit = subprocess.run(
        ["git", "rev-parse", "--short", "HEAD"], capture_output=True, text=True, cwd=ROOT
    ).stdout.strip()
    rows = list(csv.DictReader(CSV.open()))
    streams = sorted({int(r["streams"]) for r in rows if r["kind"] == "transfer"})
    runs = {
        s: [float(r["mb_per_s"]) for r in rows if r["kind"] == "transfer" and int(r["streams"]) == s]
        for s in streams
    }
    stats = {s: (statistics.median(v), min(v), max(v), len(v)) for s, v in runs.items()}

    top = max(hi for _, _, hi, _ in stats.values())
    step = 100
    y_max = (int(top // step) + 1) * step
    plot_w, plot_h = W - LEFT - RIGHT, H - TOP - BOTTOM

    def y(v: float) -> float:
        return TOP + plot_h * (1 - v / y_max)

    band = plot_w / len(streams)
    out = [
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" width="{W}" height="{H}" '
        'role="img" aria-labelledby="t d" font-family="-apple-system, Segoe UI, Roboto, sans-serif">',
        "<style>",
        ":root{--surface:#fcfcfb;--ink:#0b0b0b;--ink2:#52514e;--grid:#e4e3df;--series:#2a78d6}",
        "@media (prefers-color-scheme: dark){:root{--surface:#1a1a19;--ink:#f0efec;--ink2:#c3c2b7;"
        "--grid:#3a3936;--series:#3987e5}}",
        "text{fill:var(--ink2);font-size:12px}.title{fill:var(--ink);font-size:15px;font-weight:600}",
        ".value{fill:var(--ink);font-size:12px;font-weight:600}",
        "</style>",
        '<title id="t">Loopback transfer speed by parallel streams</title>',
        f'<desc id="d">Median MB/s over {stats[streams[0]][3]} runs of a 1 GiB file, {machine}, commit {commit}.</desc>',
        f'<rect width="{W}" height="{H}" rx="8" fill="var(--surface)"/>',
        f'<text class="title" x="{LEFT}" y="28">Transfer speed on loopback (MB/s, median)</text>',
        f'<text x="{LEFT}" y="48">1 GiB file, QUIC + TLS 1.3 + BLAKE3 + disk writes · {machine} · commit {commit}</text>',
    ]
    for v in range(0, y_max + 1, step):
        out.append(f'<line x1="{LEFT}" x2="{W - RIGHT}" y1="{y(v):.1f}" y2="{y(v):.1f}" stroke="var(--grid)" stroke-width="1"/>')
        out.append(f'<text x="{LEFT - 8}" y="{y(v) + 4:.1f}" text-anchor="end">{v}</text>')

    for i, s in enumerate(streams):
        median, lo, hi, n = stats[s]
        cx = LEFT + band * (i + 0.5)
        x0, base, top_y = cx - BAR / 2, y(0), y(median)
        r = 4
        # Column with a 4px rounded top and a square base.
        out.append(
            f'<path d="M{x0:.1f},{base:.1f} V{top_y + r:.1f} Q{x0:.1f},{top_y:.1f} {x0 + r:.1f},{top_y:.1f} '
            f'H{x0 + BAR - r:.1f} Q{x0 + BAR:.1f},{top_y:.1f} {x0 + BAR:.1f},{top_y + r:.1f} V{base:.1f} Z" '
            f'fill="var(--series)"><title>{s} streams: median {median:.0f} MB/s (min {lo:.0f}, max {hi:.0f}, {n} runs)</title></path>'
        )
        # Range of the runs, beside the column so it never hides the value.
        rx = x0 + BAR + 8
        out.append(f'<line x1="{rx:.1f}" x2="{rx:.1f}" y1="{y(lo):.1f}" y2="{y(hi):.1f}" stroke="var(--ink2)" stroke-width="2" stroke-linecap="round"/>')
        out.append(f'<text class="value" x="{cx:.1f}" y="{top_y - 8:.1f}" text-anchor="middle">{median:.0f}</text>')
        out.append(f'<text x="{cx:.1f}" y="{base + 20:.1f}" text-anchor="middle">{s}</text>')

    out.append(f'<text x="{LEFT + plot_w / 2:.1f}" y="{H - 12}" text-anchor="middle">parallel chunk streams · thin line: slowest to fastest run</text>')
    out.append("</svg>")
    SVG.parent.mkdir(parents=True, exist_ok=True)
    SVG.write_text("\n".join(out) + "\n")

    print("| Streams | Median MB/s | Min | Max |")
    print("|---|---|---|---|")
    for s in streams:
        median, lo, hi, _ = stats[s]
        print(f"| {s} | {median:.0f} | {lo:.0f} | {hi:.0f} |")
    print(f"wrote {SVG.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
