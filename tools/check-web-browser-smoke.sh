#!/usr/bin/env bash
# Browser smoke for the Rust CanvasKit web bundle.
#
# Covers both browser paths:
#   1. `/smoke/step-1b.html` — pure CanvasKit mount harness.
#   2. `/` — daemon-served host page, including `/api/mcp/sync-reset`.
#
# The script intentionally uses Chrome/Chromium headless directly instead of a
# new test dependency. The pages write `data-op-smoke="ok"` only after the wasm
# bundle has loaded and `mount_ck()` has resolved.
set -euo pipefail

cd "$(dirname "$0")/.."

fail() {
  printf 'FAIL: %s\n' "$1" >&2
  exit 1
}

need() {
  command -v "$1" >/dev/null 2>&1 || {
    printf 'missing prerequisite: %s\n' "$1" >&2
    exit 2
  }
}

find_chrome() {
  if [ -n "${OPENPENCIL_CHROME_BIN:-}" ]; then
    printf '%s\n' "$OPENPENCIL_CHROME_BIN"
    return
  fi
  for bin in google-chrome-stable google-chrome chromium chromium-browser; do
    if command -v "$bin" >/dev/null 2>&1; then
      command -v "$bin"
      return
    fi
  done
  for app in \
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" \
    "/Applications/Chromium.app/Contents/MacOS/Chromium"; do
    if [ -x "$app" ]; then
      printf '%s\n' "$app"
      return
    fi
  done
  return 1
}

pick_port() {
  python3 - <<'PY'
import socket
s = socket.socket()
s.bind(("127.0.0.1", 0))
print(s.getsockname()[1])
s.close()
PY
}

wait_for_health() {
  local url="$1"
  local tries="${OPENPENCIL_BROWSER_SMOKE_WAIT_TRIES:-100}"
  for _ in $(seq 1 "$tries"); do
    if curl -fsS "$url" >/dev/null 2>&1; then
      return 0
    fi
    sleep 0.1
  done
  return 1
}

assert_contains() {
  local file="$1"
  local needle="$2"
  local label="$3"
  if ! grep -Fq "$needle" "$file"; then
    printf 'FAIL: %s\nexpected to find: %s\n--- %s ---\n' "$label" "$needle" "$file" >&2
    sed -n '1,220p' "$file" >&2
    return 1
  fi
}

chrome_dump() {
  local url="$1"
  local out="$2"
  local profile="$TMP_DIR/chrome-$(basename "$out" .html)"
  local timeout="${OPENPENCIL_BROWSER_SMOKE_TIMEOUT_SECS:-45}"
  mkdir -p "$profile"
  "$CHROME_BIN" \
    --headless=new \
    --no-sandbox \
    --disable-dev-shm-usage \
    --disable-background-networking \
    --disable-component-update \
    --disable-default-apps \
    --disable-sync \
    --ignore-gpu-blocklist \
    --metrics-recording-only \
    --no-default-browser-check \
    --no-first-run \
    --use-gl=swiftshader \
    --enable-unsafe-swiftshader \
    --window-size=1280,800 \
    --user-data-dir="$profile" \
    --enable-logging=stderr \
    --virtual-time-budget="${OPENPENCIL_BROWSER_SMOKE_VIRTUAL_TIME_MS:-15000}" \
    --dump-dom "$url" >"$out" 2>"$out.stderr" &
  local chrome_pid=$!
  for _ in $(seq 1 "$timeout"); do
    # The editor holds a live SSE stream, which pauses headless Chrome's
    # virtual clock and keeps --dump-dom from ever flushing. The console
    # mirror on stderr arrives incrementally, so it is the readiness
    # signal; a successful mount synthesizes the DOM line the assertions
    # expect.
    if grep -Fq 'op-smoke:ok' "$out.stderr" 2>/dev/null; then
      printf '<html data-op-smoke="ok"><!-- synthesized from console marker --></html>\n' >"$out"
      kill -9 "$chrome_pid" >/dev/null 2>&1 || true
      wait "$chrome_pid" >/dev/null 2>&1 || true
      return 0
    fi
    if grep -Fq 'data-op-smoke="ok"' "$out" 2>/dev/null; then
      kill -9 "$chrome_pid" >/dev/null 2>&1 || true
      wait "$chrome_pid" >/dev/null 2>&1 || true
      return 0
    fi
    if ! kill -0 "$chrome_pid" >/dev/null 2>&1; then
      wait "$chrome_pid"
      return $?
    fi
    sleep 1
  done
  # SIGKILL, not SIGTERM: a headless Chrome wedged mid-GPU-stall has been
  # observed ignoring TERM, which turns the wait below into a permanent hang.
  kill -9 "$chrome_pid" >/dev/null 2>&1 || true
  wait "$chrome_pid" >/dev/null 2>&1 || true
  printf 'headless Chrome timed out for %s after %ss\n' "$url" "$timeout" >&2
  return 124
}

