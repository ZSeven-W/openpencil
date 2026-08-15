#!/usr/bin/env bash
# Secret-free structural checks for the TestFlight release lane.
# shellcheck disable=SC2016 # GitHub expressions and source literals stay literal.

set -euo pipefail

script_dir=$(CDPATH='' cd "$(dirname "$0")" && pwd)
repo_root=$(CDPATH='' cd "$script_dir/.." && pwd)
workflow=${OPENPENCIL_IOS_APP_STORE_WORKFLOW:-$repo_root/.github/workflows/ios-app-store.yml}
publisher=${OPENPENCIL_IOS_TESTFLIGHT_PUBLISHER:-$repo_root/scripts/publish-ios-testflight.sh}
matrix_verifier=$repo_root/tools/check-op-auth-release-matrix.sh
remote_ref_gate=$repo_root/tools/check-op-auth-remote-ref.sh
ios_build_number=$repo_root/tools/ios-build-number.sh
relay_verifier=$repo_root/tools/check-collab-bootstrap-urls.py
project=$repo_root/packaging/ios-player/project.yml
pinned_tools=$repo_root/tools/pinned-release-tools.sh
engine_manifest=$repo_root/crates/op-engine-ffi/Cargo.toml

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
    "$workflow" "$publisher" "$matrix_verifier" "$remote_ref_gate" \
    "$ios_build_number" \
    "$relay_verifier" "$project" "$pinned_tools" "$engine_manifest"; do
    [[ -f "$file" && ! -L "$file" ]] || {
        printf 'error: missing TestFlight contract file: %s\n' "$file" >&2
        exit 1
    }
done

bash -n \
    "$publisher" "$matrix_verifier" "$remote_ref_gate" "$ios_build_number"
"$remote_ref_gate" --self-test
"$ios_build_number" --self-test
python3 "$relay_verifier" --self-test
ruby -e 'require "yaml"; YAML.parse_file(ARGV.fetch(0))' "$workflow"
ruby - "$workflow" <<'RUBY'
require "yaml"

document = YAML.safe_load(File.read(ARGV.fetch(0)), aliases: true)
triggers = document["on"] || document[true]
raise "missing workflow triggers" unless triggers.is_a?(Hash)
unless triggers.keys.sort == %w[workflow_call workflow_dispatch]
  raise "App Store workflow must be reusable and independently dispatchable without a direct tag trigger"
end
%w[workflow_call workflow_dispatch].each do |trigger_name|
  inputs = triggers.fetch(trigger_name).fetch("inputs")
  unless inputs.keys.sort == %w[release_ref release_sha]
    raise "#{trigger_name} must select exact release source and ref inputs"
  end
  inputs.each do |name, input|
    raise "#{trigger_name} #{name} must be required" unless input.fetch("required") == true
    raise "#{trigger_name} #{name} must remain a string" unless input.fetch("type") == "string"
  end
end
call_secrets = triggers.fetch("workflow_call").fetch("secrets")
expected_call_secrets = %w[
  APPLE_TEAM_ID
  OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_CN
  OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_GLOBAL
]
unless call_secrets.keys.sort == expected_call_secrets.sort
  raise "reusable App Store workflow must accept only the reviewed repository secrets"
end
call_secrets.each do |name, secret|
  raise "workflow_call #{name} must be required" unless secret.fetch("required") == true
end

jobs = document.fetch("jobs")
verify = jobs.fetch("verify")
publish = jobs.fetch("publish")
unless document.fetch("concurrency") == {
    "group" => "ios-app-store-tech-zseven-openpencil",
    "cancel-in-progress" => false,
  }
  raise "all App Store uploads must share one non-canceling global queue"
end
raise "verify job must remain outside the protected environment" if verify.key?("environment")
if verify.inspect.include?("secrets.") || verify.inspect.include?("vars.")
  raise "verify job must not read protected configuration"
end
raise "publish job must use the testflight environment" unless publish.fetch("environment") == "testflight"
raise "publish job must use the macos-26 image" unless publish.fetch("runs-on") == "macos-26"
raise "publish job must not expose secrets at job scope" if publish.key?("env")

verify_steps = verify.fetch("steps")
preflight = verify_steps.fetch(0)
unless preflight.fetch("name") == "Bind requested source to the trusted trigger ref before checkout"
  raise "the first verify step must bind user inputs before any checkout or repository code"
