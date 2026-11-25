#!/usr/bin/env python3
"""Fold `perf script` output into a simple SVG flamegraph.

Release binaries keep symbol names unless stripped, so frames resolve without
DWARF line tables. Hex-only frames are kept so a missing symbol is visible.
"""

from __future__ import annotations

import sys
from collections import Counter
from pathlib import Path
from xml.sax.saxutils import escape


def fold(path: Path) -> Counter:
    counts: Counter = Counter()
    stack: list[str] = []
    for line in path.read_text(errors="replace").splitlines():
        if not line.strip():
            if stack:
                counts[";".join(reversed(stack))] += 1
                stack = []
            continue
        if line.startswith("\t") or line.startswith("        "):
            frame = line.strip().split()[0]
            frame = frame.split("+", 1)[0]
            if frame.startswith("("):
                continue
            stack.append(frame[:80])
        elif stack:
            counts[";".join(reversed(stack))] += 1
            stack = []
    if stack:
        counts[";".join(reversed(stack))] += 1
    return counts


def svg(counts: Counter, dest: Path) -> None:
    rows = sorted(counts.items(), key=lambda item: item[0])
    total = sum(counts.values()) or 1
    width = 1200
    row_h = 16
    # Build prefix weights.
    prefix: Counter = Counter()
    for stack, count in counts.items():
        parts = stack.split(";")
        for i in range(1, len(parts) + 1):
            prefix[";".join(parts[:i])] += count
    depth = max((key.count(";") + 1 for key in prefix), default=1)
    height = 40 + depth * row_h
    parts_xml = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}">',
        '<rect width="100%" height="100%" fill="#ffffff"/>',
        f'<text x="8" y="18" font-size="14" font-family="sans-serif">Lixa Gallery perf samples ({total} stacks)</text>',
    ]
    # Assign x positions by walking sorted stacks.
    x_of: dict[str, float] = {}
    cursor = 0.0
    for stack, count in rows:
        parts = stack.split(";")
        acc = ""
        for i, part in enumerate(parts):
            acc = part if i == 0 else acc + ";" + part
            if acc not in x_of:
                # place later
                pass
        cursor += count
    # Second pass: x is the start of the first stack that has this prefix.
    seen_start: dict[str, float] = {}
    cursor = 0.0
    for stack, count in rows:
        parts = stack.split(";")
        acc = ""
        for i, part in enumerate(parts):
            acc = part if i == 0 else acc + ";" + part
            seen_start.setdefault(acc, cursor)
        cursor += count
    for key, count in prefix.items():
        depth_n = key.count(";")
        x = seen_start.get(key, 0) / total * width
        w = count / total * width
        if w < 0.4:
            continue
        y = 28 + (depth - 1 - depth_n) * row_h
        name = key.rsplit(";", 1)[-1]
        parts_xml.append(
            f'<g><rect x="{x:.2f}" y="{y}" width="{w:.2f}" height="{row_h - 1}" fill="#c6d4e1" stroke="#ffffff"/>'
            f'<text x="{x + 2:.2f}" y="{y + 12}" font-size="10" font-family="sans-serif">{escape(name)}</text></g>'
        )
    parts_xml.append("</svg>")
    dest.write_text("\n".join(parts_xml), encoding="utf-8")


def main() -> None:
    src = Path(sys.argv[1])
    dest = Path(sys.argv[2])
    counts = fold(src)
    if not counts:
        print("flamegraph: no stacks")
        return
    svg(counts, dest)
    print(f"flame stacks={sum(counts.values())} unique={len(counts)} svg={dest.name}")


if __name__ == "__main__":
    main()
