#!/usr/bin/env bash
# Bind a production dispatch to the exact workflow branch, source SHA, and lease.

set -euo pipefail

private_run_id=
private_head_sha=
openpencil_sha=
target_branch=
workflow_sha=
workflow_ref=
remote_url=https://github.com/ZSeven-W/openpencil.git

usage() {
    cat >&2 <<'EOF'
usage: validate-auth-promotion-identity.sh \
  --private-run-id ID --private-head-sha FULL_SHA \
  --openpencil-sha FULL_SHA --target-branch BRANCH \
  --workflow-sha FULL_SHA --workflow-ref FULL_REF \
  [--remote-url URL]
EOF
}

while [[ "$#" -gt 0 ]]; do
    case "$1" in
        --private-run-id|--private-head-sha|--openpencil-sha|--target-branch|\
        --workflow-sha|--workflow-ref|--remote-url)
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
    private_run_id private_head_sha openpencil_sha target_branch workflow_sha \
    workflow_ref remote_url; do
    [[ -n "${!required}" ]] || {
        printf 'error: --%s is required\n' "${required//_/-}" >&2
        exit 2
    }
done

[[ "$private_run_id" =~ ^[1-9][0-9]*$ ]] || {
    printf 'error: private run ID must be a positive integer\n' >&2
    exit 2
}
[[ "$private_head_sha" =~ ^[0-9a-f]{40}$ ]] || {
    printf 'error: private head SHA must be full lowercase hexadecimal\n' >&2
    exit 2
}
[[ "$openpencil_sha" =~ ^[0-9a-f]{40}$ \
    && "$workflow_sha" =~ ^[0-9a-f]{40}$ ]] || {
    printf 'error: public and workflow SHAs must be full lowercase hexadecimal\n' >&2
    exit 2
}
if [[ "$target_branch" == main ]]; then
    :
elif [[ "$target_branch" \
    =~ ^v[0-9]+\.[0-9]+\.[0-9]+([-+][0-9A-Za-z.-]+)?$ ]]; then
    :
else
    printf 'error: target branch must be main or an exact version branch\n' >&2
    exit 2
fi

expected_ref=refs/heads/$target_branch
[[ "$workflow_ref" == "$expected_ref" ]] || {
    printf 'error: workflow dispatch branch is not the target branch\n' >&2
    exit 1
}
[[ "$workflow_sha" == "$openpencil_sha" ]] || {
    printf 'error: workflow implementation is not exact public source S\n' >&2
    exit 1
}

matches=$(git ls-remote --exit-code --refs "$remote_url" "$expected_ref")
match_count=$(printf '%s\n' "$matches" | awk 'NF { count += 1 } END { print count + 0 }')
[[ "$match_count" -eq 1 ]] || {
    printf 'error: target branch did not resolve exactly once\n' >&2
    exit 1
}
read -r remote_sha returned_ref extra <<< "$matches"
[[ -z "${extra:-}" && "$returned_ref" == "$expected_ref" \
    && "$remote_sha" == "$openpencil_sha" ]] || {
    printf 'error: target branch is not at exact public source S\n' >&2
    exit 1
}

printf 'validate-auth-promotion-identity: exact workflow branch and S lease verified.\n'
