#!/usr/bin/env bash
# tools/check-wasm-bundle.sh — Step 1b §6 + §7.1 local bundle gate.
#
# What this script enforces (matches the CanvasKit production bundle gate
# and the env.* import count guard):
#   1. Required local tools are available: cargo, wasm-bindgen, wasm-opt,
#      node, and gzip. CanvasKit needs no EMSDK / skia-safe / libc shim.
#   2. cargo build → wasm-bindgen → wasm-opt -Oz pipeline produces the
#      served crates/op-host-web/pkg/op_host_web_bg.wasm. `wasm-opt` is invoked
#      with the core WebAssembly features emitted by current rustc.
#   3. Post-bindgen bundle has 0 env.* imports
#      (i.e. all imports come from `./op_host_web_bg.js`,
#      the wasm-bindgen JS shim). Any env.* import = LinkError at
#      load time → regression → fail.
#   4. Post wasm-opt -Oz gzip size ≤ STEP1B_SHELL_WASM_GZIP_LIMIT_BYTES
#      (default 6 291 456 bytes = 6 MiB for the full CanvasKit app logic).
#
# This script is the local counterpart to
# `.github/workflows/wasm-bundle-build.yml`; keep the two recipes aligned.
#
# Exit semantics:
#   0   all four checks PASS.
#   1   any check FAILED — message names which one.
#   2   prerequisite missing (cargo / wasm-bindgen / wasm-opt / node / gzip).

set -euo pipefail

CRATE_DIR="crates/op-host-web"
PKG_DIR="${CRATE_DIR}/pkg"
WASM_RAW="${PKG_DIR}/op_host_web_bg.wasm"
WASM_OPT="${PKG_DIR}/op_host_web_bg.opt.wasm"
TARGET_WASM="target/wasm32-unknown-unknown/release/op_host_web.wasm"
WASM_OPT_FEATURES=(
  --enable-bulk-memory
  --enable-bulk-memory-opt
  --enable-nontrapping-float-to-int
)

# Ceiling for the CanvasKit production bundle's gzipped wasm. It is far above
# the retired skia raster path's 1 MiB (spec §6) because this bundle now carries
# the FULL app logic absorbed from the skia path (codegen AI pipeline, Figma
# parser, AI/live-sync). ~4.5 MiB today. TODO(perf): code-split / lazy-load the
# codegen + Figma paths to shrink the initial download. Override via env.
LIMIT="${STEP1B_SHELL_WASM_GZIP_LIMIT_BYTES:-6291456}"

step() { printf '\n[step %d/%d] %s\n' "$1" "$2" "$3"; }
fail() { printf 'FAIL: %s\n' "$1" >&2; exit 1; }
need() { command -v "$1" >/dev/null 2>&1 || { printf 'missing prerequisite: %s\n' "$1" >&2; exit 2; }; }

step 1 5 "Verify prerequisites"
need cargo
need wasm-bindgen
need wasm-opt
need node
need gzip
# CanvasKit needs NO EMSDK / skia-safe / libc shim — the editor renders through
# the official CanvasKit skia WASM (loaded separately from /canvaskit/) and the
# Rust bundle is pure logic. (The retired skia raster path needed EMSDK.)

# `canvaskit` is the production browser feature set: the editor renders through
# CanvasKit on the GPU and bundles the full app logic absorbed from the retired
# skia raster path — daemon-backed AI chat (`web_chat`), live-sync, browser file
# IO, clipboard/Figma paste, icon search, system fonts, and the codegen pipeline.
step 2 5 "Build shell-web wasm32-unknown-unknown with --features canvaskit"
cargo build -p op-host-web \
  --target wasm32-unknown-unknown --no-default-features --features canvaskit --release >/dev/null

step 3 5 "wasm-bindgen --target web → ${PKG_DIR}/"
wasm-bindgen --target web --out-dir "${PKG_DIR}" "${TARGET_WASM}" >/dev/null

step 4 5 "Verify 0 env.* imports (spec §7.1 import guard)"
env_count="$(node -e '
const fs = require("fs");
const buf = fs.readFileSync(process.argv[1]);
WebAssembly.compile(buf).then(mod => {
  const imps = WebAssembly.Module.imports(mod);
  const env = imps.filter(i => i.module === "env");
  console.log(env.length);
}).catch(e => { console.error("compile failed:", e); process.exit(1); });
' "${WASM_RAW}")"
if [ "${env_count}" != "0" ]; then
  fail "env.* import count = ${env_count} (must be 0); the CanvasKit bundle has no libc shim, so an env.* import means a dep pulled non-wasm-clean code — find and gate it out"
fi
printf '  ✓ 0 env.* imports\n'

step 5 5 "Verify gzip size ≤ ${LIMIT} bytes (spec §6 ceiling)"
wasm-opt "${WASM_OPT_FEATURES[@]}" -Oz "${WASM_RAW}" -o "${WASM_OPT}" >/dev/null
cp "${WASM_OPT}" "${WASM_RAW}"
gz_bytes="$(gzip -c "${WASM_OPT}" | wc -c | tr -d ' ')"
if [ "${gz_bytes}" -gt "${LIMIT}" ]; then
  fail "shell-web wasm gzip size ${gz_bytes} bytes > ceiling ${LIMIT} bytes"
fi
pct=$(( (gz_bytes * 100) / LIMIT ))
printf '  ✓ gzip size %s bytes (%d%% of %s ceiling)\n' "${gz_bytes}" "${pct}" "${LIMIT}"

printf '\nAll Step 1b §6 + §7.1 bundle gates PASS.\n'
