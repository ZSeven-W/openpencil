#!/usr/bin/env bash
# Download one exact successful private production run's ten unsigned artifacts.

set -euo pipefail

script_dir=$(CDPATH='' cd "$(dirname "$0")" && pwd)
# shellcheck source=op-auth-candidate-targets.sh
source "$script_dir/op-auth-candidate-targets.sh"

private_run_id=
private_head_sha=
output_root=
private_repository=ZSeven-W/op-platform
workflow_path=.github/workflows/prebuilt-production.yml

usage() {
    cat >&2 <<'EOF'
usage: download-op-auth-candidates.sh \
  --private-run-id NUMERIC_ID --private-head-sha FULL_SHA \
  --output-root NEW_DIRECTORY

GH_TOKEN must be a narrowly scoped credential with Actions read access to the
private ZSeven-W/op-platform repository and no public repository write access.
EOF
}

while [[ "$#" -gt 0 ]]; do
    case "$1" in
        --private-run-id|--private-head-sha|--output-root)
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

[[ "$private_run_id" =~ ^[1-9][0-9]*$ ]] || {
    printf 'error: private run id must be a positive integer\n' >&2
    exit 2
}
[[ "$private_head_sha" =~ ^[0-9a-f]{40}$ ]] || {
    printf 'error: private head SHA must be 40 lowercase hexadecimal characters\n' >&2
    exit 2
}
[[ -n "$output_root" && ! -e "$output_root" && ! -L "$output_root" ]] || {
    printf 'error: output root must be a new path\n' >&2
    exit 2
}
[[ -n "${GH_TOKEN:-}" ]] || {
    printf 'error: GH_TOKEN is required for private Actions artifact reads\n' >&2
    exit 2
}
for command in gh jq python3; do
    command -v "$command" >/dev/null 2>&1 || {
        printf 'error: required command is unavailable: %s\n' "$command" >&2
        exit 1
    }
done

temp_dir=$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/op-auth-download.XXXXXX")
success=0
cleanup() {
    rm -rf "$temp_dir"
    if [[ "$success" -ne 1 ]]; then
        rm -rf "$output_root"
    fi
}
trap cleanup EXIT HUP INT TERM

api_version=2022-11-28
run_json=$temp_dir/run.json
workflow_json=$temp_dir/workflow.json
artifacts_json=$temp_dir/artifacts.json

gh api \
    -H 'Accept: application/vnd.github+json' \
    -H "X-GitHub-Api-Version: $api_version" \
    "repos/$private_repository/actions/runs/$private_run_id" > "$run_json"

jq -e \
    --arg repository "$private_repository" \
    --arg workflow_path "$workflow_path" \
    --argjson run_id "$private_run_id" \
    --arg head_sha "$private_head_sha" \
    '.id == $run_id
      and .repository.full_name == $repository
      and .path == $workflow_path
      and .head_branch == "main"
      and .head_sha == $head_sha
      and .event == "workflow_dispatch"
      and .status == "completed"
      and .conclusion == "success"
      and (.run_attempt | type == "number" and . >= 1)
      and (.workflow_id | type == "number" and . >= 1)' \
    "$run_json" >/dev/null || {
    printf 'error: private workflow run identity or successful state mismatch\n' >&2
    exit 1
}
workflow_id=$(jq -r '.workflow_id' "$run_json")
run_attempt=$(jq -r '.run_attempt' "$run_json")

gh api \
    -H 'Accept: application/vnd.github+json' \
    -H "X-GitHub-Api-Version: $api_version" \
    "repos/$private_repository/actions/workflows/$workflow_id" > "$workflow_json"
jq -e \
    --arg workflow_path "$workflow_path" \
    --argjson workflow_id "$workflow_id" \
    '.id == $workflow_id
      and .name == "prebuilt-production"
      and .path == $workflow_path
      and .state == "active"' \
    "$workflow_json" >/dev/null || {
    printf 'error: private workflow definition identity mismatch\n' >&2
    exit 1
}

