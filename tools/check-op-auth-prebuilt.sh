#!/usr/bin/env bash
# Audit committed op-auth archives without modifying their bytes.
#
# Legacy ABI-v1 artifacts are integrity-pinned compatibility inputs and may
# still contain source/debug metadata. ABI-v2+ artifacts are production
# collaboration inputs: they must pass the hardened profile and signed
# provenance checks in addition to exposing only the documented C ABI.

set -euo pipefail

script_dir=$(CDPATH= cd "$(dirname "$0")" && pwd)
repo_root=$(CDPATH= cd "$script_dir/.." && pwd)
cd "$repo_root"

require_hardened=0
if [[ "${1:-}" == "--require-hardened" ]]; then
    require_hardened=1
    shift
fi
if [[ "$#" -ne 0 ]]; then
    printf 'usage: %s [--require-hardened]\n' "$0" >&2
    exit 2
fi

prebuilt_root=${OP_AUTH_PREBUILT_ROOT:-crates/op-auth-bridge/prebuilt}
trusted_public_key=$repo_root/crates/op-auth-bridge/prebuilt/PROVENANCE_PUBKEY
failures=()
artifact_count=0
signature_verification_error=
verification_dir=

record_failure() {
    failures+=("$1")
}

has_exact_manifest_line() {
    local expected=$1
    local manifest=$2
    local count
    count=$(LC_ALL=C grep -Fxc -- "$expected" "$manifest" || true)
    [[ "$count" -eq 1 ]]
}

sha256_file() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | awk '{ print $1 }'
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$1" | awk '{ print $1 }'
    else
        return 127
    fi
}

prepare_verification_dir() {
    if [[ -n "$verification_dir" ]]; then
        return 0
    fi
    verification_dir=$(mktemp -d "${TMPDIR:-/tmp}/op-auth-prebuilt-verify.XXXXXX") \
        || {
            signature_verification_error="could not create a provenance verification directory"
            return 1
        }
    if ! chmod 700 "$verification_dir"; then
        signature_verification_error="could not secure the provenance verification directory"
        return 1
    fi
}

verify_ed25519_hex_signature() {
    local payload=$1
    local signature_file=$2
    local public_key_hex signature_hex

    signature_verification_error=
    if ! command -v openssl >/dev/null 2>&1; then
        signature_verification_error="openssl is required for provenance verification"
        return 1
    fi
    if ! command -v xxd >/dev/null 2>&1; then
        signature_verification_error="xxd is required for provenance verification"
        return 1
    fi

    public_key_hex=$(tr -d '[:space:]' < "$trusted_public_key")
    signature_hex=$(tr -d '[:space:]' < "$signature_file")
    if [[ ! "$public_key_hex" =~ ^[0-9a-f]{64}$ ]]; then
        signature_verification_error="release provenance public key encoding is invalid"
        return 1
    fi
    if [[ ! "$signature_hex" =~ ^[0-9a-f]{128}$ ]]; then
        signature_verification_error="provenance signature encoding is invalid"
        return 1
    fi

    prepare_verification_dir || return 1
    if ! printf '302a300506032b6570032100%s' "$public_key_hex" \
        | xxd -r -p > "$verification_dir/public.der"; then
        signature_verification_error="release provenance public key decoding failed"
        return 1
    fi
    if ! printf '%s' "$signature_hex" \
        | xxd -r -p > "$verification_dir/signature.bin"; then
        signature_verification_error="provenance signature decoding failed"
        return 1
    fi
    if ! openssl pkey -pubin -inform DER \
        -in "$verification_dir/public.der" \
        -out "$verification_dir/public.pem" >/dev/null 2>&1; then
        signature_verification_error="release provenance public key is invalid"
        return 1
    fi
    if ! openssl pkeyutl -verify -rawin -pubin \
        -inkey "$verification_dir/public.pem" \
        -in "$payload" \
        -sigfile "$verification_dir/signature.bin" >/dev/null 2>&1; then
        signature_verification_error="provenance signature verification failed"
        return 1
    fi
}

archive_symbols() {
    LC_ALL=C strings -a "$1" \
        | LC_ALL=C sed 's/^_//' \
        | LC_ALL=C grep -E '^op_auth_[A-Za-z0-9_]+$' \
        | LC_ALL=C sort -u \
        || true
}

expected_symbols() {
    printf '%s\n' \
        op_auth_abi_version \
        op_auth_cancel \
        op_auth_login_begin \
        op_auth_poll \
        op_auth_restore \
        op_auth_runtime_init \
        op_auth_sign_out \
        op_auth_string_free
    if [[ "$1" -ge 2 ]]; then
        printf '%s\n' \
            op_auth_collab_ticket_begin \
            op_auth_collab_ticket_cancel \
            op_auth_collab_ticket_poll
    fi
    if [[ "$1" -ge 3 ]]; then
        printf '%s\n' \
            op_auth_collab_relay_token_begin
    fi
}

