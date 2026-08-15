#!/usr/bin/env bash
# Resolve a release branch or tag on the canonical repository and prove that it
# selects the exact source commit being built. Annotated tags are accepted only
# when their peeled commit is that source; lightweight tags and branches must
# point to it directly.

set -euo pipefail

release_sha=${OPENPENCIL_RELEASE_SHA:-}
release_ref=${OPENPENCIL_RELEASE_REF:-}
canonical_remote=${OPENPENCIL_CANONICAL_REMOTE:-}

validate_inputs() {
    [[ "$release_sha" =~ ^[0-9a-f]{40}$ ]] || {
        printf 'error: release source input must be a full lowercase commit SHA\n' >&2
        return 2
    }
    [[ "$release_ref" \
        =~ ^refs/(heads|tags)/v[0-9]+\.[0-9]+\.[0-9]+([-+][0-9A-Za-z.-]+)?$ ]] || {
        printf 'error: release ref must be an exact version branch or tag\n' >&2
        return 2
    }
    [[ "$canonical_remote" == https://github.com/ZSeven-W/openpencil.git ]] || {
        printf 'error: release ref must use the canonical OpenPencil repository\n' >&2
        return 2
    }
}

validate_remote_lines() {
    local remote_lines=$1
    local direct_sha= peeled_sha= remote_sha remote_name remainder
    local line_count=0

    while IFS=$'\t' read -r remote_sha remote_name remainder; do
        [[ -n "$remote_sha" && "$remote_sha" =~ ^[0-9a-f]{40}$ \
            && -n "$remote_name" && -z "${remainder:-}" ]] || {
            printf 'error: canonical release ref returned malformed data\n' >&2
            return 1
        }
        line_count=$((line_count + 1))
        case "$remote_name" in
            "$release_ref")
                [[ -z "$direct_sha" ]] || {
                    printf 'error: canonical release ref returned a duplicate direct ref\n' >&2
                    return 1
                }
                direct_sha=$remote_sha
                ;;
            "$release_ref^{}")
                [[ -z "$peeled_sha" ]] || {
                    printf 'error: canonical release ref returned a duplicate peeled ref\n' >&2
                    return 1
                }
                peeled_sha=$remote_sha
                ;;
            *)
                printf 'error: canonical release lookup returned an unexpected ref\n' >&2
                return 1
                ;;
        esac
    done <<< "$remote_lines"

    [[ -n "$direct_sha" ]] || {
        printf 'error: canonical release ref has no direct target\n' >&2
        return 1
    }
    case "$release_ref" in
        refs/heads/*)
            [[ "$line_count" -eq 1 && -z "$peeled_sha" \
                && "$direct_sha" == "$release_sha" ]] || {
                printf 'error: canonical release branch does not point directly at the source commit\n' >&2
                return 1
            }
            ;;
        refs/tags/*)
            if [[ -n "$peeled_sha" ]]; then
                [[ "$line_count" -eq 2 && "$peeled_sha" == "$release_sha" ]] || {
                    printf 'error: canonical annotated release tag does not peel to the source commit\n' >&2
                    return 1
                }
            else
                [[ "$line_count" -eq 1 && "$direct_sha" == "$release_sha" ]] || {
                    printf 'error: canonical lightweight release tag does not point directly at the source commit\n' >&2
                    return 1
                }
            fi
            ;;
    esac
}

self_test() {
    local a=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
    local tag_object=bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
    local wrong=cccccccccccccccccccccccccccccccccccccccc
    local tab=$'\t'
    local fixture

    release_sha=$a
    canonical_remote=https://github.com/ZSeven-W/openpencil.git

    release_ref=refs/heads/v1.2.3
    validate_inputs
    validate_remote_lines "$a$tab$release_ref"

    release_ref=refs/tags/v1.2.3
    validate_inputs
    validate_remote_lines "$a$tab$release_ref"
    fixture="$tag_object$tab$release_ref"$'\n'"$a$tab$release_ref^{}"
    validate_remote_lines "$fixture"

    for fixture in \
        "$wrong$tab$release_ref" \
        "$tag_object$tab$release_ref"$'\n'"$wrong$tab$release_ref^{}" \
        "$a$tab$release_ref^{}" \
        "$a$tab$release_ref"$'\n'"$a$tab$release_ref" \
        "$a$tab$release_ref"$'\n'"$a${tab}refs/tags/v9.9.9"; do
        if validate_remote_lines "$fixture" >/dev/null 2>&1; then
            printf 'error: invalid remote release fixture was accepted\n' >&2
            return 1
        fi
    done

    release_ref=refs/heads/v1.2.3
    fixture="$a$tab$release_ref"$'\n'"$a$tab$release_ref^{}"
    if validate_remote_lines "$fixture" >/dev/null 2>&1; then
        printf 'error: peeled branch fixture was accepted\n' >&2
        return 1
    fi

    printf 'check-op-auth-remote-ref.sh: annotated and direct ref fixtures passed.\n'
}

case ${1-} in
    --self-test)
        [[ $# -eq 1 ]] || {
            printf 'usage: %s [--self-test]\n' "$0" >&2
            exit 2
        }
        self_test
        exit
        ;;
    '')
        [[ $# -eq 0 ]] || exit 2
        ;;
    *)
        printf 'usage: %s [--self-test]\n' "$0" >&2
        exit 2
        ;;
esac

validate_inputs
if ! remote_lines=$(git ls-remote --exit-code "$canonical_remote" \
    "$release_ref" "$release_ref^{}"); then
    printf 'error: release ref does not exist on the canonical repository\n' >&2
    exit 1
fi
validate_remote_lines "$remote_lines"
printf 'check-op-auth-remote-ref.sh: verified %s at %s.\n' \
    "$release_ref" "$release_sha"
