#!/usr/bin/env bash
# Secret-free structural checks for the reusable Android release lane.
# shellcheck disable=SC2016 # GitHub expressions are intentional literals.

set -euo pipefail

script_dir=$(CDPATH='' cd "$(dirname "$0")" && pwd)
repo_root=$(CDPATH='' cd "$script_dir/.." && pwd)
workflow=${OPENPENCIL_ANDROID_RELEASE_WORKFLOW:-$repo_root/.github/workflows/android-release.yml}
builder=${OPENPENCIL_ANDROID_RELEASE_BUILDER:-$repo_root/scripts/build-android-release.sh}
signer=${OPENPENCIL_ANDROID_RELEASE_SIGNER:-$repo_root/scripts/sign-android-release.sh}
app_gradle=${OPENPENCIL_ANDROID_APP_GRADLE:-$repo_root/packaging/android-player/app/build.gradle.kts}
root_gradle=${OPENPENCIL_ANDROID_ROOT_GRADLE:-$repo_root/packaging/android-player/build.gradle.kts}
wrapper=${OPENPENCIL_ANDROID_GRADLE_WRAPPER:-$repo_root/packaging/android-player/gradle/wrapper/gradle-wrapper.properties}
verification=${OPENPENCIL_ANDROID_VERIFICATION_METADATA:-$repo_root/packaging/android-player/gradle/verification-metadata.xml}
jni_manifest=${OPENPENCIL_ANDROID_JNI_MANIFEST:-$repo_root/crates/op-engine-jni/Cargo.toml}
pinned_tools=${OPENPENCIL_ANDROID_PINNED_TOOLS:-$repo_root/tools/pinned-release-tools.sh}
sdk_installer=${OPENPENCIL_ANDROID_SDK_INSTALLER:-$repo_root/tools/install-pinned-android-sdk.sh}
mobile_auth_gate=${OPENPENCIL_ANDROID_AUTH_GATE:-$repo_root/tools/check-mobile-auth-link-input.sh}

require_literal() {
    grep -Fq -- "$1" "$2" || {
        printf 'error: %s is missing required Android release contract: %s\n' \
            "$2" "$1" >&2
        exit 1
    }
}

reject_literal() {
    if grep -Fq -- "$1" "$2"; then
        printf 'error: %s contains forbidden Android release contract: %s\n' \
            "$2" "$1" >&2
        exit 1
    fi
}

for file in \
    "$workflow" "$builder" "$signer" "$app_gradle" "$root_gradle" \
    "$wrapper" "$verification" "$jni_manifest" "$pinned_tools" \
    "$sdk_installer" "$mobile_auth_gate"; do
    [[ -f "$file" && ! -L "$file" ]] || {
        printf 'error: missing Android release contract file: %s\n' "$file" >&2
        exit 1
    }
done
bash -n "$builder" "$signer" "$pinned_tools" "$sdk_installer" "$mobile_auth_gate"
ruby -e 'require "yaml"; YAML.parse_file(ARGV.fetch(0))' "$workflow"
ruby -e 'require "rexml/document"; REXML::Document.new(File.read(ARGV.fetch(0)))' \
    "$verification"

ruby - "$workflow" <<'RUBY'
require "yaml"

document = YAML.safe_load(File.read(ARGV.fetch(0)), aliases: true)
events = document.fetch("on")
unless events.is_a?(Hash) && events.keys == ["workflow_call"]
  raise "Android release must be workflow_call-only"
end
call_outputs = events.fetch("workflow_call").fetch("outputs")
%w[version artifact_name artifact_id].each { |name| call_outputs.fetch(name) }
call_secrets = events.fetch("workflow_call").fetch("secrets")
expected_call_secrets = %w[
  OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_CN
  OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_GLOBAL
]
unless call_secrets.keys.sort == expected_call_secrets &&
    call_secrets.values.all? { |contract| contract.fetch("required") == true }
  raise "Android workflow_call must require exactly the two relay secrets"
end

jobs = document.fetch("jobs")
raise "unexpected Android release job graph" unless jobs.keys == %w[verify build sign]
verify = jobs.fetch("verify")
build = jobs.fetch("build")
sign = jobs.fetch("sign")
raise "verify must not use an environment" if verify.key?("environment")
raise "build must not use an environment" if build.key?("environment")
raise "sign must use release-production" unless sign.fetch("environment") == "release-production"
raise "sign job must not expose credentials at job scope" if sign.key?("env")

