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
gate_prebuilt_root=

write_sha256() {
    artifact=$1
    output=$2
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$artifact" | awk '{ print $1 }' > "$output"
    else
        shasum -a 256 "$artifact" | awk '{ print $1 }' > "$output"
    fi
}

write_public_key() {
    key=$1
    output=$2
    openssl pkey -in "$key" -pubout -outform DER \
        | tail -c 32 | xxd -p -c 256 > "$output"
}

sign_provenance() {
    key=$1
    signature=$2
    binary_signature=$fixture_root/provenance.sig.bin
    openssl pkeyutl -sign -rawin \
        -inkey "$key" \
        -in "$target_dir/PROVENANCE" \
        -out "$binary_signature"
    xxd -p -c 256 "$binary_signature" > "$signature"
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

make_signed_abi_v2() {
    signing_key=$fixture_root/signing.pem
    openssl genpkey -algorithm Ed25519 -out "$signing_key" >/dev/null 2>&1
    printf '%s\n' \
        op_auth_collab_ticket_begin \
        op_auth_collab_ticket_cancel \
        op_auth_collab_ticket_poll \
        >> "$artifact"
    printf '2\n' > "$target_dir/ABI_VERSION"
    write_sha256 "$artifact" "$target_dir/SHA256"
    artifact_sha=$(tr -d '[:space:]' < "$target_dir/SHA256")
    cat > "$target_dir/PROVENANCE" <<EOF
format=1
target=x86_64-unknown-linux-gnu
artifact=libop_auth.a
version=0.8.3
abi=2
sha256=$artifact_sha
hardening=op-auth-hardened-v1
source_revision=1111111111111111111111111111111111111111
build_id=op-auth-prebuilt-test.a2
EOF
    write_public_key \
        "$signing_key" "$fixture_root/crates/op-auth-bridge/prebuilt/PROVENANCE_PUBKEY"
    sign_provenance "$signing_key" "$target_dir/PROVENANCE.sig"
}

run_gate() {
    set +e
    if [[ -n "$gate_prebuilt_root" ]]; then
        gate_output=$(cd "$fixture_root" \
            && OP_AUTH_PREBUILT_ROOT="$gate_prebuilt_root" \
                bash tools/check-op-auth-prebuilt.sh "$@" 2>&1)
    else
        gate_output=$(cd "$fixture_root" \
            && bash tools/check-op-auth-prebuilt.sh "$@" 2>&1)
    fi
    gate_status=$?
    set -e
    gate_prebuilt_root=
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
    "signed ABI-v2+ PROVENANCE is missing"

new_fixture signed-abi-v2
make_signed_abi_v2
expect_pass "accepts valid Ed25519-signed ABI-v2 provenance" --require-hardened

new_fixture forged-signature
make_signed_abi_v2
attacker_key=$fixture_root/attacker.pem
openssl genpkey -algorithm Ed25519 -out "$attacker_key" >/dev/null 2>&1
sign_provenance "$attacker_key" "$target_dir/PROVENANCE.sig"
expect_failure "rejects provenance signed by an untrusted key" \
    "provenance signature verification failed" --require-hardened

new_fixture corrupt-signature
make_signed_abi_v2
printf 'not-a-signature\n' > "$target_dir/PROVENANCE.sig"
expect_failure "rejects a malformed provenance signature" \
    "provenance signature encoding is invalid" --require-hardened

new_fixture corrupt-public-key
make_signed_abi_v2
printf 'not-a-public-key\n' \
    > "$fixture_root/crates/op-auth-bridge/prebuilt/PROVENANCE_PUBKEY"
expect_failure "rejects a malformed checkout provenance public key" \
    "release provenance public key encoding is invalid" --require-hardened

new_fixture tampered-signed-field
make_signed_abi_v2
sed -i.bak 's/^version=0\.8\.3$/version=0.8.4/' "$target_dir/PROVENANCE"
expect_failure "rejects tampering with a signed provenance field" \
    "provenance signature verification failed" --require-hardened

new_fixture replayed-signed-metadata
make_signed_abi_v2
printf 'replacement archive bytes\n' >> "$artifact"
write_sha256 "$artifact" "$target_dir/SHA256"
expect_failure "rejects replaying valid signed metadata over replacement bytes" \
    "signed provenance does not match sha256=" --require-hardened

new_fixture external-trust-root
make_signed_abi_v2
external_root=$fixture_root/untrusted-prebuilt
mkdir -p "$external_root"
cp -R "$target_dir" "$external_root/x86_64-unknown-linux-gnu"
attacker_key=$fixture_root/attacker.pem
openssl genpkey -algorithm Ed25519 -out "$attacker_key" >/dev/null 2>&1
target_dir=$external_root/x86_64-unknown-linux-gnu
write_public_key "$attacker_key" "$external_root/PROVENANCE_PUBKEY"
sign_provenance "$attacker_key" "$target_dir/PROVENANCE.sig"
gate_prebuilt_root=$external_root
expect_failure "ignores an external prebuilt root's attacker-controlled public key" \
    "provenance signature verification failed" --require-hardened

if [[ "$failure_count" -ne 0 ]]; then
    printf 'check-op-auth-prebuilt.test.sh: %s test(s) failed.\n' "$failure_count" >&2
    exit 1
fi

printf 'check-op-auth-prebuilt.test.sh: all %s mutation tests pass.\n' "$test_index"
