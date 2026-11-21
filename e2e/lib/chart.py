#!/usr/bin/env python3
"""Chart resource CSVs written by sample.py."""

from __future__ import annotations

import csv
import sys
from collections import defaultdict
from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt


def load(path: Path) -> list[dict]:
    with path.open(newline="") as handle:
        return list(csv.DictReader(handle))


def f(row: dict, key: str) -> float:
    try:
        return float(row.get(key) or 0)
    except ValueError:
        return 0.0


def main() -> None:
    out_dir = Path(sys.argv[1])
    csvs = sorted(out_dir.glob("*/samples.csv"))
    if not csvs:
        print("no samples.csv")
        return
    fig, axes = plt.subplots(2, 1, figsize=(12, 7), sharex=False)
    for path in csvs:
        rows = load(path)
        if not rows:
            continue
        branch = path.parent.name
        t = [f(row, "t_ms") / 1000.0 for row in rows]
        rss = [(f(row, "rss_app_kb") + f(row, "rss_webkit_kb")) / 1024.0 for row in rows]
        proc_cpu = [f(row, "cpu_app_pct") + f(row, "cpu_webkit_pct") for row in rows]
        # Process comm is the binary file name (lixa-pr, lixa-develop). If the
        # sampler was matching only lixa-gallery, fall back to the sum of cores.
        if max(proc_cpu or [0]) < 1:
            cpu = [
                f(row, "core0_pct") + f(row, "core1_pct") + f(row, "core2_pct") + f(row, "core3_pct")
                for row in rows
            ]
        else:
            cpu = proc_cpu
        thumb = [f(row, "thumb_bytes") / (1024 * 1024) for row in rows]
        axes[0].plot(t, cpu, label=branch, linewidth=1.0)
        axes[1].plot(t, thumb, label=branch, linewidth=1.0)
        peak_rss = max(rss) if rss else 0
        peak_cpu = max(cpu) if cpu else 0
        print(f"{branch} samples={len(rows)} peak_rss_mb={peak_rss:.0f} peak_cpu_pct={peak_cpu:.0f}")
    axes[0].set_title("Machine CPU while the gallery was open (sum of 4 cores)")
    axes[0].set_ylabel("CPU (percent, 100 = one core)")
    axes[0].set_xlabel("Time since sampler start (s)")
    axes[0].legend()
    axes[0].grid(True, alpha=0.3)
    axes[1].set_title("Thumbnail cache size on disk")
    axes[1].set_ylabel("Cache (MB)")
    axes[1].set_xlabel("Time since sampler start (s)")
    axes[1].legend()
    axes[1].grid(True, alpha=0.3)
    fig.tight_layout()
    dest = out_dir / "resources.png"
    fig.savefig(dest, dpi=120)
    print(f"wrote {dest}")

    # Per-scenario peaks for the report table.
    summary = out_dir / "resource-by-scenario.csv"
    with summary.open("w", newline="") as handle:
        writer = csv.writer(handle)
        writer.writerow(["branch", "scenario", "samples", "peak_rss_mb", "peak_cpu_pct", "peak_fds", "peak_threads", "end_thumb_mb"])
        for path in csvs:
            rows = load(path)
            grouped: dict[str, list[dict]] = defaultdict(list)
            for row in rows:
                grouped[row.get("scenario") or "(unset)"].append(row)
            for name, group in grouped.items():
                rss = [(f(r, "rss_app_kb") + f(r, "rss_webkit_kb")) / 1024.0 for r in group]
                cpu = [f(r, "cpu_app_pct") + f(r, "cpu_webkit_pct") for r in group]
                if max(cpu or [0]) < 1:
                    cpu = [
                        f(r, "core0_pct") + f(r, "core1_pct") + f(r, "core2_pct") + f(r, "core3_pct")
                        for r in group
                    ]
                fds = [f(r, "fds_app") + f(r, "fds_webkit") for r in group]
                threads = [f(r, "threads_app") + f(r, "threads_webkit") for r in group]
                thumb = f(group[-1], "thumb_bytes") / (1024 * 1024) if group else 0
                writer.writerow(
                    [
                        path.parent.name,
                        name,
                        len(group),
                        round(max(rss) if rss else 0, 1),
                        round(max(cpu) if cpu else 0, 1),
                        int(max(fds) if fds else 0),
                        int(max(threads) if threads else 0),
                        round(thumb, 1),
                    ]
                )


if __name__ == "__main__":
    main()
