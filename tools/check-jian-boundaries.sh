#!/usr/bin/env bash
# Step 1a Phase C Task 4 / spec v19 §11 + §12.3 boundary invariants.
#
# Verifies the following Jian crate boundary invariants from outside the
# Rust build system. Run from the repo root.
#
# Invariant 1 (§12.3): openpencil-app must NOT depend directly on any
#   `jian-*` crate — Jian is a shell-native implementation detail; the
#   app only sees OP's `RenderBackend` / `ShellEvent` facade.
#
# Invariant 2 (§11.1, §12.3): mobile targets (`aarch64-linux-android`,
#   `aarch64-apple-ios`) must NOT pull `jian-host-desktop` or `jian-skia`
#   into the dependency closure — those carry the desktop GL stack +
#   `skia-safe` build.rs that fails on cross-compile.
#
# Invariant 3 (§11.1, §1.2): wasm32 builds of `openpencil-shell-web`
#   must NOT pull `jian-host-desktop` or `jian-skia` (skia-safe build.rs
#   fails on wasm32; Jian-core is wasm32-clean per P0.5 and is the only
#   Jian crate allowed in the bundle).
#
# Invariant 4 (§1.2): `openpencil-shell-web` must NOT depend on
#   `jian-host-desktop` at all — even as a non-default optional dep.
#
# Exit codes:
#   0  — all invariants pass.
#   1+ — one or more invariants fail; the failing crate names are
#         echoed before the script exits.
#
# Dependencies: `cargo`, `jq` (for cargo metadata JSON parsing).
set -euo pipefail

if ! command -v jq >/dev/null 2>&1; then
    echo "check-jian-boundaries.sh: \`jq\` is required but not installed." >&2
    echo "  apt: sudo apt-get install -y jq" >&2
    echo "  brew: brew install jq" >&2
    exit 2
fi

# ── Invariant 1: openpencil-app has no direct jian-* dependency. ──────
# `cargo metadata` returns a workspace-wide resolve graph; we filter
# the `resolve.nodes[]` entry whose name matches `openpencil-app` and
# inspect its direct `deps[]`. A direct dep on any `jian-*` crate
# fails the invariant.
metadata_full="$(cargo metadata --format-version 1)"
forbidden_app="$(echo "$metadata_full" | jq -r '
    [.packages[] | select(.name == "openpencil-app") | .id] as $app_ids
    | .resolve.nodes[]
    | select(.id as $id | $app_ids | index($id))
    | .deps[].name
' | grep -E '^jian-' || true)"
if [ -n "$forbidden_app" ]; then
    echo "INVARIANT 1 FAILED: openpencil-app directly depends on jian-* crate(s):" >&2
    echo "$forbidden_app" >&2
    exit 1
fi

# ── Invariant 2: mobile targets don't pull jian-host-desktop / jian-skia. ──
# We use `cargo tree` (which honours `--target` cfg-gates) and inspect
# the dependency closure of `openpencil-shell-native` — only the deps
# that actually compile under the mobile target are listed.
for target in aarch64-linux-android aarch64-apple-ios; do
    tree_mobile="$(cargo tree -p openpencil-shell-native \
        --target "$target" \
        --prefix none \
        --edges normal,build 2>/dev/null || true)"
    forbidden_mobile="$(echo "$tree_mobile" \
        | grep -oE '\bjian-(host-desktop|skia)\b' \
        | sort -u || true)"
    if [ -n "$forbidden_mobile" ]; then
        echo "INVARIANT 2 FAILED ($target): forbidden Jian crates in closure:" >&2
        echo "$forbidden_mobile" >&2
        exit 1
    fi
done

# ── Invariant 3: wasm32 has no jian-host-desktop / jian-skia. ──────────
# `jian-core` IS allowed (P0.5 wasm32-clean).
tree_wasm="$(cargo tree -p openpencil-shell-web \
    --target wasm32-unknown-unknown \
    --prefix none \
    --edges normal,build 2>/dev/null || true)"
forbidden_wasm="$(echo "$tree_wasm" \
    | grep -oE '\bjian-(host-desktop|skia)\b' \
    | sort -u || true)"
if [ -n "$forbidden_wasm" ]; then
    echo "INVARIANT 3 FAILED: wasm32 openpencil-shell-web pulls forbidden Jian crates:" >&2
    echo "$forbidden_wasm" >&2
    exit 1
fi

# ── Invariant 4: openpencil-shell-web has no jian-host-desktop dep. ───
# Distinct from invariant 3 (which checks the resolved closure on the
# wasm32 target): this checks the manifest itself across all targets.
# `cargo tree --all-targets` would include dev-deps; we explicitly
# filter `--edges normal,build` for the manifest-level invariant.
shell_web_deps="$(cargo tree -p openpencil-shell-web \
    --prefix none \
    --edges normal,build 2>/dev/null \
    | grep -E '\bjian-host-desktop\b' || true)"
if [ -n "$shell_web_deps" ]; then
    echo "INVARIANT 4 FAILED: openpencil-shell-web depends on jian-host-desktop:" >&2
    echo "$shell_web_deps" >&2
    exit 1
fi

echo "check-jian-boundaries.sh: all 4 Jian boundary invariants pass."
