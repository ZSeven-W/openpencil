#!/usr/bin/env bash
# Build, verify and zip the extension for a Chrome Web Store submission.
#
# The store uploads a zip whose ROOT is the extension directory — no wrapping
# folder, no build tooling, no docs. This script produces exactly that, and
# refuses to produce it at all if any of the checks that guard the shipped
# behaviour fail: a broken build that reaches review costs days.
#
# Steps:
#   1. Build the Rust logic core (scripts/build-wasm.sh, which itself runs the
#      LinkError, no-eval and service-worker import guards).
#   2. cargo test -p op-chrome-extension-core — the whole logic surface,
#      including the assertion that manifest.json grants exactly the hub
#      origins the core is compiled against. Skippable with --skip-tests when
#      the caller has just run it.
#   3. node --check every runtime .js, so a syntax error cannot ship.
#   4. The drift/privacy guards: vendored extractor, service-worker import
#      graph, 15 locale catalogs and bounded design evidence.
#   5. Stage the runtime files — and only those — into a clean directory.
#   6. Add pt_BR and pt_PT as copies of pt (see below).
#   7. Zip the STAGE ROOT and list what went in.
#
# The `pt` caveat, and why the copies are made here rather than committed:
#
#   The popup resolves its own catalogs (`i18n.js` reads
#   `_locales/<locale>/messages.json` directly), so `pt` serves every
#   Portuguese user inside the popup. The extension NAME and DESCRIPTION are
#   different: Chrome resolves those `__MSG_*` placeholders itself, against
#   the browser's UI language, and its lookup for `pt-BR` tries `pt_BR` then
#   `pt` — but a store listing is matched on the exact directory. Shipping
#   `pt_BR` and `pt_PT` is what makes the listing render in Portuguese for
#   both.
#
#   They are generated here instead of being committed because a committed
#   copy is a third catalog to keep in sync, and `scripts/check-locales.mjs`
#   would then have to either police three identical files or exempt them —
#   both worse than deriving them from `pt` at the one moment they matter.
#   The switcher's 15 rows are unchanged: `pt_BR` and `pt_PT` are never
#   selectable, only resolvable.
#
# Exit semantics:
#   0   the zip is in dist/.
#   1   a check failed — the message names which one.
#   2   a required prerequisite is missing.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EXTENSION_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
WORKSPACE_ROOT="$(cd "${EXTENSION_DIR}/../.." && pwd)"

DIST_DIR="${EXTENSION_DIR}/dist"
STAGE_DIR="${DIST_DIR}/.stage"

SKIP_TESTS=0
for argument in "$@"; do
  case "${argument}" in
    --skip-tests) SKIP_TESTS=1 ;;
    -h|--help)
      sed -n '2,48p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *) printf 'unknown option: %s\n' "${argument}" >&2; exit 2 ;;
  esac
done

step() { printf '\n[step %d/7] %s\n' "$1" "$2"; }
fail() { printf 'FAIL: %s\n' "$1" >&2; exit 1; }
need() { command -v "$1" >/dev/null 2>&1 || { printf 'missing prerequisite: %s\n' "$1" >&2; exit 2; }; }

need node
need zip
need cargo

# The runtime surface, spelled out. A glob would quietly ship whatever a
# future edit dropped into the directory; this list is the manifest of what
# a user's browser is allowed to receive, and step 5 fails on anything in it
# that is missing.
RUNTIME_FILES=(
  manifest.json
  popup.html
  popup.css
  account.js
  background.js
  capture.js
  client.js
  core-registry.js
  design-capture.js
  design-evidence.js
  design-md.js
  i18n.js
  picker.js
  popup.js
  popup-status.js
  popup-worker-actions.js
  wasm-core.js
  vendor/snapshot-extractor.js
  wasm/op_chrome_extension_core.js
  wasm/op_chrome_extension_core_bg.wasm
  icons/icon-16.png
  icons/icon-48.png
  icons/icon-128.png
)

