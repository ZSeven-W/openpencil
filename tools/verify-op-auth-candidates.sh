#!/usr/bin/env bash
# Fail closed on the exact unsigned ten-target production candidate schema.

set -euo pipefail

script_dir=$(CDPATH='' cd "$(dirname "$0")" && pwd)
# shellcheck source=op-auth-candidate-targets.sh
source "$script_dir/op-auth-candidate-targets.sh"

candidate_root=
private_run_id=
private_head_sha=
openpencil_sha=
version=
private_repository=ZSeven-W/op-platform
workflow_path=.github/workflows/prebuilt-production.yml

usage() {
    cat >&2 <<'EOF'
usage: verify-op-auth-candidates.sh \
  --candidate-root DIR --private-run-id NUMERIC_ID \
  --private-head-sha FULL_SHA --openpencil-sha FULL_SHA --version VERSION
EOF
}

while [[ "$#" -gt 0 ]]; do
    case "$1" in
        --candidate-root|--private-run-id|--private-head-sha|--openpencil-sha|--version)
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

[[ -d "$candidate_root" && ! -L "$candidate_root" ]] || {
    printf 'error: candidate root must be a non-symlink directory\n' >&2
    exit 2
}
[[ "$private_run_id" =~ ^[1-9][0-9]*$ ]] || {
    printf 'error: private run id must be a positive integer\n' >&2
    exit 2
}
for revision_name in private_head_sha openpencil_sha; do
    [[ "${!revision_name}" =~ ^[0-9a-f]{40}$ ]] || {
        printf 'error: %s must be 40 lowercase hexadecimal characters\n' \
            "$revision_name" >&2
        exit 2
    }
done
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+([+-][0-9A-Za-z.-]+)?$ ]] || {
    printf 'error: invalid OpenPencil version\n' >&2
    exit 2
}

temp_dir=$(mktemp -d "${TMPDIR:-/tmp}/op-auth-candidate-verify.XXXXXX")
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

sha256_stdin() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum | awk '{ print $1 }'
    else
        shasum -a 256 | awk '{ print $1 }'
    fi
}

require_regular_file() {
    [[ -f "$1" && ! -L "$1" ]] || {
        printf 'error: required regular non-symlink file is missing: %s\n' "$1" >&2
        return 1
    }
}

single_value() {
    local key=$1
    local file=$2
    local values count
    values=$(sed -n "s/^${key}=//p" "$file")
    count=$(printf '%s\n' "$values" | sed '/^$/d' | wc -l | tr -d '[:space:]')
    [[ "$count" -eq 1 ]] || {
        printf 'error: field %s is missing or duplicated in %s\n' "$key" "$file" >&2
        return 1
    }
    printf '%s\n' "$values"
}

require_value() {
    local key=$1
    local expected=$2
    local file=$3
    [[ "$(single_value "$key" "$file")" == "$expected" ]] || {
        printf 'error: field %s mismatch in %s\n' "$key" "$file" >&2
        return 1
    }
}

download_manifest=$candidate_root/DOWNLOAD-MANIFEST
require_regular_file "$download_manifest"
[[ "$(wc -l < "$download_manifest" | tr -d '[:space:]')" -eq 18 ]] \
    && ! LC_ALL=C grep -q $'\r' "$download_manifest" || {
    printf 'error: candidate download manifest must be exactly 18 LF lines\n' >&2
    exit 1
}
require_value format op-auth-candidate-download-v1 "$download_manifest"
require_value private_repository "$private_repository" "$download_manifest"
require_value workflow_path "$workflow_path" "$download_manifest"
require_value workflow_run_id "$private_run_id" "$download_manifest"
require_value private_head_sha "$private_head_sha" "$download_manifest"
require_value artifact_count 10 "$download_manifest"
workflow_id=$(single_value workflow_id "$download_manifest")
run_attempt=$(single_value workflow_run_attempt "$download_manifest")
[[ "$workflow_id" =~ ^[1-9][0-9]*$ && "$run_attempt" =~ ^[1-9][0-9]*$ ]] || {
    printf 'error: invalid workflow identity in candidate download manifest\n' >&2
    exit 1
}

