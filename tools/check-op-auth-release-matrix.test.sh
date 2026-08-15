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

write_policy() {
    local repo=$1
    local public_key_file=$2
    local release_manifest=$3
    local public_key manifest_sha
    public_key=$(tr -d '[:space:]' < "$public_key_file")
    manifest_sha=$(sha256_file "$release_manifest")
    {
        printf 'format=op-auth-release-policy-v1\n'
        printf 'abi=3\n'
        printf 'public_key=%s\n' "$public_key"
        printf 'release_manifest_sha256=%s\n' "$manifest_sha"
        printf 'source_revision=%s\n' "$source_revision"
        printf 'build_id=%s\n' "$build_id"
    } > "$repo/crates/op-auth-bridge/AUTH-RELEASE-POLICY"
}

mkdir -p \
    "$fixture_repo/tools" "$fixture_repo/scripts" \
    "$attacker_repo/tools" "$attacker_repo/scripts" \
    "$attacker_repo/crates/op-auth-bridge/prebuilt" \
    "$trusted_prebuilt" "$prebuilt"
cp "$script_dir/check-op-auth-release-matrix.sh" "$fixture_repo/tools/"
cp "$script_dir/check-op-auth-release-matrix.sh" "$attacker_repo/tools/"
printf '#!/usr/bin/env bash\nprintf "0.0.1\\n"\n' \
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
write_policy "$fixture_repo" \
    "$trusted_prebuilt/PROVENANCE_PUBKEY" "$prebuilt/RELEASE-MANIFEST"
valid_manifest=$temp_root/valid-release-manifest
valid_signature=$temp_root/valid-release-signature
valid_prebuilt=$temp_root/valid-prebuilt
cp "$prebuilt/RELEASE-MANIFEST" "$valid_manifest"
cp "$prebuilt/RELEASE-MANIFEST.sig" "$valid_signature"
mkdir -p "$valid_prebuilt"
cp -R "$prebuilt/." "$valid_prebuilt/"

OP_AUTH_PREBUILT_ROOT=$prebuilt \
    "$fixture_repo/tools/check-op-auth-release-matrix.sh" >/dev/null

OP_AUTH_PREBUILT_ROOT=$prebuilt \
    OP_AUTH_RELEASE_WORKSPACE_VERSION=$version \
    OP_AUTH_RELEASE_EXPECTED_OPENPENCIL_REVISION=$openpencil_revision \
    "$fixture_repo/tools/check-op-auth-release-matrix.sh" >/dev/null

if OP_AUTH_PREBUILT_ROOT=$prebuilt \
    OP_AUTH_RELEASE_WORKSPACE_VERSION=$version \
    "$fixture_repo/tools/check-op-auth-release-matrix.sh" >/dev/null 2>&1; then
    printf 'error: partial strict promotion identity was accepted\n' >&2
    exit 1
fi

if OP_AUTH_PREBUILT_ROOT=$prebuilt \
    OP_AUTH_RELEASE_WORKSPACE_VERSION=$version \
    OP_AUTH_RELEASE_EXPECTED_OPENPENCIL_REVISION=3333333333333333333333333333333333333333 \
    "$fixture_repo/tools/check-op-auth-release-matrix.sh" >/dev/null 2>&1; then
    printf 'error: mismatched expected public revision was accepted\n' >&2
    exit 1
fi

# Strict promotion may validate a newly generated, self-consistent signed
# matrix before the public source policy adopts its exact manifest digest.
new_version=9.8.8
new_matrix_lines=$temp_root/new-matrix-lines
: > "$new_matrix_lines"
for target in "${targets[@]}"; do
    target_dir=$prebuilt/$target
    artifact=libop_auth.a
    [[ "$target" == *-pc-windows-msvc ]] && artifact=op_auth.lib
    printf '%s\n' "$new_version" > "$target_dir/VERSION"
    sed "s/^version=$version$/version=$new_version/" \
        "$target_dir/PROVENANCE" > "$target_dir/PROVENANCE.next"
    mv "$target_dir/PROVENANCE.next" "$target_dir/PROVENANCE"
    sign_hex "$private_key" \
        "$target_dir/PROVENANCE" "$target_dir/PROVENANCE.sig"
    artifact_sha=$(sha256_file "$target_dir/$artifact")
    provenance_sha=$(sha256_file "$target_dir/PROVENANCE")
    hardening_sha=$(sha256_file "$target_dir/HARDENING-ATTESTATION")
    printf 'target=%s|artifact=%s|sha256=%s|provenance_sha256=%s|hardening_sha256=%s\n' \
        "$target" "$artifact" "$artifact_sha" "$provenance_sha" "$hardening_sha" \
        >> "$new_matrix_lines"
