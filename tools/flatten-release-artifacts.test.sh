#!/usr/bin/env bash

set -euo pipefail

repo_root=$(cd "$(dirname "$0")/.." && pwd)
helper=$repo_root/tools/flatten-release-artifacts.sh
temporary=$(mktemp -d)
cleanup() {
    rm -rf "$temporary"
}
trap cleanup EXIT HUP INT TERM

make_fixture() {
    local root=$1
    mkdir -p "$root/desktop" "$root/cli" "$root/sdk" "$root/vsix" "$root/android"
    printf 'desktop\n' > "$root/desktop/openpencil-desktop-linux-x86_64.tar.gz"
    printf 'cli\n' > "$root/cli/op-cli-linux-x86_64.tar.gz"
    for index in 1 2 3; do
        printf 'sdk-%s\n' "$index" > "$root/sdk/sdk-${index}.tgz"
    done
    for index in 1 2 3 4 5 6; do
        printf 'vsix-%s\n' "$index" > "$root/vsix/editor-${index}.vsix"
    done
    printf 'apk\n' > "$root/android/OpenPencil-0.8.5-android.apk"
    printf 'aab\n' > "$root/android/OpenPencil-0.8.5-android.aab"
    (
        cd "$root/android"
        sha256sum OpenPencil-0.8.5-android.aab OpenPencil-0.8.5-android.apk \
            > SHA256SUMS.android.txt
    )
    printf 'ignored\n' > "$root/android/internal-build.log"
}

expect_rejected() {
    local label=$1
    local input=$2
    local output=$3
    if "$helper" "$input" "$output" 0.8.5 >/dev/null 2>&1; then
        printf 'error: accepted invalid fixture: %s\n' "$label" >&2
        exit 1
    fi
    if [ -e "$output" ]; then
        printf 'error: invalid fixture left release output: %s\n' "$label" >&2
        exit 1
    fi
}

valid=$temporary/valid
make_fixture "$valid"
"$helper" "$valid" "$temporary/release" 0.8.5 >/dev/null
test -f "$temporary/release/OpenPencil-0.8.5-android.apk"
test -f "$temporary/release/OpenPencil-0.8.5-android.aab"
test ! -e "$temporary/release/internal-build.log"

duplicate=$temporary/duplicate
make_fixture "$duplicate"
mkdir -p "$duplicate/other"
cp "$duplicate/sdk/sdk-1.tgz" "$duplicate/other/sdk-1.tgz"
expect_rejected duplicate-basename "$duplicate" "$temporary/duplicate-output"

bad_checksum=$temporary/bad-checksum
make_fixture "$bad_checksum"
printf 'tampered\n' >> "$bad_checksum/android/OpenPencil-0.8.5-android.apk"
expect_rejected invalid-android-checksum "$bad_checksum" "$temporary/checksum-output"

extra_apk=$temporary/extra-apk
make_fixture "$extra_apk"
printf 'extra\n' > "$extra_apk/android/OpenPencil-0.8.5-debug.apk"
expect_rejected extra-android-apk "$extra_apk" "$temporary/extra-apk-output"

symlinked=$temporary/symlinked
make_fixture "$symlinked"
ln -s ../sdk/sdk-1.tgz "$symlinked/android/linked.tgz"
expect_rejected symlink "$symlinked" "$temporary/symlink-output"

printf 'flatten-release-artifacts.test.sh: release asset fixtures passed.\n'