end
preflight_env = preflight.fetch("env")
unless preflight_env == {
    "REQUESTED_RELEASE_SHA" => "${{ inputs.release_sha }}",
    "REQUESTED_RELEASE_REF" => "${{ inputs.release_ref }}",
  }
  raise "pre-checkout source binding must consume only the two explicit inputs"
end
preflight_script = preflight.fetch("run")
unless preflight_script.include?('"$REQUESTED_RELEASE_SHA" == "$GITHUB_SHA"') &&
    preflight_script.include?('"$REQUESTED_RELEASE_REF" == "$GITHUB_REF"') &&
    preflight_script.include?('^refs/(heads|tags)/v[0-9]+\.[0-9]+\.[0-9]+$') &&
    !preflight_script.include?("tools/") && !preflight_script.include?("scripts/")
  raise "pre-checkout binding must be inline and pin exact SHA/ref before repository code"
end

release_step = verify.fetch("steps").find do |step|
  step["name"] == "Validate the exact canonical source and release version"
end
raise "missing exact release source gate" unless release_step
release_env = release_step.fetch("env")
unless release_env.fetch("OPENPENCIL_RELEASE_SHA") == "${{ inputs.release_sha }}"
  raise "release source must come from the explicit full-SHA input"
end
unless release_env.fetch("OPENPENCIL_RELEASE_REF") == "${{ inputs.release_ref }}"
  raise "source gate must validate the explicit release ref"
end
unless release_env.fetch("OPENPENCIL_CANONICAL_REMOTE") ==
    "https://github.com/ZSeven-W/openpencil.git"
  raise "release ref must be resolved against the canonical public repository"
end
release_script = release_step.fetch("run")
unless release_script.scan("tools/check-op-auth-remote-ref.sh").length == 2 &&
    release_script.include?("tools/check-op-auth-remote-ref.sh --self-test") &&
    release_script.scan("tools/ios-build-number.sh").length == 2 &&
    release_script.include?("tools/ios-build-number.sh --self-test") &&
    release_script.include?('"$(git rev-parse HEAD)" == "$OPENPENCIL_RELEASE_SHA"') &&
    release_script.include?('"$OPENPENCIL_RELEASE_REF"')
  raise "exact release gate must validate the canonical source and version"
end
checkout = verify.fetch("steps").find do |step|
  step["name"] == "Checkout the exact release source without credentials"
end
raise "missing exact release source checkout" unless checkout
unless verify_steps.index(checkout) == 1
  raise "exact source checkout must run immediately after the inline trust preflight"
end
unless checkout.fetch("with").fetch("ref") == "${{ inputs.release_sha }}"
  raise "verify job must check out the explicit release SHA"
end
matrix_step = verify.fetch("steps").find do |step|
  step["name"] == "Verify signed ABI 3 release matrix and hardened symbol surface"
end
unless matrix_step && !matrix_step.key?("env") &&
    matrix_step.fetch("run").include?("tools/check-op-auth-release-matrix.sh") &&
    matrix_step.fetch("run").include?("tools/check-op-auth-prebuilt.sh --require-hardened")
  raise "signed matrix gate must validate the adopted source-tree matrix without equality overrides"
end

publish_checkout = publish.fetch("steps").find do |step|
  step["name"] == "Checkout the exact release source without credentials"
end
unless publish_checkout && publish_checkout.fetch("with").fetch("ref") ==
    "${{ needs.verify.outputs.source_sha }}"
  raise "publisher must build the exact verified release source"
end
if document.inspect.include?(".op-auth-artifact") ||
    document.inspect.include?("OP_AUTH_RELEASE_EXPECTED_OPENPENCIL_REVISION")
  raise "App Store workflow must not overlay an Auth-only child or bind its parent"
end

toolchain_gate = publish.fetch("steps").find do |step|
  step["name"] == "Validate Xcode 26, iOS 26 SDK, and source before credentials are exposed"
end
raise "missing pre-secret Apple toolchain gate" unless toolchain_gate
gate_script = toolchain_gate.fetch("run")
[
  "xcode_version=$(xcodebuild -version)",
  "xcode_major=${BASH_REMATCH[1]}",
  "if (( xcode_major < 26 )); then",
  "iphoneos_sdk_version=$(xcrun --sdk iphoneos --show-sdk-version)",
  "iphoneos_sdk_major=${BASH_REMATCH[1]}",
  "if (( iphoneos_sdk_major < 26 )); then",
].each { |literal| raise "missing Apple toolchain gate: #{literal}" unless gate_script.include?(literal) }

