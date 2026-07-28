#!/usr/bin/env bash
# Mutation tests for check-op-auth-prebuilt.sh.

set -euo pipefail

script_dir=$(CDPATH= cd "$(dirname "$0")" && pwd)
checker=$script_dir/check-op-auth-prebuilt.sh
test_root=$(mktemp -d "${TMPDIR:-/tmp}/op-auth-archive-gate.XXXXXX")
trap 'rm -rf "$test_root"' EXIT

test_index=0
failure_count=0
fixture_root=
gate_output=
gate_status=0

write_sha256() {
    artifact=$1
    output=$2
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$artifact" | awk '{ print $1 }' > "$output"
    else
        shasum -a 256 "$artifact" | awk '{ print $1 }' > "$output"
    fi
}

new_fixture() {
    fixture_root=$test_root/$1
    target_dir=$fixture_root/crates/op-auth-bridge/prebuilt/x86_64-unknown-linux-gnu
    mkdir -p "$fixture_root/tools" "$target_dir"
    cp "$checker" "$fixture_root/tools/check-op-auth-prebuilt.sh"
    artifact=$target_dir/libop_auth.a
    printf '%s\n' \
        op_auth_abi_version \
        op_auth_cancel \
        op_auth_login_begin \
        op_auth_poll \
        op_auth_restore \
        op_auth_runtime_init \
        op_auth_sign_out \
        op_auth_string_free \
        > "$artifact"
    printf '0.8.3\n' > "$target_dir/VERSION"
    write_sha256 "$artifact" "$target_dir/SHA256"
}

run_gate() {
    set +e
    gate_output=$(cd "$fixture_root" && bash tools/check-op-auth-prebuilt.sh "$@" 2>&1)
    gate_status=$?
    set -e
}

pass_case() {
    test_index=$((test_index + 1))
    printf 'ok %s - %s\n' "$test_index" "$1"
}

fail_case() {
    test_index=$((test_index + 1))
    failure_count=$((failure_count + 1))
    printf 'not ok %s - %s\n' "$test_index" "$1"
    printf '%s\n' "$gate_output" | sed 's/^/# /'
}

expect_pass() {
    label=$1
    shift
    run_gate "$@"
    if [[ "$gate_status" -eq 0 ]]; then
        pass_case "$label"
    else
        fail_case "$label"
    fi
}

expect_failure() {
    label=$1
    expected=$2
    shift 2
    run_gate "$@"
    if [[ "$gate_status" -ne 0 && "$gate_output" == *"$expected"* ]]; then
        pass_case "$label"
    else
        fail_case "$label (expected failure containing '$expected')"
    fi
}

new_fixture baseline
expect_pass "accepts an integrity-pinned legacy ABI-v1 archive"

new_fixture substituted
printf 'substitution\n' >> "$artifact"
expect_failure "rejects archive substitution" "artifact SHA-256 mismatch"

new_fixture expanded-c-abi
printf 'op_auth_private_debug_dump\n' >> "$artifact"
write_sha256 "$artifact" "$target_dir/SHA256"
expect_failure "rejects an undocumented op_auth C ABI symbol" \
    "undocumented op_auth C ABI symbols are exposed"

new_fixture hardened-path-leak
printf '/Users/private/op_auth_core/src/lib.rs\n' >> "$artifact"
write_sha256 "$artifact" "$target_dir/SHA256"
expect_failure "rejects source paths in hardened mode" \
    "source/build path strings" --require-hardened

new_fixture hardened-private-symbol
printf '_RNvNtCs123_12op_auth_core6secret\n' >> "$artifact"
write_sha256 "$artifact" "$target_dir/SHA256"
expect_failure "rejects private Rust module symbols in hardened mode" \
    "private Rust symbol/module strings" --require-hardened

new_fixture unsigned-abi-v2
printf '%s\n' \
    op_auth_collab_ticket_begin \
    op_auth_collab_ticket_cancel \
    op_auth_collab_ticket_poll \
    >> "$artifact"
printf '2\n' > "$target_dir/ABI_VERSION"
write_sha256 "$artifact" "$target_dir/SHA256"
expect_failure "rejects unsigned ABI-v2 provenance" \
    "signed ABI-v2 PROVENANCE is missing"

if [[ "$failure_count" -ne 0 ]]; then
    printf 'check-op-auth-prebuilt.test.sh: %s test(s) failed.\n' "$failure_count" >&2
    exit 1
fi

printf 'check-op-auth-prebuilt.test.sh: all %s mutation tests pass.\n' "$test_index"