verify_text = verify.inspect
if verify_text.include?("secrets.") || verify_text.include?("vars.")
  raise "secret-free verify job reads protected configuration"
end
unless verify.fetch("outputs") == {
    "source_sha" => '${{ steps.release.outputs.source_sha }}',
    "version" => '${{ steps.release.outputs.version }}',
  }
  raise "verify must expose only the exact release source and version"
end
verify_checkout = verify.fetch("steps").find do |step|
  step["name"] == "Checkout the exact release source without credentials"
end
unless verify_checkout && verify_checkout.fetch("with") == {
    "fetch-depth" => 1,
    "persist-credentials" => false,
    "ref" => '${{ github.sha }}',
  }
  raise "verify must check out the exact event source without credentials"
end
release_gate = verify.fetch("steps").find { |step| step["id"] == "release" }
unless release_gate && release_gate.fetch("run").include?('"$(git rev-parse HEAD)" == "$GITHUB_SHA"') &&
    release_gate.fetch("run").include?('"$GITHUB_REF" == "refs/tags/v$version"') &&
    !release_gate.inspect.include?("OP_AUTH_ARTIFACT")
  raise "verify must derive the release directly from the exact tag source"
end
matrix_gate = verify.fetch("steps").find do |step|
  step["name"] == "Verify Android release and production Auth contracts"
end
unless matrix_gate && !matrix_gate.key?("env") &&
    matrix_gate.fetch("run").include?("tools/check-op-auth-release-matrix.sh") &&
    matrix_gate.fetch("run").include?("tools/check-op-auth-prebuilt.sh --require-hardened")
  raise "verify must validate the source-tree Auth matrix without source/version overrides"
end

build_steps = build.fetch("steps")
build_secret_steps = build_steps.select { |step| step.fetch("env", {}).values.any? { |value| value.to_s.include?("secrets.") } }
unless build_secret_steps.length == 1 &&
    build_secret_steps.fetch(0).fetch("name") == "Build collaboration-enabled unsigned release handoff"
  raise "relay secrets must be scoped to the unsigned build step"
end
build_secret_names = build_secret_steps.fetch(0).fetch("env").select do |_name, value|
  value.to_s.include?("secrets.")
end.keys.sort
unless build_secret_names == %w[OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_CN OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_GLOBAL]
  raise "build runner may receive only the two relay bootstrap secrets"
end
%w[
  ANDROID_RELEASE_KEYSTORE_BASE64
  ANDROID_RELEASE_KEYSTORE_PASSWORD
  ANDROID_RELEASE_KEY_ALIAS
  ANDROID_RELEASE_KEY_PASSWORD
].each do |name|
  raise "build runner references Android signing secret #{name}" if build.inspect.include?(name)
end

sign_steps = sign.fetch("steps")
raise "protected signer must not invoke sdkmanager" if sign.inspect.include?("sdkmanager")
raise "protected signer must not fall back to ANDROID_HOME" if sign.inspect.include?("ANDROID_HOME")
sensitive = %w[
  ANDROID_RELEASE_KEYSTORE_BASE64
  ANDROID_RELEASE_KEYSTORE_PASSWORD
  ANDROID_RELEASE_KEY_ALIAS
  ANDROID_RELEASE_KEY_PASSWORD
]
holders = sign_steps.select do |step|
  env = step.fetch("env", {})
  sensitive.any? { |name| env.key?(name) }
end
unless holders.length == 1 &&
    holders.fetch(0).fetch("name") == "Sign and verify without invoking Cargo or Gradle"
  raise "Android signing credentials must be scoped to one signer step"
end
sign_env = holders.fetch(0).fetch("env")
sensitive.each { |name| raise "missing signer secret #{name}" unless sign_env.key?(name) }
unless sign_env.fetch("ANDROID_RELEASE_CERT_SHA256").to_s.include?("vars.ANDROID_RELEASE_CERT_SHA256")
  raise "signer must pin the protected certificate fingerprint"
end

