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

cd "$E2E"
if [[ ! -d node_modules/selenium-webdriver ]]; then
  npm install --no-audit --no-fund
fi
chmod +x "$E2E/lib/launch.sh" "$E2E/fixtures/fetch.py" "$E2E/run.sh"
mkdir -p "$OUT"

if [[ "${LIXA_SKIP_FIXTURES:-0}" != "1" ]]; then
  python3 "$E2E/fixtures/fetch.py" --cache "$CACHE" --sets "$SETS"
  python3 "$E2E/fixtures/fetch.py" --layout-only --cache "$CACHE" --sets "$SETS"
fi

if [[ ! -S "/tmp/.X11-unix/X${DISPLAY_NUM#:}" ]]; then
  Xvfb "$DISPLAY_NUM" -screen 0 1440x900x24 >"$OUT/xvfb.log" 2>&1 &
  echo $! >"$OUT/xvfb.pid"
  sleep 0.4
fi
export DISPLAY="$DISPLAY_NUM"
if [[ -S /run/user/$(id -u)/bus ]]; then
  export DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/$(id -u)/bus"
fi
if ! pgrep -f "openbox" >/dev/null 2>&1; then
  openbox >"$OUT/openbox.log" 2>&1 &
fi

export LIXA_E2E_ENV="${LIXA_E2E_ENV:-/tmp/lixa-e2e-current.env}"
export LIXA_LAUNCHER="$E2E/lib/launch.sh"
export TAURI_DRIVER_URL="${TAURI_DRIVER_URL:-http://127.0.0.1:4444/}"

if ! pgrep -x tauri-driver >/dev/null 2>&1; then
  tauri-driver --port 4444 >"$OUT/tauri-driver.log" 2>&1 &
  echo $! >"$OUT/tauri-driver.pid"
  sleep 0.4
fi

run_branch() {
  local name="$1" bin="$2"
  mkdir -p "$OUT/$name"
  export LIXA_SCENARIO_FILE="$OUT/$name/scenario.txt"
  python3 "$E2E/lib/sample.py" "$OUT/$name/samples.csv" "$OUT/$name/xdg" &
  local sampler=$!
  set +e
  LIXA_BIN="$bin" LIXA_SETS="$SETS" LIXA_OUT="$OUT/$name" LIXA_BRANCH="$name" LIXA_RUNS="$RUNS" \
    node "$E2E/scenarios/run-all.mjs"
  local status=$?
  set -e
  kill "$sampler" >/dev/null 2>&1 || true
  wait "$sampler" 2>/dev/null || true
  echo "branch $name exit $status"
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

python3 "$E2E/lib/chart.py" "$OUT" || true
if [[ -f "$OUT/pr/perf.script" ]]; then
  python3 "$E2E/lib/flame.py" "$OUT/pr/perf.script" "$OUT/pr/flame.svg" || true
fi
if [[ -f "$OUT/develop/perf.script" ]]; then
  python3 "$E2E/lib/flame.py" "$OUT/develop/perf.script" "$OUT/develop/flame.svg" || true
fi

echo "results in $OUT"
