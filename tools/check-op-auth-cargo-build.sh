#!/usr/bin/env bash
# Prove that a just-finished Cargo release build selected the signed ABI 3
# op-auth path instead of silently compiling the open-source stub.

set -euo pipefail

script_dir=$(CDPATH='' cd "$(dirname "$0")" && pwd)
repo_root=$(CDPATH='' cd "$script_dir/.." && pwd)
target=${OP_AUTH_CARGO_TARGET:-}
profile=${OP_AUTH_CARGO_PROFILE:-release}
target_dir=${CARGO_TARGET_DIR:-$repo_root/target}

if [[ "$#" -ne 0 ]]; then
    printf 'usage: OP_AUTH_CARGO_TARGET=<triple> %s\n' "$0" >&2
    exit 2
fi
case "$target" in
    aarch64-apple-darwin | aarch64-apple-ios | aarch64-linux-android | aarch64-pc-windows-msvc | \
        aarch64-unknown-linux-gnu | x86_64-apple-darwin | x86_64-linux-android | \
        x86_64-pc-windows-msvc | x86_64-unknown-linux-gnu) ;;
    *)
        printf 'error: unsupported auth release target: %s\n' "$target" >&2
        exit 2
        ;;
esac
[[ "$profile" == release ]] || {
    printf 'error: production auth build evidence must come from the release profile\n' >&2
    exit 2
}

case "$target_dir" in
    /* | [A-Za-z]:[\\/]*) ;;
    *) target_dir=$repo_root/$target_dir ;;
esac
build_root=$target_dir/$target/$profile/build
[[ -d "$build_root" && ! -L "$build_root" ]] || {
    printf 'error: Cargo build-script output directory is missing for %s\n' "$target" >&2
    exit 1
}

shopt -s nullglob
outputs=("$build_root"/op-auth-bridge-*/output)
[[ "${#outputs[@]}" -gt 0 ]] || {
    printf 'error: Cargo emitted no op-auth build-script evidence for %s\n' "$target" >&2
    exit 1
}
for output in "${outputs[@]}"; do
    [[ -f "$output" && ! -L "$output" ]] || {
        printf 'error: invalid op-auth Cargo build-script output for %s\n' "$target" >&2
        exit 1
    }
    for required in \
        cargo:rustc-cfg=op_auth_prebuilt \
        cargo:rustc-cfg=op_auth_collab_ticket_prebuilt \
        cargo:rustc-cfg=op_auth_collab_relay_token_prebuilt \
        cargo:rustc-env=OP_AUTH_PREBUILT_ABI_VERSION=3; do
        grep -Fqx "$required" "$output" || {
            printf 'error: Cargo silently omitted production ABI 3 auth for %s\n' \
                "$target" >&2
            exit 1
        }
    done
    if grep -Fqx 'cargo:rustc-cfg=op_auth_development_prebuilt' "$output"; then
        printf 'error: Cargo selected a development auth archive for %s\n' "$target" >&2
        exit 1
    fi
done

printf 'check-op-auth-cargo-build.sh: verified production ABI 3 cfg for %s.\n' \
    "$target"