build_names = build_steps.map { |step| step["name"] }
sign_names = sign_steps.map { |step| step["name"] }
required_build = [
  "Checkout the exact release source without credentials",
  "Install pinned Rust and Android toolchains",
  "Stage reviewed Android Skia and bundletool inputs",
  "Build collaboration-enabled unsigned release handoff",
  "Upload exact unsigned handoff",
]
required_sign = [
  "Checkout the exact release signer source without credentials",
  "Install digest-pinned Android signing tools",
  "Download exact unsigned artifact id",
  "Sign and verify without invoking Cargo or Gradle",
  "Upload signed Android release assets",
]
raise "Android build step order changed" unless required_build.map { |name| build_names.index(name) }.each_cons(2).all? { |a, b| a && b && a < b }
raise "Android signer step order changed" unless required_sign.map { |name| sign_names.index(name) }.each_cons(2).all? { |a, b| a && b && a < b }
build_checkout = build_steps.find { |step| step["uses"]&.start_with?("actions/checkout@") }
sign_checkout = sign_steps.find { |step| step["uses"]&.start_with?("actions/checkout@") }
unless build_checkout.fetch("with").fetch("ref") == '${{ needs.verify.outputs.source_sha }}' &&
    sign_checkout.fetch("with").fetch("ref") == '${{ needs.verify.outputs.source_sha }}'
  raise "build and sign jobs must use the exact verified release source"
end
build_handoff = build_steps.find do |step|
  step["name"] == "Build collaboration-enabled unsigned release handoff"
end
unless build_handoff.fetch("env").fetch("OPENPENCIL_RELEASE_SOURCE_SHA") ==
      '${{ needs.verify.outputs.source_sha }}' &&
    build_handoff.fetch("env").fetch("OP_AUTH_ARTIFACT_ROOT") ==
      '${{ github.workspace }}/crates/op-auth-bridge/prebuilt'
  raise "Android build must consume the verified matrix from the release source"
end
sign_handoff = sign_steps.find do |step|
  step["name"] == "Sign and verify without invoking Cargo or Gradle"
end
unless sign_handoff.fetch("env").fetch("OPENPENCIL_RELEASE_SOURCE_SHA") ==
    '${{ needs.verify.outputs.source_sha }}'
  raise "Android signer must bind the handoff to the release source"
end
raise "Android workflow must not stage a second Auth checkout" if
  document.inspect.include?(".op-auth-artifact") ||
    document.inspect.include?("OP_AUTH_RELEASE_EXPECTED_OPENPENCIL_REVISION")
RUBY

require_literal 'contents: read' "$workflow"
reject_literal 'contents: write' "$workflow"
reject_literal 'workflow_dispatch:' "$workflow"
reject_literal 'upload-google-play' "$workflow"
reject_literal 'GOOGLE_PLAY_SERVICE_ACCOUNT' "$workflow"
reject_literal 'sdkmanager' "$workflow"
require_literal 'environment: release-production' "$workflow"
require_literal 'cancel-in-progress: false' "$workflow"
require_literal 'artifact-ids: ${{ needs.build.outputs.artifact_id }}' "$workflow"
require_literal 'merge-multiple: true' "$workflow"
require_literal 'name: internal-android-unsigned-${{ github.run_id }}-${{ github.run_attempt }}' "$workflow"
require_literal 'name: openpencil-android-${{ needs.verify.outputs.version }}' "$workflow"
require_literal '${{ runner.temp }}/openpencil-android-signed/SHA256SUMS.android.txt' "$workflow"
require_literal 'tools/check-op-auth-release-matrix.test.sh' "$workflow"
require_literal 'tools/check-op-auth-release-matrix.sh' "$workflow"
require_literal 'tools/check-op-auth-prebuilt.sh --require-hardened' "$workflow"
require_literal 'OPENPENCIL_RELEASE_SOURCE_SHA: ${{ needs.verify.outputs.source_sha }}' "$workflow"
require_literal 'OP_AUTH_ARTIFACT_ROOT: ${{ github.workspace }}/crates/op-auth-bridge/prebuilt' "$workflow"
reject_literal 'tools/check-op-auth-artifact-commit.sh' "$workflow"
reject_literal 'OP_AUTH_RELEASE_EXPECTED_OPENPENCIL_REVISION:' "$workflow"
reject_literal 'OP_AUTH_RELEASE_WORKSPACE_VERSION:' "$workflow"
reject_literal '.op-auth-artifact' "$workflow"
require_literal 'tools/install-pinned-android-sdk.sh' "$workflow"
require_literal 'tools/install-pinned-android-sdk.sh --self-test' "$workflow"
require_literal 'tools/pinned-release-tools.sh skia android' "$workflow"
require_literal 'tools/pinned-release-tools.sh bundletool' "$workflow"
require_literal 'tools/pinned-release-tools.sh android-signing-tools' "$workflow"
require_literal 'export PATH="$ANDROID_JAVA_HOME/bin:$PATH"' "$workflow"
require_literal 'scripts/build-android-release.sh' "$workflow"
require_literal 'scripts/sign-android-release.sh' "$workflow"

