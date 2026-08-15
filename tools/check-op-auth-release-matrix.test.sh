#!/usr/bin/env bash
# Secret-free regression tests for the signed release-matrix trust boundary.

set -euo pipefail

script_dir=$(CDPATH='' cd "$(dirname "$0")" && pwd)
temp_root=$(mktemp -d "${TMPDIR:-/tmp}/op-auth-matrix-test.XXXXXX")
fixture_repo=$temp_root/repo
attacker_repo=$temp_root/attacker-repo
trusted_prebuilt=$fixture_repo/crates/op-auth-bridge/prebuilt
prebuilt=$temp_root/artifact-prebuilt
version=9.8.7
source_revision=1111111111111111111111111111111111111111
openpencil_revision=2222222222222222222222222222222222222222
build_id=matrix-test

cleanup() {
    rm -rf "$temp_root"
}
trap cleanup EXIT

sha256_file() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | awk '{ print $1 }'
    else
        shasum -a 256 "$1" | awk '{ print $1 }'
    fi
}

sign_hex() {
    local key=$1
    local input=$2
    local output=$3
    openssl pkeyutl -sign -rawin -inkey "$key" -in "$input" \
        | xxd -p -c 256 > "$output"
}

mkdir -p \
    "$fixture_repo/tools" "$fixture_repo/scripts" \
    "$attacker_repo/tools" "$attacker_repo/scripts" \
    "$attacker_repo/crates/op-auth-bridge/prebuilt" \
    "$trusted_prebuilt" "$prebuilt"
cp "$script_dir/check-op-auth-release-matrix.sh" "$fixture_repo/tools/"
cp "$script_dir/check-op-auth-release-matrix.sh" "$attacker_repo/tools/"
printf '#!/usr/bin/env bash\nprintf "%s\\n"\n' "$version" \
    > "$fixture_repo/scripts/workspace-version.sh"
cp "$fixture_repo/scripts/workspace-version.sh" \
    "$attacker_repo/scripts/workspace-version.sh"
chmod +x \
    "$fixture_repo/tools/check-op-auth-release-matrix.sh" \
    "$fixture_repo/scripts/workspace-version.sh" \
    "$attacker_repo/tools/check-op-auth-release-matrix.sh" \
    "$attacker_repo/scripts/workspace-version.sh"

private_key=$temp_root/private.pem
forged_key=$temp_root/forged.pem
openssl genpkey -algorithm ED25519 -out "$private_key" >/dev/null 2>&1
openssl genpkey -algorithm ED25519 -out "$forged_key" >/dev/null 2>&1
openssl pkey -in "$private_key" -pubout -outform DER \
    | tail -c 32 | xxd -p -c 256 > "$trusted_prebuilt/PROVENANCE_PUBKEY"

targets=(
    aarch64-apple-darwin
    aarch64-apple-ios
    aarch64-apple-ios-sim
    aarch64-linux-android
    aarch64-pc-windows-msvc
    aarch64-unknown-linux-gnu
    x86_64-apple-darwin
    x86_64-linux-android
    x86_64-pc-windows-msvc
    x86_64-unknown-linux-gnu
)

matrix_lines=$temp_root/matrix-lines
: > "$matrix_lines"
for target in "${targets[@]}"; do
    target_dir=$prebuilt/$target
    artifact=libop_auth.a
    [[ "$target" == *-pc-windows-msvc ]] && artifact=op_auth.lib
    mkdir -p "$target_dir"
    printf 'signed fixture for %s\n' "$target" > "$target_dir/$artifact"
    artifact_sha=$(sha256_file "$target_dir/$artifact")
    printf '%s\n' "$version" > "$target_dir/VERSION"
    printf '3\n' > "$target_dir/ABI_VERSION"
    printf '%s\n' "$artifact_sha" > "$target_dir/SHA256"
    {
        printf 'format=2\n'
        printf 'mode=production\n'
        printf 'target=%s\n' "$target"
        printf 'artifact=%s\n' "$artifact"
        printf 'artifact_sha256=%s\n' "$artifact_sha"
        printf 'source_revision=%s\n' "$source_revision"
        printf 'release_build_id=%s\n' "$build_id"
        printf 'openpencil_revision=%s\n' "$openpencil_revision"
    } > "$target_dir/HARDENING-ATTESTATION"
    hardening_sha=$(sha256_file "$target_dir/HARDENING-ATTESTATION")
    {
        printf 'format=1\n'
        printf 'target=%s\n' "$target"
        printf 'artifact=%s\n' "$artifact"
        printf 'version=%s\n' "$version"
        printf 'abi=3\n'
        printf 'sha256=%s\n' "$artifact_sha"
        printf 'hardening=op-auth-hardened-v1\n'
        printf 'source_revision=%s\n' "$source_revision"
        printf 'build_id=%s.a3.%s\n' "$build_id" "$hardening_sha"
    } > "$target_dir/PROVENANCE"
    sign_hex "$private_key" \
        "$target_dir/PROVENANCE" "$target_dir/PROVENANCE.sig"
    provenance_sha=$(sha256_file "$target_dir/PROVENANCE")
    printf 'target=%s|artifact=%s|sha256=%s|provenance_sha256=%s|hardening_sha256=%s\n' \
        "$target" "$artifact" "$artifact_sha" "$provenance_sha" "$hardening_sha" \
        >> "$matrix_lines"
