#!/usr/bin/env bash

set -euo pipefail

script_dir=$(CDPATH='' cd "$(dirname "$0")" && pwd)
repo_root=$(CDPATH='' cd "$script_dir/.." && pwd)
checker=$script_dir/check-android-release-workflow.sh

"$checker" >/dev/null

temporary=$(mktemp -d)
trap 'rm -rf "$temporary"' EXIT
mutation_index=0

expect_rejected() {
    local label=$1 override=$2 source=$3 from=$4 to=$5
    local fixture log
    mutation_index=$((mutation_index + 1))
    fixture=$temporary/mutation-$mutation_index
    log=$temporary/mutation-$mutation_index.log
    ruby - "$source" "$fixture" "$from" "$to" <<'RUBY'
source, destination, from, to = ARGV
data = File.binread(source)
abort "mutation source literal is missing" unless data.sub!(from, to)
File.binwrite(destination, data)
RUBY
    if env "$override=$fixture" "$checker" >"$log" 2>&1; then
        printf 'error: Android release mutation was accepted: %s\n' "$label" >&2
        exit 1
    fi
}

workflow=$repo_root/.github/workflows/android-release.yml
signer=$repo_root/scripts/sign-android-release.sh
app_gradle=$repo_root/packaging/android-player/app/build.gradle.kts
sdk_installer=$repo_root/tools/install-pinned-android-sdk.sh
verification=$repo_root/packaging/android-player/gradle/verification-metadata.xml

expect_rejected \
    'independent dispatch trigger' OPENPENCIL_ANDROID_RELEASE_WORKFLOW "$workflow" \
    $'"on":\n  workflow_call:' \
    $'"on":\n  workflow_dispatch:\n  workflow_call:'
expect_rejected \
    'mutable checkout action' OPENPENCIL_ANDROID_RELEASE_WORKFLOW "$workflow" \
    'actions/checkout@08eba0b27e820071cde6df949e0beb9ba4906955' \
    'actions/checkout@v4'
expect_rejected \
    'signer outside protected environment' OPENPENCIL_ANDROID_RELEASE_WORKFLOW "$workflow" \
    'environment: release-production' 'environment: release-staging'
expect_rejected \
    'keystore exposed to build runner' OPENPENCIL_ANDROID_RELEASE_WORKFLOW "$workflow" \
    $'          OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_GLOBAL: ${{ secrets.OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_GLOBAL }}\n          OPENPENCIL_RELEASE_SOURCE_SHA:' \
    $'          OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_GLOBAL: ${{ secrets.OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_GLOBAL }}\n          ANDROID_RELEASE_KEYSTORE_BASE64: ${{ secrets.ANDROID_RELEASE_KEYSTORE_BASE64 }}\n          OPENPENCIL_RELEASE_SOURCE_SHA:'
expect_rejected \
    'certificate fingerprint moved to a secret' OPENPENCIL_ANDROID_RELEASE_WORKFLOW "$workflow" \
    '${{ vars.ANDROID_RELEASE_CERT_SHA256 }}' \
    '${{ secrets.ANDROID_RELEASE_CERT_SHA256 }}'
expect_rejected \
    'nested artifact extraction' OPENPENCIL_ANDROID_RELEASE_WORKFLOW "$workflow" \
    'merge-multiple: true' 'merge-multiple: false'
expect_rejected \
    'sdkmanager on protected signer' OPENPENCIL_ANDROID_RELEASE_WORKFLOW "$workflow" \
    $'          tools/pinned-release-tools.sh android-signing-tools \\\n            "$RUNNER_TEMP/android-signing-tools"' \
    $'          sdkmanager "build-tools;36.0.0"\n          tools/pinned-release-tools.sh android-signing-tools \\\n            "$RUNNER_TEMP/android-signing-tools"'
expect_rejected \
    'Cargo invocation on protected signer' OPENPENCIL_ANDROID_RELEASE_SIGNER "$signer" \
    'chmod 0644 "$ANDROID_SIGNED_OUTPUT_DIR"/*' \
    $'cargo build --release\nchmod 0644 "$ANDROID_SIGNED_OUTPUT_DIR"/*'
expect_rejected \
    'API target downgrade' OPENPENCIL_ANDROID_APP_GRADLE "$app_gradle" \
    'targetSdk = 36' 'targetSdk = 35'
expect_rejected \
    'NDK digest drift' OPENPENCIL_ANDROID_SDK_INSTALLER "$sdk_installer" \
    'dfb20d396df28ca02a8c708314b814a4d961dc9074f9a161932746f815aa552f' \
    '0000000000000000000000000000000000000000000000000000000000000000'
expect_rejected \
    'Gradle dependency verification disabled' \
    OPENPENCIL_ANDROID_VERIFICATION_METADATA "$verification" \
    '<verify-metadata>true</verify-metadata>' \
    '<verify-metadata>false</verify-metadata>'

printf 'check-android-release-workflow.test.sh: fail-closed mutations passed.\n'
