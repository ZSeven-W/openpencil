#!/usr/bin/env bash
# Sign an already-verified unsigned matrix without executing candidate bytes.

set -euo pipefail

script_dir=$(CDPATH='' cd "$(dirname "$0")" && pwd)
# shellcheck source=op-auth-candidate-targets.sh
source "$script_dir/op-auth-candidate-targets.sh"

candidate_root=
trusted_public_key=
signing_key=
output_root=
version=
private_head_sha=
openpencil_sha=
build_id=

usage() {
    cat >&2 <<'EOF'
usage: sign-op-auth-candidates.sh \
  --candidate-root VERIFIED_DIR --trusted-public-key PROVENANCE_PUBKEY \
  --signing-key ED25519_PRIVATE_KEY.pem --output-root NEW_DIRECTORY \
  --version VERSION --private-head-sha FULL_SHA --openpencil-sha FULL_SHA \
  --build-id IMMUTABLE_ID
EOF
}

while [[ "$#" -gt 0 ]]; do
    case "$1" in
        --candidate-root|--trusted-public-key|--signing-key|--output-root|\
        --version|--private-head-sha|--openpencil-sha|--build-id)
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
    candidate_root trusted_public_key signing_key output_root version \
    private_head_sha openpencil_sha build_id; do
    [[ -n "${!required}" ]] || {
        printf 'error: --%s is required\n' "${required//_/-}" >&2
        exit 2
    }
done
[[ -d "$candidate_root" && ! -L "$candidate_root" ]] || {
    printf 'error: verified candidate root is missing or symlinked\n' >&2
    exit 2
}
for file in "$trusted_public_key" "$signing_key"; do
    [[ -f "$file" && ! -L "$file" ]] || {
        printf 'error: signing inputs must be regular non-symlink files\n' >&2
        exit 2
    }
done
[[ ! -e "$output_root" && ! -L "$output_root" ]] || {
    printf 'error: signed output root must be a new path\n' >&2
    exit 2
}
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+([+-][0-9A-Za-z.-]+)?$ \
    && "$private_head_sha" =~ ^[0-9a-f]{40}$ \
    && "$openpencil_sha" =~ ^[0-9a-f]{40}$ \
    && "$build_id" =~ ^[A-Za-z0-9]([A-Za-z0-9._-]{0,57}[A-Za-z0-9])?$ \
    && "$build_id" != *..* && "$build_id" != *.lock ]] || {
    printf 'error: invalid immutable signing metadata\n' >&2
    exit 2
}

output_parent=$(CDPATH='' cd "$(dirname "$output_root")" && pwd)
temp_dir=$(mktemp -d "$output_parent/.op-auth-sign.XXXXXX")
staged=$temp_dir/staged
success=0
cleanup() {
    chmod -R u+w "$temp_dir" 2>/dev/null || true
    rm -rf "$temp_dir"
    if [[ "$success" -ne 1 ]]; then
        chmod -R u+w "$output_root" 2>/dev/null || true
        rm -rf "$output_root"
    fi
}
trap cleanup EXIT HUP INT TERM
mkdir -m 700 "$staged"

sha256_file() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | awk '{ print $1 }'
    else
        shasum -a 256 "$1" | awk '{ print $1 }'
    fi
}

require_regular_file() {
    [[ -f "$1" && ! -L "$1" ]] || {
        printf 'error: signing input is missing or symlinked: %s\n' "$1" >&2
        return 1
    }
}

require_candidate_value() {
    local key=$1
    local expected=$2
    local candidate=$3
    [[ "$(LC_ALL=C grep -Fxc "$key=$expected" "$candidate" || true)" -eq 1 ]] || {
        printf 'error: verified candidate %s binding changed before signing\n' "$key" >&2
        return 1
    }
}

sign_hex() {
    local payload=$1
    local output=$2
    local binary_signature=$temp_dir/signature.bin
    openssl pkeyutl -sign -rawin \
        -inkey "$signing_key" \
        -in "$payload" \
        -out "$binary_signature"
    openssl pkeyutl -verify -rawin -pubin \
        -inkey "$temp_dir/public.pem" \
        -in "$payload" \
        -sigfile "$binary_signature" >/dev/null
    od -An -v -tx1 "$binary_signature" | tr -d '[:space:]' > "$output"
    printf '\n' >> "$output"
}

