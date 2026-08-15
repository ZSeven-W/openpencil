#!/usr/bin/env bash
# Build the reviewed iOS shell with the signed production collaboration inputs
# and upload it to App Store Connect through Xcode's supported export path.

set -euo pipefail

# Remove raw GitHub secrets from the exported environment before even resolving
# this script's path. Lowercase shell variables are intentionally not exported.
certificate_base64=${IOS_DISTRIBUTION_CERTIFICATE_BASE64:-}
certificate_password=${IOS_DISTRIBUTION_CERTIFICATE_PASSWORD:-}
profile_base64=${IOS_PROVISIONING_PROFILE_BASE64:-}
api_key_base64=${APP_STORE_CONNECT_API_KEY_BASE64:-}
api_key_id=${APP_STORE_CONNECT_API_KEY_ID:-}
api_key_issuer_id=${APP_STORE_CONNECT_ISSUER_ID:-}
apple_team_id=${APPLE_TEAM_ID:-}
relay_bootstrap_cn=${OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_CN:-}
relay_bootstrap_global=${OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_GLOBAL:-}
export -n \
    certificate_base64 certificate_password profile_base64 \
    api_key_base64 api_key_id api_key_issuer_id apple_team_id \
    relay_bootstrap_cn relay_bootstrap_global
unset IOS_DISTRIBUTION_CERTIFICATE_BASE64
unset IOS_DISTRIBUTION_CERTIFICATE_PASSWORD
unset IOS_PROVISIONING_PROFILE_BASE64
unset APP_STORE_CONNECT_API_KEY_BASE64
unset APP_STORE_CONNECT_API_KEY_ID
unset APP_STORE_CONNECT_ISSUER_ID
unset APPLE_TEAM_ID
unset OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_CN
unset OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_GLOBAL
umask 077

script_dir=$(CDPATH='' cd "$(dirname "$0")" && pwd)
repo_root=$(CDPATH='' cd "$script_dir/.." && pwd)
bundle_id=tech.zseven.openpencil
ios_target=aarch64-apple-ios

targets=(
    aarch64-apple-darwin
    aarch64-apple-ios
    aarch64-apple-ios-sim
    aarch64-linux-android
    aarch64-pc-windows-msvc
    aarch64-unknown-linux-gnu
    x86_64-apple-darwin
    x86_64-linux-android
    x86_64-pc-windows-msvc
    x86_64-unknown-linux-gnu
)

usage() {
    printf '%s\n' \
        'usage: scripts/publish-ios-testflight.sh' \
        '' \
        'Required environment:' \
        '  OP_AUTH_ARTIFACT_ROOT' \
        '  OP_AUTH_RELEASE_EXPECTED_OPENPENCIL_REVISION' \
        '  IOS_MARKETING_VERSION, IOS_BUILD_NUMBER, APPLE_TEAM_ID' \
        '  IOS_USES_NON_EXEMPT_ENCRYPTION (YES or NO)' \
        '  OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_CN' \
        '  OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_GLOBAL' \
        '  IOS_DISTRIBUTION_CERTIFICATE_BASE64' \
        '  IOS_DISTRIBUTION_CERTIFICATE_PASSWORD' \
        '  IOS_PROVISIONING_PROFILE_BASE64' \
        '  APP_STORE_CONNECT_API_KEY_BASE64' \
        '  APP_STORE_CONNECT_API_KEY_ID' \
        '  APP_STORE_CONNECT_ISSUER_ID' \
        '  XCODEGEN_BIN'
}

if [[ "$#" -ne 0 ]]; then
    usage >&2
    exit 2
fi

require_env() {
    local name=$1
    if [[ -z "${!name:-}" ]]; then
        printf 'error: required environment variable is missing: %s\n' "$name" >&2
        exit 2
    fi
}

require_secret_value() {
    if [[ -z "$2" ]]; then
        printf 'error: required secret is missing: %s\n' "$1" >&2
        exit 2
    fi
}

require_command() {
    command -v "$1" >/dev/null 2>&1 || {
        printf 'error: required command is unavailable: %s\n' "$1" >&2
        exit 1
    }
}