uses_count=0
checkout_count=0
upload_count=0
download_count=0
while IFS= read -r action; do
    uses_count=$((uses_count + 1))
    case "$action" in
        actions/checkout@08eba0b27e820071cde6df949e0beb9ba4906955)
            checkout_count=$((checkout_count + 1)) ;;
        actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02)
            upload_count=$((upload_count + 1)) ;;
        actions/download-artifact@d3f86a106a0bac45b974a628896c90dbdf5c8093)
            download_count=$((download_count + 1)) ;;
        *)
            printf 'error: Android release action is not an allowed full-SHA pin: %s\n' \
                "$action" >&2
            exit 1 ;;
    esac
done < <(sed -n 's/^[[:space:]]*uses: \([^ #]*\).*$/\1/p' "$workflow")
[[ "$uses_count" -eq 6 && "$checkout_count" -eq 3 \
    && "$upload_count" -eq 2 && "$download_count" -eq 1 ]] || {
    printf 'error: Android release action graph changed unexpectedly\n' >&2
    exit 1
}

require_literal 'compileSdk = 36' "$app_gradle"
require_literal 'targetSdk = 36' "$app_gradle"
require_literal 'buildToolsVersion = "36.0.0"' "$app_gradle"
require_literal 'ndkVersion = "28.2.13676358"' "$app_gradle"
require_literal 'applicationId = "tech.zseven.openpencil"' "$app_gradle"
require_literal 'abortOnError = true' "$app_gradle"
require_literal 'sourceSets["release"].jniLibs.srcDirs("src/release/jniLibs")' "$app_gradle"
reject_literal 'signingConfigs' "$app_gradle"
require_literal 'id("com.android.application") version "8.13.2"' "$root_gradle"
require_literal 'distributionUrl=https\://services.gradle.org/distributions/gradle-8.14.3-bin.zip' "$wrapper"
require_literal 'distributionSha256Sum=bd71102213493060956ec229d946beee57158dbd89d0e62b91bca0fa2c5f3531' "$wrapper"

require_literal '<verify-metadata>true</verify-metadata>' "$verification"
require_literal '<verify-signatures>false</verify-signatures>' "$verification"
require_literal 'name="gradle" version="8.13.2"' "$verification"
require_literal 'aapt2-8.13.2-14304508-linux.jar' "$verification"
require_literal '839609d6d776d6dd60a02aa577d97193ce3e650cf1deaabf062321e23bbd6bf6' "$verification"
require_literal 'name="kotlin-gradle-plugin" version="2.0.21"' "$verification"
require_literal 'name="activity-ktx" version="1.7.0"' "$verification"
require_literal 'name="appcompat" version="1.7.0"' "$verification"
require_literal 'name="junit" version="4.13.2"' "$verification"

require_literal 'pinned-skia-binaries = ["op-engine-ffi/pinned-skia-binaries"]' "$jni_manifest"
require_literal 'aarch64-linux-android) expected=82ca6dd1720bbe8b105c12c4d0c78786d2c792e9d2a7f2102ab66bb24dafa9d0' "$pinned_tools"
require_literal 'x86_64-linux-android) expected=ca217df6ffced17381cbea4df044969a493a46bddc757ee844e2fbaf54fa1257' "$pinned_tools"
require_literal 'a099cfa1543f55593bc2ed16a70a7c67fe54b1747bb7301f37fdfd6d91028e29' "$pinned_tools"
require_literal '5d9ac77fb6ff43d9da518a337b4fcf8f9097113df531d99ccefe80ef7ce8250b' "$pinned_tools"
require_literal 'f2dc5418092c43003db8f9005c4a286e1c0104fea96ccdd49e8ebd037cac9219' "$pinned_tools"
require_literal 'aarch64-linux-android' "$mobile_auth_gate"
require_literal 'x86_64-linux-android' "$mobile_auth_gate"

