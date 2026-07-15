#!/bin/sh

set -eu

if [ "$#" -ne 0 ]; then
    printf 'usage: scripts/sync-version.sh\n' >&2
    printf 'sync-version: edit [workspace.package].version in Cargo.toml, then run this command without arguments\n' >&2
    exit 2
fi

script_dir=$(CDPATH= cd "$(dirname "$0")" && pwd)
repo_root=$(CDPATH= cd "$script_dir/.." && pwd)

if ! canonical_version=$("$repo_root/scripts/workspace-version.sh"); then
    printf 'sync-version: canonical version read failed; set a valid SemVer at [workspace.package].version in Cargo.toml\n' >&2
    exit 1
fi

printf 'sync-version: canonical Cargo workspace version is %s\n' "$canonical_version"

if ! (cd "$repo_root" && cargo metadata --no-deps --format-version 1 >/dev/null); then
    printf 'sync-version: Cargo lock refresh failed\n' >&2
    exit 1
fi

if ! (cd "$repo_root/packages" && bun run sync-version); then
    printf 'sync-version: package version synchronization failed\n' >&2
    exit 1
fi

if ! (cd "$repo_root" && "$repo_root/tools/check-version-sync.sh"); then
    printf 'sync-version: version consistency check failed\n' >&2
    exit 1
fi

printf 'sync-version: all managed versions are synchronized to %s\n' "$canonical_version"
