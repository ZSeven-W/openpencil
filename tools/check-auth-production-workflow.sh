#!/usr/bin/env bash
# Secret-free structural contract for the split production-auth trust boundary.
# shellcheck disable=SC2016 # GitHub expressions intentionally remain literal.

set -euo pipefail

script_dir=$(CDPATH='' cd "$(dirname "$0")" && pwd)
repo_root=$(CDPATH='' cd "$script_dir/.." && pwd)
workflow=${AUTH_PRODUCTION_WORKFLOW:-$repo_root/.github/workflows/auth-production.yml}
downloader=$repo_root/tools/download-op-auth-candidates.sh
extractor=$repo_root/tools/extract-op-auth-candidate.py
candidate_verifier=$repo_root/tools/verify-op-auth-candidates.sh
signer=$repo_root/tools/sign-op-auth-candidates.sh
signed_verifier=$repo_root/tools/verify-signed-op-auth-matrix.sh
diff_verifier=$repo_root/tools/verify-op-auth-promotion-diff.sh
identity_verifier=$repo_root/tools/validate-auth-promotion-identity.sh
tree_digester=$repo_root/tools/digest-op-auth-tree.py

require_literal() {
    grep -Fq -- "$1" "$2" || {
        printf 'error: %s is missing required contract: %s\n' "$2" "$1" >&2
        exit 1
    }
}

reject_literal() {
    if grep -Fq -- "$1" "$2"; then
        printf 'error: %s contains forbidden contract: %s\n' "$2" "$1" >&2
        exit 1
    fi
}

for file in \
    "$workflow" "$downloader" "$extractor" "$candidate_verifier" "$signer" \
    "$signed_verifier" "$diff_verifier" "$identity_verifier" "$tree_digester"; do
    [[ -f "$file" && ! -L "$file" ]] || {
        printf 'error: missing auth-production contract file: %s\n' "$file" >&2
        exit 1
    }
done
bash -n \
    "$downloader" "$candidate_verifier" "$signer" "$signed_verifier" \
    "$diff_verifier" "$identity_verifier"
python3 "$extractor" --help >/dev/null
python3 "$tree_digester" --help >/dev/null
ruby -e 'require "yaml"; YAML.parse_file(ARGV.fetch(0))' "$workflow"
ruby - "$workflow" <<'RUBY'
require "yaml"

document = YAML.safe_load(File.read(ARGV.fetch(0)), aliases: true)
raise "workflow permissions must default to contents:read" unless
  document.fetch("permissions") == {"contents" => "read"}

jobs = document.fetch("jobs")
raise "auth production must contain exactly four trust-boundary jobs" unless
  jobs.keys.sort == %w[acquire_candidates promote sign validate]
validate = jobs.fetch("validate")
acquire = jobs.fetch("acquire_candidates")
sign = jobs.fetch("sign")
promote = jobs.fetch("promote")
raise "identity validation must stay outside protected environments" if validate.key?("environment")
raise "identity validation must not reference secrets or variables" if
  validate.inspect.match?(/(?:secrets|vars)\./)
raise "validation must bind only the reviewed repository" unless
  validate.fetch("if") == "github.repository == 'ZSeven-W/openpencil'"

{
  "acquire_candidates" => acquire,
  "sign" => sign,
  "promote" => promote,
}.each do |name, job|
  raise "#{name} must use auth-production" unless
    job.fetch("environment") == "auth-production"
  raise "#{name} must use its own GitHub-hosted runner" unless
    job.fetch("runs-on") == "ubuntu-22.04"
  raise "#{name} must not expose values at job scope" if job.key?("env")
  raise "#{name} token must remain contents:read" unless
    job.fetch("permissions") == {"contents" => "read"}
end
raise "acquisition must depend only on validation" unless acquire.fetch("needs") == "validate"
raise "signer must run after validation and candidate acquisition" unless
  sign.fetch("needs").sort == %w[acquire_candidates validate]
raise "promotion must run after all prior trust boundaries" unless
  promote.fetch("needs").sort == %w[acquire_candidates sign validate]

validate_steps = validate.fetch("steps")
acquire_steps = acquire.fetch("steps")
sign_steps = sign.fetch("steps")
promote_steps = promote.fetch("steps")
identity = validate_steps.find do |step|
  step["name"] == "Validate dispatch syntax and exact remote branch lease"
end
download = acquire_steps.find do |step|
  step["name"] == "Download exact immutable private candidate artifacts"
end
upload_candidates = acquire_steps.find do |step|
  step["name"] == "Upload the immutable verified candidate handoff"
end
download_candidates = sign_steps.find do |step|
  step["name"] == "Download the exact immutable candidate handoff"
end
signer = sign_steps.find do |step|
  step["name"] == "Sign the verified matrix and delete the root immediately"
end
upload_signed = sign_steps.find do |step|
  step["name"] == "Upload the immutable signed matrix handoff"
