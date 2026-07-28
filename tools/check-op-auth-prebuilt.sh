#!/usr/bin/env bash
# Audit committed op-auth archives without modifying their bytes.
#
# Legacy ABI-v1 artifacts are integrity-pinned compatibility inputs and may
# still contain source/debug metadata. ABI-v2 artifacts are production
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
failures=()
artifact_count=0

record_failure() {
    failures+=("$1")
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
    if [[ ! -s "$version_path" ]]; then
        record_failure "$target: VERSION is missing"
    fi

    abi_version=1
    if [[ -f "$target_dir/ABI_VERSION" ]]; then
        abi_version=$(tr -d '[:space:]' < "$target_dir/ABI_VERSION")
    fi
    if [[ ! "$abi_version" =~ ^[12]$ ]]; then
        record_failure "$target: ABI_VERSION must be 1 or 2"
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
    if [[ "$abi_version" -ge 2 ]]; then
        hardened=1
        for signed_file in PROVENANCE PROVENANCE.sig; do
            if [[ ! -s "$target_dir/$signed_file" ]]; then
                record_failure "$target: signed ABI-v2 $signed_file is missing"
            fi
        done
        if [[ ! -s "$prebuilt_root/PROVENANCE_PUBKEY" ]]; then
            record_failure "$target: release provenance public key is missing"
        fi
        if [[ -f "$target_dir/PROVENANCE" ]] \
            && ! grep -Fxq 'hardening=op-auth-hardened-v1' "$target_dir/PROVENANCE"; then
            record_failure "$target: signed hardening profile is missing"
        fi
    fi

    if [[ "$hardened" -eq 1 ]]; then
        [[ "$path_leaks" -eq 0 ]] \
            || record_failure "$target: archive leaks $path_leaks source/build path strings"
        [[ "$debug_markers" -eq 0 ]] \
            || record_failure "$target: archive contains $debug_markers debug metadata markers"
        [[ "$private_symbols" -eq 0 ]] \
            || record_failure "$target: archive exposes $private_symbols private Rust symbol/module strings"
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