gh api \
    -H 'Accept: application/vnd.github+json' \
    -H "X-GitHub-Api-Version: $api_version" \
    "repos/$private_repository/actions/runs/$private_run_id/artifacts?per_page=100" \
    > "$artifacts_json"
jq -e '.total_count == 10 and (.artifacts | length) == 10' \
    "$artifacts_json" >/dev/null || {
    printf 'error: private workflow run must contain exactly ten artifacts\n' >&2
    exit 1
}

expected_names=$temp_dir/expected-names
actual_names=$temp_dir/actual-names
while IFS= read -r target; do
    op_auth_candidate_bundle_name "$target"
done < <(op_auth_candidate_targets) | LC_ALL=C sort > "$expected_names"
jq -r '.artifacts[].name' "$artifacts_json" | LC_ALL=C sort > "$actual_names"
cmp -s "$expected_names" "$actual_names" || {
    printf 'error: private workflow artifact names do not match the exact ABI-v3 matrix\n' >&2
    diff -u "$expected_names" "$actual_names" >&2 || true
    exit 1
}

mkdir -m 700 "$output_root"
records=$temp_dir/artifact-records
: > "$records"
while IFS= read -r target; do
    bundle=$(op_auth_candidate_bundle_name "$target")
    jq -e \
        --arg bundle "$bundle" \
        --argjson run_id "$private_run_id" \
        --arg head_sha "$private_head_sha" \
        '[.artifacts[] | select(.name == $bundle)] as $matches
          | ($matches | length) == 1
          and ($matches[0].expired == false)
          and ($matches[0].id | type == "number" and . >= 1)
          and ($matches[0].digest | type == "string"
            and test("^sha256:[0-9a-f]{64}$"))
          and ($matches[0].workflow_run.id == $run_id)
          and ($matches[0].workflow_run.head_branch == "main")
          and ($matches[0].workflow_run.head_sha == $head_sha)' \
        "$artifacts_json" >/dev/null || {
        printf 'error: artifact identity mismatch for %s\n' "$bundle" >&2
        exit 1
    }
    artifact_id=$(jq -r --arg bundle "$bundle" \
        '.artifacts[] | select(.name == $bundle) | .id' "$artifacts_json")
    archive_digest=$(jq -r --arg bundle "$bundle" \
        '.artifacts[] | select(.name == $bundle) | .digest' "$artifacts_json")
    archive_digest=${archive_digest#sha256:}
    archive=$temp_dir/$artifact_id.zip
    gh api \
        -H 'Accept: application/vnd.github+json' \
        -H "X-GitHub-Api-Version: $api_version" \
        "repos/$private_repository/actions/artifacts/$artifact_id/zip" \
        > "$archive"
    actual_digest=$(sha256sum "$archive" | awk '{ print $1 }')
    [[ "$actual_digest" == "$archive_digest" ]] || {
        printf 'error: downloaded GitHub artifact digest mismatch for %s\n' "$bundle" >&2
        exit 1
    }
    python3 "$script_dir/extract-op-auth-candidate.py" \
        --zip "$archive" \
        --target "$target" \
        --output-root "$output_root"
    printf 'artifact_bundle=%s|artifact_id=%s|archive_sha256=%s\n' \
        "$bundle" "$artifact_id" "$archive_digest" >> "$records"
done < <(op_auth_candidate_targets)

{
    printf 'format=op-auth-candidate-download-v1\n'
    printf 'private_repository=%s\n' "$private_repository"
    printf 'workflow_path=%s\n' "$workflow_path"
    printf 'workflow_id=%s\n' "$workflow_id"
    printf 'workflow_run_id=%s\n' "$private_run_id"
    printf 'workflow_run_attempt=%s\n' "$run_attempt"
    printf 'private_head_sha=%s\n' "$private_head_sha"
    printf 'artifact_count=10\n'
    cat "$records"
} > "$output_root/DOWNLOAD-MANIFEST"
chmod 600 "$output_root/DOWNLOAD-MANIFEST"

success=1
printf 'download-op-auth-candidates: authenticated run %s attempt %s (%s)\n' \
    "$private_run_id" "$run_attempt" "$private_head_sha"
