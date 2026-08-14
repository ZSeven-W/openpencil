#!/usr/bin/env bash
# Build one mobile editor target against an unsigned local op-auth archive.
# This script intentionally has no release mode.

set -euo pipefail

script_dir=$(CDPATH= cd "$(dirname "$0")" && pwd)
repo_root=$(CDPATH= cd "$script_dir/.." && pwd)
platform=
archive=
abi=

usage() {
    cat >&2 <<'EOF'
usage: scripts/build-mobile-auth-dev.sh \
  --platform ios-simulator|ios-device|android-arm64|android-x86_64 \
  --archive /absolute/path/to/libop_auth.a --abi 2|3

Builds a Cargo debug artifact only. Android output is isolated under
packaging/android-player/app/src/debug/jniLibs; iOS keeps libop_auth.a as an
explicit final Xcode link input.
EOF
}

while [[ "$#" -gt 0 ]]; do
    case "$1" in
        --platform|--archive|--abi)
            [[ "$#" -ge 2 ]] || {
                usage
                exit 2
            }
            name=${1#--}
            printf -v "$name" '%s' "$2"
            shift 2
            ;;
        *)
            usage
            exit 2
            ;;
    esac
done

[[ -n "$platform" && -n "$archive" && -n "$abi" ]] || {
    usage
    exit 2
}
case "$abi" in
    2|3) ;;
    *)
        printf 'error: --abi must be 2 or 3\n' >&2
        exit 2
        ;;
esac
case "$archive" in
    /*) ;;
    *)
        printf 'error: --archive must be an absolute path\n' >&2
        exit 2
        ;;
esac
if [[ -L "$archive" || ! -f "$archive" || "$(basename "$archive")" != libop_auth.a ]]; then
    printf 'error: --archive must select a regular non-symlink libop_auth.a\n' >&2
    exit 2
fi
archive=$(cd "$(dirname "$archive")" && pwd -P)/libop_auth.a

export OPENPENCIL_DEV_OP_AUTH_ARCHIVE="$archive"
export OPENPENCIL_DEV_OP_AUTH_ABI_VERSION="$abi"
cd "$repo_root"

case "$platform" in
    ios-simulator)
        target=aarch64-apple-ios-sim
        cargo build --locked -p op-engine-ffi --target "$target" \
            --features metal,editor,mobile-auth-dev
        printf 'built %s\n' "$repo_root/target/$target/debug/libop_engine_ffi.a"
        printf 'Xcode auth link input: OP_AUTH_ARCHIVE=%s\n' "$archive"
        ;;
    ios-device)
        target=aarch64-apple-ios
        cargo build --locked -p op-engine-ffi --target "$target" \
            --features metal,editor,mobile-auth-dev
        printf 'built %s\n' "$repo_root/target/$target/debug/libop_engine_ffi.a"
        printf 'Xcode auth link input: OP_AUTH_ARCHIVE=%s\n' "$archive"
        ;;
    android-arm64)
        command -v cargo-ndk >/dev/null 2>&1 || {
            printf 'error: cargo-ndk is required\n' >&2
            exit 1
        }
        cargo ndk -t arm64-v8a \
            -o packaging/android-player/app/src/debug/jniLibs \
            build --locked -p op-engine-jni --features gl,editor,mobile-auth-dev
        ;;
    android-x86_64)
        command -v cargo-ndk >/dev/null 2>&1 || {
            printf 'error: cargo-ndk is required\n' >&2
            exit 1
        }
        cargo ndk -t x86_64 \
            -o packaging/android-player/app/src/debug/jniLibs \
            build --locked -p op-engine-jni --features gl,editor,mobile-auth-dev
        ;;
    *)
        printf 'error: unsupported --platform: %s\n' "$platform" >&2
        usage
        exit 2
        ;;
esac
