#!/usr/bin/env bash
# crates/op-web-sdk/tools/build-wasm.sh — cargo + wasm-bindgen + wasm-opt
# build + bundle gate for op-web-sdk.
#
# What this script does:
#   1. Verify prerequisites: cargo, wasm-bindgen, wasm-opt, node, gzip.
#   2. cargo build -p op-web-sdk --target wasm32-unknown-unknown \
#        --features canvaskit --release
#      Note: --no-default-features is NOT needed — op-web-sdk's [features]
#      default is empty (default = []), so there are no default features to
#      suppress.
#   3. wasm-bindgen --target web → crates/op-web-sdk/pkg/
#      Output: pkg/op_web_sdk_bg.wasm + pkg/op_web_sdk.js + pkg/*.d.ts.
#      The artifact is the raw wasm-bindgen pkg/ directory — no separate
#      CanvasKit sub-asset assembly is needed because op-web-sdk is pure
#      document logic; CanvasKit itself is loaded externally by the host page.
#   4. Assert 0 env.* imports in the raw bindgen wasm (LinkError guard).
#   5. wasm-opt -Oz (with the rustc-emitted WebAssembly feature flags) then
#      gzip size <= OP_WEB_SDK_WASM_GZIP_LIMIT_BYTES (default 6 291 456 = 6 MiB).
#      The SDK is pure logic with no CanvasKit WASM included — the ceiling
#      mirrors op-host-web's 6 MiB guard and can be tightened later.
#
# This approach mirrors tools/check-wasm-bundle.sh (op-host-web gate) exactly:
# cargo build → wasm-bindgen → wasm-opt, without wasm-pack.  This removes the
# wasm-pack version fragility (0.13.1 is broken on stable Rust) and keeps the
# two gates consistent.
#
# This script is the local counterpart to
# `.github/workflows/web-sdk-bundle.yml`; keep the two recipes aligned.
#
# Exit semantics:
#   0   all checks PASS.
#   1   any check FAILED — message names which one.
#   2   prerequisite missing.

set -euo pipefail

# Resolve workspace root regardless of where the script is invoked from.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "${SCRIPT_DIR}/../../.." && pwd)"

CRATE_DIR="${WORKSPACE_ROOT}/crates/op-web-sdk"
PKG_DIR="${CRATE_DIR}/pkg"
# wasm-bindgen names the .wasm after the lib.name in Cargo.toml ("op_web_sdk").
WASM_RAW="${PKG_DIR}/op_web_sdk_bg.wasm"
WASM_OPT="${PKG_DIR}/op_web_sdk_bg.opt.wasm"
# cargo places the raw cdylib here before wasm-bindgen post-processes it.
TARGET_WASM="${WORKSPACE_ROOT}/target/wasm32-unknown-unknown/release/op_web_sdk.wasm"

# rustc-emitted WebAssembly feature flags (matches op-host-web gate). Kept as
# CANDIDATES because binaryen versions disagree on their spelling: newer
# binaryen (v117+) split bulk-memory into `bulk-memory` + `bulk-memory-opt`,
# while older binaryen (e.g. Ubuntu's apt `binaryen`) predates the split and its
# single `--enable-bulk-memory` already covers memory.copy/fill. Passing an
# unknown flag hard-fails wasm-opt, so the invocation below filters this list
# down to only the flags the installed wasm-opt advertises in --help.
WASM_OPT_CANDIDATE_FEATURES=(
  --enable-bulk-memory
  --enable-bulk-memory-opt
  --enable-nontrapping-float-to-int
)

# Gzip ceiling — same 6 MiB as op-host-web; the SDK bundle is pure logic.
# Override via env when intentionally re-baselining.
LIMIT="${OP_WEB_SDK_WASM_GZIP_LIMIT_BYTES:-6291456}"

step() { printf '\n[step %d/%d] %s\n' "$1" "$2" "$3"; }
fail() { printf 'FAIL: %s\n' "$1" >&2; exit 1; }
need() { command -v "$1" >/dev/null 2>&1 || { printf 'missing prerequisite: %s\n' "$1" >&2; exit 2; }; }

step 1 5 "Verify prerequisites"
need cargo
need wasm-bindgen
need wasm-opt
need node
need gzip

step 2 5 "cargo build -p op-web-sdk --target wasm32-unknown-unknown --features canvaskit --release"
# Run from the workspace root so cargo resolves the workspace Cargo.toml.
# --no-default-features is omitted intentionally: op-web-sdk has no default
# features (default = [] in Cargo.toml), so the flag is a no-op and omitting
# it keeps the invocation consistent with what a consumer would normally use.
cd "${WORKSPACE_ROOT}"
cargo build -p op-web-sdk \
  --target wasm32-unknown-unknown \
  --features canvaskit \
  --release

step 3 5 "wasm-bindgen --target web → ${PKG_DIR}/"
mkdir -p "${PKG_DIR}"
wasm-bindgen --target web --out-dir "${PKG_DIR}" "${TARGET_WASM}"

step 4 5 "Verify 0 env.* imports (LinkError guard)"
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
  fail "env.* import count = ${env_count} (must be 0); a dep pulled non-wasm-clean code — find and gate it out"
fi
printf '  ✓ 0 env.* imports\n'

step 5 5 "wasm-opt -Oz + gzip size <= ${LIMIT} bytes"
# Keep only the candidate feature flags this wasm-opt understands (see the
# WASM_OPT_CANDIDATE_FEATURES note) so an older binaryen doesn't hard-fail on
# `--enable-bulk-memory-opt`. `--enable-bulk-memory` alone still covers
# memory.copy/fill on those versions.
wasm_opt_help="$(wasm-opt --help 2>&1 || true)"
WASM_OPT_FEATURES=()
for flag in "${WASM_OPT_CANDIDATE_FEATURES[@]}"; do
  if printf '%s\n' "${wasm_opt_help}" | grep -qF -- "${flag}"; then
    WASM_OPT_FEATURES+=("${flag}")
  fi
done
wasm-opt "${WASM_OPT_FEATURES[@]}" -Oz "${WASM_RAW}" -o "${WASM_OPT}"
# Replace the raw wasm with the optimised version in-place so pkg/ is
# ready to deploy / upload as-is (mirrors check-wasm-bundle.sh behaviour).
cp "${WASM_OPT}" "${WASM_RAW}"
gz_bytes="$(gzip -c "${WASM_OPT}" | wc -c | tr -d ' ')"
if [ "${gz_bytes}" -gt "${LIMIT}" ]; then
  fail "op-web-sdk wasm gzip size ${gz_bytes} bytes > ceiling ${LIMIT} bytes"
fi
pct=$(( (gz_bytes * 100) / LIMIT ))
printf '  ✓ gzip size %s bytes (%d%% of %s ceiling)\n' "${gz_bytes}" "${pct}" "${LIMIT}"

printf '\nAll op-web-sdk bundle gates PASS.\n'
printf 'Output: %s/\n' "${PKG_DIR}"