require_regular_file() {
    if [[ -L "$1" || ! -f "$1" ]]; then
        printf 'error: required regular non-symlink file is missing: %s\n' "$1" >&2
        exit 1
    fi
}

sha256_file() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | awk '{ print $1 }'
    else
        shasum -a 256 "$1" | awk '{ print $1 }'
    fi
}

required_env=(
    OP_AUTH_ARTIFACT_ROOT
    OP_AUTH_RELEASE_EXPECTED_OPENPENCIL_REVISION
    IOS_MARKETING_VERSION
    IOS_BUILD_NUMBER
    IOS_USES_NON_EXEMPT_ENCRYPTION
    SKIA_BINARIES_URL
    XCODEGEN_BIN
)
for name in "${required_env[@]}"; do
    require_env "$name"
done
[[ -z ${FORCE_SKIA_BINARIES_DOWNLOAD:-} && $SKIA_BINARIES_URL == file://* ]] || {
    printf 'error: TestFlight builds require the verified local Skia binary cache\n' >&2
    exit 2
}
skia_key=da8fc6731fc439bc3b6a-aarch64-apple-ios-jpegd-jpege-metal-pdf-textlayout
skia_archive_url=${SKIA_BINARIES_URL/\{key\}/$skia_key}
skia_archive=${skia_archive_url#file://}
require_regular_file "$skia_archive"
[[ "$(sha256_file "$skia_archive")" \
    == 4abbaea5e4e8934a6f19c5de44eaba9bf9238af4abbe57dbac5f2dc03923b182 ]] || {
    printf 'error: staged iOS Skia archive digest mismatch\n' >&2
    exit 1
}
require_secret_value IOS_DISTRIBUTION_CERTIFICATE_BASE64 "$certificate_base64"
require_secret_value IOS_DISTRIBUTION_CERTIFICATE_PASSWORD "$certificate_password"
require_secret_value IOS_PROVISIONING_PROFILE_BASE64 "$profile_base64"
require_secret_value APP_STORE_CONNECT_API_KEY_BASE64 "$api_key_base64"
require_secret_value APP_STORE_CONNECT_API_KEY_ID "$api_key_id"
require_secret_value APP_STORE_CONNECT_ISSUER_ID "$api_key_issuer_id"
require_secret_value APPLE_TEAM_ID "$apple_team_id"
require_secret_value OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_CN "$relay_bootstrap_cn"
require_secret_value OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_GLOBAL "$relay_bootstrap_global"

for command in \
    base64 cargo codesign find grep openssl plutil python3 rustup security \
    ruby sed shasum strings xcodebuild xcrun xxd; do
    require_command "$command"
done

workspace_version=$("$repo_root/scripts/workspace-version.sh")
[[ "$IOS_MARKETING_VERSION" == "$workspace_version" ]] || {
    printf 'error: iOS marketing version must match the OpenPencil workspace\n' >&2
    exit 2
}
[[ "$IOS_BUILD_NUMBER" =~ ^[1-9][0-9]*(\.[1-9][0-9]*){1,2}$ ]] || {
    printf 'error: iOS build number must contain two or three positive numeric components\n' >&2
    exit 2
}
[[ "$apple_team_id" =~ ^[A-Z0-9]{10}$ ]] || {
    printf 'error: APPLE_TEAM_ID must be a 10-character team identifier\n' >&2
    exit 2
}
[[ "$api_key_id" =~ ^[A-Z0-9]{10}$ ]] || {
    printf 'error: APP_STORE_CONNECT_API_KEY_ID is malformed\n' >&2
    exit 2
}
[[ "$api_key_issuer_id" \
    =~ ^[0-9A-Fa-f]{8}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{12}$ ]] || {
    printf 'error: APP_STORE_CONNECT_ISSUER_ID is malformed\n' >&2
    exit 2
}
[[ "$IOS_USES_NON_EXEMPT_ENCRYPTION" == YES \
    || "$IOS_USES_NON_EXEMPT_ENCRYPTION" == NO ]] || {
    printf 'error: IOS_USES_NON_EXEMPT_ENCRYPTION must be explicitly set to YES or NO\n' >&2
    exit 2
}
encryption_export_code=${IOS_ENCRYPTION_EXPORT_COMPLIANCE_CODE:-}
if [[ "$IOS_USES_NON_EXEMPT_ENCRYPTION" == YES ]]; then
    [[ "$encryption_export_code" =~ ^[A-Za-z0-9._-]{1,128}$ ]] || {
        printf '%s\n' \
            'error: non-exempt encryption requires IOS_ENCRYPTION_EXPORT_COMPLIANCE_CODE' \
            >&2
        exit 2
    }
fi
[[ "$OP_AUTH_RELEASE_EXPECTED_OPENPENCIL_REVISION" =~ ^[0-9a-f]{40}$ ]] || {
    printf 'error: expected OpenPencil auth revision must be a full commit SHA\n' >&2
    exit 2
}
printf '%s\0%s\0' "$relay_bootstrap_cn" "$relay_bootstrap_global" \
    | python3 "$repo_root/tools/check-collab-bootstrap-urls.py"

case "$OP_AUTH_ARTIFACT_ROOT" in
    /*) ;;
    *)
        printf 'error: OP_AUTH_ARTIFACT_ROOT must be absolute\n' >&2
        exit 2
        ;;
esac
[[ -d "$OP_AUTH_ARTIFACT_ROOT" && ! -L "$OP_AUTH_ARTIFACT_ROOT" ]] || {
    printf 'error: op-auth artifact root must be a non-symlink directory\n' >&2
    exit 1
}
[[ -f "$XCODEGEN_BIN" && -x "$XCODEGEN_BIN" && ! -L "$XCODEGEN_BIN" ]] || {
    printf 'error: XCODEGEN_BIN must be an executable regular file\n' >&2
    exit 1
}

work_root=${RUNNER_TEMP:-${TMPDIR:-/tmp}}
temp_dir=$(mktemp -d "$work_root/openpencil-testflight.XXXXXX")
staged_prebuilt=$temp_dir/prebuilt
keychain_path=$temp_dir/signing.keychain-db
canonical_prebuilt=$repo_root/crates/op-auth-bridge/prebuilt
canonical_ios=$canonical_prebuilt/$ios_target
canonical_ios_backup=$temp_dir/canonical-ios-backup
canonical_ios_existed=0
canonical_ios_replaced=0
profile_install_path=
profile_install_backup=$temp_dir/profile-backup.mobileprovision
profile_install_existed=0
profile_installed=0
keychain_created=0
original_keychains=()

while IFS= read -r keychain_line; do
    keychain_line=${keychain_line#*\"}
    keychain_line=${keychain_line%\"*}
    [[ -n "$keychain_line" ]] && original_keychains+=("$keychain_line")
done < <(security list-keychains -d user)

cleanup() {
    if [[ "${#original_keychains[@]}" -gt 0 ]]; then
        security list-keychains -d user -s "${original_keychains[@]}" >/dev/null 2>&1 || true
    fi
    if [[ "$keychain_created" -eq 1 ]]; then
        security delete-keychain "$keychain_path" >/dev/null 2>&1 || true
    fi
    if [[ "$profile_installed" -eq 1 ]]; then
        rm -f "$profile_install_path"
        if [[ "$profile_install_existed" -eq 1 ]]; then
            cp -p "$profile_install_backup" "$profile_install_path" >/dev/null 2>&1 || true
        fi
    fi
    if [[ "$canonical_ios_replaced" -eq 1 ]]; then
        rm -rf "$canonical_ios"
        if [[ "$canonical_ios_existed" -eq 1 ]]; then
            cp -R "$canonical_ios_backup" "$canonical_ios" >/dev/null 2>&1 || true
        fi
    fi
    rm -rf "$temp_dir"
}
trap cleanup EXIT

artifact_name_for_target() {
    if [[ "$1" == *-pc-windows-msvc ]]; then
        printf 'op_auth.lib\n'
    else
        printf 'libop_auth.a\n'
    fi
}

copy_artifact_file() {
    local source=$1
    local destination=$2
    require_regular_file "$source"
    cp -p "$source" "$destination"
}

# Copy only signed data into an isolated directory. In particular, ignore any
# key or executable in the artifact checkout. The matrix verifier uses the
# current reviewed source checkout's canonical key and script.
mkdir -p "$staged_prebuilt"
copy_artifact_file \
    "$OP_AUTH_ARTIFACT_ROOT/RELEASE-MANIFEST" \
    "$staged_prebuilt/RELEASE-MANIFEST"
copy_artifact_file \
    "$OP_AUTH_ARTIFACT_ROOT/RELEASE-MANIFEST.sig" \
    "$staged_prebuilt/RELEASE-MANIFEST.sig"
for target in "${targets[@]}"; do
    source_dir=$OP_AUTH_ARTIFACT_ROOT/$target
    destination_dir=$staged_prebuilt/$target
    [[ -d "$source_dir" && ! -L "$source_dir" ]] || {
        printf 'error: artifact target directory is missing: %s\n' "$target" >&2
        exit 1
    }
    artifact_name=$(artifact_name_for_target "$target")
    expected_files=$(printf '%s\n' \
        ABI_VERSION HARDENING-ATTESTATION PROVENANCE PROVENANCE.sig \
        SHA256 VERSION "$artifact_name" | LC_ALL=C sort)
    actual_files=$(find "$source_dir" -mindepth 1 -maxdepth 1 -print \
        | sed 's#^.*/##' | LC_ALL=C sort)
    [[ "$actual_files" == "$expected_files" ]] || {
        printf 'error: artifact target %s has missing or unexpected files\n' "$target" >&2
        exit 1
    }
    mkdir -p "$destination_dir"
    for file in \
        ABI_VERSION HARDENING-ATTESTATION PROVENANCE PROVENANCE.sig \
        SHA256 VERSION "$artifact_name"; do
        copy_artifact_file "$source_dir/$file" "$destination_dir/$file"
    done
done

# check-op-auth-prebuilt only checks that a key exists; matrix signature
# verification below always reads the canonical source key directly. This
# staged copy is from that trusted key, never from the artifact checkout.
trusted_public_key=$repo_root/crates/op-auth-bridge/prebuilt/PROVENANCE_PUBKEY
require_regular_file "$trusted_public_key"
trusted_public_key_sha=$(sha256_file "$trusted_public_key")
cp -p "$trusted_public_key" "$staged_prebuilt/PROVENANCE_PUBKEY"

OP_AUTH_PREBUILT_ROOT=$staged_prebuilt \
OP_AUTH_RELEASE_WORKSPACE_VERSION=$workspace_version \
OP_AUTH_RELEASE_EXPECTED_OPENPENCIL_REVISION=$OP_AUTH_RELEASE_EXPECTED_OPENPENCIL_REVISION \
    "$repo_root/tools/check-op-auth-release-matrix.sh"
OP_AUTH_PREBUILT_ROOT=$staged_prebuilt \
    "$repo_root/tools/check-op-auth-prebuilt.sh" --require-hardened

# Cargo's build script intentionally discovers only the canonical target path.
# Install the already-verified iOS directory there, preserve the trusted key,
# and final-link this exact archive into the Xcode app.
if [[ -e "$canonical_ios" || -L "$canonical_ios" ]]; then
    [[ -d "$canonical_ios" && ! -L "$canonical_ios" ]] || {
        printf 'error: canonical iOS auth path is not a regular directory\n' >&2
        exit 1
    }
    cp -R "$canonical_ios" "$canonical_ios_backup"
    canonical_ios_existed=1
fi
rm -rf "$canonical_ios"
canonical_ios_replaced=1
cp -R "$staged_prebuilt/$ios_target" "$canonical_ios"
[[ "$(sha256_file "$trusted_public_key")" == "$trusted_public_key_sha" ]] || {
    printf 'error: trusted provenance key changed during auth staging\n' >&2
    exit 1
}
auth_archive=$canonical_ios/libop_auth.a
[[ "$(sha256_file "$auth_archive")" \
    == "$(sha256_file "$staged_prebuilt/$ios_target/libop_auth.a")" ]] || {
    printf 'error: canonical iOS auth archive differs from the verified matrix\n' >&2
    exit 1
}
CONFIGURATION=Release \
OP_AUTH_ARCHIVE=$auth_archive \
OP_AUTH_TARGET=$ios_target \
    "$repo_root/tools/check-mobile-auth-link-input.sh"

cd "$repo_root"
rustup target add --toolchain 1.94 aarch64-apple-ios
cargo clean -p skia-bindings --target aarch64-apple-ios --release
RUSTUP_TOOLCHAIN=1.94 \
OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_CN=$relay_bootstrap_cn \
OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_GLOBAL=$relay_bootstrap_global \
    cargo build --locked --release -p op-engine-ffi \
        --target aarch64-apple-ios --features metal,editor,pinned-skia-binaries
engine_archive=$repo_root/target/aarch64-apple-ios/release/libop_engine_ffi.a
require_regular_file "$engine_archive"
undefined_symbols=$temp_dir/engine-undefined-symbols
xcrun nm -u "$engine_archive" > "$undefined_symbols"
for symbol in \
    op_auth_abi_version \
    op_auth_collab_ticket_begin \
    op_auth_collab_relay_token_begin; do
    grep -Eq "(^|[[:space:]])_?${symbol}$" "$undefined_symbols" || {
        printf 'error: iOS engine did not link the production ABI 3 auth path: %s\n' \
            "$symbol" >&2
        exit 1
    }
done
LC_ALL=C grep -aFq "$relay_bootstrap_cn" "$engine_archive" || {
    printf 'error: the CN relay bootstrap URL was not embedded in the engine\n' >&2
    exit 1
}
LC_ALL=C grep -aFq "$relay_bootstrap_global" "$engine_archive" || {
    printf 'error: the global relay bootstrap URL was not embedded in the engine\n' >&2
    exit 1
}
player_dir=$repo_root/packaging/ios-player
(cd "$player_dir" && "$XCODEGEN_BIN" generate --spec project.yml)
ruby - "$player_dir/project.yml" \
    "$player_dir/OpenPencilPlayer.xcodeproj/project.pbxproj" <<'RUBY'
require "yaml"

project = YAML.safe_load(File.read(ARGV.fetch(0)), aliases: true)
target = project.fetch("targets").fetch("OpenPencilPlayer")
scripts = target.fetch("preBuildScripts")
raise "iOS project must have exactly one shell build phase" unless scripts.length == 1
gate = scripts.fetch(0)
raise "unexpected iOS shell build phase" unless gate.fetch("name") == "Validate optional op-auth archive"
expected = <<~'SH'
  if [ -n "${OP_AUTH_ARCHIVE:-}" ]; then
    case "${PLATFORM_NAME:-}" in
      iphoneos) export OP_AUTH_TARGET=aarch64-apple-ios ;;
      iphonesimulator) export OP_AUTH_TARGET=aarch64-apple-ios-sim ;;
      *) echo "error: unsupported auth link platform: ${PLATFORM_NAME:-unset}" >&2; exit 1 ;;
    esac
    bash "$SRCROOT/../../tools/check-mobile-auth-link-input.sh"
  fi