steps = publish.fetch("steps")
skia_index = steps.index { |step| step["name"] == "Stage digest-pinned iOS Skia binary cache" }
config_index = steps.index { |step| step["name"] == "Validate protected production release configuration" }
publish_index = steps.index { |step| step["name"] == "Publish the collaboration-enabled build to TestFlight" }
raise "verified iOS Skia cache must be staged before protected configuration" unless
  skia_index && config_index && publish_index && skia_index < config_index && config_index < publish_index
skia_step = steps.fetch(skia_index)
unless skia_step.fetch("run") ==
    'tools/pinned-release-tools.sh skia ios aarch64-apple-ios "$RUNNER_TEMP/openpencil-skia-aarch64-apple-ios"'
  raise "iOS Skia cache must use the reviewed repository downloader"
end

sensitive = %w[
  APPLE_TEAM_ID
  APP_STORE_CONNECT_API_KEY_BASE64
  APP_STORE_CONNECT_API_KEY_ID
  APP_STORE_CONNECT_ISSUER_ID
  IOS_DISTRIBUTION_CERTIFICATE_BASE64
  IOS_DISTRIBUTION_CERTIFICATE_PASSWORD
  IOS_PROVISIONING_PROFILE_BASE64
]
holders = steps.select do |step|
  env = step.fetch("env", {})
  sensitive.any? { |name| env.key?(name) }
end
unless holders.length == 1 && holders.fetch(0).fetch("name") ==
    "Publish the collaboration-enabled build to TestFlight"
  raise "raw Apple signing credentials must be scoped to one publish step"
end
RUBY

require_literal 'contents: read' "$workflow"
reject_literal 'contents: write' "$workflow"
require_literal 'name: iOS App Store / TestFlight' "$workflow"
require_literal 'workflow_call:' "$workflow"
require_literal 'workflow_dispatch:' "$workflow"
require_literal 'release_sha:' "$workflow"
require_literal 'release_ref:' "$workflow"
require_literal 'Bind requested source to the trusted trigger ref before checkout' "$workflow"
require_literal '[[ "$REQUESTED_RELEASE_SHA" == "$GITHUB_SHA" ]]' "$workflow"
require_literal '[[ "$REQUESTED_RELEASE_REF" == "$GITHUB_REF" ]]' "$workflow"
require_literal 'IOS_USES_NON_EXEMPT_ENCRYPTION (must be YES' "$workflow"
require_literal 'IOS_ENCRYPTION_EXPORT_COMPLIANCE_CODE (required; keep its value out of the repository)' "$workflow"
require_literal 'OPENPENCIL_RELEASE_SHA: ${{ inputs.release_sha }}' "$workflow"
require_literal 'OPENPENCIL_RELEASE_REF: ${{ inputs.release_ref }}' "$workflow"
require_literal 'OPENPENCIL_CANONICAL_REMOTE: https://github.com/ZSeven-W/openpencil.git' "$workflow"
require_literal 'tools/check-op-auth-remote-ref.sh --self-test' "$workflow"
require_literal 'tools/ios-build-number.sh --self-test' "$workflow"
require_literal 'build_number=$(tools/ios-build-number.sh)' "$workflow"
require_literal '^[1-9][0-9]{0,3}\.([0-9]|[1-9][0-9])\.([0-9]|[1-9][0-9])$' "$workflow"
reject_literal 'build_number="$GITHUB_RUN_ID.$GITHUB_RUN_ATTEMPT"' "$workflow"
reject_literal 'build_number="$GITHUB_RUN_NUMBER.$GITHUB_RUN_ATTEMPT"' "$workflow"
reject_literal "tags: ['v*']" "$workflow"
reject_literal 'auth_artifact_sha:' "$workflow"
reject_literal 'OP_AUTH_ARTIFACT_COMMIT:' "$workflow"
require_literal 'environment: testflight' "$workflow"
require_literal 'group: ios-app-store-tech-zseven-openpencil' "$workflow"
require_literal 'cancel-in-progress: false' "$workflow"
require_literal 'tools/check-op-auth-release-matrix.test.sh' "$workflow"
require_literal 'tools/check-op-auth-release-matrix.sh' "$workflow"
require_literal 'tools/check-op-auth-prebuilt.sh --require-hardened' "$workflow"
reject_literal 'tools/check-op-auth-artifact-commit.sh' "$workflow"
reject_literal 'tools/check-op-auth-artifact-commit.test.sh' "$workflow"
reject_literal 'OP_AUTH_RELEASE_EXPECTED_OPENPENCIL_REVISION:' "$workflow"
reject_literal 'OP_AUTH_RELEASE_WORKSPACE_VERSION:' "$workflow"
reject_literal '.op-auth-artifact' "$workflow"
require_literal 'IOS_BUILD_NUMBER: ${{ needs.verify.outputs.build_number }}' "$workflow"
require_literal 'IOS_MARKETING_VERSION: ${{ needs.verify.outputs.version }}' "$workflow"
require_literal 'IOS_USES_NON_EXEMPT_ENCRYPTION: ${{ vars.IOS_USES_NON_EXEMPT_ENCRYPTION }}' "$workflow"
require_literal 'IOS_ENCRYPTION_EXPORT_COMPLIANCE_CODE: ${{ vars.IOS_ENCRYPTION_EXPORT_COMPLIANCE_CODE }}' "$workflow"
require_literal '[[ "$USES_NON_EXEMPT_ENCRYPTION" == YES ]]' "$workflow"
require_literal '[[ "$EXPORT_COMPLIANCE_CODE" =~ ^[A-Za-z0-9._-]{1,128}$ ]]' "$workflow"
reject_literal '"$USES_NON_EXEMPT_ENCRYPTION" == NO' "$workflow"
require_literal 'OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_CN: ${{ secrets.OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_CN }}' "$workflow"
require_literal 'OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_GLOBAL: ${{ secrets.OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_GLOBAL }}' "$workflow"
reject_literal 'OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_CN: ${{ vars.OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_CN }}' "$workflow"
reject_literal 'OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_GLOBAL: ${{ vars.OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_GLOBAL }}' "$workflow"
require_literal 'XCODEGEN_SHA256: 090ec29491aad50aec10631bf6e62253fed733c50f3aab0f5ffc86bc170bdbef' "$workflow"
require_literal "python3 tools/check-collab-bootstrap-urls.py" "$workflow"
require_literal 'tools/pinned-release-tools.sh skia ios aarch64-apple-ios' "$workflow"
reject_literal 'actions/upload-artifact' "$workflow"
reject_literal 'mobile-auth-dev' "$workflow"

