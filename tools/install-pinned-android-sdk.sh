#!/usr/bin/env bash
# Install the exact Android SDK inputs reviewed for production release builds.

set -euo pipefail

build_tools_sha=5d9ac77fb6ff43d9da518a337b4fcf8f9097113df531d99ccefe80ef7ce8250b
platform_sha=37607369a28c5b640b3a7998868d45898ebcb777565a0e85f9acf36f29631d2e
ndk_sha=dfb20d396df28ca02a8c708314b814a4d961dc9074f9a161932746f815aa552f

sha256_file() {
    sha256sum "$1" | awk '{ print $1 }'
}

verify_sha256() {
    local expected=$1 file=$2
    [[ "$expected" =~ ^[0-9a-f]{64}$ \
        && -f "$file" && ! -L "$file" \
        && "$(sha256_file "$file")" == "$expected" ]] || {
        printf 'error: Android SDK archive SHA-256 mismatch: %s\n' "$file" >&2
        return 1
    }
}

download_verified() {
    local name=$1 expected=$2 destination=$3 temporary
    [[ "$name" =~ ^[A-Za-z0-9._-]+\.zip$ ]] || {
        printf 'error: malformed Android repository archive name\n' >&2
        exit 1
    }
    temporary=$(mktemp "${destination%/*}/.android-sdk.XXXXXX")
    if ! curl --fail --location --proto '=https' --tlsv1.2 \
        --retry 10 --retry-all-errors --silent --show-error \
        "https://dl.google.com/android/repository/$name" \
        --output "$temporary"; then
        rm -f "$temporary"
        exit 1
    fi
    if ! verify_sha256 "$expected" "$temporary"; then
        rm -f "$temporary"
        exit 1
    fi
    mv "$temporary" "$destination"
}

verify_archive_members() {
    local archive=$1 prefix=$2 member count=0
    while IFS= read -r member; do
        ((count += 1))
        [[ "$member" == "$prefix"* \
            && "$member" != *'../'* && "$member" != /* \
            && "$member" != *"\\"* ]] || {
            printf 'error: unsafe Android SDK archive member: %s\n' "$member" >&2
            exit 1
        }
    done < <(unzip -Z1 "$archive")
    (( count > 0 )) || {
        printf 'error: Android SDK archive is empty\n' >&2
        exit 1
    }
}

self_test() {
    local temporary payload
    temporary=$(mktemp -d)
    payload=$temporary/payload
    printf test > "$payload"
    verify_sha256 \
        9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08 \
        "$payload"
    if verify_sha256 \
        0000000000000000000000000000000000000000000000000000000000000000 \
        "$payload" >/dev/null 2>&1; then
        printf 'error: tampered Android SDK fixture was accepted\n' >&2
        exit 1
    fi
    rm -rf "$temporary"
    printf 'install-pinned-android-sdk.sh: checksum rejection self-test passed.\n'
}

if [[ "${1:-}" == --self-test ]]; then
    [[ "$#" -eq 1 ]] || { printf 'usage: %s --self-test\n' "$0" >&2; exit 2; }
    self_test
    exit 0
fi

[[ "$#" -eq 1 ]] || {
    printf 'usage: %s INSTALL_ROOT\n' "$0" >&2
    exit 2
}
install_root=$1
[[ "$install_root" == /* && ! -e "$install_root" && ! -L "$install_root" ]] || {
    printf 'error: Android SDK install root must be a new absolute path\n' >&2
    exit 2
}
[[ "$(uname -s)" == Linux && "$(uname -m)" == x86_64 ]] || {
    printf 'error: pinned Android SDK is available for Linux x86_64 only\n' >&2
    exit 2
}
for command in curl find readlink sha256sum unzip; do
    command -v "$command" >/dev/null 2>&1 || {
        printf 'error: required Android SDK installer command is missing: %s\n' \
            "$command" >&2
        exit 1
    }
done

temporary=$(mktemp -d)
build_tools_archive=$temporary/build-tools.zip
platform_archive=$temporary/platform.zip
ndk_archive=$temporary/ndk.zip
download_verified build-tools_r36_linux.zip "$build_tools_sha" "$build_tools_archive"
download_verified platform-36_r02.zip "$platform_sha" "$platform_archive"
download_verified android-ndk-r28c-linux.zip "$ndk_sha" "$ndk_archive"
verify_archive_members "$build_tools_archive" android-16/
verify_archive_members "$platform_archive" android-36/
verify_archive_members "$ndk_archive" android-ndk-r28c/

mkdir -p "$install_root/build-tools" "$install_root/platforms" "$install_root/ndk"
unzip -q "$build_tools_archive" -d "$temporary/build-tools"
unzip -q "$platform_archive" -d "$temporary/platform"
unzip -q "$ndk_archive" -d "$temporary/ndk"
mv "$temporary/build-tools/android-16" "$install_root/build-tools/36.0.0"
mv "$temporary/platform/android-36" "$install_root/platforms/android-36"
mv "$temporary/ndk/android-ndk-r28c" "$install_root/ndk/28.2.13676358"

while IFS= read -r link; do
    resolved=$(readlink -f "$link")
    [[ "$resolved" == "$install_root/"* ]] || {
        printf 'error: Android SDK symlink escapes the verified install root\n' >&2
        exit 1
    }
done < <(find "$install_root" -type l -print)
grep -Eq '^Pkg\.Revision[[:space:]]*=[[:space:]]*36\.0\.0$' \
    "$install_root/build-tools/36.0.0/source.properties"
grep -Eq '^AndroidVersion\.ApiLevel=36$' \
    "$install_root/platforms/android-36/source.properties"
grep -Eq '^Pkg\.Revision[[:space:]]*=[[:space:]]*28\.2\.13676358$' \
    "$install_root/ndk/28.2.13676358/source.properties"
for tool in aapt apksigner zipalign; do
    [[ -f "$install_root/build-tools/36.0.0/$tool" \
        && ! -L "$install_root/build-tools/36.0.0/$tool" \
        && -x "$install_root/build-tools/36.0.0/$tool" ]] || {
        printf 'error: verified Android Build-Tools lacks %s\n' "$tool" >&2
        exit 1
    }
done
{
    printf 'android_build_tools_sha256=%s\n' "$build_tools_sha"
    printf 'android_platform_36_sha256=%s\n' "$platform_sha"
    printf 'android_ndk_r28c_sha256=%s\n' "$ndk_sha"
} > "$install_root/VERIFIED-DIGESTS"
chmod 0444 "$install_root/VERIFIED-DIGESTS"
rm -rf "$temporary"

if [[ -n "${GITHUB_ENV:-}" ]]; then
    {
        printf 'ANDROID_HOME=%s\n' "$install_root"
        printf 'ANDROID_SDK_ROOT=%s\n' "$install_root"
        printf 'ANDROID_NDK_HOME=%s\n' "$install_root/ndk/28.2.13676358"
    } >> "$GITHUB_ENV"
else
    printf '%s\n' "$install_root"
fi
