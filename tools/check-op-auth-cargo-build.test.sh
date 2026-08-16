#!/usr/bin/env bash
# Secret-free regression tests for the Cargo production-auth cfg gate.

set -euo pipefail

script_dir=$(CDPATH='' cd "$(dirname "$0")" && pwd)
temp_root=$(mktemp -d "${TMPDIR:-/tmp}/op-auth-cargo-test.XXXXXX")
target=x86_64-unknown-linux-gnu
output_dir=$temp_root/$target/release/build/op-auth-bridge-fixture
output=$output_dir/output

cleanup() {
    rm -rf "$temp_root"
}
trap cleanup EXIT

mkdir -p "$output_dir"
write_valid_output() {
    printf '%s\n' \
        cargo:rustc-cfg=op_auth_prebuilt \
        cargo:rustc-cfg=op_auth_collab_ticket_prebuilt \
        cargo:rustc-cfg=op_auth_collab_relay_token_prebuilt \
        cargo:rustc-env=OP_AUTH_PREBUILT_ABI_VERSION=3 \
        > "$output"
}

write_valid_output
CARGO_TARGET_DIR=$temp_root \
OP_AUTH_CARGO_TARGET=$target \
    "$script_dir/check-op-auth-cargo-build.sh" >/dev/null

android_target=aarch64-linux-android
android_output_dir=$temp_root/$android_target/release/build/op-auth-bridge-fixture
mkdir -p "$android_output_dir"
cp "$output" "$android_output_dir/output"
CARGO_TARGET_DIR=$temp_root \
OP_AUTH_CARGO_TARGET=$android_target \
    "$script_dir/check-op-auth-cargo-build.sh" >/dev/null

ios_target=aarch64-apple-ios
ios_output_dir=$temp_root/$ios_target/release/build/op-auth-bridge-fixture
mkdir -p "$ios_output_dir"
cp "$output" "$ios_output_dir/output"
CARGO_TARGET_DIR=$temp_root \
OP_AUTH_CARGO_TARGET=$ios_target \
    "$script_dir/check-op-auth-cargo-build.sh" >/dev/null

printf '%s\n' \
    cargo:rustc-cfg=op_auth_prebuilt \
    cargo:rustc-cfg=op_auth_collab_ticket_prebuilt \
    cargo:rustc-env=OP_AUTH_PREBUILT_ABI_VERSION=3 \
    > "$output"
if CARGO_TARGET_DIR=$temp_root \
    OP_AUTH_CARGO_TARGET=$target \
    "$script_dir/check-op-auth-cargo-build.sh" >/dev/null 2>&1; then
    printf 'error: missing ABI 3 relay-token cfg was accepted\n' >&2
    exit 1
fi

write_valid_output
printf '%s\n' cargo:rustc-cfg=op_auth_development_prebuilt >> "$output"
if CARGO_TARGET_DIR=$temp_root \
    OP_AUTH_CARGO_TARGET=$target \
    "$script_dir/check-op-auth-cargo-build.sh" >/dev/null 2>&1; then
    printf 'error: development auth cfg was accepted\n' >&2
    exit 1
fi

printf 'check-op-auth-cargo-build.test.sh: production cfg gates passed.\n'