SH
raise "iOS auth build gate changed unexpectedly" unless gate.fetch("script") == expected

pbx = File.read(ARGV.fetch(1))
raise "generated project shell phase count changed" unless pbx.scan("isa = PBXShellScriptBuildPhase;").length == 1
encoded = expected.gsub("\\", "\\\\").gsub('"', '\\"').gsub("\n", "\\n")
raise "generated project does not contain the reviewed auth gate" unless pbx.include?("shellScript = \"#{encoded}\";")
RUBY

certificate_path=$temp_dir/distribution.p12
profile_path=$temp_dir/distribution.mobileprovision
profile_plist=$temp_dir/profile.plist
if ! printf '%s' "$certificate_base64" \
    | base64 --decode > "$certificate_path"; then
    printf 'error: distribution certificate secret is not valid base64\n' >&2
    exit 1
fi
if ! printf '%s' "$profile_base64" \
    | base64 --decode > "$profile_path"; then
    printf 'error: provisioning profile secret is not valid base64\n' >&2
    exit 1
fi
chmod 600 "$certificate_path" "$profile_path"
certificate_base64=
profile_base64=

security cms -D -i "$profile_path" > "$profile_plist"
profile_uuid=$(/usr/libexec/PlistBuddy -c 'Print :UUID' "$profile_plist")
profile_name=$(/usr/libexec/PlistBuddy -c 'Print :Name' "$profile_plist")
profile_team=$(/usr/libexec/PlistBuddy -c 'Print :TeamIdentifier:0' "$profile_plist")
profile_app_id=$(
    /usr/libexec/PlistBuddy -c 'Print :Entitlements:application-identifier' \
        "$profile_plist"
)
profile_get_task_allow=$(
    /usr/libexec/PlistBuddy -c 'Print :Entitlements:get-task-allow' \
        "$profile_plist" 2>/dev/null || true
)
PROFILE_PLIST=$profile_plist python3 - <<'PY'
import datetime
import os
import plistlib
import sys

