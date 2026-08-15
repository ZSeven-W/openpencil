#!/usr/bin/env bash

set -euo pipefail

script_dir=$(CDPATH= cd "$(dirname "$0")" && pwd)
checker=$script_dir/check-mobile-auth-link-input.sh
test_dir=$(mktemp -d "${TMPDIR:-/tmp}/op-mobile-auth-link-test.XXXXXX")
cleanup() {
    rm -rf "$test_dir"
}
trap cleanup EXIT

archive=$test_dir/libop_auth.a
printf '!<arch>\n' > "$archive"

if grep -Fq 'workspace_version' "$checker" \
    || ! grep -Fq '"version=$artifact_version"' "$checker" \
    || ! grep -Fq '[[ "$abi" == 3 ]]' "$checker"; then
    printf 'not ok - Release link gate must trust signed version metadata and require ABI 3\n' >&2
    exit 1
fi

CONFIGURATION=Debug \
OP_AUTH_ARCHIVE="$archive" \
OPENPENCIL_DEV_OP_AUTH_ABI_VERSION=3 \
    bash "$checker" >/dev/null

signed_archive=$script_dir/../crates/op-auth-bridge/prebuilt/aarch64-apple-ios-sim/libop_auth.a
CONFIGURATION=Release \
OP_AUTH_ARCHIVE="$(cd "$(dirname "$signed_archive")" && pwd -P)/libop_auth.a" \
OP_AUTH_TARGET=aarch64-apple-ios-sim \
    bash "$checker" >/dev/null

expect_failure() {
    label=$1
    expected=$2
    shift 2
    set +e
    output=$("$@" 2>&1)
    result=$?
    set -e
    if [[ "$result" -eq 0 || "$output" != *"$expected"* ]]; then
        printf 'not ok - %s\n%s\n' "$label" "$output" >&2
        exit 1
    fi
    printf 'ok - %s\n' "$label"
}

expect_failure \
    "Debug rejects unsupported ABI" \
    "requires OPENPENCIL_DEV_OP_AUTH_ABI_VERSION=2 or 3" \
    env CONFIGURATION=Debug OP_AUTH_ARCHIVE="$archive" \
        OPENPENCIL_DEV_OP_AUTH_ABI_VERSION=1 bash "$checker"

expect_failure \
    "Release rejects an unsigned external archive" \
    "Release may link only the repository signed archive" \
    env CONFIGURATION=Release OP_AUTH_ARCHIVE="$archive" \
        OP_AUTH_TARGET=aarch64-apple-ios-sim bash "$checker"

expect_failure \
    "Android Release rejects an unsigned external archive" \
    "Release may link only the repository signed archive" \
    env CONFIGURATION=Release OP_AUTH_ARCHIVE="$archive" \
        OP_AUTH_TARGET=aarch64-linux-android bash "$checker"

expect_failure \
    "Release rejects the development ABI selector" \
    "unsigned development auth lane is forbidden in Release" \
    env CONFIGURATION=Release OP_AUTH_ARCHIVE="$archive" \
        OP_AUTH_TARGET=aarch64-apple-ios-sim \
        OPENPENCIL_DEV_OP_AUTH_ABI_VERSION=3 bash "$checker"

expect_failure \
    "Release rejects an unsupported mobile target" \
    "requires an explicit supported OP_AUTH_TARGET" \
    env CONFIGURATION=Release OP_AUTH_ARCHIVE="$archive" \
        OP_AUTH_TARGET=armv7-linux-androideabi bash "$checker"

printf 'mobile auth link-input contract tests pass\n'
