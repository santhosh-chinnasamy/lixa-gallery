#!/usr/bin/env bash
# One-command real-app run.
#   ./e2e/run.sh
# Point it at binaries you already built:
#   LIXA_BIN_PR=.../lixa-gallery LIXA_BIN_DEVELOP=.../lixa-gallery ./e2e/run.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
E2E="$ROOT/e2e"
OUT="${LIXA_OUT:-$E2E/out}"
SETS="${LIXA_SETS:-$E2E/fixtures/sets}"
CACHE="${LIXA_FIXTURE_CACHE:-$E2E/fixtures/cache}"
RUNS="${LIXA_RUNS:-3}"
DISPLAY_NUM="${LIXA_DISPLAY:-:99}"

log() {
  printf '%s %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$*"
}

cleanup() {
  local status=$?
  log "cleanup: run.sh exiting status=${status}"
}
trap cleanup EXIT

require_cmd() {
  local cmd="$1"
  if ! command -v "$cmd" >/dev/null 2>&1; then
    log "prereq missing: ${cmd}"
    exit 1
  fi
  log "prereq ok: ${cmd} ($(command -v "$cmd"))"
}

log "start: root=${ROOT} out=${OUT} runs=${RUNS} display=${DISPLAY_NUM}"
log "prereq: checking tools"
require_cmd node
require_cmd python3
require_cmd Xvfb
require_cmd openbox
require_cmd xdotool
require_cmd tauri-driver
require_cmd WebKitWebDriver

log "prereq: node modules"
cd "$E2E"
if [[ ! -d node_modules/selenium-webdriver ]]; then
  log "prereq: npm install selenium-webdriver"
  npm install --no-audit --no-fund
else
  log "prereq: selenium-webdriver already installed"
fi
chmod +x "$E2E/lib/launch.sh" "$E2E/fixtures/fetch.py" "$E2E/run.sh"
mkdir -p "$OUT"
log "prereq: output directory ready ${OUT}"

if [[ "${LIXA_SKIP_FIXTURES:-0}" != "1" ]]; then
  log "fixtures: download start cache=${CACHE}"
  mkdir -p "$CACHE"
  python3 -u "$E2E/fixtures/fetch.py" --cache "$CACHE" --sets "$SETS"
  log "fixtures: download finished, refreshing layout"
  python3 -u "$E2E/fixtures/fetch.py" --layout-only --cache "$CACHE" --sets "$SETS"
  log "fixtures: layout finished sets=${SETS}"
else
  log "fixtures: skipped (LIXA_SKIP_FIXTURES=1) sets=${SETS}"
  if [[ ! -d "$SETS" ]]; then
    log "fixtures: missing sets directory ${SETS}"
    exit 1
  fi
fi

log "display: checking Xvfb ${DISPLAY_NUM}"
if [[ ! -S "/tmp/.X11-unix/X${DISPLAY_NUM#:}" ]]; then
  log "display: starting Xvfb ${DISPLAY_NUM}"
  Xvfb "$DISPLAY_NUM" -screen 0 1440x900x24 >"$OUT/xvfb.log" 2>&1 &
  echo $! >"$OUT/xvfb.pid"
  sleep 0.4
else
  log "display: Xvfb ${DISPLAY_NUM} already up"
fi
export DISPLAY="$DISPLAY_NUM"
if [[ -S /run/user/$(id -u)/bus ]]; then
  export DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/$(id -u)/bus"
  log "display: using session bus"
else
  log "display: no user session bus"
fi
if ! pgrep -f "openbox" >/dev/null 2>&1; then
  log "display: starting openbox"
  openbox >"$OUT/openbox.log" 2>&1 &
else
  log "display: openbox already running"
fi

export LIXA_E2E_ENV="${LIXA_E2E_ENV:-/tmp/lixa-e2e-current.env}"
export LIXA_LAUNCHER="$E2E/lib/launch.sh"
export TAURI_DRIVER_URL="${TAURI_DRIVER_URL:-http://127.0.0.1:4444/}"

