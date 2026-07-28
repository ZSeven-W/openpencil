#!/usr/bin/env bash
# Package an already-hardened private op-auth ABI-v2 archive.
#
# This script intentionally does not strip or obfuscate an existing archive:
# those transformations must happen while rebuilding from private source, where
# the linker can prove the result. The script audits, signs, and stages new
# bytes in a fresh output directory without touching committed artifacts.

set -euo pipefail

script_dir=$(CDPATH= cd "$(dirname "$0")" && pwd)
artifact=
target=
version=
source_revision=
build_id=
signing_key=
output_root=

usage() {
    cat >&2 <<'EOF'
usage: package-op-auth-prebuilt.sh \
  --artifact PATH --target TARGET --version VERSION \
  --source-revision FULL_HEX_REVISION --build-id ID \
  --signing-key ED25519_PRIVATE_KEY.pem --output-root NEW_DIRECTORY
EOF
}

while [[ "$#" -gt 0 ]]; do
    case "$1" in
        --artifact|--target|--version|--source-revision|--build-id|--signing-key|--output-root)
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

for required in artifact target version source_revision build_id signing_key output_root; do
    if [[ -z "${!required}" ]]; then
        printf 'error: --%s is required\n' "${required//_/-}" >&2
        exit 2
    fi
done

case "$target" in
    aarch64-apple-darwin|aarch64-pc-windows-msvc|aarch64-unknown-linux-gnu|\
    x86_64-apple-darwin|x86_64-pc-windows-msvc|x86_64-unknown-linux-gnu)
        ;;
    *)
        printf 'error: unsupported target: %s\n' "$target" >&2
        exit 2
        ;;
esac

[[ -f "$artifact" ]] || {
    printf 'error: artifact is not a regular file\n' >&2
    exit 2
}
[[ -f "$signing_key" ]] || {
    printf 'error: Ed25519 signing key is not a regular file\n' >&2
    exit 2
}
[[ "$source_revision" =~ ^([0-9a-fA-F]{40}|[0-9a-fA-F]{64})$ ]] || {
    printf 'error: source revision must be a full 40- or 64-digit hexadecimal revision\n' >&2
    exit 2
}
[[ "$build_id" =~ ^[A-Za-z0-9._-]{1,128}$ ]] || {
    printf 'error: build id must use 1-128 ASCII letters, digits, dot, underscore, or hyphen\n' >&2
    exit 2
}
[[ "$version" =~ ^[0-9A-Za-z.+-]{1,64}$ ]] || {
    printf 'error: version contains unsupported characters\n' >&2
    exit 2
}
[[ ! -e "$output_root" ]] || {
    printf 'error: output root already exists; refusing to overwrite it\n' >&2
    exit 2
}
command -v openssl >/dev/null 2>&1 || {
    printf 'error: openssl is required for Ed25519 provenance signing\n' >&2
    exit 1
}

artifact_name=libop_auth.a
if [[ "$target" == *-pc-windows-msvc ]]; then
    artifact_name=op_auth.lib
fi

temp_dir=$(mktemp -d "${TMPDIR:-/tmp}/op-auth-package.XXXXXX")
cleanup() {
    rm -rf "$temp_dir"
}
trap cleanup EXIT

target_dir=$output_root/$target
mkdir -p "$target_dir"
cp "$artifact" "$target_dir/$artifact_name"
printf '%s\n' "$version" > "$target_dir/VERSION"
printf '2\n' > "$target_dir/ABI_VERSION"

if command -v sha256sum >/dev/null 2>&1; then
    sha256=$(sha256sum "$target_dir/$artifact_name" | awk '{ print $1 }')
else
    sha256=$(shasum -a 256 "$target_dir/$artifact_name" | awk '{ print $1 }')
fi
printf '%s\n' "$sha256" > "$target_dir/SHA256"

cat > "$target_dir/PROVENANCE" <<EOF
format=1
target=$target
artifact=$artifact_name
version=$version
abi=2
sha256=$sha256
hardening=op-auth-hardened-v1
source_revision=$source_revision
build_id=$build_id
EOF

openssl pkey -in "$signing_key" -pubout -out "$temp_dir/public.pem" >/dev/null
public_der_hex=$(
    openssl pkey -pubin -in "$temp_dir/public.pem" -outform DER \
        | od -An -v -tx1 \
        | tr -d '[:space:]'
)
public_prefix=302a300506032b6570032100
case "$public_der_hex" in
    "$public_prefix"[0-9a-f][0-9a-f][0-9a-f][0-9a-f]*)
        ;;
    *)
        printf 'error: signing key is not an Ed25519 private key\n' >&2
        exit 1
        ;;
esac
public_key_hex=${public_der_hex#"$public_prefix"}
[[ ${#public_key_hex} -eq 64 ]] || {
    printf 'error: unexpected Ed25519 public key encoding\n' >&2
    exit 1
}
printf '%s\n' "$public_key_hex" > "$output_root/PROVENANCE_PUBKEY"

openssl pkeyutl -sign -rawin \
    -inkey "$signing_key" \
    -in "$target_dir/PROVENANCE" \
    -out "$temp_dir/provenance.sig"
openssl pkeyutl -verify -rawin -pubin \
    -inkey "$temp_dir/public.pem" \
    -in "$target_dir/PROVENANCE" \
    -sigfile "$temp_dir/provenance.sig" \
    >/dev/null
od -An -v -tx1 "$temp_dir/provenance.sig" \
    | tr -d '[:space:]' \
    > "$target_dir/PROVENANCE.sig"
printf '\n' >> "$target_dir/PROVENANCE.sig"

OP_AUTH_PREBUILT_ROOT="$output_root" \
    bash "$script_dir/check-op-auth-prebuilt.sh" --require-hardened

printf 'staged signed op-auth ABI-v2 artifact at %s\n' "$target_dir"