expected_root_entries=$temp_dir/expected-root-entries
actual_root_entries=$temp_dir/actual-root-entries
{
    printf '%s\n' DOWNLOAD-MANIFEST
    op_auth_candidate_targets
} | LC_ALL=C sort > "$expected_root_entries"
find "$candidate_root" -mindepth 1 -maxdepth 1 -print \
    | sed 's#^.*/##' | LC_ALL=C sort > "$actual_root_entries"
cmp -s "$expected_root_entries" "$actual_root_entries" || {
    printf 'error: candidate root contains missing or unexpected entries\n' >&2
    diff -u "$expected_root_entries" "$actual_root_entries" >&2 || true
    exit 1
}

attestation_keys=$temp_dir/expected-attestation-keys
cat > "$attestation_keys" <<'EOF'
format
mode
abi
target
artifact
artifact_sha256
artifact_size
source_revision
source_tree_state
source_date_epoch
profile
paths
debug
symbols
dead_code
audit_profile
abi_allowlist_sha256
global_defined_symbols_sha256
section_inventory_sha256
object_format_count
debug_section_count
link_validation
link_execution
target_platform
minimum_platform_version
linked_binary_sha256
obfuscation_review
protection_tool_sha256
review_binding_sha256
linux_glibc_baseline
linux_sysroot
zig_version
zig_sha256
zig_sysroot_sha256
cargo_zigbuild_version
cargo_zigbuild_sha256
toolchain
staging_format
release_build_id
openpencil_revision
EOF

common_build_id=
download_line=9
while IFS= read -r target; do
    target_root=$candidate_root/$target
    [[ -d "$target_root" && ! -L "$target_root" ]] || {
        printf 'error: missing candidate target directory: %s\n' "$target" >&2
        exit 1
    }
    artifact=$(op_auth_candidate_artifact_name "$target")
    bundle=$(op_auth_candidate_bundle_name "$target")
    expected_files=$temp_dir/expected-files
    actual_files=$temp_dir/actual-files
    printf '%s\n' \
        "$artifact" ABI_VERSION CANDIDATE HARDENING-ATTESTATION SHA256 VERSION \
        | LC_ALL=C sort > "$expected_files"
    find "$target_root" -mindepth 1 -maxdepth 1 -print \
        | sed 's#^.*/##' | LC_ALL=C sort > "$actual_files"
    cmp -s "$expected_files" "$actual_files" || {
        printf 'error: candidate target %s has missing or unexpected files\n' "$target" >&2
        exit 1
    }
    for name in "$artifact" ABI_VERSION CANDIDATE HARDENING-ATTESTATION SHA256 VERSION; do
        require_regular_file "$target_root/$name"
    done

    candidate=$target_root/CANDIDATE
    attestation=$target_root/HARDENING-ATTESTATION
    [[ "$(wc -l < "$candidate" | tr -d '[:space:]')" -eq 15 ]] \
        && ! LC_ALL=C grep -q $'\r' "$candidate" || {
        printf 'error: %s candidate metadata must be exactly 15 LF lines\n' "$target" >&2
        exit 1
    }
    build_id=$(single_value build_id "$candidate")
    artifact_sha=$(sha256_file "$target_root/$artifact")
    hardening_sha=$(sha256_file "$attestation")
    [[ "$build_id" =~ ^[A-Za-z0-9]([A-Za-z0-9._-]{0,57}[A-Za-z0-9])?$ \
        && "$build_id" != *..* && "$build_id" != *.lock ]] || {
        printf 'error: invalid immutable candidate build id for %s\n' "$target" >&2
        exit 1
    }
    if [[ -z "$common_build_id" ]]; then
        common_build_id=$build_id
    fi
    [[ "$build_id" == "$common_build_id" ]] || {
        printf 'error: candidate matrix contains multiple build ids\n' >&2
        exit 1
    }

    expected_candidate=$temp_dir/expected-candidate
    cat > "$expected_candidate" <<EOF