require_literal 'build-tools_r36_linux.zip' "$sdk_installer"
require_literal 'platform-36_r02.zip' "$sdk_installer"
require_literal 'android-ndk-r28c-linux.zip' "$sdk_installer"
require_literal '5d9ac77fb6ff43d9da518a337b4fcf8f9097113df531d99ccefe80ef7ce8250b' "$sdk_installer"
require_literal '37607369a28c5b640b3a7998868d45898ebcb777565a0e85f9acf36f29631d2e' "$sdk_installer"
require_literal 'dfb20d396df28ca02a8c708314b814a4d961dc9074f9a161932746f815aa552f' "$sdk_installer"
require_literal 'https://dl.google.com/android/repository/$name' "$sdk_installer"

require_literal 'export -n relay_bootstrap_cn relay_bootstrap_global' "$builder"
require_literal 'unset OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_CN' "$builder"
require_literal 'expected_sdk_digests=' "$builder"
require_literal 'AndroidVersion\.ApiLevel=36' "$builder"
require_literal '--dependency-verification strict' "$builder"
require_literal 'verify-skia android' "$builder"
require_literal 'PAGE_ALIGNMENT_16K' "$builder"
require_literal 'alignment >= 0x4000' "$builder"
require_literal '--features gl,editor,pinned-skia-binaries' "$builder"
require_literal 'tools/check-op-auth-cargo-build.sh' "$builder"
require_literal 'OPENPENCIL_RELEASE_SOURCE_SHA' "$builder"
require_literal 'format=openpencil-android-unsigned-v2' "$builder"
require_literal 'auth_matrix_sha256=$auth_matrix_sha' "$builder"
reject_literal 'OP_AUTH_RELEASE_EXPECTED_OPENPENCIL_REVISION' "$builder"
reject_literal 'OP_AUTH_RELEASE_WORKSPACE_VERSION' "$builder"
reject_literal 'auth_artifact_revision=' "$builder"
reject_literal 'ANDROID_RELEASE_KEYSTORE' "$builder"
reject_literal 'ANDROID_RELEASE_KEY_PASSWORD' "$builder"

require_literal 'unset ANDROID_RELEASE_KEYSTORE_BASE64' "$signer"
require_literal 'ANDROID_RELEASE_CERT_SHA256' "$signer"
require_literal 'ANDROID_SIGNING_TOOLS_ROOT' "$signer"
require_literal 'VERIFIED-DIGESTS' "$signer"
require_literal '64 lowercase hex characters' "$signer"
require_literal '-storepass:file' "$signer"
require_literal '--ks-pass "file:$store_password_file"' "$signer"
require_literal '"$keytool_bin" -printcert -rfc -jarfile' "$signer"
require_literal '"$apksigner_bin" verify --verbose --print-certs' "$signer"
require_literal '"$jarsigner_bin" -verify -verbose -certs' "$signer"
require_literal 'PAGE_ALIGNMENT_16K' "$signer"
require_literal 'OPENPENCIL_RELEASE_SOURCE_SHA' "$signer"
require_literal 'openpencil-android-unsigned-v2' "$signer"
require_literal 'auth_matrix_sha256' "$signer"
require_literal '"$(wc -l < "$manifest" | tr -d '\''[:space:]'\'')" -eq 13' "$signer"
reject_literal 'OP_AUTH_RELEASE_EXPECTED_OPENPENCIL_REVISION' "$signer"
reject_literal 'OP_AUTH_ARTIFACT_COMMIT' "$signer"
require_literal 'OpenPencil-$ANDROID_RELEASE_VERSION-android.apk' "$signer"
require_literal 'OpenPencil-$ANDROID_RELEASE_VERSION-android.aab' "$signer"
if grep -Eq '^[[:space:]]*(cargo|\./gradlew|gradle)[[:space:]]' "$signer"; then
    printf 'error: protected Android signer must not invoke Cargo or Gradle\n' >&2
    exit 1
fi

for source in "$builder" "$signer"; do
    lines=$(wc -l < "$source" | tr -d '[:space:]')
    [[ "$lines" -le 800 ]] || {
        printf 'error: %s exceeds the 800-line repository limit\n' "$source" >&2
        exit 1
    }
done

printf 'check-android-release-workflow.sh: reusable build/sign contracts passed.\n'