trusted_hex=$(tr -d '[:space:]' < "$trusted_public_key")
[[ "$trusted_hex" =~ ^[0-9a-f]{64}$ ]] || {
    printf 'error: trusted public root must be exactly 32 lowercase hex bytes\n' >&2
    exit 1
}
openssl pkey -in "$signing_key" -pubout -outform DER \
    | od -An -v -tx1 | tr -d '[:space:]' > "$temp_dir/signing-public.hex"
signing_der=$(cat "$temp_dir/signing-public.hex")
[[ "$signing_der" == "302a300506032b6570032100$trusted_hex" ]] || {
    printf 'error: signing key does not match the trusted public release root\n' >&2
    exit 1
}
printf '302a300506032b6570032100%s' "$trusted_hex" \
    | xxd -r -p > "$temp_dir/public.der"
openssl pkey -pubin -inform DER \
    -in "$temp_dir/public.der" -out "$temp_dir/public.pem" >/dev/null

matrix_lines=$temp_dir/matrix-lines
: > "$matrix_lines"
while IFS= read -r target; do
    source_target=$candidate_root/$target
    artifact=$(op_auth_candidate_artifact_name "$target")
    candidate=$source_target/CANDIDATE
    for name in "$artifact" ABI_VERSION CANDIDATE HARDENING-ATTESTATION SHA256 VERSION; do
        require_regular_file "$source_target/$name"
    done
    artifact_sha=$(sha256_file "$source_target/$artifact")
    hardening_sha=$(sha256_file "$source_target/HARDENING-ATTESTATION")
    require_candidate_value target "$target" "$candidate"
    require_candidate_value version "$version" "$candidate"
    require_candidate_value abi 3 "$candidate"
    require_candidate_value source_revision "$private_head_sha" "$candidate"
    require_candidate_value openpencil_revision "$openpencil_sha" "$candidate"
    require_candidate_value build_id "$build_id" "$candidate"
    require_candidate_value artifact "$artifact" "$candidate"
    require_candidate_value artifact_sha256 "$artifact_sha" "$candidate"
    require_candidate_value hardening_sha256 "$hardening_sha" "$candidate"

    target_root=$staged/$target
    mkdir -m 700 "$target_root"
    cp "$source_target/$artifact" "$target_root/$artifact"
    cp "$source_target/ABI_VERSION" "$target_root/ABI_VERSION"
    cp "$source_target/HARDENING-ATTESTATION" "$target_root/HARDENING-ATTESTATION"
    cp "$source_target/SHA256" "$target_root/SHA256"
    cp "$source_target/VERSION" "$target_root/VERSION"

    hardening=op-auth-hardened-v1
    [[ "$target" == *-pc-windows-msvc ]] \
        && hardening=op-auth-signed-unobfuscated-v1
    provenance=$target_root/PROVENANCE
    cat > "$provenance" <<EOF
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
    sign_hex "$provenance" "$target_root/PROVENANCE.sig"
    provenance_sha=$(sha256_file "$provenance")
    printf 'target=%s|artifact=%s|sha256=%s|provenance_sha256=%s|hardening_sha256=%s\n' \
        "$target" "$artifact" "$artifact_sha" "$provenance_sha" "$hardening_sha" \
        >> "$matrix_lines"
done < <(op_auth_candidate_targets)

manifest=$staged/RELEASE-MANIFEST
{
    printf 'format=op-auth-release-matrix-v1\n'
    printf 'version=%s\n' "$version"
    printf 'abi=3\n'
    printf 'source_revision=%s\n' "$private_head_sha"
    printf 'openpencil_revision=%s\n' "$openpencil_sha"
    printf 'build_id=%s\n' "$build_id"
    printf 'target_count=10\n'
    cat "$matrix_lines"
} > "$manifest"
sign_hex "$manifest" "$staged/RELEASE-MANIFEST.sig"

mv "$staged" "$output_root"
chmod -R a-w "$output_root"
success=1
printf 'sign-op-auth-candidates: signed complete ABI-v3 matrix (%s)\n' "$build_id"