metadata_leak_count() {
    LC_ALL=C strings -a "$1" \
        | LC_ALL=C grep -Ec \
            '/Users/|/home/|/builds/|/workspace/|/private/tmp/|[A-Za-z]:[/\\]Users[/\\]|\.cargo[/\\]registry[/\\]src|/rustc/|op_auth_(core|native)[/\\]src[/\\]' \
        || true
}

debug_marker_count() {
    LC_ALL=C strings -a "$1" \
        | LC_ALL=C grep -Ec \
            '^(\.debug_(abbrev|info|line|line_str|ranges|rnglists|str)|__debug_(abbrev|info|line|str)|\.debug\$[A-Z]|.*\.pdb)$' \
        || true
}

private_symbol_count() {
    LC_ALL=C strings -a "$1" \
        | LC_ALL=C grep -Ec \
            'op_auth_core|op_auth_native|_ZN[0-9A-Za-z_$.]*op_auth|_R[0-9A-Za-z_$.]*op_auth' \
        || true
}

if [[ ! -d "$prebuilt_root" ]]; then
    printf 'check-op-auth-prebuilt.sh: no committed prebuilt directory; stub build only.\n'
    exit 0
fi

cleanup() {
    if [[ -n "$verification_dir" ]]; then
        rm -rf "$verification_dir"
    fi
}
trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

