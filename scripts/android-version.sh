#!/bin/sh

set -eu

if [ "$#" -gt 1 ]; then
    printf 'usage: scripts/android-version.sh [Cargo.toml]\n' >&2
    exit 2
fi

script_dir=$(CDPATH= cd "$(dirname "$0")" && pwd -P)
repo_root=$(CDPATH= cd "$script_dir/.." && pwd -P)
manifest=${1:-"$repo_root/Cargo.toml"}

if ! version=$("$script_dir/workspace-version.sh" "$manifest"); then
    printf 'android-version: canonical workspace version read failed\n' >&2
    exit 1
fi

# Android versionCode has no portable representation for SemVer pre-release
# precedence. Reject suffixes instead of allowing two releases to collide.
case "$version" in
    *[!0-9.]* | *.*.*.*)
        printf 'android-version: Android packages require a stable X.Y.Z workspace version (got %s)\n' \
            "$version" >&2
        exit 1
        ;;
esac

old_ifs=$IFS
IFS=.
set -- $version
IFS=$old_ifs
if [ "$#" -ne 3 ]; then
    printf 'android-version: Android packages require a stable X.Y.Z workspace version (got %s)\n' \
        "$version" >&2
    exit 1
fi

major=$1
minor=$2
patch=$3

# Policy: major * 1,000,000 + minor * 1,000 + patch. Keeping minor and
# patch within three decimal digits makes every supported SemVer increase a
# strict versionCode increase, including the 0.x line.
if [ "${#major}" -gt 4 ] || [ "$major" -gt 2100 ] || \
        [ "${#minor}" -gt 3 ] || [ "$minor" -gt 999 ] || \
        [ "${#patch}" -gt 3 ] || [ "$patch" -gt 999 ]; then
    printf 'android-version: version components exceed the supported versionCode ranges (got %s)\n' \
        "$version" >&2
    exit 1
fi

version_code=$((major * 1000000 + minor * 1000 + patch))
if [ "$version_code" -lt 1 ] || [ "$version_code" -gt 2100000000 ]; then
    printf 'android-version: computed versionCode %s is outside Android range 1..2100000000\n' \
        "$version_code" >&2
    exit 1
fi

printf 'versionName=%s\nversionCode=%s\n' "$version" "$version_code"
