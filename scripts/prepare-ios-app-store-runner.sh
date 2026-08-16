#!/usr/bin/env bash
set -euo pipefail

: "${OPENPENCIL_RELEASE_SHA:?OPENPENCIL_RELEASE_SHA is required}"
: "${OPENPENCIL_RELEASE_VERSION:?OPENPENCIL_RELEASE_VERSION is required}"
: "${GITHUB_OUTPUT:?GITHUB_OUTPUT is required}"
: "${GITHUB_WORKSPACE:?GITHUB_WORKSPACE is required}"
: "${RUNNER_TEMP:?RUNNER_TEMP is required}"

[[ "$(git rev-parse HEAD)" == "$OPENPENCIL_RELEASE_SHA" ]] || {
    echo "error: checkout does not match the validated release source" >&2
    exit 1
}

tools/ios-build-number.sh --self-test
build_number=$(tools/ios-build-number.sh)
[[ "$build_number" =~ ^[1-9][0-9]{0,3}\.([0-9]|[1-9][0-9])\.([0-9]|[1-9][0-9])$ ]]
CONFIGURATION=Release \
OP_AUTH_ARCHIVE="$GITHUB_WORKSPACE/crates/op-auth-bridge/prebuilt/aarch64-apple-ios/libop_auth.a" \
OP_AUTH_TARGET=aarch64-apple-ios \
    tools/check-mobile-auth-link-input.sh

rustup toolchain install 1.94 --profile minimal
rustup target add --toolchain 1.94 aarch64-apple-ios

xcodegen_sha256=090ec29491aad50aec10631bf6e62253fed733c50f3aab0f5ffc86bc170bdbef
xcodegen_url=https://github.com/yonaskolb/XcodeGen/releases/download/2.45.4/xcodegen.zip
xcodegen_archive="$RUNNER_TEMP/xcodegen-2.45.4.zip"
xcodegen_root="$RUNNER_TEMP/xcodegen-2.45.4"
curl --fail --location --proto '=https' --tlsv1.2 \
    --silent --show-error "$xcodegen_url" --output "$xcodegen_archive"
actual=$(shasum -a 256 "$xcodegen_archive" | awk '{ print $1 }')
[[ "$actual" == "$xcodegen_sha256" ]] || {
    echo "error: XcodeGen archive digest mismatch" >&2
    exit 1
}
ditto -x -k "$xcodegen_archive" "$xcodegen_root"
xcodegen_binary="$xcodegen_root/xcodegen/bin/xcodegen"
[[ -x "$xcodegen_binary" && ! -L "$xcodegen_binary" ]]
[[ "$($xcodegen_binary --version)" == 'Version: 2.45.4' ]]

tools/pinned-release-tools.sh skia ios aarch64-apple-ios \
    "$RUNNER_TEMP/openpencil-skia-aarch64-apple-ios"

xcode_version=$(xcodebuild -version)
printf '%s\n' "$xcode_version"
xcode_header=${xcode_version%%$'\n'*}
if [[ ! $xcode_header =~ ^Xcode[[:space:]]+([0-9]+)([.][0-9]+)*$ ]] ||
    ((BASH_REMATCH[1] < 26)); then
    echo "error: App Store Connect requires Xcode 26 or newer" >&2
    exit 1
fi
iphoneos_sdk_version=$(xcrun --sdk iphoneos --show-sdk-version)
if [[ ! $iphoneos_sdk_version =~ ^([0-9]+)([.][0-9]+)*$ ]] ||
    ((BASH_REMATCH[1] < 26)); then
    echo "error: App Store Connect requires the iOS 26 SDK or newer" >&2
    exit 1
fi
printf 'iPhoneOS SDK %s\n' "$iphoneos_sdk_version"
packaging/ios/Tests/validate_sources.sh

printf 'build_number=%s\n' "$build_number" >> "$GITHUB_OUTPUT"
printf 'version=%s\n' "$OPENPENCIL_RELEASE_VERSION" >> "$GITHUB_OUTPUT"
printf 'xcodegen_binary=%s\n' "$xcodegen_binary" >> "$GITHUB_OUTPUT"
