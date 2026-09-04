#!/usr/bin/env bash
# tools/stage-web-assets.sh — copy the runtime-fetched product assets into a
# web bundle's `assets/` directory.
#
# The wasm bundle deliberately omits these (see `op_editor_core::web_assets`):
# the browser fetches each on demand from `/pkg/assets/<dir>/<file>`, which the
# daemon serves straight out of the resolved bundle directory. The desktop
# binary still embeds them with `include_bytes!` / `include_str!`, so this
# script exists only for the web deployment.
#
# The layout under the destination MUST match the route literals in
# `prompt_center_previews.rs`, `scene_template_previews.rs`,
# `scene_template_catalog.rs`, `icon_catalog.rs`, `bundled_fonts_web.rs` and
# `op_i18n::catalog_route` — those are `concat!`ed / formatted at compile time,
# so a mismatch is a silent 404 per asset rather than a build error. The Rust
# side pins its half with route tests (the font manifest additionally asserts
# every listed file exists in the source directories staged below); this script
# is the other half.
#
# Usage: tools/stage-web-assets.sh <dest-assets-dir>
# Exit: 0 staged, 1 a source/catalog/prerequisite check failed.

set -euo pipefail

DEST="${1:?usage: stage-web-assets.sh <dest-assets-dir>}"
UI_ASSETS="crates/op-editor-ui/assets"
CORE_ASSETS="crates/op-editor-core/assets"
DESKTOP_FONTS="crates/op-host-desktop/assets/fonts"
NATIVE_ASSETS="crates/op-host-native/assets"
I18N_EXPORTER="tools/export-i18n-catalogs.py"

command -v python3 >/dev/null 2>&1 || {
  printf 'FAIL: python3 is required to export runtime i18n catalogs\n' >&2
  exit 1
}

copy_dir() {
  local src="$1" name="$2"
  [ -d "${src}" ] || { printf 'FAIL: missing asset source %s\n' "${src}" >&2; exit 1; }
  mkdir -p "${DEST}/${name}"
  # `-R` not `-a`: no need to preserve ownership into a container image, and
  # BusyBox cp (the Docker build stage) has no `-a`.
  cp -R "${src}/." "${DEST}/${name}/"
  # Provenance manifests are a build-time record of which model produced each
  # preview; they are not fetched by anything and have no business in a
  # published bundle.
  rm -f "${DEST}/${name}/preview_provenance.json"
}

copy_file() {
  local src="$1" name="$2"
  [ -f "${src}" ] || { printf 'FAIL: missing asset source %s\n' "${src}" >&2; exit 1; }
  # `name` may carry a subdirectory (e.g. `fonts/Roboto-Regular.ttf`), so mkdir
  # the destination's parent rather than only `${DEST}`.
  mkdir -p "$(dirname "${DEST}/${name}")"
  cp "${src}" "${DEST}/${name}"
}

mkdir -p "${DEST}"
copy_dir "${UI_ASSETS}/prompt_center_previews" "prompt_center_previews"
copy_dir "${UI_ASSETS}/scene_template_previews" "scene_template_previews"
# Scene-template `.op` documents — fetched when a template is instantiated.
copy_dir "${CORE_ASSETS}/scene_templates" "scene_templates"
# Core (lucide + feather) icon catalog — fetched when the icon panel opens.
copy_file "${UI_ASSETS}/iconify-catalog-core.json" "iconify-catalog-core.json"
# The OFL design fonts the desktop binary embeds with `include_bytes!`. The
# browser has no bundled fonts of its own, so without these a document using
# e.g. Inter pops the missing-fonts modal and renders a fallback face; the web
# host fetches them all at mount (`op-host-web/src/bundled_fonts_web.rs`).
copy_dir "${DESKTOP_FONTS}" "fonts"
# Roboto lives with the native host (it is also that backend's last-resort
# face), not in the desktop font directory, so it is staged separately.
copy_file "${NATIVE_ASSETS}/Roboto-Regular.ttf" "fonts/Roboto-Regular.ttf"

# Web wasm embeds en-US + zh-CN only. Validate all 15 canonical Rust tables,
# then export the other 13 under their Locale::code() BCP-47 filenames.
python3 "${I18N_EXPORTER}" --check
I18N_DEST="${DEST}/i18n"
python3 "${I18N_EXPORTER}" --output-dir "${I18N_DEST}"

expected_i18n_files="$(
  printf '%s\n' \
    de.json es.json fr.json hi.json id.json ja.json ko.json pt.json ru.json \
    th.json tr.json vi.json zh-TW.json | LC_ALL=C sort
)"
actual_i18n_files="$(
  find "${I18N_DEST}" -mindepth 1 -maxdepth 1 -exec basename {} \; | LC_ALL=C sort
)"
if [ "${actual_i18n_files}" != "${expected_i18n_files}" ]; then
  printf 'FAIL: expected exactly 13 runtime i18n catalogs, found:\n%s\n' \
    "${actual_i18n_files}" >&2
  exit 1
fi

staged="$(du -sk "${DEST}" | cut -f1)"
printf '  ✓ staged runtime assets into %s (%s KiB)\n' "${DEST}" "${staged}"
