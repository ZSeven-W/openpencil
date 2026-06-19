#!/usr/bin/env bash
# Headless-boundary guard for the web / MCP server binary.
#
# The whole point of `op-host-web-server` (Approach Y) is that it links the
# extracted `op-host-services` and NOTHING from the desktop GUI stack: no winit /
# glutin / casement (windowing), no muda (native menus), no accesskit platform
# ADAPTERS (a11y bridges), and skia-safe WITHOUT the `gl` feature (raster only).
# This guard fails the build if any of those leak into the isolated dep graph,
# so the web image stays GUI-free.
#
# Codex Issue 1: the bare `accesskit` CORE crate is allowed — `op-editor-ui`
# pulls it unconditionally for the platform-free `Node` / `TreeUpdate` types
# (no GUI runtime). We grep the platform ADAPTER crates only.
set -euo pipefail

cd "$(dirname "$0")/.."

fail=0

# 1. No windowing / menu / a11y-adapter crates in the feature-resolved tree.
gui=$(cargo tree -p op-host-web-server -e features 2>/dev/null \
  | grep -E 'winit|glutin|casement|muda|accesskit_(macos|unix|windows|winit)' || true)
if [ -n "${gui}" ]; then
  printf 'FAIL: op-host-web-server links a desktop GUI crate:\n%s\n\n' "${gui}" >&2
  fail=1
fi

# 2. skia-safe must be raster — NO `gl` feature. Under Approach Y, gl arrives
#    only via the desktop-only `gl-host` edge; its presence in this isolated
#    graph means a consumer leaked gl-host into the headless server.
skia_feats=$(cargo tree -p op-host-web-server -f '{p} {f}' 2>/dev/null \
  | grep 'skia-safe v' | sed 's/.*skia-safe v[^ ]*//' | head -1)
if printf '%s' "${skia_feats}" | grep -qE '(^| |,)gl(,| |$)'; then
  printf 'FAIL: op-host-web-server skia-safe carries the `gl` feature (must be raster):\n  features:%s\n\n' "${skia_feats}" >&2
  fail=1
fi

if [ "${fail}" -ne 0 ]; then
  printf 'FAIL: op-host-web-server headless boundary check\n' >&2
  exit 1
fi

echo "PASS: op-host-web-server headless boundary check (no winit/glutin/casement/muda/accesskit-adapter; skia raster, no gl)"