while IFS= read -r artifact; do
    artifact_count=$((artifact_count + 1))
    target_dir=$(dirname "$artifact")
    target=$(basename "$target_dir")
    artifact_name=$(basename "$artifact")
    expected_name=libop_auth.a
    if [[ "$target" == *-pc-windows-msvc ]]; then
        expected_name=op_auth.lib
    fi
    if [[ "$artifact_name" != "$expected_name" ]]; then
        record_failure "$target: artifact must be named $expected_name"
    fi

    checksum_path=$target_dir/SHA256
    expected_sha256=
    actual_sha256=
    if [[ ! -f "$checksum_path" ]]; then
        record_failure "$target: SHA256 is missing"
    else
        expected_sha256=$(tr -d '[:space:]' < "$checksum_path")
        actual_sha256=$(sha256_file "$artifact") \
            || {
                record_failure "$target: no SHA-256 implementation is available"
                actual_sha256=
            }
        if [[ ! "$expected_sha256" =~ ^[0-9a-f]{64}$ ]]; then
            record_failure "$target: SHA256 must be one lowercase hexadecimal digest"
        elif [[ "$expected_sha256" != "$actual_sha256" ]]; then
            record_failure "$target: artifact SHA-256 mismatch"
        fi
    fi

    version_path=$target_dir/VERSION
    artifact_version=
    if [[ ! -s "$version_path" ]]; then
        record_failure "$target: VERSION is missing"
    else
        artifact_version=$(tr -d '[:space:]' < "$version_path")
    fi

    abi_version=1
    if [[ -f "$target_dir/ABI_VERSION" ]]; then
        abi_version=$(tr -d '[:space:]' < "$target_dir/ABI_VERSION")
    fi
    if [[ ! "$abi_version" =~ ^[123]$ ]]; then
        record_failure "$target: ABI_VERSION must be 1, 2, or 3"
        abi_version=1
    fi

    actual_symbols=$(archive_symbols "$artifact")
    required_symbols=$(expected_symbols "$abi_version" | LC_ALL=C sort -u)
    if [[ "$actual_symbols" != "$required_symbols" ]]; then
        missing_symbols=$(comm -23 \
            <(printf '%s\n' "$required_symbols") \
            <(printf '%s\n' "$actual_symbols"))
        extra_symbols=$(comm -13 \
            <(printf '%s\n' "$required_symbols") \
            <(printf '%s\n' "$actual_symbols"))
        [[ -z "$missing_symbols" ]] \
            || record_failure "$target: required C ABI symbols are missing: ${missing_symbols//$'\n'/, }"
        [[ -z "$extra_symbols" ]] \
            || record_failure "$target: undocumented op_auth C ABI symbols are exposed: ${extra_symbols//$'\n'/, }"
    fi

    path_leaks=$(metadata_leak_count "$artifact")
    debug_markers=$(debug_marker_count "$artifact")
    private_symbols=$(private_symbol_count "$artifact")
    hardened=$require_hardened
    # Obfuscated profiles must additionally scrub private Rust symbol strings,
    # source paths, and debug markers. The signed-unobfuscated profile is an
    # explicit, signature-bound opt-out of those scrubs; signature presence and
    # the C ABI surface are still enforced.
    obfuscated=1
    if [[ "$abi_version" -ge 2 ]]; then
        hardened=1
        provenance_path=$target_dir/PROVENANCE
        signature_path=$target_dir/PROVENANCE.sig
        signed_files_ready=1
        for signed_file in "$provenance_path" "$signature_path"; do
            if [[ -L "$signed_file" \
                || ! -f "$signed_file" \
                || ! -s "$signed_file" ]]; then
                signed_name=$(basename "$signed_file")
                record_failure "$target: signed ABI-v2+ $signed_name is missing"
                signed_files_ready=0
            fi
        done
        trusted_key_ready=1
        if [[ -L "$trusted_public_key" \
            || ! -f "$trusted_public_key" \
            || ! -s "$trusted_public_key" ]]; then
            record_failure "$target: release provenance public key is missing"
            trusted_key_ready=0
        fi
        manifest_for_checks=
        signature_for_checks=
        if [[ "$signed_files_ready" -eq 1 ]]; then
            signature_verification_error=
            if prepare_verification_dir; then
                manifest_for_checks=$verification_dir/$artifact_count.PROVENANCE
                signature_for_checks=$verification_dir/$artifact_count.PROVENANCE.sig
                if ! cp "$provenance_path" "$manifest_for_checks" \
                    || ! cp "$signature_path" "$signature_for_checks"; then
                    record_failure "$target: signed provenance could not be snapshotted"
                    manifest_for_checks=
                    signature_for_checks=
                fi
            else
                record_failure "$target: $signature_verification_error"
            fi
        fi
        if [[ -n "$manifest_for_checks" && "$trusted_key_ready" -eq 1 ]] \
            && ! verify_ed25519_hex_signature \
                "$manifest_for_checks" "$signature_for_checks"; then
            record_failure "$target: $signature_verification_error"
        fi
        if [[ -n "$manifest_for_checks" ]]; then
            for signed_field in \
                'format=1' \
                "target=$target" \
                "artifact=$artifact_name" \
                "version=$artifact_version" \
                "abi=$abi_version" \
                "sha256=$actual_sha256"; do
                has_exact_manifest_line "$signed_field" "$manifest_for_checks" \
                    || record_failure \
                        "$target: signed provenance does not match $signed_field"
            done
            hardened_profile_count=$(LC_ALL=C grep -Fxc -- \
                'hardening=op-auth-hardened-v1' "$manifest_for_checks" || true)
            unobfuscated_profile_count=$(LC_ALL=C grep -Fxc -- \
                'hardening=op-auth-signed-unobfuscated-v1' \
                "$manifest_for_checks" || true)
            if [[ "$hardened_profile_count" -eq 1 \
                && "$unobfuscated_profile_count" -eq 0 ]]; then
                obfuscated=1
            elif [[ "$hardened_profile_count" -eq 0 \
                && "$unobfuscated_profile_count" -eq 1 ]]; then
                obfuscated=0
            else
                record_failure "$target: signed hardening profile is missing or unrecognized"
            fi
        fi
    fi

    if [[ "$hardened" -eq 1 && "$obfuscated" -eq 1 ]]; then
        [[ "$path_leaks" -eq 0 ]] \
            || record_failure "$target: archive leaks $path_leaks source/build path strings"
        [[ "$debug_markers" -eq 0 ]] \
            || record_failure "$target: archive contains $debug_markers debug metadata markers"
        [[ "$private_symbols" -eq 0 ]] \
            || record_failure "$target: archive exposes $private_symbols private Rust symbol/module strings"
    elif [[ "$hardened" -eq 1 && "$obfuscated" -eq 0 ]]; then
        printf \
            'warning: %s is signed-unobfuscated (%s path strings, %s debug markers, %s private Rust symbol/module strings retained by design)\n' \
            "$target" "$path_leaks" "$debug_markers" "$private_symbols" >&2
    elif [[ "$path_leaks" -gt 0 || "$debug_markers" -gt 0 || "$private_symbols" -gt 0 ]]; then
        printf \
            'warning: %s is legacy ABI-v1 (%s path strings, %s debug markers, %s private Rust symbol/module strings); rebuild in private CI before production ABI-v2 use\n' \
            "$target" "$path_leaks" "$debug_markers" "$private_symbols" >&2
    fi
done < <(
    find "$prebuilt_root" -mindepth 2 -maxdepth 2 \
        -type f \( -name '*.a' -o -name '*.lib' \) \
        -print | LC_ALL=C sort
)

if [[ "$artifact_count" -eq 0 && "$require_hardened" -eq 1 ]]; then
    record_failure "no op-auth archive was supplied for hardened release validation"
fi

if [[ "${#failures[@]}" -ne 0 ]]; then
    printf 'check-op-auth-prebuilt.sh: %s failure(s):\n' "${#failures[@]}" >&2
    printf '  - %s\n' "${failures[@]}" >&2
    exit 1
fi

printf 'check-op-auth-prebuilt.sh: audited %s archive(s); no bytes were modified.\n' "$artifact_count"
