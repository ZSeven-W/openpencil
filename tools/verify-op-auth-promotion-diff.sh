#!/usr/bin/env bash
# Require the staged child A to be an exact, complete auth-only replacement.

set -euo pipefail

script_dir=$(CDPATH='' cd "$(dirname "$0")" && pwd)
# shellcheck source=op-auth-candidate-targets.sh
source "$script_dir/op-auth-candidate-targets.sh"

repo=
expected_parent=
prefix=crates/op-auth-bridge/prebuilt

usage() {
    printf '%s\n' \
        'usage: verify-op-auth-promotion-diff.sh --repo OPENPENCIL_CHECKOUT --expected-parent FULL_SHA' \
        >&2
}

while [[ "$#" -gt 0 ]]; do
    case "$1" in
        --repo|--expected-parent)
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
[[ -n "$repo" && "$expected_parent" =~ ^[0-9a-f]{40}$ ]] || {
    usage
    exit 2
}
repo=$(CDPATH='' cd "$repo" && pwd -P)
[[ "$(git -C "$repo" rev-parse --show-toplevel)" == "$repo" \
    && "$(git -C "$repo" rev-parse HEAD)" == "$expected_parent" ]] || {
    printf 'error: promotion checkout is not the immutable public source parent\n' >&2
    exit 1
}
git -C "$repo" diff --quiet || {
    printf 'error: promotion checkout contains unstaged tracked changes\n' >&2
    exit 1
}
[[ -z "$(git -C "$repo" ls-files --others --exclude-standard)" ]] || {
    printf 'error: promotion checkout contains unstaged untracked files\n' >&2
    exit 1
}

changed=$(git -C "$repo" diff --cached --name-only --no-renames)
[[ -n "$changed" ]] || {
    printf 'error: complete Auth Release rebuild produced no staged changes\n' >&2
    exit 1
}
for root_file in RELEASE-MANIFEST RELEASE-MANIFEST.sig; do
    grep -Fxq "$prefix/$root_file" <<< "$changed" || {
        printf 'error: auth-only child is missing staged %s\n' "$root_file" >&2
        exit 1
    }
    [[ -f "$repo/$prefix/$root_file" && ! -L "$repo/$prefix/$root_file" ]] || {
        printf 'error: signed matrix root file is missing or symlinked: %s\n' \
            "$root_file" >&2
        exit 1
    }
done
git -C "$repo" diff --cached --exit-code -- "$prefix/PROVENANCE_PUBKEY"

while IFS=$'\t' read -r status path extra; do
    [[ "$status" =~ ^[AM]$ && -z "${extra:-}" ]] || {
        printf 'error: auth-only child contains a deletion, rename, or malformed path\n' >&2
        exit 1
    }
    case "$path" in
        "$prefix/RELEASE-MANIFEST"|"$prefix/RELEASE-MANIFEST.sig")
            ;;
        "$prefix/"*)
            relative=${path#"$prefix/"}
            target=${relative%%/*}
            file=${relative#*/}
            [[ "$file" != "$relative" ]] \
                && op_auth_candidate_artifact_name "$target" >/dev/null || {
                printf 'error: auth-only child contains unsupported target path: %s\n' \
                    "$path" >&2
                exit 1
            }
            artifact=$(op_auth_candidate_artifact_name "$target")
            case "$file" in
                "$artifact"|ABI_VERSION|HARDENING-ATTESTATION|PROVENANCE|\
                PROVENANCE.sig|SHA256|VERSION)
                    ;;
                *)
                    printf 'error: auth-only child contains unexpected matrix file: %s\n' \
                        "$path" >&2
                    exit 1
                    ;;
            esac
            ;;
        *)
            printf 'error: auth-only child changed a path outside the prebuilt matrix: %s\n' \
                "$path" >&2
            exit 1
            ;;
    esac
done < <(git -C "$repo" diff --cached --name-status --no-renames)

while IFS= read -r target; do
    target_root=$repo/$prefix/$target
    [[ -d "$target_root" && ! -L "$target_root" ]] || {
        printf 'error: signed target directory is missing or symlinked: %s\n' "$target" >&2
        exit 1
    }
    artifact=$(op_auth_candidate_artifact_name "$target")
    expected_entries=$(
        printf '%s\n' \
            "$artifact" ABI_VERSION HARDENING-ATTESTATION PROVENANCE \
            PROVENANCE.sig SHA256 VERSION | LC_ALL=C sort
    )
    actual_entries=$(
        find "$target_root" -mindepth 1 -maxdepth 1 -print \
            | sed 's#.*/##' | LC_ALL=C sort
    )
    [[ "$actual_entries" == "$expected_entries" ]] || {
        printf 'error: target directory is not the exact release schema: %s\n' \
            "$target" >&2
        exit 1
    }
    while IFS= read -r file; do
        [[ -f "$target_root/$file" && ! -L "$target_root/$file" ]] || {
            printf 'error: target release entry is missing or symlinked: %s/%s\n' \
                "$target" "$file" >&2
            exit 1
        }
    done <<< "$expected_entries"
    git -C "$repo" diff --cached --name-only -- "$prefix/$target/" \
        | grep -q . || {
        printf 'error: auth-only child did not replace rebuilt target %s\n' "$target" >&2
        exit 1
    }
done < <(op_auth_candidate_targets)

printf 'verify-op-auth-promotion-diff: exact auth-only child staged on %s\n' \
    "$expected_parent"
