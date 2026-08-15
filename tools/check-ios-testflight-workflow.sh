#!/usr/bin/env bash
# Secret-free structural checks for the TestFlight release lane.
# shellcheck disable=SC2016 # GitHub expressions and source literals stay literal.

set -euo pipefail

script_dir=$(CDPATH='' cd "$(dirname "$0")" && pwd)
repo_root=$(CDPATH='' cd "$script_dir/.." && pwd)
workflow=${OPENPENCIL_IOS_TESTFLIGHT_WORKFLOW:-$repo_root/.github/workflows/ios-testflight.yml}
publisher=${OPENPENCIL_IOS_TESTFLIGHT_PUBLISHER:-$repo_root/scripts/publish-ios-testflight.sh}
matrix_verifier=$repo_root/tools/check-op-auth-release-matrix.sh
artifact_gate=$repo_root/tools/check-op-auth-artifact-commit.sh
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
    "$workflow" "$publisher" "$matrix_verifier" "$artifact_gate" \
    "$relay_verifier" "$project" "$pinned_tools" "$engine_manifest"; do
    [[ -f "$file" && ! -L "$file" ]] || {
        printf 'error: missing TestFlight contract file: %s\n' "$file" >&2
        exit 1
    }
done

bash -n "$publisher" "$matrix_verifier" "$artifact_gate"
python3 "$relay_verifier" --self-test
ruby -e 'require "yaml"; YAML.parse_file(ARGV.fetch(0))' "$workflow"
ruby - "$workflow" <<'RUBY'
require "yaml"

document = YAML.safe_load(File.read(ARGV.fetch(0)), aliases: true)
jobs = document.fetch("jobs")
verify = jobs.fetch("verify")
publish = jobs.fetch("publish")
raise "verify job must remain outside the protected environment" if verify.key?("environment")
if verify.inspect.include?("secrets.") || verify.inspect.include?("vars.")
  raise "verify job must not read protected configuration"
end
raise "publish job must use the testflight environment" unless publish.fetch("environment") == "testflight"
raise "publish job must use the macos-26 image" unless publish.fetch("runs-on") == "macos-26"
raise "publish job must not expose secrets at job scope" if publish.key?("env")

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
require_literal 'environment: testflight' "$workflow"
require_literal 'cancel-in-progress: false' "$workflow"
require_literal 'tools/check-op-auth-release-matrix.test.sh' "$workflow"
require_literal 'tools/check-op-auth-artifact-commit.sh' "$workflow"
require_literal 'tools/check-op-auth-artifact-commit.test.sh' "$workflow"
require_literal 'OP_AUTH_RELEASE_EXPECTED_OPENPENCIL_REVISION:' "$workflow"
require_literal 'IOS_BUILD_NUMBER: ${{ needs.verify.outputs.build_number }}' "$workflow"
require_literal 'IOS_USES_NON_EXEMPT_ENCRYPTION: ${{ vars.IOS_USES_NON_EXEMPT_ENCRYPTION }}' "$workflow"
require_literal 'IOS_ENCRYPTION_EXPORT_COMPLIANCE_CODE: ${{ vars.IOS_ENCRYPTION_EXPORT_COMPLIANCE_CODE }}' "$workflow"
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
[[ "$uses_count" -eq 3 ]] || {
    printf 'error: TestFlight workflow must use exactly three pinned checkout actions\n' >&2
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
require_literal 'non-exempt encryption requires IOS_ENCRYPTION_EXPORT_COMPLIANCE_CODE' "$publisher"
reject_literal 'INFOPLIST_KEY_ITSAppUsesNonExemptEncryption=NO' "$publisher"
reject_literal '--features metal,editor,mobile-auth-dev' "$publisher"
require_literal 'SKIA_BINARIES_URL' "$publisher"
require_literal '[[ -z ${FORCE_SKIA_BINARIES_DOWNLOAD:-}' "$publisher"
require_literal 'skia_key=da8fc6731fc439bc3b6a-aarch64-apple-ios-jpegd-jpege-metal-pdf-textlayout' "$publisher"
require_literal '4abbaea5e4e8934a6f19c5de44eaba9bf9238af4abbe57dbac5f2dc03923b182' "$publisher"
require_literal 'cargo clean -p skia-bindings --target aarch64-apple-ios --release' "$publisher"
require_literal '--features metal,editor,pinned-skia-binaries' "$publisher"
require_literal 'pinned-skia-binaries = ["skia-safe/no-compile"]' "$engine_manifest"
require_literal '4abbaea5e4e8934a6f19c5de44eaba9bf9238af4abbe57dbac5f2dc03923b182' "$pinned_tools"

require_literal 'public_key=$repo_root/crates/op-auth-bridge/prebuilt/PROVENANCE_PUBKEY' "$matrix_verifier"
require_literal 'build_id=$build_id.a3.$hardening_sha' "$matrix_verifier"
reject_literal 'public_key=$prebuilt_root/PROVENANCE_PUBKEY' "$matrix_verifier"
require_literal 'tools/check-op-auth-release-matrix.sh' "$artifact_gate"
require_literal 'tools/check-op-auth-prebuilt.sh" --require-hardened' "$artifact_gate"

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
done < <(printf '%s\n' "$publisher" "$matrix_verifier" "$artifact_gate")

printf 'check-ios-testflight-workflow.sh: workflow and signing contracts passed.\n'
