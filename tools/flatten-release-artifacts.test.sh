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
    printf 'setup-x64\n' > "$root/desktop/OpenPencil-0.8.5-x64-win-setup.exe"
    printf 'setup-arm64\n' > "$root/desktop/OpenPencil-0.8.5-arm64-win-setup.exe"
    # `openpencil-daemon-<label>` artifacts also match release-draft's
    # `openpencil-*` download pattern, so the flattener sees them.
    for label in macos-aarch64 macos-x86_64 linux-x86_64 linux-aarch64; do
        mkdir -p "$root/openpencil-daemon-${label}"
        printf 'daemon-%s\n' "$label" \
            > "$root/openpencil-daemon-${label}/op-host-web-server"
    done
    for label in windows-x86_64 windows-aarch64; do
        mkdir -p "$root/openpencil-daemon-${label}"
        printf 'daemon-%s\n' "$label" \
            > "$root/openpencil-daemon-${label}/op-host-web-server.exe"
    done
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
# The Windows installers are the `.exe` assets a release publishes.
test -f "$temporary/release/OpenPencil-0.8.5-x64-win-setup.exe"
test -f "$temporary/release/OpenPencil-0.8.5-arm64-win-setup.exe"
# The vsix's bundled daemon is not one of them, on either platform.
test ! -e "$temporary/release/op-host-web-server"
test ! -e "$temporary/release/op-host-web-server.exe"

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