with open(os.environ["PROFILE_PLIST"], "rb") as source:
    profile = plistlib.load(source)
expiration = profile.get("ExpirationDate")
if not isinstance(expiration, datetime.datetime):
    raise SystemExit("error: provisioning profile ExpirationDate is missing")
if expiration.tzinfo is None:
    expiration = expiration.replace(tzinfo=datetime.timezone.utc)
if expiration <= datetime.datetime.now(datetime.timezone.utc):
    raise SystemExit("error: provisioning profile is expired")
if profile.get("ProvisionsAllDevices") is True:
    raise SystemExit("error: enterprise provisioning profiles cannot publish to TestFlight")
entitlements = profile.get("Entitlements", {})
if entitlements.get("beta-reports-active") is not True:
    raise SystemExit("error: provisioning profile is not enabled for TestFlight beta reports")
PY
[[ "$profile_uuid" \
    =~ ^[0-9A-Fa-f]{8}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{12}$ \
    && -n "$profile_name" ]] || {
    printf 'error: provisioning profile metadata is malformed\n' >&2
    exit 1
}
[[ "$profile_team" == "$apple_team_id" \
    && "$profile_app_id" == "$apple_team_id.$bundle_id" ]] || {
    printf 'error: provisioning profile does not match the configured team and app ID\n' >&2
    exit 1
}
[[ "$profile_get_task_allow" == false ]] || {
    printf 'error: TestFlight requires an App Store distribution profile\n' >&2
    exit 1
}
if /usr/libexec/PlistBuddy -c 'Print :ProvisionedDevices' "$profile_plist" \
    >/dev/null 2>&1; then
    printf 'error: device-limited provisioning profiles cannot publish to TestFlight\n' >&2
    exit 1
