#!/usr/bin/env bash
# Validate the explicit auth archive passed to a mobile final-link step.

set -euo pipefail

script_dir=$(CDPATH= cd "$(dirname "$0")" && pwd)
repo_root=$(CDPATH= cd "$script_dir/.." && pwd)
configuration=${CONFIGURATION:-}
archive=${OP_AUTH_ARCHIVE:-}
target=${OP_AUTH_TARGET:-}

[[ -n "$archive" ]] || exit 0
case "$archive" in
    /*) ;;
    *)
        printf 'error: OP_AUTH_ARCHIVE must be an absolute path\n' >&2
        exit 1
        ;;
esac
if [[ -L "$archive" || ! -f "$archive" || "$(basename "$archive")" != libop_auth.a ]]; then
    printf 'error: OP_AUTH_ARCHIVE must select a regular non-symlink libop_auth.a\n' >&2
    exit 1
fi
archive=$(cd "$(dirname "$archive")" && pwd -P)/libop_auth.a

if [[ "$configuration" == Debug ]]; then
    case "${OPENPENCIL_DEV_OP_AUTH_ABI_VERSION:-}" in
        2|3) ;;
        *)
            printf '%s\n' \
                'error: Debug auth linking requires OPENPENCIL_DEV_OP_AUTH_ABI_VERSION=2 or 3' \
                >&2
            exit 1
            ;;
    esac
    printf 'warning: linking unsigned local op-auth into a Debug mobile app\n' >&2
    exit 0
fi

if [[ "$configuration" != Release ]]; then
    printf 'error: auth archive linking requires CONFIGURATION=Debug or Release\n' >&2
    exit 1
fi
if [[ -n "${OPENPENCIL_DEV_OP_AUTH_ABI_VERSION:-}" ]]; then
    printf 'error: the unsigned development auth lane is forbidden in Release\n' >&2
    exit 1
fi
case "$target" in
    aarch64-apple-ios|aarch64-apple-ios-sim) ;;
    *)
        printf 'error: Release auth linking requires an explicit supported OP_AUTH_TARGET\n' >&2
        exit 1
        ;;
esac

prebuilt_root=$repo_root/crates/op-auth-bridge/prebuilt
target_dir=$prebuilt_root/$target
expected_archive=$target_dir/libop_auth.a
if [[ ! -f "$expected_archive" ]]; then
    printf 'error: no signed production auth archive is committed for %s\n' "$target" >&2
    exit 1
fi
expected_archive=$(cd "$target_dir" && pwd -P)/libop_auth.a
if [[ "$archive" != "$expected_archive" ]]; then
    printf 'error: Release may link only the repository signed archive for %s\n' "$target" >&2
    exit 1
fi

workspace_version=$(
    sed -n '/^\[workspace\.package\]$/,/^\[/s/^version = "\([^"]*\)"$/\1/p' \
        "$repo_root/Cargo.toml" | head -n 1
)
artifact_version=$(tr -d '[:space:]' < "$target_dir/VERSION")
if [[ -z "$workspace_version" || "$artifact_version" != "$workspace_version" ]]; then
    printf 'error: Release auth archive version does not match OpenPencil\n' >&2
    exit 1
fi

abi=$(tr -d '[:space:]' < "$target_dir/ABI_VERSION")
case "$abi" in
    2|3) ;;
    *)
        printf 'error: Release auth archive must use signed ABI 2 or 3\n' >&2
        exit 1
        ;;
esac

if command -v sha256sum >/dev/null 2>&1; then
    actual_sha256=$(sha256sum "$archive" | awk '{ print $1 }')
else
    actual_sha256=$(shasum -a 256 "$archive" | awk '{ print $1 }')
fi
expected_sha256=$(tr -d '[:space:]' < "$target_dir/SHA256")
if [[ "$actual_sha256" != "$expected_sha256" ]]; then
    printf 'error: Release auth archive SHA-256 does not match\n' >&2
    exit 1
fi

manifest=$target_dir/PROVENANCE
signature=$target_dir/PROVENANCE.sig
public_key=$prebuilt_root/PROVENANCE_PUBKEY
for signed_input in "$manifest" "$signature" "$public_key"; do
    [[ -s "$signed_input" ]] || {
        printf 'error: signed Release auth provenance is incomplete\n' >&2
        exit 1
    }
done
for field in \
    "target=$target" \
    'artifact=libop_auth.a' \
    "version=$workspace_version" \
    "abi=$abi" \
    "sha256=$actual_sha256"; do
    grep -Fxq "$field" "$manifest" || {
        printf 'error: signed Release auth provenance does not match %s\n' "$field" >&2
        exit 1
    }
done

command -v openssl >/dev/null 2>&1 || {
    printf 'error: openssl is required to verify Release auth provenance\n' >&2
    exit 1
}
command -v xxd >/dev/null 2>&1 || {
    printf 'error: xxd is required to verify Release auth provenance\n' >&2
    exit 1
}
public_key_hex=$(tr -d '[:space:]' < "$public_key")
signature_hex=$(tr -d '[:space:]' < "$signature")
[[ "$public_key_hex" =~ ^[0-9a-f]{64}$ && "$signature_hex" =~ ^[0-9a-f]{128}$ ]] || {
    printf 'error: signed Release auth provenance encoding is invalid\n' >&2
    exit 1
}

temp_dir=$(mktemp -d "${TMPDIR:-/tmp}/op-auth-mobile-link.XXXXXX")
cleanup() {
    rm -rf "$temp_dir"
}
trap cleanup EXIT
printf '302a300506032b6570032100%s' "$public_key_hex" \
    | xxd -r -p > "$temp_dir/public.der"
printf '%s' "$signature_hex" | xxd -r -p > "$temp_dir/provenance.sig"
openssl pkey -pubin -inform DER \
    -in "$temp_dir/public.der" -out "$temp_dir/public.pem" >/dev/null
openssl pkeyutl -verify -rawin -pubin \
    -inkey "$temp_dir/public.pem" \
    -in "$manifest" \
    -sigfile "$temp_dir/provenance.sig" \
    >/dev/null || {
        printf 'error: Release auth provenance signature verification failed\n' >&2
        exit 1
    }

OP_AUTH_PREBUILT_ROOT="$prebuilt_root" \
    bash "$repo_root/tools/check-op-auth-prebuilt.sh" --require-hardened >/dev/null