if ! pgrep -x tauri-driver >/dev/null 2>&1; then
  log "driver: starting tauri-driver on ${TAURI_DRIVER_URL}"
  tauri-driver --port 4444 >"$OUT/tauri-driver.log" 2>&1 &
  echo $! >"$OUT/tauri-driver.pid"
  sleep 0.4
else
  log "driver: tauri-driver already running"
fi

run_branch() {
  local name="$1" bin="$2"
  if [[ ! -x "$bin" ]]; then
    log "branch ${name}: binary is not executable: ${bin}"
    return 0
  fi
  log "branch start: ${name} bin=${bin} runs=${RUNS}"
  mkdir -p "$OUT/$name"
  export LIXA_SCENARIO_FILE="$OUT/$name/scenario.txt"
  log "branch ${name}: resource sampler start -> ${OUT}/${name}/samples.csv"
  python3 -u "$E2E/lib/sample.py" "$OUT/$name/samples.csv" "$OUT/$name/xdg" &
  local sampler=$!
  set +e
  # stdbuf keeps scenario start/finish lines visible as they are printed.
  LIXA_BIN="$bin" LIXA_SETS="$SETS" LIXA_OUT="$OUT/$name" LIXA_BRANCH="$name" LIXA_RUNS="$RUNS" \
    stdbuf -oL -eL node "$E2E/scenarios/run-all.mjs"
  local status=$?
  set -e
  log "branch ${name}: stopping resource sampler"
  kill "$sampler" >/dev/null 2>&1 || true
  wait "$sampler" 2>/dev/null || true
  log "branch finish: ${name} exit=${status}"
  log "branch ${name}: mid-decode folder switch"
  set +e
  LIXA_BIN="$bin" LIXA_BRANCH="$name" LIXA_OUT="$OUT/$name" LIXA_RUNS="$RUNS" \
    LIXA_FOLDER_A="$SETS/A-flat-2000" LIXA_FOLDER_D="$SETS/D-flat-1000" \
    stdbuf -oL -eL node "$E2E/scenarios/switch-during-decode.mjs"
  local switch_status=$?
  set -e
  log "branch ${name}: mid-decode switch exit=${switch_status}"
  return 0
}

if [[ -n "${LIXA_BIN_PR:-}" ]]; then
  run_branch pr "$LIXA_BIN_PR"
fi
if [[ -n "${LIXA_BIN_DEVELOP:-}" ]]; then
  run_branch develop "$LIXA_BIN_DEVELOP"
fi
if [[ -z "${LIXA_BIN_PR:-}${LIXA_BIN_DEVELOP:-}" && -n "${LIXA_BIN:-}" ]]; then
  run_branch "${LIXA_BRANCH:-app}" "$LIXA_BIN"
fi
if [[ -z "${LIXA_BIN_PR:-}${LIXA_BIN_DEVELOP:-}${LIXA_BIN:-}" ]]; then
  log "branch: no LIXA_BIN_PR, LIXA_BIN_DEVELOP, or LIXA_BIN set; nothing to drive"
fi

log "plots: resource chart"
python3 "$E2E/lib/chart.py" "$OUT" || log "plots: chart.py failed"
if [[ -f "$OUT/pr/perf.script" ]]; then
  log "plots: pr flamegraph"
  python3 "$E2E/lib/flame.py" "$OUT/pr/perf.script" "$OUT/pr/flame.svg" || log "plots: pr flamegraph failed"
fi
if [[ -f "$OUT/develop/perf.script" ]]; then
  log "plots: develop flamegraph"
  python3 "$E2E/lib/flame.py" "$OUT/develop/perf.script" "$OUT/develop/flame.svg" || log "plots: develop flamegraph failed"
fi
for script in "$OUT"/*/perf.script; do
  [[ -f "$script" ]] || continue
  branch="$(basename "$(dirname "$script")")"
  if [[ "$branch" != "pr" && "$branch" != "develop" ]]; then
    log "plots: ${branch} flamegraph"
    python3 "$E2E/lib/flame.py" "$script" "$(dirname "$script")/flame.svg" || log "plots: ${branch} flamegraph failed"
  fi
done

log "done: results in ${OUT}"
