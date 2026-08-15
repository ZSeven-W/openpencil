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
# The wasm feature flags rustc emits for wasm32-unknown-unknown that wasm-opt
# must be told to accept. Kept as CANDIDATES because binaryen versions disagree
# on their spelling: newer binaryen (dev machines, v117+) split bulk-memory into
# `bulk-memory` (memory.init/data.drop) + `bulk-memory-opt` (memory.copy/fill),
# while older binaryen (e.g. Ubuntu's apt `binaryen`) predates the split and its
# single `--enable-bulk-memory` already covers memory.copy/fill. Passing an
# unknown flag hard-fails wasm-opt ("Unknown option"), so we filter the list
# down to only the flags the installed wasm-opt actually advertises in --help.
WASM_OPT_CANDIDATE_FEATURES=(
  --enable-bulk-memory
  --enable-bulk-memory-opt
  --enable-nontrapping-float-to-int
)

# Ceiling for the CanvasKit production bundle's gzipped wasm. It is far above
# the retired skia raster path's 1 MiB (spec §6) because this bundle carries the
# FULL app logic absorbed from the skia path (codegen AI pipeline, Figma parser,
# AI/live-sync, collaboration) plus the remaining embedded product assets —
# scene-template .op documents, the iconify core catalog, and the AI skill
# corpus.
#
# Re-baselined from 8 MiB to 6 MiB once the ~2.4 MiB of preview JPEGs moved out
# of the binary and behind the runtime `/pkg/assets/` fetch (step 4 above;
# `op_editor_core::web_assets`). The bundle measures ~5.35 MiB today, so the
# ceiling is still a runaway-regression tripwire rather than a budget — but a
# tripwire 3 MiB above the real number catches nothing, which is why it moves
# down with the bundle. TODO(perf): the template documents and icon catalog can
# follow the previews out; code-splitting the codegen + Figma paths is the
# larger remaining win. Override via env.
LIMIT="${STEP1B_SHELL_WASM_GZIP_LIMIT_BYTES:-6291456}"

step() { printf '\n[step %d/%d] %s\n' "$1" "$2" "$3"; }
fail() { printf 'FAIL: %s\n' "$1" >&2; exit 1; }
need() { command -v "$1" >/dev/null 2>&1 || { printf 'missing prerequisite: %s\n' "$1" >&2; exit 2; }; }

step 1 7 "Verify prerequisites"
need cargo
need wasm-bindgen
need wasm-opt
need node
need gzip
# CanvasKit needs NO EMSDK / skia-safe / libc shim — the editor renders through
# the official CanvasKit skia WASM (loaded separately from /canvaskit/) and the
# Rust bundle is pure logic. (The retired skia raster path needed EMSDK.)

step 2 7 "Verify CanvasKit bridge scale bucketing"
node --test crates/op-host-web/tests/op_ck_bridge_scale.test.mjs

# `canvaskit` is the production browser feature set: the editor renders through
# CanvasKit on the GPU and bundles the full app logic absorbed from the retired
# skia raster path — daemon-backed AI chat (`web_chat`), live-sync, browser file
# IO, clipboard/Figma paste, icon search, system fonts, and the codegen pipeline.
step 3 7 "Build shell-web wasm32-unknown-unknown with --features canvaskit"
cargo build -p op-host-web \
  --target wasm32-unknown-unknown --no-default-features --features canvaskit --release --locked >/dev/null

step 4 7 "wasm-bindgen --target web → ${PKG_DIR}/"
wasm-bindgen --target web --out-dir "${PKG_DIR}" "${TARGET_WASM}" >/dev/null

step 5 7 "Stage runtime product assets into ${PKG_DIR}/assets/"
# The wasm bundle no longer embeds the preview JPEGs, template documents and
# icon catalog (see `op_editor_core::web_assets`): the browser fetches each on
# demand from `/pkg/assets/…`, which the daemon already serves out of the
# resolved bundle directory. Staging them here is what makes that route
# resolve — a bundle shipped without this step degrades every preview to its
# placeholder. Keep in sync with `.github/workflows/wasm-bundle-build.yml` and
# `Dockerfile.web-rust`.
bash tools/stage-web-assets.sh "${PKG_DIR}/assets"

step 6 7 "Verify 0 env.* imports (spec §7.1 import guard)"
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

step 7 7 "Verify gzip size ≤ ${LIMIT} bytes (spec §6 ceiling)"
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
wasm-opt "${WASM_OPT_FEATURES[@]}" -Oz "${WASM_RAW}" -o "${WASM_OPT}" >/dev/null
cp "${WASM_OPT}" "${WASM_RAW}"
gz_bytes="$(gzip -c "${WASM_OPT}" | wc -c | tr -d ' ')"
if [ "${gz_bytes}" -gt "${LIMIT}" ]; then
  fail "shell-web wasm gzip size ${gz_bytes} bytes > ceiling ${LIMIT} bytes"
fi
pct=$(( (gz_bytes * 100) / LIMIT ))
printf '  ✓ gzip size %s bytes (%d%% of %s ceiling)\n' "${gz_bytes}" "${pct}" "${LIMIT}"

printf '\nAll Step 1b §6 + §7.1 bundle gates PASS.\n'