fi

profile_install_dir=$HOME/Library/MobileDevice/Provisioning\ Profiles
mkdir -p "$profile_install_dir"
profile_install_path=$profile_install_dir/$profile_uuid.mobileprovision
if [[ -e "$profile_install_path" || -L "$profile_install_path" ]]; then
    [[ -f "$profile_install_path" && ! -L "$profile_install_path" ]] || {
        printf 'error: existing provisioning-profile path is not a regular file\n' >&2
        exit 1
    }
    cp -p "$profile_install_path" "$profile_install_backup"
    profile_install_existed=1
fi
cp -p "$profile_path" "$profile_install_path"
profile_installed=1

keychain_password=$(openssl rand -hex 32)
security create-keychain -p "$keychain_password" "$keychain_path"
keychain_created=1
security set-keychain-settings -lut 21600 "$keychain_path"
security unlock-keychain -p "$keychain_password" "$keychain_path"
security import "$certificate_path" \
    -k "$keychain_path" \
    -P "$certificate_password" \
    -x -T /usr/bin/codesign -t cert -f pkcs12 >/dev/null
certificate_password=
security find-key -t private "$keychain_path" >/dev/null || {
    printf 'error: imported PKCS#12 contains no private key\n' >&2
    exit 1
}
security set-key-partition-list \
    -S apple-tool:,apple:,codesign: -s \
    -k "$keychain_password" "$keychain_path" >/dev/null
