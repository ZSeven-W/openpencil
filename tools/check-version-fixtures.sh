#!/usr/bin/env bash

set -euo pipefail

script_dir=$(CDPATH= cd "$(dirname "$0")" && pwd)
repo_root=$(CDPATH= cd "$script_dir/.." && pwd)
current_version=$(bash "$repo_root/scripts/workspace-version.sh")

cd "$repo_root"

rg_status=0
matches=$(rg \
    --fixed-strings \
    --line-number \
    --with-filename \
    --color never \
    --glob '*.rs' \
    --glob '!**/op-host-desktop/src/update_check.rs' \
    "$current_version" \
    crates) || rg_status=$?

if [[ "$rg_status" -gt 1 ]]; then
    printf 'version-fixtures: failed to scan Rust sources with rg (status %s)\n' "$rg_status" >&2
    exit "$rg_status"
fi

if [[ -n "$matches" ]]; then
    printf 'version-fixtures: ordinary Rust fixtures copy current product version %s:\n' \
        "$current_version" >&2
    printf '%s\n' "$matches" >&2
    printf 'version-fixtures: use stable 1.0.0 test data unless a test explicitly covers compatibility, migration, or updates\n' >&2
    exit 1
fi

printf 'version-fixtures: no ordinary Rust fixtures copy current product version %s\n' \
    "$current_version"
