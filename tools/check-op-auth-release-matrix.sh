#!/usr/bin/env bash
# Verify one complete, signed op-auth production release matrix without
# modifying it. The public key and this verifier always come from the trusted
# OpenPencil checkout, never from the artifact commit being inspected.

set -euo pipefail

script_dir=$(CDPATH='' cd "$(dirname "$0")" && pwd)
repo_root=$(CDPATH='' cd "$script_dir/.." && pwd)
prebuilt_root=${OP_AUTH_PREBUILT_ROOT:-$repo_root/crates/op-auth-bridge/prebuilt}
expected_version=${OP_AUTH_RELEASE_WORKSPACE_VERSION:-}
expected_openpencil_revision=${OP_AUTH_RELEASE_EXPECTED_OPENPENCIL_REVISION:-}

if [[ "$#" -ne 0 ]]; then
    printf 'usage: %s\n' "$0" >&2
    exit 2
fi

if [[ -z "$expected_version" ]]; then
    expected_version=$("$repo_root/scripts/workspace-version.sh")
fi
if [[ ! "$expected_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+([+-][0-9A-Za-z.-]+)?$ ]]; then
    printf 'error: invalid expected OpenPencil version\n' >&2
    exit 2
fi
if [[ ! "$expected_openpencil_revision" =~ ^[0-9a-f]{40}$ ]]; then
    printf '%s\n' \
        'error: OP_AUTH_RELEASE_EXPECTED_OPENPENCIL_REVISION must be 40 lowercase hex characters' \
        >&2
    exit 2
fi

sha256_file() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | awk '{ print $1 }'
    else
        shasum -a 256 "$1" | awk '{ print $1 }'
    fi
}

require_regular_file() {
    if [[ -L "$1" || ! -f "$1" ]]; then
        printf 'error: required regular non-symlink file is missing: %s\n' "$1" >&2
        exit 1
    fi
}

require_exact_line() {
    local needle=$1
    local file=$2
    local count
    count=$(LC_ALL=C grep -Fxc -- "$needle" "$file" || true)
    if [[ "$count" -ne 1 ]]; then
        printf 'error: expected exactly one %s line in %s\n' "$needle" "$file" >&2
        exit 1
    fi
}

verify_ed25519_hex_signature() {
    local payload=$1
    local signature_file=$2
    local public_key_file=$3
    local label=$4
    local public_key_hex signature_hex verify_dir

    require_regular_file "$payload"
    require_regular_file "$signature_file"
    require_regular_file "$public_key_file"
    public_key_hex=$(tr -d '[:space:]' < "$public_key_file")
    signature_hex=$(tr -d '[:space:]' < "$signature_file")
    if [[ ! "$public_key_hex" =~ ^[0-9a-f]{64}$ \
        || ! "$signature_hex" =~ ^[0-9a-f]{128}$ ]]; then
        printf 'error: invalid Ed25519 hex encoding for %s\n' "$label" >&2
        exit 1
    fi

    verify_dir=$(mktemp -d "${TMPDIR:-/tmp}/op-auth-signature.XXXXXX")
    printf '302a300506032b6570032100%s' "$public_key_hex" \
        | xxd -r -p > "$verify_dir/public.der"
    printf '%s' "$signature_hex" | xxd -r -p > "$verify_dir/signature.bin"
    openssl pkey -pubin -inform DER \
        -in "$verify_dir/public.der" -out "$verify_dir/public.pem" >/dev/null
    if ! openssl pkeyutl -verify -rawin -pubin \
        -inkey "$verify_dir/public.pem" \
        -in "$payload" \
        -sigfile "$verify_dir/signature.bin" >/dev/null; then
        printf 'error: invalid Ed25519 signature for %s\n' "$label" >&2
        rm -rf "$verify_dir"
        exit 1
    fi
    rm -rf "$verify_dir"
}

manifest=$prebuilt_root/RELEASE-MANIFEST
manifest_signature=$prebuilt_root/RELEASE-MANIFEST.sig
# The artifact root is untrusted until verification succeeds. Its copy of the
# key, if any, is deliberately ignored: only the key in the current reviewed
# source checkout may authenticate a release matrix.
public_key=$repo_root/crates/op-auth-bridge/prebuilt/PROVENANCE_PUBKEY
verify_ed25519_hex_signature \
    "$manifest" "$manifest_signature" "$public_key" 'release matrix'

line_count=$(wc -l < "$manifest" | tr -d '[:space:]')
if [[ "$line_count" -ne 17 ]] || LC_ALL=C grep -q $'\r' "$manifest"; then
    printf 'error: release matrix must contain exactly 17 LF-terminated lines\n' >&2
    exit 1
fi

format_line=$(sed -n '1p' "$manifest")
version_line=$(sed -n '2p' "$manifest")
abi_line=$(sed -n '3p' "$manifest")
source_line=$(sed -n '4p' "$manifest")
openpencil_line=$(sed -n '5p' "$manifest")
build_line=$(sed -n '6p' "$manifest")
count_line=$(sed -n '7p' "$manifest")

[[ "$format_line" == 'format=op-auth-release-matrix-v1' ]] || {
    printf 'error: unsupported op-auth release matrix format\n' >&2
    exit 1
}
[[ "$version_line" == "version=$expected_version" ]] || {
    printf 'error: op-auth release matrix version does not match OpenPencil\n' >&2
    exit 1
}
[[ "$abi_line" == 'abi=3' ]] || {
    printf 'error: op-auth release matrix must be ABI 3\n' >&2
    exit 1
}
[[ "$count_line" == 'target_count=10' ]] || {
    printf 'error: op-auth release matrix must contain all 10 targets\n' >&2
    exit 1
}

source_revision=${source_line#source_revision=}
openpencil_revision=${openpencil_line#openpencil_revision=}
build_id=${build_line#build_id=}
[[ "$source_line" == "source_revision=$source_revision" \
    && "$source_revision" =~ ^[0-9a-f]{40}$ ]] || {
    printf 'error: invalid private source revision in release matrix\n' >&2
    exit 1
}
[[ "$openpencil_line" == "openpencil_revision=$openpencil_revision" \
    && "$openpencil_revision" =~ ^[0-9a-f]{40}$ ]] || {
    printf 'error: invalid OpenPencil revision in release matrix\n' >&2
    exit 1
}
[[ "$build_line" == "build_id=$build_id" \
    && "$build_id" =~ ^[A-Za-z0-9][A-Za-z0-9._-]{0,58}$ \
    && "$build_id" != *..* \
    && "$build_id" != *.lock ]] || {
    printf 'error: invalid immutable build id in release matrix\n' >&2
    exit 1
}
if [[ "$openpencil_revision" != "$expected_openpencil_revision" ]]; then
    printf 'error: release matrix is not bound to the expected public base revision\n' >&2
    exit 1
fi

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

index=0
for target in "${targets[@]}"; do
    line_number=$((index + 8))
    matrix_line=$(sed -n "${line_number}p" "$manifest")
    if [[ ! "$matrix_line" =~ ^target=([^|]+)\|artifact=([^|]+)\|sha256=([0-9a-f]{64})\|provenance_sha256=([0-9a-f]{64})\|hardening_sha256=([0-9a-f]{64})$ ]]; then
        printf 'error: malformed release matrix target line %s\n' "$line_number" >&2
        exit 1
    fi
    line_target=${BASH_REMATCH[1]}
    artifact_name=${BASH_REMATCH[2]}
    artifact_sha=${BASH_REMATCH[3]}
    provenance_sha=${BASH_REMATCH[4]}
    hardening_sha=${BASH_REMATCH[5]}
    [[ "$line_target" == "$target" ]] || {
        printf 'error: release matrix targets are incomplete or not canonically ordered\n' >&2
        exit 1
    }

    expected_artifact=libop_auth.a
    [[ "$target" == *-pc-windows-msvc ]] && expected_artifact=op_auth.lib
    [[ "$artifact_name" == "$expected_artifact" ]] || {
        printf 'error: release matrix has the wrong artifact name for %s\n' "$target" >&2
        exit 1
    }

    target_dir=$prebuilt_root/$target
    [[ -d "$target_dir" && ! -L "$target_dir" ]] || {
        printf 'error: release matrix target directory is missing: %s\n' "$target" >&2
        exit 1
    }
    expected_files=$(printf '%s\n' \
        ABI_VERSION HARDENING-ATTESTATION PROVENANCE PROVENANCE.sig \
        SHA256 VERSION "$artifact_name" | LC_ALL=C sort)
    actual_files=$(find "$target_dir" -mindepth 1 -maxdepth 1 -print \
        | sed 's#^.*/##' | LC_ALL=C sort)
    [[ "$actual_files" == "$expected_files" ]] || {
        printf 'error: release matrix target %s has missing or unexpected files\n' "$target" >&2
        exit 1
    }

    artifact=$target_dir/$artifact_name
    provenance=$target_dir/PROVENANCE
    hardening=$target_dir/HARDENING-ATTESTATION
    for file in \
        "$artifact" "$target_dir/VERSION" "$target_dir/ABI_VERSION" \
        "$target_dir/SHA256" "$provenance" "$target_dir/PROVENANCE.sig" \
        "$hardening"; do
        require_regular_file "$file"
    done

    [[ "$(tr -d '[:space:]' < "$target_dir/VERSION")" == "$expected_version" ]] || {
        printf 'error: %s VERSION does not match OpenPencil\n' "$target" >&2
        exit 1
    }
    [[ "$(tr -d '[:space:]' < "$target_dir/ABI_VERSION")" == 3 ]] || {
        printf 'error: %s must be ABI 3\n' "$target" >&2
        exit 1
    }
    [[ "$(tr -d '[:space:]' < "$target_dir/SHA256")" == "$artifact_sha" \
        && "$(sha256_file "$artifact")" == "$artifact_sha" ]] || {
        printf 'error: %s artifact digest does not match the release matrix\n' "$target" >&2
        exit 1
    }
    [[ "$(sha256_file "$provenance")" == "$provenance_sha" ]] || {
        printf 'error: %s provenance digest does not match the release matrix\n' "$target" >&2
        exit 1
    }
    [[ "$(sha256_file "$hardening")" == "$hardening_sha" ]] || {
        printf 'error: %s hardening digest does not match the release matrix\n' "$target" >&2
        exit 1
    }

    verify_ed25519_hex_signature \
        "$provenance" "$target_dir/PROVENANCE.sig" "$public_key" "$target provenance"
    require_exact_line "target=$target" "$provenance"
    require_exact_line "artifact=$artifact_name" "$provenance"
    require_exact_line "version=$expected_version" "$provenance"
    require_exact_line 'abi=3' "$provenance"
    require_exact_line "sha256=$artifact_sha" "$provenance"
    require_exact_line "source_revision=$source_revision" "$provenance"
    require_exact_line "build_id=$build_id.a3.$hardening_sha" "$provenance"
    if ! grep -Eq '^hardening=op-auth-(hardened-v1|signed-unobfuscated-v1)$' \
        "$provenance"; then
        printf 'error: %s provenance has no recognized hardening profile\n' "$target" >&2
        exit 1
    fi
    require_exact_line "target=$target" "$hardening"
    require_exact_line "artifact=$artifact_name" "$hardening"
    require_exact_line "artifact_sha256=$artifact_sha" "$hardening"
    require_exact_line "source_revision=$source_revision" "$hardening"
    require_exact_line "release_build_id=$build_id" "$hardening"
    require_exact_line "openpencil_revision=$openpencil_revision" "$hardening"
    index=$((index + 1))
done

printf 'check-op-auth-release-matrix.sh: verified signed ABI 3 matrix for %s (%s).\n' \
    "$expected_version" "$build_id"
