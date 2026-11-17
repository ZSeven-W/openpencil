#!/usr/bin/env bash
# The SDK used for the final link decides whether the app adapts to Duo.
set -euo pipefail

xcodebuild -version
sdk_version=$(xcrun --sdk iphoneos --show-sdk-version)
if [[ ! $sdk_version =~ ^([0-9]+)\.([0-9]+)(\.[0-9]+)*$ ]] ||
    (( BASH_REMATCH[1] < 27 || (BASH_REMATCH[1] == 27 && BASH_REMATCH[2] < 1) )); then
    printf 'error: iPhone Duo requires the iOS 27.1 SDK or newer (selected: %s)\n' "$sdk_version" >&2
    exit 1
fi
printf 'iPhoneOS SDK %s\n' "$sdk_version"