end
download_signed = promote_steps.find do |step|
  step["name"] == "Download the exact immutable signed handoff"
end
promotion = promote_steps.find do |step|
  step["name"] == "Verify signed bytes, create A, and push with an exact S lease"
end
raise "missing exact workflow/source identity validation" unless identity
raise "missing private artifact download step" unless download
raise "missing immutable candidate upload" unless upload_candidates
raise "missing exact candidate download" unless download_candidates
raise "missing root-only signer" unless signer
raise "missing immutable signed upload" unless upload_signed
raise "missing exact signed download" unless download_signed
raise "missing write-only promotion" unless promotion

sensitive = {
  "OP_AUTH_CANDIDATE_READ_TOKEN" => ["acquire_candidates", download],
  "OP_AUTH_PROVENANCE_SIGNING_KEY_PEM" => ["sign", signer],
  "OPENPENCIL_AUTH_PUSH_TOKEN" => ["promote", promotion],
}
sensitive.each do |secret, expected|
  holders = []
  jobs.each do |job_name, job|
    job.fetch("steps").each do |step|
      if step.fetch("env", {}).values.any? { |value| value.to_s.include?(secret) }
        holders << [job_name, step]
      end
    end
  end
  raise "#{secret} must exist in exactly one expected job and step" unless
    holders == [expected]
end

{
  "acquire_candidates" => acquire,
  "sign" => sign,
  "promote" => promote,
}.each do |job_name, job|
  present = sensitive.keys.count { |secret| job.inspect.include?(secret) }
  raise "#{job_name} co-resides with multiple production credentials" unless present == 1
end
raise "signer must not receive GH_TOKEN" if signer.fetch("env", {}).key?("GH_TOKEN")