security list-keychains -d user -s "$keychain_path" "${original_keychains[@]}"
security find-identity -v -p codesigning "$keychain_path" \
    | grep -Fq 'Apple Distribution' || {
    printf 'error: imported PKCS#12 has no Apple Distribution identity\n' >&2
    exit 1
}

archive_path=$temp_dir/OpenPencilPlayer.xcarchive
derived_data=$temp_dir/DerivedData
encryption_build_settings=(
    "INFOPLIST_KEY_ITSAppUsesNonExemptEncryption=$IOS_USES_NON_EXEMPT_ENCRYPTION"
)
if [[ "$IOS_USES_NON_EXEMPT_ENCRYPTION" == YES ]]; then
    encryption_build_settings+=(
        "INFOPLIST_KEY_ITSEncryptionExportComplianceCode=$encryption_export_code"
    )
fi
xcodebuild \
    -project "$player_dir/OpenPencilPlayer.xcodeproj" \
    -scheme OpenPencilPlayer \
    -configuration Release \
    -destination 'generic/platform=iOS' \
    -archivePath "$archive_path" \
    -derivedDataPath "$derived_data" \
    -hideShellScriptEnvironment \
    DEVELOPMENT_TEAM="$apple_team_id" \
    CODE_SIGN_STYLE=Manual \
    CODE_SIGN_IDENTITY='Apple Distribution' \
    OTHER_CODE_SIGN_FLAGS="--keychain $keychain_path" \
    PROVISIONING_PROFILE_SPECIFIER="$profile_name" \
    PRODUCT_BUNDLE_IDENTIFIER="$bundle_id" \
    MARKETING_VERSION="$IOS_MARKETING_VERSION" \
    CURRENT_PROJECT_VERSION="$IOS_BUILD_NUMBER" \
    INFOPLIST_KEY_CFBundleShortVersionString="$IOS_MARKETING_VERSION" \
    INFOPLIST_KEY_CFBundleVersion="$IOS_BUILD_NUMBER" \
    "${encryption_build_settings[@]}" \
    OP_AUTH_ARCHIVE="$auth_archive" \
    archive

