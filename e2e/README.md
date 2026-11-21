# Real-app end-to-end harness

Drives a **release** build of Lixa Gallery with tauri-driver and WebKitWebDriver under Xvfb. Photos are real Creative Commons / public-domain JPEGs from Wikimedia Commons and real CC0 camera RAW files from raw.pixls.us. Nothing in this folder changes application code.

## Prereqs

- Linux with a desktop WebKitGTK 4.1 stack: `libwebkit2gtk-4.1-0`, `webkit2gtk-driver` (`WebKitWebDriver` on `PATH`), `xvfb`, `openbox`, `xdotool`, `xclip`, ImageMagick `import`.
- Rust stable, and the Tauri Linux dev packages (`libgtk-3-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`, `patchelf`).
- Node 22 and `tauri-driver` 2.x (`cargo install tauri-driver --locked`).
- Python 3 with matplotlib (`python3-matplotlib`) for the resource chart.
- `perf` is optional; the scroll scenario records a flamegraph when it works.
- Cold runs call `sudo -n sh -c 'sync; echo 3 > /proc/sys/vm/drop_caches'`. If that is not permitted the run continues and records `droppedCaches: false`.

## Build a binary the harness can open

`cargo build --release` alone compiles the app in Tauri's **dev** mode, so the window tries to load `http://localhost:1420` and shows "Connection refused". Production builds enable Tauri's `custom-protocol` feature, which embeds `build/` and serves it from `tauri://localhost`. That is what `tauri build` does. From `src-tauri`:

```bash
# frontend first (repo root)
node .yarn/releases/yarn-4.9.2.cjs build
cd src-tauri
cargo build --release --features tauri/custom-protocol
```

The binary is `src-tauri/target/release/lixa-gallery`. Build each branch you want to compare. Do not mix a binary with another branch's frontend.

## Fixtures

```bash
python3 e2e/fixtures/fetch.py --cache e2e/fixtures/cache --sets e2e/fixtures/sets
```

The script is idempotent: complete files are skipped by checksum. It asks the Wikimedia Commons API for JPEGs roughly 12–48 MB with a Creative Commons, CC0, or public-domain license, and it crawls raw.pixls.us for CR2, CR3, NEF, ARW, RAF, ORF, RW2, and DNG. Details go to `e2e/fixtures/cache/fetch.log`. Stdout is one summary line.

Hardlinked folders (same inode, so the kernel page cache is shared across names):

| Set | What |
| --- | --- |
| `A-flat-2000` | 2,000 large JPEGs, one directory |
| `B-mixed-500` | about 500 files, including RAW+JPEG pairs that share a basename |
| `C-nested` | several folders of about 50–220 photos |
| `D-flat-1000` | a second large JPEG folder |
| `E-export` | 24 JPEGs reserved for export, so a move cannot relocate A or D |

`--layout-only` rebuilds those folders from whatever is already in the cache.

Wikimedia answers `429` if downloads are parallel. The fetcher retries with a delay. A re-run after a 429 continues from the cache.

## Run

From the repo root, after binaries exist:

```bash
LIXA_SKIP_FIXTURES=1 \
LIXA_SETS="$PWD/e2e/fixtures/sets" \
LIXA_BIN_PR=/path/to/pr/lixa-gallery \
LIXA_BIN_DEVELOP=/path/to/develop/lixa-gallery \
LIXA_RUNS=3 \
./e2e/run.sh
```

`LIXA_RUNS` is the repeat count for the timed scenarios (first load, folder switch, scroll). Default is 3.

One branch only:

```bash
LIXA_BIN=/path/to/lixa-gallery LIXA_BRANCH=pr ./e2e/run.sh
```

Omit `LIXA_SKIP_FIXTURES` to download (or resume) photos first. Cache and sets are gitignored.

## What it drives