done

{
    printf 'format=op-auth-release-matrix-v1\n'
    printf 'version=%s\n' "$version"
    printf 'abi=3\n'
    printf 'source_revision=%s\n' "$source_revision"
    printf 'openpencil_revision=%s\n' "$openpencil_revision"
    printf 'build_id=%s\n' "$build_id"
    printf 'target_count=10\n'
    # The target array is already sorted by target name. Sorting the complete
    # lines would compare the separator after a short name with a later '-'.
    # That is a different order from sorting target names themselves.
    cat "$matrix_lines"
} > "$prebuilt/RELEASE-MANIFEST"
sign_hex "$private_key" \
    "$prebuilt/RELEASE-MANIFEST" "$prebuilt/RELEASE-MANIFEST.sig"
valid_manifest=$temp_root/valid-release-manifest
valid_signature=$temp_root/valid-release-signature
cp "$prebuilt/RELEASE-MANIFEST" "$valid_manifest"
cp "$prebuilt/RELEASE-MANIFEST.sig" "$valid_signature"

OP_AUTH_PREBUILT_ROOT=$prebuilt \
OP_AUTH_RELEASE_WORKSPACE_VERSION=$version \
OP_AUTH_RELEASE_EXPECTED_OPENPENCIL_REVISION=$openpencil_revision \
    "$fixture_repo/tools/check-op-auth-release-matrix.sh" >/dev/null

if OP_AUTH_PREBUILT_ROOT=$prebuilt \
    OP_AUTH_RELEASE_WORKSPACE_VERSION=$version \
    "$fixture_repo/tools/check-op-auth-release-matrix.sh" >/dev/null 2>&1; then
    printf 'error: missing expected public revision was accepted\n' >&2
    exit 1
fi

if OP_AUTH_PREBUILT_ROOT=$prebuilt \
    OP_AUTH_RELEASE_WORKSPACE_VERSION=$version \
    OP_AUTH_RELEASE_EXPECTED_OPENPENCIL_REVISION=3333333333333333333333333333333333333333 \
    "$fixture_repo/tools/check-op-auth-release-matrix.sh" >/dev/null 2>&1; then
    printf 'error: mismatched expected public revision was accepted\n' >&2
    exit 1
fi

# Swapping two otherwise valid, signed rows must fail the canonical target-name
# order contract shared with the private full-matrix generator.
awk '
    NR == 8 { first = $0; next }
    NR == 9 { print; print first; next }
    { print }
' "$valid_manifest" > "$prebuilt/RELEASE-MANIFEST"
sign_hex "$private_key" \
    "$prebuilt/RELEASE-MANIFEST" "$prebuilt/RELEASE-MANIFEST.sig"
if OP_AUTH_PREBUILT_ROOT=$prebuilt \
    OP_AUTH_RELEASE_WORKSPACE_VERSION=$version \
    OP_AUTH_RELEASE_EXPECTED_OPENPENCIL_REVISION=$openpencil_revision \
    "$fixture_repo/tools/check-op-auth-release-matrix.sh" >/dev/null 2>&1; then
    printf 'error: non-canonical target order was accepted\n' >&2
    exit 1