app_path=$archive_path/Products/Applications/OpenPencilPlayer.app
app_info=$app_path/Info.plist
require_regular_file "$app_info"
[[ "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$app_info")" \
    == "$bundle_id" ]]
[[ "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$app_info")" \
    == "$IOS_MARKETING_VERSION" ]]
[[ "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleVersion' "$app_info")" \
    == "$IOS_BUILD_NUMBER" ]]
expected_encryption_plist=false
[[ "$IOS_USES_NON_EXEMPT_ENCRYPTION" == YES ]] && expected_encryption_plist=true
[[ "$(/usr/libexec/PlistBuddy -c 'Print :ITSAppUsesNonExemptEncryption' "$app_info")" \
    == "$expected_encryption_plist" ]] || {
    printf 'error: export-compliance declaration is missing from the archived app\n' >&2
    exit 1
}
if [[ "$IOS_USES_NON_EXEMPT_ENCRYPTION" == YES ]]; then
    [[ "$(/usr/libexec/PlistBuddy \
        -c 'Print :ITSEncryptionExportComplianceCode' "$app_info")" \
        == "$encryption_export_code" ]] || {
        printf 'error: encryption export compliance code is missing from the archived app\n' >&2
        exit 1
    }
elif /usr/libexec/PlistBuddy -c 'Print :ITSEncryptionExportComplianceCode' \
    "$app_info" >/dev/null 2>&1; then
    printf 'error: exempt-encryption builds must not include an export compliance code\n' >&2
    exit 1
fi
local_network_usage=$(
    /usr/libexec/PlistBuddy -c 'Print :NSLocalNetworkUsageDescription' "$app_info" \
        2>/dev/null || true
)
[[ -n "$local_network_usage" ]] || {
    printf 'error: manual LAN collaboration needs NSLocalNetworkUsageDescription\n' >&2
    exit 1
}
if /usr/libexec/PlistBuddy -c 'Print :NSBonjourServices' "$app_info" \
    >/dev/null 2>&1; then
    printf 'error: Bonjour services must stay disabled while raw discovery is disabled\n' >&2
    exit 1
fi

entitlements_path=$temp_dir/app-entitlements.plist
codesign -d --entitlements :- "$app_path" > "$entitlements_path" 2>/dev/null
signed_app_id=$(
    /usr/libexec/PlistBuddy -c 'Print :application-identifier' "$entitlements_path"
)
signed_team=$(
    /usr/libexec/PlistBuddy -c 'Print :com.apple.developer.team-identifier' \
        "$entitlements_path"
)
signed_get_task_allow=$(
    /usr/libexec/PlistBuddy -c 'Print :get-task-allow' "$entitlements_path" \
        2>/dev/null || true
)
[[ "$signed_app_id" == "$apple_team_id.$bundle_id" \
    && "$signed_team" == "$apple_team_id" \
    && "$signed_get_task_allow" == false ]] || {
    printf 'error: archived app entitlements do not match TestFlight distribution\n' >&2
    exit 1
}
if /usr/libexec/PlistBuddy \
    -c 'Print :com.apple.developer.networking.multicast' "$entitlements_path" \
    >/dev/null 2>&1; then
    printf 'error: the disabled discovery path must not request multicast entitlement\n' >&2
    exit 1
fi
codesign --verify --strict "$app_path"
app_executable_name=$(
    /usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' "$app_info"
)
app_executable=$app_path/$app_executable_name
require_regular_file "$app_executable"
LC_ALL=C grep -aFq "$relay_bootstrap_cn" "$app_executable" || {
    printf 'error: final app binary does not contain the CN relay bootstrap\n' >&2
    exit 1
}
LC_ALL=C grep -aFq "$relay_bootstrap_global" "$app_executable" || {
    printf 'error: final app binary does not contain the global relay bootstrap\n' >&2
    exit 1
}

# Materialize the upload credential only after the archive and its signature
# have passed every local check. No network-fetched or unpinned tool runs after
# this point; xcodebuild performs the App Store Connect upload directly.
api_key_path=$temp_dir/AuthKey.p8
if ! printf '%s' "$api_key_base64" \
    | base64 --decode > "$api_key_path"; then
    printf 'error: App Store Connect API key secret is not valid base64\n' >&2
    exit 1
fi
chmod 600 "$api_key_path"
api_key_base64=
grep -Fq -- '-----BEGIN PRIVATE KEY-----' "$api_key_path" || {
    printf 'error: App Store Connect API key is not a PKCS#8 private key\n' >&2
    exit 1
}

export_options=$temp_dir/ExportOptions.plist
EXPORT_OPTIONS_PATH=$export_options \
EXPORT_PROFILE_NAME=$profile_name \
EXPORT_TEAM_ID=$apple_team_id \
EXPORT_BUNDLE_ID=$bundle_id \
python3 - <<'PY'
import os
import plistlib

options = {
    "destination": "upload",
    "manageAppVersionAndBuildNumber": False,
    "method": "app-store-connect",
    "provisioningProfiles": {
        os.environ["EXPORT_BUNDLE_ID"]: os.environ["EXPORT_PROFILE_NAME"],
    },
    "signingCertificate": "Apple Distribution",
    "signingStyle": "manual",
    "stripSwiftSymbols": True,
    "teamID": os.environ["EXPORT_TEAM_ID"],
    "uploadSymbols": True,
}
with open(os.environ["EXPORT_OPTIONS_PATH"], "wb") as output:
    plistlib.dump(options, output, sort_keys=True)
PY

xcodebuild \
    -exportArchive \
    -archivePath "$archive_path" \
    -exportPath "$temp_dir/export" \
    -exportOptionsPlist "$export_options" \
    -authenticationKeyPath "$api_key_path" \
    -authenticationKeyID "$api_key_id" \
    -authenticationKeyIssuerID "$api_key_issuer_id"

printf 'Uploaded %s %s (%s) to App Store Connect for TestFlight processing.\n' \
    "$bundle_id" "$IOS_MARKETING_VERSION" "$IOS_BUILD_NUMBER"
