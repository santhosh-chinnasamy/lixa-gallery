#!/usr/bin/env bash
# tauri-driver launches this script. It applies the per-session environment
# (isolated XDG dirs, stdout log) and execs the real binary.
set -euo pipefail
ENV_FILE="${LIXA_E2E_ENV:-/tmp/lixa-e2e-current.env}"
if [[ ! -f "$ENV_FILE" ]]; then
  echo "missing e2e env file: $ENV_FILE" >&2
  exit 1
fi
set -a
# shellcheck disable=SC1090
source "$ENV_FILE"
set +a
export GDK_BACKEND="${GDK_BACKEND:-x11}"
export LIXA_LOG_STDOUT=1
mkdir -p "$(dirname "$LIXA_APP_LOG")"
exec >>"$LIXA_APP_LOG" 2>&1
exec "$LIXA_REAL_BIN"