final_run = signer.fetch("run")
%w[cargo rustup wget].each do |command|
  raise "signer step must not run #{command}" if final_run.match?(/(^|\s)#{command}(\s|$)/)
end
raise "signer step must not download through curl" if final_run.match?(/(^|\s)curl(\s|$)/)
raise "signer step must not call the GitHub API" if final_run.include?("gh api")
raise "signer step must not mutate Git history" if
  final_run.match?(/git\s+-C\s+public-source\s+(?:add|commit|push)/)
raise "signer step must not replace public artifacts" if final_run.include?("rm -rf")
promotion_run = promotion.fetch("run")
raise "write-token step must not execute scripts from source S" if
  promotion_run.include?("public-source/tools/")
raise "write-token step must not run Cargo" if promotion_run.match?(/(^|\s)cargo(\s|$)/)

identity_run = identity.fetch("run")
raise "dispatch must validate exact workflow ref" unless
  identity_run.include?('--workflow-ref "$WORKFLOW_REF"')
raise "dispatch must validate exact workflow SHA" unless
  identity_run.include?('--workflow-sha "$WORKFLOW_SHA"')
raise "workflow ref must come from github.ref" unless
  identity.fetch("env").fetch("WORKFLOW_REF") == "${{ github.ref }}"
raise "workflow SHA must come from github.sha" unless
  identity.fetch("env").fetch("WORKFLOW_SHA") == "${{ github.sha }}"

{
  upload_candidates => "verified-op-auth-candidates-${{ github.run_id }}-${{ github.run_attempt }}",
  upload_signed => "signed-op-auth-matrix-${{ github.run_id }}-${{ github.run_attempt }}",
}.each do |step, expected_name|
  inputs = step.fetch("with")
  raise "handoff upload must use its exact run-unique name" unless
    inputs.fetch("name") == expected_name
  raise "handoff artifact retention must be one day" unless inputs.fetch("retention-days") == 1
  raise "handoff upload must reject overwrites" unless inputs.fetch("overwrite") == false
  raise "handoff upload must reject missing files" unless inputs.fetch("if-no-files-found") == "error"
end
raise "candidate handoff must expose archive digest" unless
  acquire.fetch("outputs").fetch("artifact_digest").include?("artifact-digest")
raise "candidate handoff must expose tree digest" unless
  acquire.fetch("outputs").key?("tree_digest")
raise "signed handoff must expose archive digest" unless
  sign.fetch("outputs").fetch("artifact_digest").include?("artifact-digest")
raise "signed handoff must expose tree digest" unless
  sign.fetch("outputs").key?("tree_digest")
raise "signer must download candidate by exact artifact ID" unless
  download_candidates.fetch("with").fetch("artifact-ids") ==
    "${{ needs.acquire_candidates.outputs.artifact_id }}"
raise "promotion must download signed matrix by exact artifact ID" unless
  download_signed.fetch("with").fetch("artifact-ids") ==
    "${{ needs.sign.outputs.artifact_id }}"
RUBY

[[ "$(grep -Fc 'secrets.OP_AUTH_PROVENANCE_SIGNING_KEY_PEM' "$workflow")" -eq 1 ]] || {
    printf 'error: production root must occur exactly once in the workflow\n' >&2
    exit 1
}
[[ "$(grep -Fc 'secrets.OP_AUTH_CANDIDATE_READ_TOKEN' "$workflow")" -eq 1 ]] || {
    printf 'error: private read credential must occur exactly once in the workflow\n' >&2
    exit 1
}
[[ "$(grep -Fc 'secrets.OPENPENCIL_AUTH_PUSH_TOKEN' "$workflow")" -eq 1 ]] || {
    printf 'error: public push credential must occur exactly once in the workflow\n' >&2
    exit 1
}
require_literal "github.repository == 'ZSeven-W/openpencil'" "$workflow"
require_literal 'environment: auth-production' "$workflow"
require_literal 'cancel-in-progress: false' "$workflow"
require_literal 'expected_ref=refs/heads/$target_branch' "$identity_verifier"
require_literal '[[ "$workflow_ref" == "$expected_ref" ]]' "$identity_verifier"
require_literal '[[ "$workflow_sha" == "$openpencil_sha" ]]' "$identity_verifier"
require_literal 'private_repository=ZSeven-W/op-platform' "$candidate_verifier"
require_literal 'workflow_path=.github/workflows/prebuilt-production.yml' "$candidate_verifier"
require_literal 'and .conclusion == "success"' "$downloader"
require_literal 'and .path == $workflow_path' "$downloader"
require_literal '.total_count == 10' "$downloader"
require_literal 'actual_digest' "$downloader"
require_literal 'chmod -R a-w candidates' "$workflow"
require_literal 'artifact-ids: ${{ needs.acquire_candidates.outputs.artifact_id }}' "$workflow"
require_literal 'artifact-ids: ${{ needs.sign.outputs.artifact_id }}' "$workflow"
require_literal 'trusted_key=public-source/crates/op-auth-bridge/prebuilt/PROVENANCE_PUBKEY' "$workflow"
require_literal 'rm -f "$key_file"' "$workflow"
require_literal '--force-with-lease="refs/heads/$TARGET_BRANCH:$OPENPENCIL_SHA"' "$workflow"
require_literal 'git -C public-source rev-parse HEAD^' "$workflow"
reject_literal 'pull_request_target' "$workflow"
reject_literal 'OPENPENCIL_PUSH_TOKEN' "$workflow"
reject_literal '${{ github.token }}' "$workflow"
reject_literal 'contents: write' "$workflow"
reject_literal 'OP_AUTH_PROVENANCE_SIGNING_KEY_PEM' "$signer"
for forbidden in 'cargo ' 'curl ' 'wget ' 'gh api'; do
    reject_literal "$forbidden" "$signer"
done

delete_line=$(grep -n '^[[:space:]]*rm -f "$key_file"' "$workflow" \
    | tail -n 1 | cut -d: -f1)
verify_line=$(grep -n 'verify-signed-op-auth-matrix.sh' "$workflow" \
    | tail -n 1 | cut -d: -f1)
commit_line=$(grep -n 'git -C public-source commit' "$workflow" \
    | tail -n 1 | cut -d: -f1)
push_line=$(grep -n 'git -C public-source push' "$workflow" \
    | tail -n 1 | cut -d: -f1)
[[ "$delete_line" -lt "$verify_line" && "$verify_line" -lt "$commit_line" \
    && "$commit_line" -lt "$push_line" ]] || {
    printf 'error: key deletion, verification, commit, and push order is unsafe\n' >&2
    exit 1
}

checkout_count=0
download_count=0
upload_count=0
while IFS= read -r action; do
    case "$action" in
        actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683)
            checkout_count=$((checkout_count + 1))
            ;;
        actions/download-artifact@d3f86a106a0bac45b974a628896c90dbdf5c8093)
            download_count=$((download_count + 1))
            ;;
        actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02)
            upload_count=$((upload_count + 1))
            ;;
        *)
            printf 'error: auth-production action is not an allowed full-SHA pin: %s\n' \
                "$action" >&2
            exit 1
            ;;
    esac
done < <(sed -n 's/^[[:space:]]*uses: \([^ #]*\).*$/\1/p' "$workflow")
[[ "$checkout_count" -eq 7 && "$download_count" -eq 2 \
    && "$upload_count" -eq 2 ]] || {
    printf 'error: auth-production must use exactly seven checkout and two artifact hops\n' >&2
    exit 1
}

for file in \
    "$workflow" "$downloader" "$extractor" "$candidate_verifier" "$signer" \
    "$signed_verifier" "$diff_verifier" "$identity_verifier" "$tree_digester"; do
    lines=$(wc -l < "$file" | tr -d '[:space:]')
    [[ "$lines" -le 800 ]] || {
        printf 'error: %s exceeds the 800-line repository limit\n' "$file" >&2
        exit 1
    }
done

printf 'check-auth-production-workflow: isolated runners and exact-lease contracts passed.\n'
