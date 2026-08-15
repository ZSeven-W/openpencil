#!/usr/bin/env bash
# Validate the immutable public transition used by every production release:
# source commit S -> one auth-matrix-only child A. The signed matrix binds S,
# while the release/tag/workflow must run from exact A.

set -euo pipefail

script_dir=$(CDPATH='' cd "$(dirname "$0")" && pwd)
repo_root=$(CDPATH='' cd "$script_dir/.." && pwd)
artifact_commit=${OP_AUTH_ARTIFACT_COMMIT:-}
selected_commit=${OP_AUTH_ARTIFACT_SELECTED_COMMIT:-}
source_ref=${OP_AUTH_ARTIFACT_REF:-}
output_file=${OP_AUTH_ARTIFACT_OUTPUT:-}

if [[ "$#" -ne 0 ]]; then
    printf 'usage: OP_AUTH_ARTIFACT_COMMIT=<sha> %s\n' "$0" >&2
    exit 2
fi
for value in "$artifact_commit" "$selected_commit"; do
    [[ "$value" =~ ^[0-9a-f]{40}$ ]] || {
        printf 'error: auth artifact selection must use full lowercase commit SHAs\n' >&2
        exit 2
    }
done
[[ "$selected_commit" == "$artifact_commit" ]] || {
    printf 'error: run the release from the exact selected auth artifact commit\n' >&2
    exit 2
}
[[ "$(git -C "$repo_root" rev-parse HEAD)" == "$artifact_commit" ]] || {
    printf 'error: checked-out source is not the selected auth artifact commit\n' >&2
    exit 1
}

ref_version=
if [[ "$source_ref" == refs/heads/main ]]; then
    :
elif [[ "$source_ref" \
    =~ ^refs/heads/v([0-9]+\.[0-9]+\.[0-9]+([-+][0-9A-Za-z.-]+)?)$ ]]; then
    ref_version=${BASH_REMATCH[1]}
elif [[ "$source_ref" \
    =~ ^refs/tags/v([0-9]+\.[0-9]+\.[0-9]+([-+][0-9A-Za-z.-]+)?)$ ]]; then
    ref_version=${BASH_REMATCH[1]}
else
    printf '%s\n' \
        'error: production releases require main, an exact version branch, or an exact version tag' \
        >&2
    exit 2
fi

read -r commit parent extra < <(
    git -C "$repo_root" rev-list --parents -n 1 "$artifact_commit"
)
[[ "$commit" == "$artifact_commit" && "$parent" =~ ^[0-9a-f]{40}$ \
    && -z "${extra:-}" ]] || {
    printf 'error: the auth artifact commit must have exactly one parent\n' >&2
    exit 1
}

temp_root=${RUNNER_TEMP:-${TMPDIR:-/tmp}}
allowed_changes=$(mktemp "$temp_root/op-auth-allowed.XXXXXX")
actual_changes=$(mktemp "$temp_root/op-auth-actual.XXXXXX")
cleanup() {
    rm -f "$allowed_changes" "$actual_changes"
}
trap cleanup EXIT

{
    printf '%s\n' \
        crates/op-auth-bridge/prebuilt/RELEASE-MANIFEST \
        crates/op-auth-bridge/prebuilt/RELEASE-MANIFEST.sig
    for target in \
        aarch64-apple-darwin \
        aarch64-apple-ios \
        aarch64-apple-ios-sim \
        aarch64-linux-android \
        aarch64-pc-windows-msvc \
        aarch64-unknown-linux-gnu \
        x86_64-apple-darwin \
        x86_64-linux-android \
        x86_64-pc-windows-msvc \
        x86_64-unknown-linux-gnu; do
        artifact=libop_auth.a
        [[ "$target" == *-pc-windows-msvc ]] && artifact=op_auth.lib
        for file in \
            ABI_VERSION HARDENING-ATTESTATION PROVENANCE PROVENANCE.sig \
            SHA256 VERSION "$artifact"; do
            printf 'crates/op-auth-bridge/prebuilt/%s/%s\n' "$target" "$file"
        done
    done
} | LC_ALL=C sort > "$allowed_changes"
git -C "$repo_root" diff-tree --no-commit-id --name-only --no-renames -r \
    "$parent" "$artifact_commit" | LC_ALL=C sort > "$actual_changes"
unexpected=$(LC_ALL=C comm -23 "$actual_changes" "$allowed_changes")
[[ -z "$unexpected" ]] || {
    printf 'error: artifact commit changed a path outside the auth matrix allowlist\n' >&2
    printf '%s\n' "$unexpected" >&2
    exit 1
}
for required_change in \
    crates/op-auth-bridge/prebuilt/RELEASE-MANIFEST \
    crates/op-auth-bridge/prebuilt/RELEASE-MANIFEST.sig \
    crates/op-auth-bridge/prebuilt/aarch64-apple-ios/libop_auth.a; do
    grep -Fxq "$required_change" "$actual_changes" || {
        printf 'error: artifact commit did not update %s\n' "$required_change" >&2
        exit 1
    }
done

version=$("$repo_root/scripts/workspace-version.sh")
if [[ -n "$ref_version" && "$ref_version" != "$version" ]]; then
    printf 'error: branch or tag version does not match the Cargo workspace\n' >&2
    exit 1
fi
OP_AUTH_PREBUILT_ROOT=$repo_root/crates/op-auth-bridge/prebuilt \
OP_AUTH_RELEASE_EXPECTED_OPENPENCIL_REVISION=$parent \
OP_AUTH_RELEASE_WORKSPACE_VERSION=$version \
    "$repo_root/tools/check-op-auth-release-matrix.sh"
OP_AUTH_PREBUILT_ROOT=$repo_root/crates/op-auth-bridge/prebuilt \
    "$repo_root/tools/check-op-auth-prebuilt.sh" --require-hardened

if [[ -n "$output_file" ]]; then
    case "$output_file" in
        /*) ;;
        *)
            printf 'error: OP_AUTH_ARTIFACT_OUTPUT must be an absolute path\n' >&2
            exit 2
            ;;
    esac
    [[ -f "$output_file" && ! -L "$output_file" ]] || {
        printf 'error: OP_AUTH_ARTIFACT_OUTPUT must be a regular existing file\n' >&2
        exit 2
    }
    {
        printf 'artifact_sha=%s\n' "$artifact_commit"
        printf 'base_sha=%s\n' "$parent"
        printf 'version=%s\n' "$version"
    } >> "$output_file"
fi

printf 'check-op-auth-artifact-commit.sh: verified %s as auth-only child of %s.\n' \
    "$artifact_commit" "$parent"