uses_count=0
while IFS= read -r action; do
    uses_count=$((uses_count + 1))
    [[ "$action" =~ ^actions/checkout@[0-9a-f]{40}$ ]] || {
        printf 'error: TestFlight action is not an allowed full-SHA pin: %s\n' "$action" >&2
        exit 1
    }
done < <(sed -n 's/^[[:space:]]*uses: \([^ #]*\).*$/\1/p' "$workflow")
[[ "$uses_count" -eq 2 ]] || {
    printf 'error: App Store workflow must use exactly two pinned checkout actions\n' >&2
    exit 1
}

capture_line=$(grep -n '^certificate_base64=' "$publisher" | head -n 1 | cut -d: -f1)
relay_capture_line=$(grep -n '^relay_bootstrap_cn=' "$publisher" | head -n 1 | cut -d: -f1)
path_line=$(grep -n '^script_dir=' "$publisher" | cut -d: -f1)
unset_line=$(grep -n '^unset IOS_DISTRIBUTION_CERTIFICATE_BASE64' "$publisher" | cut -d: -f1)
relay_unset_line=$(grep -n '^unset OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_CN' "$publisher" | cut -d: -f1)
[[ "$capture_line" -lt "$path_line" && "$unset_line" -lt "$path_line" \
    && "$relay_capture_line" -lt "$path_line" \
    && "$relay_unset_line" -lt "$path_line" ]] || {
    printf 'error: protected release values must be captured and unset before child processes\n' >&2
    exit 1
}
require_literal "bundle_id=tech.zseven.openpencil" "$publisher"
require_literal 'umask 077' "$publisher"
require_literal 'export -n' "$publisher"
require_literal 'iOS build number must use conservative 4.2.2 numeric components' "$publisher"
require_literal '^[1-9][0-9]{0,3}\.([0-9]|[1-9][0-9])\.([0-9]|[1-9][0-9])$' "$publisher"
require_literal 'python3 "$repo_root/tools/check-collab-bootstrap-urls.py"' "$publisher"
require_literal "CODE_SIGN_IDENTITY='Apple Distribution'" "$publisher"
require_literal '-x -T /usr/bin/codesign -t cert -f pkcs12' "$publisher"
require_literal 'security find-key -t private "$keychain_path"' "$publisher"
reject_literal ' -A ' "$publisher"
require_literal 'profile.get("ExpirationDate")' "$publisher"
require_literal 'profile.get("ProvisionsAllDevices") is True' "$publisher"
require_literal 'entitlements.get("beta-reports-active") is not True' "$publisher"
require_literal '"destination": "upload"' "$publisher"
require_literal '"INFOPLIST_KEY_ITSAppUsesNonExemptEncryption=$IOS_USES_NON_EXEMPT_ENCRYPTION"' "$publisher"
require_literal '"INFOPLIST_KEY_ITSEncryptionExportComplianceCode=$encryption_export_code"' "$publisher"
require_literal '[[ "$IOS_USES_NON_EXEMPT_ENCRYPTION" == YES ]]' "$publisher"
require_literal "== true ]]" "$publisher"
require_literal 'non-exempt encryption requires IOS_ENCRYPTION_EXPORT_COMPLIANCE_CODE' "$publisher"
reject_literal '"$IOS_USES_NON_EXEMPT_ENCRYPTION" == NO' "$publisher"
reject_literal 'exempt-encryption builds' "$publisher"
reject_literal 'INFOPLIST_KEY_ITSAppUsesNonExemptEncryption=NO' "$publisher"
reject_literal '--features metal,editor,mobile-auth-dev' "$publisher"
require_literal 'SKIA_BINARIES_URL' "$publisher"
require_literal '[[ -z ${FORCE_SKIA_BINARIES_DOWNLOAD:-}' "$publisher"
require_literal 'skia_key=da8fc6731fc439bc3b6a-aarch64-apple-ios-jpegd-jpege-metal-pdf-textlayout' "$publisher"
require_literal '4abbaea5e4e8934a6f19c5de44eaba9bf9238af4abbe57dbac5f2dc03923b182' "$publisher"
require_literal 'cargo clean -p skia-bindings --target aarch64-apple-ios --release' "$publisher"
require_literal '--features metal,editor,pinned-skia-binaries' "$publisher"
require_literal 'OP_AUTH_CARGO_TARGET=aarch64-apple-ios' "$publisher"
require_literal 'tools/check-op-auth-cargo-build.sh' "$publisher"
reject_literal 'OP_AUTH_RELEASE_EXPECTED_OPENPENCIL_REVISION' "$publisher"
reject_literal 'OP_AUTH_RELEASE_WORKSPACE_VERSION' "$publisher"
require_literal 'pinned-skia-binaries = ["skia-safe/no-compile"]' "$engine_manifest"
require_literal '4abbaea5e4e8934a6f19c5de44eaba9bf9238af4abbe57dbac5f2dc03923b182' "$pinned_tools"