done
{
    printf 'format=op-auth-release-matrix-v1\n'
    printf 'version=%s\n' "$new_version"
    printf 'abi=3\n'
    printf 'source_revision=%s\n' "$source_revision"
    printf 'openpencil_revision=%s\n' "$openpencil_revision"
    printf 'build_id=%s\n' "$build_id"
    printf 'target_count=10\n'
    cat "$new_matrix_lines"
} > "$prebuilt/RELEASE-MANIFEST"
sign_hex "$private_key" \
    "$prebuilt/RELEASE-MANIFEST" "$prebuilt/RELEASE-MANIFEST.sig"
OP_AUTH_PREBUILT_ROOT=$prebuilt \
OP_AUTH_RELEASE_WORKSPACE_VERSION=$new_version \
OP_AUTH_RELEASE_EXPECTED_OPENPENCIL_REVISION=$openpencil_revision \
    "$fixture_repo/tools/check-op-auth-release-matrix.sh" >/dev/null
if OP_AUTH_PREBUILT_ROOT=$prebuilt \
    "$fixture_repo/tools/check-op-auth-release-matrix.sh" >/dev/null 2>&1; then
    printf 'error: generic consumer accepted an unadopted signed matrix\n' >&2
    exit 1
fi
cp -R "$valid_prebuilt/." "$prebuilt/"

if OP_AUTH_PREBUILT_ROOT=$prebuilt \
    OP_AUTH_RELEASE_WORKSPACE_VERSION=9.8.6 \
    OP_AUTH_RELEASE_EXPECTED_OPENPENCIL_REVISION=$openpencil_revision \
    "$fixture_repo/tools/check-op-auth-release-matrix.sh" >/dev/null 2>&1; then
    printf 'error: mismatched strict expected matrix version was accepted\n' >&2
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
write_policy "$fixture_repo" \
    "$trusted_prebuilt/PROVENANCE_PUBKEY" "$prebuilt/RELEASE-MANIFEST"
if OP_AUTH_PREBUILT_ROOT=$prebuilt \
    OP_AUTH_RELEASE_WORKSPACE_VERSION=$version \
    OP_AUTH_RELEASE_EXPECTED_OPENPENCIL_REVISION=$openpencil_revision \
    "$fixture_repo/tools/check-op-auth-release-matrix.sh" >/dev/null 2>&1; then
    printf 'error: non-canonical target order was accepted\n' >&2
    exit 1
fi
cp "$valid_manifest" "$prebuilt/RELEASE-MANIFEST"
cp "$valid_signature" "$prebuilt/RELEASE-MANIFEST.sig"
write_policy "$fixture_repo" \
    "$trusted_prebuilt/PROVENANCE_PUBKEY" "$prebuilt/RELEASE-MANIFEST"

# A different matrix signed by the same trusted private key remains a rollback
# or substitution until reviewed source explicitly adopts its exact digest.
sed 's/^version=9\.8\.7$/version=9.8.6/' \
    "$valid_manifest" > "$prebuilt/RELEASE-MANIFEST"
sign_hex "$private_key" \
    "$prebuilt/RELEASE-MANIFEST" "$prebuilt/RELEASE-MANIFEST.sig"
if OP_AUTH_PREBUILT_ROOT=$prebuilt \
    "$fixture_repo/tools/check-op-auth-release-matrix.sh" >/dev/null 2>&1; then
    printf 'error: same-key unadopted release matrix was accepted\n' >&2
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
write_policy "$attacker_repo" \
    "$attacker_repo/crates/op-auth-bridge/prebuilt/PROVENANCE_PUBKEY" \
    "$prebuilt/RELEASE-MANIFEST"
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
    'check-op-auth-release-matrix.test.sh: reusable identity, source adoption, rollback, trust-root, completeness, and tamper gates passed.'
