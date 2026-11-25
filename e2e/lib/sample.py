#!/usr/bin/env python3
"""Sample app + WebKit CPU, RSS, fds, threads, disk read, and thumb cache.

Writes one CSV row every 500 ms. The current scenario name is read from
$LIXA_SCENARIO_FILE so the chart can mark phases.
"""

from __future__ import annotations

import csv
import os
import sys
import time
from pathlib import Path

CLK = os.sysconf("SC_CLK_TCK")
INTERVAL = 0.5


def read_text(path: Path) -> str:
    try:
        return path.read_text(errors="replace")
    except OSError:
        return ""


def pids_for(name: str) -> list[int]:
    found: list[int] = []
    proc = Path("/proc")
    for entry in proc.iterdir():
        if not entry.name.isdigit():
            continue
        comm = read_text(entry / "comm").strip()
        if comm == name or name in comm:
            found.append(int(entry.name))
    return found


def proc_cpu_jiffies(pid: int) -> int | None:
    stat = read_text(Path(f"/proc/{pid}/stat"))
    if not stat:
        return None
    # comm is in parens and may contain spaces. Fields after the last ')'.
    try:
        rest = stat.rsplit(")", 1)[1].split()
        utime = int(rest[11])
        stime = int(rest[12])
    except (IndexError, ValueError):
        return None
    return utime + stime


def status_map(pid: int) -> dict[str, str]:
    out: dict[str, str] = {}
    for line in read_text(Path(f"/proc/{pid}/status")).splitlines():
        if ":" in line:
            key, value = line.split(":", 1)
            out[key.strip()] = value.strip()
    return out


def rss_kb(pid: int) -> int:
    raw = status_map(pid).get("VmRSS", "0").split()
    try:
        return int(raw[0])
    except (IndexError, ValueError):
        return 0


def threads(pid: int) -> int:
    try:
        return int(status_map(pid).get("Threads", "0"))
    except ValueError:
        return 0


def fd_count(pid: int) -> int:
    fd_dir = Path(f"/proc/{pid}/fd")
    try:
        return sum(1 for _ in fd_dir.iterdir())
    except OSError:
        return 0


def read_bytes(pid: int) -> int:
    for line in read_text(Path(f"/proc/{pid}/io")).splitlines():
        if line.startswith("read_bytes:"):
            try:
                return int(line.split()[1])
            except (IndexError, ValueError):
                return 0
    return 0


def core_jiffies() -> list[int]:
    cores: list[int] = []
    for line in read_text(Path("/proc/stat")).splitlines():
        if not line.startswith("cpu") or line.startswith("cpu "):
            continue
        parts = line.split()[1:]
        cores.append(sum(int(part) for part in parts))
    return cores


def newest_pid(name: str) -> int | None:
    pids = pids_for(name)
    if not pids and name == "lixa-gallery":
        # The harness copies binaries to lixa-pr / lixa-develop. Match any of them.
        pids = [pid for pid, _comm in ((pid, "") for pid in [])]
        found = []
        proc = Path("/proc")
        for entry in proc.iterdir():
            if not entry.name.isdigit():
                continue
            comm = read_text(entry / "comm").strip()
            if comm.startswith("lixa"):
                found.append(int(entry.name))
        pids = found
    if not pids:
        return None
    return max(pids)


def thumb_bytes(root: Path | None) -> int:
    if root is None or not root.exists():
        return 0
    total = 0
    for dirpath, _dirs, files in os.walk(root):
        if ".thumbnails" not in dirpath and not dirpath.endswith(".thumbnails"):
            # Still walk; the cache lives under the XDG data dir.
            pass
        for name in files:
            if not name.endswith(".webp"):
                continue
            try:
                total += (Path(dirpath) / name).stat().st_size
            except OSError:
                continue
    return total


def main() -> None:
    out = Path(sys.argv[1])
    thumb_root = Path(sys.argv[2]) if len(sys.argv) > 2 else None
    scenario_file = Path(os.environ.get("LIXA_SCENARIO_FILE", ""))
    out.parent.mkdir(parents=True, exist_ok=True)
    prev_app: tuple[int, int, float] | None = None
    prev_wk: tuple[int, int, float] | None = None
    prev_cores: list[int] | None = None
    prev_core_t = time.monotonic()
    with out.open("w", newline="") as handle:
        writer = csv.writer(handle)
        writer.writerow(
            [
                "t_ms",
                "scenario",
                "pid_app",
                "pid_webkit",
                "cpu_app_pct",
                "cpu_webkit_pct",
                "rss_app_kb",
                "rss_webkit_kb",
                "fds_app",
                "fds_webkit",
                "threads_app",
                "threads_webkit",
                "read_app",
                "read_webkit",
                "core0_pct",
                "core1_pct",
                "core2_pct",
                "core3_pct",
                "thumb_bytes",
            ]
        )
        start = time.time()
        while True:
            now = time.time()
            mono = time.monotonic()
            scenario = ""
            if scenario_file and scenario_file.is_file():
                scenario = scenario_file.read_text(errors="replace").strip()
            app = newest_pid("lixa-gallery")
            wk = newest_pid("WebKitWebProcess")
            if wk is None:
                wk = newest_pid("WebKitNetworkProcess")

            def cpu_pct(pid: int | None, prev: tuple[int, int, float] | None):
                if pid is None:
                    return 0.0, None
                jiffies = proc_cpu_jiffies(pid)
                if jiffies is None:
                    return 0.0, None
                if prev is None or prev[0] != pid:
                    return 0.0, (pid, jiffies, mono)
                dt = mono - prev[2]
                if dt <= 0:
                    return 0.0, (pid, jiffies, mono)
                pct = ((jiffies - prev[1]) / CLK) / dt * 100.0
                return max(0.0, pct), (pid, jiffies, mono)

            app_pct, prev_app = cpu_pct(app, prev_app)
            wk_pct, prev_wk = cpu_pct(wk, prev_wk)
            cores = core_jiffies()
            core_pcts = [0.0, 0.0, 0.0, 0.0]
            dt_cores = mono - prev_core_t
            if prev_cores is not None and dt_cores > 0:
                for i in range(min(4, len(cores), len(prev_cores))):
                    core_pcts[i] = max(0.0, ((cores[i] - prev_cores[i]) / CLK) / dt_cores * 100.0)
            prev_cores = cores
            prev_core_t = mono
            writer.writerow(
                [
                    int((now - start) * 1000),
                    scenario,
                    app or 0,
                    wk or 0,
                    round(app_pct, 2),
                    round(wk_pct, 2),
                    rss_kb(app) if app else 0,
                    rss_kb(wk) if wk else 0,
                    fd_count(app) if app else 0,
                    fd_count(wk) if wk else 0,
                    threads(app) if app else 0,
                    threads(wk) if wk else 0,
                    read_bytes(app) if app else 0,
                    read_bytes(wk) if wk else 0,
                    round(core_pcts[0], 2),
                    round(core_pcts[1], 2),
                    round(core_pcts[2], 2),
                    round(core_pcts[3], 2),
                    thumb_bytes(thumb_root),
                ]
            )
            handle.flush()
            time.sleep(INTERVAL)


if __name__ == "__main__":
    main()