require_literal 'public_key=$repo_root/crates/op-auth-bridge/prebuilt/PROVENANCE_PUBKEY' "$matrix_verifier"
require_literal 'build_id=$build_id.a3.$hardening_sha' "$matrix_verifier"
reject_literal 'public_key=$prebuilt_root/PROVENANCE_PUBKEY' "$matrix_verifier"
require_literal 'git ls-remote --exit-code "$canonical_remote"' "$remote_ref_gate"
require_literal '"$release_ref" "$release_ref^{}"' "$remote_ref_gate"
require_literal 'refs/heads/*)' "$remote_ref_gate"
require_literal 'refs/tags/*)' "$remote_ref_gate"
require_literal '"$peeled_sha" == "$release_sha"' "$remote_ref_gate"
require_literal 'canonical annotated release tag does not peel to the source commit' "$remote_ref_gate"
require_literal '[[ "$epoch_minutes" =~ ^[1-9][0-9]{7}$ ]]' "$ios_build_number"
require_literal 'epoch_minutes=$((10#$epoch_seconds / 60))' "$ios_build_number"
require_literal '10000000 1000.0.0' "$ios_build_number"
require_literal '99999999 9999.99.99' "$ios_build_number"

require_literal 'PRODUCT_BUNDLE_IDENTIFIER: tech.zseven.openpencil' "$project"
require_literal 'INFOPLIST_KEY_NSLocalNetworkUsageDescription:' "$project"
reject_literal 'INFOPLIST_KEY_NSBonjourServices' "$project"
reject_literal 'com.apple.developer.networking.multicast' "$project"

while IFS= read -r source; do
    lines=$(wc -l < "$source" | tr -d '[:space:]')
    [[ "$lines" -le 800 ]] || {
        printf 'error: %s exceeds the 800-line repository limit\n' "$source" >&2
        exit 1
    }
done < <(
    printf '%s\n' \
        "$publisher" "$matrix_verifier" "$remote_ref_gate" "$ios_build_number"
)

printf 'check-ios-app-store-workflow.sh: workflow and signing contracts passed.\n'