# Every root-level `.js` is part of the runtime. Cross-checking the directory
# against the list above is what catches a module that was added to the popup
# and forgotten here — it would load fine unpacked and 404 from the store zip.
step 1 "Build the logic core"
bash "${SCRIPT_DIR}/build-wasm.sh"

step 2 "cargo test -p op-chrome-extension-core"
if [ "${SKIP_TESTS}" -eq 1 ]; then
  printf '  – skipped (--skip-tests)\n'
else
  (cd "${WORKSPACE_ROOT}" && cargo test -p op-chrome-extension-core --quiet)
  printf '  ✓ logic core, manifest host permissions included\n'
fi

step 3 "node --check every runtime script"
checked=0
for source in "${EXTENSION_DIR}"/*.js "${EXTENSION_DIR}"/vendor/*.js; do
  node --check "${source}" >/dev/null || fail "syntax error in ${source##*/}"
  checked=$((checked + 1))
done
printf '  ✓ %d script(s) parse\n' "${checked}"

step 4 "Drift/privacy guards: extractor, service worker, locales, design evidence"
bash "${SCRIPT_DIR}/check-extractor-sync.sh"
node "${SCRIPT_DIR}/check-sw-imports.mjs"
node "${SCRIPT_DIR}/check-locales.mjs"
node "${SCRIPT_DIR}/check-design-evidence.mjs"

step 5 "Stage the runtime files"
# A stale stage would silently ship a file that has since been deleted.
rm -rf "${STAGE_DIR}"
mkdir -p "${STAGE_DIR}"

for relative in "${RUNTIME_FILES[@]}"; do
  source="${EXTENSION_DIR}/${relative}"
  [ -f "${source}" ] || fail "runtime file is missing: ${relative} (build first?)"
  mkdir -p "${STAGE_DIR}/$(dirname "${relative}")"
  cp "${source}" "${STAGE_DIR}/${relative}"
done

# Every root `.js` must be accounted for above.
for source in "${EXTENSION_DIR}"/*.js; do
  name="$(basename "${source}")"
  [ -f "${STAGE_DIR}/${name}" ] || \
    fail "${name} is in the extension but not in RUNTIME_FILES — add it or delete it"
done

cp -R "${EXTENSION_DIR}/_locales" "${STAGE_DIR}/_locales"
locale_count="$(find "${STAGE_DIR}/_locales" -mindepth 1 -maxdepth 1 -type d | wc -l | tr -d ' ')"
[ "${locale_count}" = "15" ] || fail "expected 15 locale catalogs, staged ${locale_count}"
printf '  ✓ %d runtime file(s) + %s locale catalogs\n' "${#RUNTIME_FILES[@]}" "${locale_count}"

step 6 "Add pt_BR and pt_PT as copies of pt"
for regional in pt_BR pt_PT; do
  cp -R "${STAGE_DIR}/_locales/pt" "${STAGE_DIR}/_locales/${regional}"
done
printf '  ✓ pt_BR, pt_PT (manifest name/description only; the popup uses pt)\n'

step 7 "Zip"
VERSION="$(node -e 'process.stdout.write(require(process.argv[1]).version)' \
  "${STAGE_DIR}/manifest.json")"
[ -n "${VERSION}" ] || fail "manifest.json carries no version"
ARCHIVE="${DIST_DIR}/op-chrome-extension-${VERSION}.zip"
rm -f "${ARCHIVE}"
# `-X` drops the extra file attributes (uid/gid, macOS resource forks) that
# make an otherwise identical build produce a different archive.
(cd "${STAGE_DIR}" && zip -q -X -r -9 "${ARCHIVE}" .)
rm -rf "${STAGE_DIR}"

printf '\n%s\n' "${ARCHIVE}"
printf '%s bytes\n\n' "$(wc -c <"${ARCHIVE}" | tr -d ' ')"
unzip -l "${ARCHIVE}"
