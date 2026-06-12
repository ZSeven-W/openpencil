#!/usr/bin/env bash
# tools/check-wasm-bundle.sh — Step 1b §6 + §7.1 local bundle gate.
#
# What this script enforces (matches spec §6 ceilings and the §7.1
# env.* import count guard):
#   1. EMSDK is set so the build can resolve emsdk's libcxx headers
#      + wasm-aware clang (build-time only; emscripten runtime is NOT
#      linked into the bundle — see spec §2.2).
#   2. cargo build → wasm-bindgen → wasm-opt -Oz pipeline produces
#      crates/op-host-web/pkg/op_host_web_bg.wasm.
#   3. Post-bindgen bundle has 0 env.* imports
#      (i.e. all imports come from `./op_host_web_bg.js`,
#      the wasm-bindgen JS shim). Any env.* import = LinkError at
#      load time → regression → fail.
#   4. Post wasm-opt -Oz gzip size ≤ STEP1B_SHELL_WASM_GZIP_LIMIT_BYTES
#      (default 1 048 576 bytes = 1 MiB; spec §6 ceiling for the
#      shell-web wasm sub-component).
#
# Why this is a local script and not (yet) a CI workflow: the C-hard
# pipeline needs brew-emscripten + EMSDK + a .wasm.a → .a symlink hack
# in the skia-bindings out/ dir before the render bundle can be built
# in CI. That automation is tracked as DEFERRED in
# `.github/workflows/rust-release.yml`. Until it lands, this script
# is the source-of-truth gate for developers running the build
# locally before merging Phase A-E changes.
#
# Exit semantics:
#   0   all four checks PASS.
#   1   any check FAILED — message names which one.
#   2   prerequisite missing (EMSDK / wasm-bindgen / wasm-opt / node).

set -euo pipefail

CRATE_DIR="crates/op-host-web"
PKG_DIR="${CRATE_DIR}/pkg"
WASM_RAW="${PKG_DIR}/op_host_web_bg.wasm"
WASM_OPT="${PKG_DIR}/op_host_web_bg.opt.wasm"
TARGET_WASM="target/wasm32-unknown-unknown/release/op_host_web.wasm"

# Spec §6 row "Per-component ceiling — shell-web wasm (cdylib) gzip"
# = 1 MiB. Override via env for experiments only.
LIMIT="${STEP1B_SHELL_WASM_GZIP_LIMIT_BYTES:-1048576}"

step() { printf '\n[step %d/%d] %s\n' "$1" "$2" "$3"; }
fail() { printf 'FAIL: %s\n' "$1" >&2; exit 1; }
need() { command -v "$1" >/dev/null 2>&1 || { printf 'missing prerequisite: %s\n' "$1" >&2; exit 2; }; }

step 1 5 "Verify prerequisites"
need cargo
need wasm-bindgen
need wasm-opt
need node
need gzip
[ -n "${EMSDK:-}" ] || { printf 'EMSDK env var unset (needed for emsdk libcxx headers + wasm-aware clang)\n' >&2; exit 2; }

# `codegen` (which pulls `skia`) is the production browser feature set:
# it compiles the daemon-backed AI chat (`web_chat`), browser file IO,
# clipboard/Figma paste, and the codegen pipeline. A skia-only bundle
# would ship a canvas whose chat can only error ("transport not
# compiled in") even when `op start --web`'s daemon is serving it.
step 2 5 "Build shell-web wasm32-unknown-unknown with --features codegen"
cargo build -p op-host-web \
  --target wasm32-unknown-unknown --features codegen --release >/dev/null

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
  fail "env.* import count = ${env_count} (spec §7.1 requires 0); a new symbol leaked — add it to crates/wasm-libc-shim/src/imp.rs"
fi
printf '  ✓ 0 env.* imports\n'

step 5 5 "Verify gzip size ≤ ${LIMIT} bytes (spec §6 ceiling)"
wasm-opt -Oz "${WASM_RAW}" -o "${WASM_OPT}" >/dev/null
gz_bytes="$(gzip -c "${WASM_OPT}" | wc -c | tr -d ' ')"
if [ "${gz_bytes}" -gt "${LIMIT}" ]; then
  fail "shell-web wasm gzip size ${gz_bytes} bytes > ceiling ${LIMIT} bytes"
fi
pct=$(( (gz_bytes * 100) / LIMIT ))
printf '  ✓ gzip size %s bytes (%d%% of %s ceiling)\n' "${gz_bytes}" "${pct}" "${LIMIT}"

printf '\nAll Step 1b §6 + §7.1 bundle gates PASS.\n'