need curl
need python3

CHROME_BIN="$(find_chrome)" || {
  printf 'missing prerequisite: Chrome/Chromium (set OPENPENCIL_CHROME_BIN)\n' >&2
  exit 2
}

if [ "${OPENPENCIL_SKIP_WASM_BUILD:-0}" != "1" ]; then
  bash tools/check-wasm-bundle.sh
fi

SERVER_BIN="${OPENPENCIL_WEB_SERVER_BIN:-target/release/op-host-web-server}"
if [ "${OPENPENCIL_SKIP_SERVER_BUILD:-0}" != "1" ]; then
  cargo build -p op-host-web-server --release >/dev/null
fi
[ -x "$SERVER_BIN" ] || fail "server binary is not executable: $SERVER_BIN"

TMP_DIR="$(mktemp -d)"
SERVER_PID=""
cleanup() {
  local status=$?
  set +e
  if [ -n "$SERVER_PID" ] && kill -0 "$SERVER_PID" >/dev/null 2>&1; then
    kill -9 "$SERVER_PID" >/dev/null 2>&1 || true
    wait "$SERVER_PID" >/dev/null 2>&1 || true
  fi
  # Chrome renderer/GPU children can briefly keep writing their profile after
  # the browser PID exits. Deletion is housekeeping: retry it, but never turn
  # a successful smoke into a failure because a disposable runner profile is
  # still being released.
  for _ in $(seq 1 20); do
    rm -rf "$TMP_DIR" >/dev/null 2>&1 && break
    sleep 0.1
  done
  return "$status"
}
trap cleanup EXIT

PORT="${OPENPENCIL_WEB_SMOKE_PORT:-$(pick_port)}"
BASE_URL="http://127.0.0.1:${PORT}"
SERVER_LOG="$TMP_DIR/server.log"

OPENPENCIL_WEB_BUNDLE_DIR="${OPENPENCIL_WEB_BUNDLE_DIR:-$PWD/crates/op-host-web/pkg}" \
OPENPENCIL_CANVASKIT_DIR="${OPENPENCIL_CANVASKIT_DIR:-$PWD/crates/op-host-web/assets/canvaskit}" \
  "$SERVER_BIN" --serve-web "$PORT" --host 127.0.0.1 >"$SERVER_LOG" 2>&1 &
SERVER_PID="$!"

wait_for_health "$BASE_URL/api/mcp/server" || {
  printf 'server did not become healthy\n--- server log ---\n' >&2
  cat "$SERVER_LOG" >&2
  exit 1
}

health="$TMP_DIR/health.json"
curl -fsS "$BASE_URL/api/mcp/server" >"$health"
assert_contains "$health" '"mode":"web-canvas"' "daemon health should report web-canvas mode"
curl -fsS "$BASE_URL/pkg/op_host_web.js" >/dev/null
curl -fsS "$BASE_URL/canvaskit/canvaskit.js" >/dev/null
curl -fsS "$BASE_URL/assets/iconify-catalog-brands.json" >/dev/null

pure="$TMP_DIR/pure-smoke.html"
chrome_dump "$BASE_URL/smoke/step-1b.html" "$pure"
assert_contains "$pure" 'data-op-smoke="ok"' "pure CanvasKit smoke should mount"

daemon="$TMP_DIR/daemon-smoke.html"
chrome_dump "$BASE_URL/" "$daemon"
assert_contains "$daemon" 'data-op-smoke="ok"' "daemon host page should mount"

printf 'PASS: browser smoke mounted CanvasKit via /smoke/step-1b.html and daemon /\n'