format=op-auth-unsigned-candidate-v1
private_repository=$private_repository
workflow_path=$workflow_path
workflow_run_id=$private_run_id
workflow_run_attempt=$run_attempt
artifact_bundle=$bundle
target=$target
version=$version
abi=3
source_revision=$private_head_sha
openpencil_revision=$openpencil_sha
build_id=$build_id
artifact=$artifact
artifact_sha256=$artifact_sha
hardening_sha256=$hardening_sha
EOF
    cmp -s "$expected_candidate" "$candidate" || {
        printf 'error: non-canonical or mismatched candidate metadata for %s\n' "$target" >&2
        exit 1
    }
    printf '%s\n' "$version" > "$temp_dir/expected-version"
    printf '3\n' > "$temp_dir/expected-abi"
    printf '%s\n' "$artifact_sha" > "$temp_dir/expected-sha"
    cmp -s "$temp_dir/expected-version" "$target_root/VERSION" \
        && cmp -s "$temp_dir/expected-abi" "$target_root/ABI_VERSION" \
        && cmp -s "$temp_dir/expected-sha" "$target_root/SHA256" || {
        printf 'error: candidate digest/version/ABI mismatch for %s\n' "$target" >&2
        exit 1
    }

    actual_attestation_keys=$temp_dir/actual-attestation-keys
    LC_ALL=C sed -n 's/^\([A-Za-z0-9_]*\)=.*$/\1/p' "$attestation" \
        > "$actual_attestation_keys"
    [[ "$(wc -l < "$attestation" | tr -d '[:space:]')" \
        -eq "$(wc -l < "$actual_attestation_keys" | tr -d '[:space:]')" ]] \
        && ! LC_ALL=C grep -q $'\r' "$attestation" \
        && cmp -s "$attestation_keys" "$actual_attestation_keys" || {
        printf 'error: hardening attestation schema/order mismatch for %s\n' "$target" >&2
        exit 1
    }
    require_value format 3 "$attestation"
    require_value mode production "$attestation"
    require_value abi 3 "$attestation"
    require_value target "$target" "$attestation"
    require_value artifact "$artifact" "$attestation"
    require_value artifact_sha256 "$artifact_sha" "$attestation"
    require_value source_revision "$private_head_sha" "$attestation"
    require_value source_tree_state clean "$attestation"
    require_value profile private-ci-fat-lto "$attestation"
    require_value paths remapped "$attestation"
    require_value debug none "$attestation"
    require_value symbols stripped "$attestation"
    require_value dead_code eliminated "$attestation"
    require_value audit_profile op-auth-hardened-v3 "$attestation"
    require_value staging_format 1 "$attestation"
    require_value release_build_id "$build_id" "$attestation"
    require_value openpencil_revision "$openpencil_sha" "$attestation"
    attested_size=$(single_value artifact_size "$attestation")
    [[ "$attested_size" =~ ^[1-9][0-9]*$ \
        && "$attested_size" -eq "$(wc -c < "$target_root/$artifact" | tr -d '[:space:]')" ]] || {
        printf 'error: artifact size mismatch for %s\n' "$target" >&2
        exit 1
    }
    for digest_key in \
        abi_allowlist_sha256 global_defined_symbols_sha256 linked_binary_sha256 \
        protection_tool_sha256 review_binding_sha256 section_inventory_sha256; do
        [[ "$(single_value "$digest_key" "$attestation")" =~ ^[0-9a-f]{64}$ ]] || {
            printf 'error: invalid %s for %s\n' "$digest_key" "$target" >&2
            exit 1
        }
    done
    review_id=source-$private_head_sha
    require_value obfuscation_review "$review_id" "$attestation"
    protector_sha=$(single_value protection_tool_sha256 "$attestation")
    expected_review_binding=$(
        printf '%s\n' \
            op-auth-protector-binding-v1 \
            "target=$target" \
            "review_id=$review_id" \
            "protector_sha256=$protector_sha" \
            | sha256_stdin
    )
    require_value review_binding_sha256 "$expected_review_binding" "$attestation"
    [[ "$(single_value source_date_epoch "$attestation")" =~ ^[1-9][0-9]*$ \
        && "$(single_value debug_section_count "$attestation")" =~ ^[0-9]+$ \
        && "$(single_value object_format_count "$attestation")" =~ ^[1-9][0-9]*$ \
        && "$(single_value obfuscation_review "$attestation")" \
            =~ ^[A-Za-z0-9._-]{1,128}$ ]] || {
        printf 'error: malformed hardening audit metadata for %s\n' "$target" >&2
        exit 1
    }

    platform=macos
    link_validation=native-link-run
    link_execution=passed
    minimum_version=native
    case "$target" in
        aarch64-apple-ios)
            platform=ios
            link_validation=cross-final-link
            link_execution=not-applicable
            minimum_version=15.0
            ;;
        aarch64-apple-ios-sim)
            platform=ios-simulator
            link_validation=cross-final-link
            link_execution=not-applicable
            minimum_version=15.0
            ;;
        *-linux-android)
            platform=android
            link_validation=cross-final-link
            link_execution=not-applicable
            minimum_version=21
            ;;
        *-unknown-linux-gnu) platform=linux ;;
        *-pc-windows-msvc) platform=windows ;;
    esac
    require_value target_platform "$platform" "$attestation"
    require_value link_validation "$link_validation" "$attestation"
    require_value link_execution "$link_execution" "$attestation"
    require_value minimum_platform_version "$minimum_version" "$attestation"
    if [[ "$target" != *-pc-windows-msvc ]]; then
        require_value debug_section_count 0 "$attestation"
    fi
    baseline=$(single_value linux_glibc_baseline "$attestation")
    if [[ "$target" == *-unknown-linux-gnu && "$baseline" == 2.17 ]]; then
        require_value linux_sysroot zig-glibc-2.17 "$attestation"
        for digest_key in zig_sha256 zig_sysroot_sha256 cargo_zigbuild_sha256; do
            [[ "$(single_value "$digest_key" "$attestation")" \
                =~ ^[0-9a-f]{64}$ ]] || {
                printf 'error: invalid reviewed Linux toolchain digest for %s\n' \
                    "$target" >&2
                exit 1
            }
        done
        for version_key in zig_version cargo_zigbuild_version; do
            value=$(single_value "$version_key" "$attestation")
            [[ "$value" != none && "$value" =~ ^[0-9A-Za-z.+-]{1,64}$ ]] || {
                printf 'error: invalid reviewed Linux toolchain version for %s\n' \
                    "$target" >&2
                exit 1
            }
        done
    else
        require_value linux_glibc_baseline none "$attestation"
        require_value linux_sysroot none "$attestation"
        require_value zig_version none "$attestation"
        require_value zig_sha256 none "$attestation"
        require_value zig_sysroot_sha256 none "$attestation"
        require_value cargo_zigbuild_version none "$attestation"
        require_value cargo_zigbuild_sha256 none "$attestation"
    fi
    [[ -n "$(single_value toolchain "$attestation")" ]] || {
        printf 'error: empty toolchain binding for %s\n' "$target" >&2
        exit 1
    }

    download_record=$(sed -n "${download_line}p" "$download_manifest")
    [[ "$download_record" =~ ^artifact_bundle=${bundle}\|artifact_id=[1-9][0-9]*\|archive_sha256=[0-9a-f]{64}$ ]] || {
        printf 'error: non-canonical download record for %s\n' "$target" >&2
        exit 1
    }
    download_line=$((download_line + 1))
done < <(op_auth_candidate_targets)

printf 'verify-op-auth-candidates: verified unsigned ABI-v3 matrix (%s, run %s attempt %s)\n' \
    "$common_build_id" "$private_run_id" "$run_attempt"
