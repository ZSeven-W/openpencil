#!/usr/bin/env bash
# Verify the exact signed matrix before it is staged into an auth-only commit.

set -euo pipefail

script_dir=$(CDPATH='' cd "$(dirname "$0")" && pwd)
# shellcheck source=op-auth-candidate-targets.sh
source "$script_dir/op-auth-candidate-targets.sh"

signed_root=
trusted_public_key=
version=
private_head_sha=
openpencil_sha=
build_id=

usage() {
    cat >&2 <<'EOF'
usage: verify-signed-op-auth-matrix.sh \
  --signed-root DIR --trusted-public-key PROVENANCE_PUBKEY \
  --version VERSION --private-head-sha FULL_SHA --openpencil-sha FULL_SHA \
  --build-id IMMUTABLE_ID
EOF
}

while [[ "$#" -gt 0 ]]; do
    case "$1" in
        --signed-root|--trusted-public-key|--version|--private-head-sha|\
        --openpencil-sha|--build-id)
            [[ "$#" -ge 2 ]] || {
                usage
                exit 2
            }
            name=${1#--}
            name=${name//-/_}
            printf -v "$name" '%s' "$2"
            shift 2
            ;;
        *)
            usage
            exit 2
            ;;
    esac
done

for required in \
    signed_root trusted_public_key version private_head_sha openpencil_sha build_id; do
    [[ -n "${!required}" ]] || {
        printf 'error: --%s is required\n' "${required//_/-}" >&2
        exit 2
    }
done
[[ -d "$signed_root" && ! -L "$signed_root" \
    && -f "$trusted_public_key" && ! -L "$trusted_public_key" \
    && "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+([+-][0-9A-Za-z.-]+)?$ \
    && "$private_head_sha" =~ ^[0-9a-f]{40}$ \
    && "$openpencil_sha" =~ ^[0-9a-f]{40}$ ]] || {
    printf 'error: invalid signed-matrix verification input\n' >&2
    exit 2
}

temp_dir=$(mktemp -d "${TMPDIR:-/tmp}/op-auth-signed-verify.XXXXXX")
cleanup() {
    rm -rf "$temp_dir"
}
trap cleanup EXIT HUP INT TERM

sha256_file() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | awk '{ print $1 }'
    else
        shasum -a 256 "$1" | awk '{ print $1 }'
    fi
}

require_regular_file() {
    [[ -f "$1" && ! -L "$1" ]] || {
        printf 'error: required signed matrix file is missing or symlinked: %s\n' "$1" >&2
        return 1
    }
}

verify_signature() {
    local payload=$1
    local signature_hex_file=$2
    local label=$3
    local signature_hex binary_signature=$temp_dir/signature.bin
    require_regular_file "$payload"
    require_regular_file "$signature_hex_file"
    signature_hex=$(tr -d '[:space:]' < "$signature_hex_file")
    [[ "$signature_hex" =~ ^[0-9a-f]{128}$ ]] || {
        printf 'error: invalid Ed25519 signature encoding for %s\n' "$label" >&2
        return 1
    }
    printf '%s' "$signature_hex" | xxd -r -p > "$binary_signature"
    openssl pkeyutl -verify -rawin -pubin \
        -inkey "$temp_dir/public.pem" \
        -in "$payload" \
        -sigfile "$binary_signature" >/dev/null || {
        printf 'error: invalid Ed25519 signature for %s\n' "$label" >&2
        return 1
    }
}

trusted_hex=$(tr -d '[:space:]' < "$trusted_public_key")
[[ "$trusted_hex" =~ ^[0-9a-f]{64}$ ]] || {
    printf 'error: malformed trusted public release root\n' >&2
    exit 1
}
printf '302a300506032b6570032100%s' "$trusted_hex" \
    | xxd -r -p > "$temp_dir/public.der"
openssl pkey -pubin -inform DER \
    -in "$temp_dir/public.der" -out "$temp_dir/public.pem" >/dev/null

expected_root_entries=$temp_dir/expected-root-entries
actual_root_entries=$temp_dir/actual-root-entries
{
    printf '%s\n' RELEASE-MANIFEST RELEASE-MANIFEST.sig
    op_auth_candidate_targets
} | LC_ALL=C sort > "$expected_root_entries"
find "$signed_root" -mindepth 1 -maxdepth 1 -print \
    | sed 's#^.*/##' | LC_ALL=C sort > "$actual_root_entries"
cmp -s "$expected_root_entries" "$actual_root_entries" || {
    printf 'error: signed matrix root has missing or unexpected entries\n' >&2
    exit 1
}

matrix_lines=$temp_dir/matrix-lines
: > "$matrix_lines"
while IFS= read -r target; do
    target_root=$signed_root/$target
    [[ -d "$target_root" && ! -L "$target_root" ]] || {
        printf 'error: signed target directory is missing or symlinked: %s\n' "$target" >&2
        exit 1
    }
    artifact=$(op_auth_candidate_artifact_name "$target")
    expected_files=$temp_dir/expected-files
    actual_files=$temp_dir/actual-files
    printf '%s\n' \
        "$artifact" ABI_VERSION HARDENING-ATTESTATION PROVENANCE \
        PROVENANCE.sig SHA256 VERSION | LC_ALL=C sort > "$expected_files"
    find "$target_root" -mindepth 1 -maxdepth 1 -print \
        | sed 's#^.*/##' | LC_ALL=C sort > "$actual_files"
    cmp -s "$expected_files" "$actual_files" || {
        printf 'error: signed target %s has missing or unexpected files\n' "$target" >&2
        exit 1
    }
    for name in \
        "$artifact" ABI_VERSION HARDENING-ATTESTATION PROVENANCE \
        PROVENANCE.sig SHA256 VERSION; do
        require_regular_file "$target_root/$name"
    done
    artifact_sha=$(sha256_file "$target_root/$artifact")
    hardening_sha=$(sha256_file "$target_root/HARDENING-ATTESTATION")
    printf '%s\n' "$version" > "$temp_dir/expected-version"
    printf '3\n' > "$temp_dir/expected-abi"
    printf '%s\n' "$artifact_sha" > "$temp_dir/expected-sha"
    cmp -s "$temp_dir/expected-version" "$target_root/VERSION" \
        && cmp -s "$temp_dir/expected-abi" "$target_root/ABI_VERSION" \
        && cmp -s "$temp_dir/expected-sha" "$target_root/SHA256" || {
        printf 'error: signed artifact metadata mismatch for %s\n' "$target" >&2
        exit 1
    }
    hardening=op-auth-hardened-v1
    [[ "$target" == *-pc-windows-msvc ]] \
        && hardening=op-auth-signed-unobfuscated-v1
    expected_provenance=$temp_dir/expected-provenance
    cat > "$expected_provenance" <<EOF
format=1
target=$target
artifact=$artifact
version=$version
abi=3
sha256=$artifact_sha
hardening=$hardening
source_revision=$private_head_sha
build_id=$build_id.a3.$hardening_sha
EOF
    cmp -s "$expected_provenance" "$target_root/PROVENANCE" || {
        printf 'error: non-canonical signed provenance for %s\n' "$target" >&2
        exit 1
    }
    verify_signature \
        "$target_root/PROVENANCE" "$target_root/PROVENANCE.sig" "$target provenance"
    provenance_sha=$(sha256_file "$target_root/PROVENANCE")
    printf 'target=%s|artifact=%s|sha256=%s|provenance_sha256=%s|hardening_sha256=%s\n' \
        "$target" "$artifact" "$artifact_sha" "$provenance_sha" "$hardening_sha" \
        >> "$matrix_lines"
done < <(op_auth_candidate_targets)

expected_manifest=$temp_dir/expected-manifest
{
    printf 'format=op-auth-release-matrix-v1\n'
    printf 'version=%s\n' "$version"
    printf 'abi=3\n'
    printf 'source_revision=%s\n' "$private_head_sha"
    printf 'openpencil_revision=%s\n' "$openpencil_sha"
    printf 'build_id=%s\n' "$build_id"
    printf 'target_count=10\n'
    cat "$matrix_lines"
} > "$expected_manifest"
require_regular_file "$signed_root/RELEASE-MANIFEST"
require_regular_file "$signed_root/RELEASE-MANIFEST.sig"
cmp -s "$expected_manifest" "$signed_root/RELEASE-MANIFEST" || {
    printf 'error: signed release manifest is incomplete, stale, or non-canonical\n' >&2
    exit 1
}
verify_signature \
    "$signed_root/RELEASE-MANIFEST" \
    "$signed_root/RELEASE-MANIFEST.sig" \
    'release matrix'

printf 'verify-signed-op-auth-matrix: verified signed ABI-v3 matrix (%s)\n' "$build_id"