fi
cp "$valid_manifest" "$prebuilt/RELEASE-MANIFEST"
cp "$valid_signature" "$prebuilt/RELEASE-MANIFEST.sig"

# Model a fully self-consistent attacker-controlled artifact root: it carries
# its own public key and every signature is made with the corresponding forged
# private key. A verifier that accidentally reads the artifact-root key would
# accept this fixture; the reviewed verifier must use the canonical source key.
openssl pkey -in "$forged_key" -pubout -outform DER \
    | tail -c 32 | xxd -p -c 256 > "$prebuilt/PROVENANCE_PUBKEY"
cp "$prebuilt/PROVENANCE_PUBKEY" \
    "$attacker_repo/crates/op-auth-bridge/prebuilt/PROVENANCE_PUBKEY"
for target in "${targets[@]}"; do
    sign_hex "$forged_key" \
        "$prebuilt/$target/PROVENANCE" "$prebuilt/$target/PROVENANCE.sig"
done
sign_hex "$forged_key" \
    "$prebuilt/RELEASE-MANIFEST" "$prebuilt/RELEASE-MANIFEST.sig"
OP_AUTH_PREBUILT_ROOT=$prebuilt \
OP_AUTH_RELEASE_WORKSPACE_VERSION=$version \
OP_AUTH_RELEASE_EXPECTED_OPENPENCIL_REVISION=$openpencil_revision \
    "$attacker_repo/tools/check-op-auth-release-matrix.sh" >/dev/null
if OP_AUTH_PREBUILT_ROOT=$prebuilt \
    OP_AUTH_RELEASE_WORKSPACE_VERSION=$version \
    OP_AUTH_RELEASE_EXPECTED_OPENPENCIL_REVISION=$openpencil_revision \
    "$fixture_repo/tools/check-op-auth-release-matrix.sh" >/dev/null 2>&1; then
    printf 'error: forged release-matrix signing key was accepted\n' >&2
    exit 1
fi
for target in "${targets[@]}"; do
    sign_hex "$private_key" \
        "$prebuilt/$target/PROVENANCE" "$prebuilt/$target/PROVENANCE.sig"
done
cp "$valid_signature" "$prebuilt/RELEASE-MANIFEST.sig"

held_target=$temp_root/held-target
mv "$prebuilt/x86_64-unknown-linux-gnu" "$held_target"
if OP_AUTH_PREBUILT_ROOT=$prebuilt \
    OP_AUTH_RELEASE_WORKSPACE_VERSION=$version \
    OP_AUTH_RELEASE_EXPECTED_OPENPENCIL_REVISION=$openpencil_revision \
    "$fixture_repo/tools/check-op-auth-release-matrix.sh" >/dev/null 2>&1; then
    printf 'error: incomplete release target set was accepted\n' >&2
    exit 1
fi
mv "$held_target" "$prebuilt/x86_64-unknown-linux-gnu"

held_file=$temp_root/held-abi-version
mv "$prebuilt/aarch64-apple-ios/ABI_VERSION" "$held_file"
if OP_AUTH_PREBUILT_ROOT=$prebuilt \
    OP_AUTH_RELEASE_WORKSPACE_VERSION=$version \
    OP_AUTH_RELEASE_EXPECTED_OPENPENCIL_REVISION=$openpencil_revision \
    "$fixture_repo/tools/check-op-auth-release-matrix.sh" >/dev/null 2>&1; then
    printf 'error: incomplete release target metadata was accepted\n' >&2
    exit 1
fi
mv "$held_file" "$prebuilt/aarch64-apple-ios/ABI_VERSION"

printf 'tampered\n' >> "$prebuilt/aarch64-apple-ios/libop_auth.a"
if OP_AUTH_PREBUILT_ROOT=$prebuilt \
    OP_AUTH_RELEASE_WORKSPACE_VERSION=$version \
    OP_AUTH_RELEASE_EXPECTED_OPENPENCIL_REVISION=$openpencil_revision \
    "$fixture_repo/tools/check-op-auth-release-matrix.sh" >/dev/null 2>&1; then
    printf 'error: tampered iOS auth archive was accepted\n' >&2
    exit 1
fi

printf '%s\n' \
    'check-op-auth-release-matrix.test.sh: trust-root, revision, order, completeness, and tamper gates passed.'
