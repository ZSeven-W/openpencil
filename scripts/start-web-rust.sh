#!/usr/bin/env bash
# Build and serve the Rust CanvasKit web editor without Docker.
#
# This is the script-publish counterpart to Dockerfile.web-rust. It builds the
# deployable wasm bundle, builds the GUI-free op-host-web-server daemon, then
# serves the browser editor through `--serve-web`.
set -euo pipefail

cd "$(dirname "$0")/.."

PORT="${OPENPENCIL_SERVE_PORT:-3100}"
HOST="${OPENPENCIL_SERVE_HOST:-127.0.0.1}"
DOC_PATH="${1:-}"

if [ "${OPENPENCIL_SKIP_WASM_BUILD:-0}" != "1" ]; then
  bash tools/check-wasm-bundle.sh
fi

if [ "${OPENPENCIL_SKIP_SERVER_BUILD:-0}" != "1" ]; then
  cargo build -p op-host-web-server --release
fi

SERVER_BIN="${OPENPENCIL_WEB_SERVER_BIN:-target/release/op-host-web-server}"
if [ ! -x "$SERVER_BIN" ]; then
  echo "error: server binary is not executable: $SERVER_BIN" >&2
  exit 1
fi

export OPENPENCIL_WEB_BUNDLE_DIR="${OPENPENCIL_WEB_BUNDLE_DIR:-$PWD/crates/op-host-web/pkg}"
export OPENPENCIL_CANVASKIT_DIR="${OPENPENCIL_CANVASKIT_DIR:-$PWD/crates/op-host-web/assets/canvaskit}"

echo "OpenPencil Rust web editor: http://${HOST}:${PORT}/"
echo "bundle: ${OPENPENCIL_WEB_BUNDLE_DIR}"
echo "canvaskit: ${OPENPENCIL_CANVASKIT_DIR}"

if [ -n "$DOC_PATH" ]; then
  exec "$SERVER_BIN" --serve-web "$PORT" "$DOC_PATH" --host "$HOST"
else
  exec "$SERVER_BIN" --serve-web "$PORT" --host "$HOST"
fi
