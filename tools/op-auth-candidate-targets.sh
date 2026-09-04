#!/usr/bin/env bash
# Canonical production op-auth ABI-v3 target and artifact mapping.

op_auth_candidate_targets() {
    printf '%s\n' \
        aarch64-apple-darwin \
        aarch64-apple-ios \
        aarch64-apple-ios-sim \
        aarch64-linux-android \
        aarch64-pc-windows-msvc \
        aarch64-unknown-linux-gnu \
        x86_64-apple-darwin \
        x86_64-linux-android \
        x86_64-pc-windows-msvc \
        x86_64-unknown-linux-gnu
}

op_auth_candidate_artifact_name() {
    case "$1" in
        aarch64-pc-windows-msvc|x86_64-pc-windows-msvc)
            printf '%s\n' op_auth.lib
            ;;
        aarch64-apple-darwin|aarch64-apple-ios|aarch64-apple-ios-sim|\
        aarch64-linux-android|aarch64-unknown-linux-gnu|\
        x86_64-apple-darwin|x86_64-linux-android|x86_64-unknown-linux-gnu)
            printf '%s\n' libop_auth.a
            ;;
        *)
            printf 'error: unsupported production op-auth target: %s\n' "$1" >&2
            return 2
            ;;
    esac
}

op_auth_candidate_bundle_name() {
    op_auth_candidate_artifact_name "$1" >/dev/null
    printf 'unsigned-op-auth-abi-v3-%s\n' "$1"
}