`e2e/scenarios/run-all.mjs` opens the real window through tauri-driver. Timed folder opens use the sidebar **Recent Folders** button, which calls the same `loadPath` as a click on a previous session. On a fresh profile the harness writes `localStorage.recent_folders` and reloads, then clicks that button. `openFolder(..., { forceDialog: true })` also drives the GTK chooser (Ctrl+L, paste, Return, then a click on Open). On Xvfb the location bar accepts the path, but activating Open is unreliable; the timed path does not depend on it.

Scenarios, on each branch:

1. First load of A, D, B, the nested root, and `C-nested/shoot-03`, cold and warm, `LIXA_RUNS` times. Cold drops the page cache when permitted and uses a fresh XDG data dir. Warm goes Home and opens the same folder again so thumbnail files are reused.
2. Folder switch while the first folder is still decoding: early (~2s), mid (~15s), late (~40s). Counts "Failed to load" cells across a scroll sample.
3. Scroll: slow, flick to the bottom, End/Home, scroll during culling. Records `requestAnimationFrame` gaps and PerformanceObserver long tasks. The first run also `perf record`s the app for 12s.
4. Cull: 200 keyboard picks, a short run on the RAW+JPEG folder, quit, relaunch, reopen, compare the picked count.
5. Favourites: open, scroll, unpick, export copy, start a move and hit cancel.
6. Viewer: Enter, 50 arrow-rights, zoom `+`, then a RAW cell when one is on screen.
7. Settings: open the page and turn on Dark.

Each scenario gets its own app log (`LIXA_LOG_STDOUT=1`) and an isolated `XDG_DATA_HOME` except the persistence relaunch.

## How to read the output

Everything lands in `e2e/out/<branch>/` (or `LIXA_OUT`):

| File | Meaning |
| --- | --- |
| `results.jsonl` | One JSON object per measurement: `gridMs`, `firstThumbMs`, `visibleMs`, `failed`, `how`, frame stats, cull counts |
| `samples.csv` | Every 500 ms: per-core CPU, app and WebKit RSS, fds, threads, bytes read, thumbnail bytes on disk |
| `screenshots/` | Webview PNG plus a full X root PNG (`*-root.png`) |
| `logs/` | App stdout, including `Scan folder` / `cancelled` lines |
| `perf.script` and `flame.svg` | Present when `perf` could attach |
| `../resources.png` | RSS and CPU for every branch, from `e2e/lib/chart.py` |
| `../resource-by-scenario.csv` | Peak RSS, CPU, fds, threads by scenario |

`how` is `dialog`, `recent`, or `seeded-recent`. Timings for `dialog` include the file chooser. `gridMs` is measured after the path is submitted.

`e2e/lib/sample.py` is the sampler. `e2e/lib/chart.py` and `e2e/lib/flame.py` redraw charts from an existing out dir:

```bash
python3 e2e/lib/chart.py e2e/out
python3 e2e/lib/flame.py e2e/out/pr/perf.script e2e/out/pr/flame.svg
```

## Known limitations

- Hardlinked copies share the page cache. A cold run drops caches when sudo allows it; otherwise later folders look warmer than a reboot would.
- Virtualized grids only mount visible cells. "All visible thumbs" is the cells in the DOM, not all 2,000 files. Failure sweeps scroll a sample of positions and de-dupe names.
- The GTK file chooser is driven with xdotool. If the dialog does not focus, the scenario falls back to the Recent Folders control and records that in `how`.
- Export of hundreds of 20 MB files is not what the export scenario does. It exports the small `E-export` set so a move cannot destroy A or D.
- WebKit long tasks require the `longtask` entry type. The result records `longTasks: 0` when the webview does not emit them; frame gaps are still recorded.
- `perf` needs permission to sample the user process (`perf_event_paranoid` ≤ 2). Release binaries keep symbol names unless stripped; there are no source line numbers.
- One background WebKit process is sampled (the newest `WebKitWebProcess`). GPU and network processes are not in the RSS sum.
- Folder sets are rebuilt with hardlinks. Re-running layout does not shrink a directory that already has more files than the target.
- This harness does not start the Vite dev server. Do not point it at `cargo build --release` without `tauri/custom-protocol`.
