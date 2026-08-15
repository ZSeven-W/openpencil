#!/usr/bin/env bash
# Sign one exact Android release handoff without invoking Cargo, Gradle, or
# repository build scripts. Passwords are passed to platform signers by file.

set -euo pipefail

keystore_base64=${ANDROID_RELEASE_KEYSTORE_BASE64:-}
keystore_password=${ANDROID_RELEASE_KEYSTORE_PASSWORD:-}
key_alias=${ANDROID_RELEASE_KEY_ALIAS:-}
key_password=${ANDROID_RELEASE_KEY_PASSWORD:-}
expected_certificate_sha256=${ANDROID_RELEASE_CERT_SHA256:-}
export -n \
    keystore_base64 keystore_password key_alias key_password \
    expected_certificate_sha256
unset ANDROID_RELEASE_KEYSTORE_BASE64
unset ANDROID_RELEASE_KEYSTORE_PASSWORD
unset ANDROID_RELEASE_KEY_ALIAS
unset ANDROID_RELEASE_KEY_PASSWORD
unset ANDROID_RELEASE_CERT_SHA256
umask 077

script_dir=$(CDPATH='' cd "$(dirname "$0")" && pwd)
repo_root=$(CDPATH='' cd "$script_dir/.." && pwd)
cd "$repo_root"

usage() {
    printf '%s\n' \
        'usage: scripts/sign-android-release.sh' \
        '' \
        'Required environment:' \
        '  ANDROID_UNSIGNED_HANDOFF_DIR' \
        '  ANDROID_SIGNED_OUTPUT_DIR' \
        '  ANDROID_SIGNING_TOOLS_ROOT' \
        '  ANDROID_BUILD_TOOLS_DIR' \
        '  ANDROID_JAVA_HOME' \
        '  BUNDLETOOL_JAR' \
        '  OPENPENCIL_RELEASE_SOURCE_SHA' \
        '  ANDROID_RELEASE_VERSION' \
        '  ANDROID_RELEASE_KEYSTORE_BASE64' \
        '  ANDROID_RELEASE_KEYSTORE_PASSWORD' \
        '  ANDROID_RELEASE_KEY_ALIAS' \
        '  ANDROID_RELEASE_KEY_PASSWORD' \
        '  ANDROID_RELEASE_CERT_SHA256'
}

[[ "$#" -eq 0 ]] || { usage >&2; exit 2; }

require_env() {
    [[ -n "${!1:-}" ]] || {
        printf 'error: required environment variable is missing: %s\n' "$1" >&2
        exit 2
    }
}

require_secret() {
    [[ -n "$2" ]] || {
        printf 'error: required Android signing secret is missing: %s\n' "$1" >&2
        exit 2
    }
}

require_regular_file() {
    [[ -f "$1" && ! -L "$1" ]] || {
        printf 'error: required regular non-symlink file is missing: %s\n' "$1" >&2
        exit 1
    }
}

sha256_file() {
    sha256sum "$1" | awk '{ print $1 }'
}

manifest_field() {
    local name=$1 manifest=$2 count value
    count=$(grep -c "^${name}=" "$manifest" || true)
    [[ "$count" -eq 1 ]] || {
        printf 'error: Android handoff manifest must contain one %s field\n' "$name" >&2
        exit 1
    }
    value=$(sed -n "s/^${name}=//p" "$manifest")
    [[ -n "$value" ]] || {
        printf 'error: Android handoff manifest field is empty: %s\n' "$name" >&2
        exit 1
    }
    printf '%s\n' "$value"
}

for name in \
    ANDROID_UNSIGNED_HANDOFF_DIR ANDROID_SIGNED_OUTPUT_DIR \
    ANDROID_SIGNING_TOOLS_ROOT ANDROID_BUILD_TOOLS_DIR \
    ANDROID_JAVA_HOME BUNDLETOOL_JAR \
    OPENPENCIL_RELEASE_SOURCE_SHA \
    ANDROID_RELEASE_VERSION; do
    require_env "$name"
done
require_secret ANDROID_RELEASE_KEYSTORE_BASE64 "$keystore_base64"
require_secret ANDROID_RELEASE_KEYSTORE_PASSWORD "$keystore_password"
require_secret ANDROID_RELEASE_KEY_ALIAS "$key_alias"
require_secret ANDROID_RELEASE_KEY_PASSWORD "$key_password"
require_secret ANDROID_RELEASE_CERT_SHA256 "$expected_certificate_sha256"
for absolute in \
    "$ANDROID_UNSIGNED_HANDOFF_DIR" "$ANDROID_SIGNED_OUTPUT_DIR" \
    "$ANDROID_SIGNING_TOOLS_ROOT" "$ANDROID_BUILD_TOOLS_DIR" \
    "$ANDROID_JAVA_HOME" "$BUNDLETOOL_JAR"; do
    [[ "$absolute" == /* ]] || {
        printf 'error: Android signing paths must be absolute: %s\n' "$absolute" >&2
        exit 2
    }
done
[[ -d "$ANDROID_UNSIGNED_HANDOFF_DIR" && ! -L "$ANDROID_UNSIGNED_HANDOFF_DIR" ]] || {
    printf 'error: unsigned Android handoff must be a non-symlink directory\n' >&2
    exit 1
}
[[ ! -e "$ANDROID_SIGNED_OUTPUT_DIR" && ! -L "$ANDROID_SIGNED_OUTPUT_DIR" ]] || {
    printf 'error: signed Android output directory must not already exist\n' >&2
    exit 1
}
require_regular_file "$BUNDLETOOL_JAR"
[[ -d "$ANDROID_SIGNING_TOOLS_ROOT" && ! -L "$ANDROID_SIGNING_TOOLS_ROOT" \
    && -d "$ANDROID_BUILD_TOOLS_DIR" && ! -L "$ANDROID_BUILD_TOOLS_DIR" \
    && -d "$ANDROID_JAVA_HOME" && ! -L "$ANDROID_JAVA_HOME" ]] || {
    printf 'error: Android signing tool roots must be regular directories\n' >&2
    exit 1
}
[[ "$ANDROID_BUILD_TOOLS_DIR" \
        == "$ANDROID_SIGNING_TOOLS_ROOT/build-tools/36.0.0" \
    && "$ANDROID_JAVA_HOME" \
        == "$ANDROID_SIGNING_TOOLS_ROOT/jdk-21.0.8+9" \
    && "$BUNDLETOOL_JAR" \
        == "$ANDROID_SIGNING_TOOLS_ROOT/bundletool/bundletool-all-1.18.3.jar" ]] || {
    printf 'error: Android signing tools do not use the reviewed canonical paths\n' >&2
    exit 1
}
tool_digests=$ANDROID_SIGNING_TOOLS_ROOT/VERIFIED-DIGESTS
require_regular_file "$tool_digests"
expected_tool_digests=$'android_build_tools_sha256=5d9ac77fb6ff43d9da518a337b4fcf8f9097113df531d99ccefe80ef7ce8250b\ntemurin_jdk_sha256=f2dc5418092c43003db8f9005c4a286e1c0104fea96ccdd49e8ebd037cac9219\nbundletool_sha256=a099cfa1543f55593bc2ed16a70a7c67fe54b1747bb7301f37fdfd6d91028e29'
[[ "$(< "$tool_digests")" == "$expected_tool_digests" ]] || {
    printf 'error: Android signing tool digest receipt is invalid\n' >&2
    exit 1
}
[[ "$OPENPENCIL_RELEASE_SOURCE_SHA" =~ ^[0-9a-f]{40}$ ]] || {
    printf 'error: Android signing source must be a full lowercase SHA\n' >&2
    exit 2
}
[[ "$ANDROID_RELEASE_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || {
    printf 'error: Android release version must be stable SemVer\n' >&2
    exit 2
}
[[ "$key_alias" =~ ^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$ ]] || {
    printf 'error: Android release key alias is malformed\n' >&2
    exit 2
}
for password in "$keystore_password" "$key_password"; do
    [[ "$password" != *$'\n'* && "$password" != *$'\r'* ]] || {
        printf 'error: Android signing passwords must be single-line values\n' >&2
        exit 2
    }
done
[[ "$expected_certificate_sha256" =~ ^[0-9a-f]{64}$ ]] || {
    printf 'error: ANDROID_RELEASE_CERT_SHA256 must be 64 lowercase hex characters\n' >&2
    exit 2
}

aapt_bin=$ANDROID_BUILD_TOOLS_DIR/aapt
apksigner_bin=$ANDROID_BUILD_TOOLS_DIR/apksigner
zipalign_bin=$ANDROID_BUILD_TOOLS_DIR/zipalign
java_bin=$ANDROID_JAVA_HOME/bin/java
jarsigner_bin=$ANDROID_JAVA_HOME/bin/jarsigner
keytool_bin=$ANDROID_JAVA_HOME/bin/keytool
for tool in "$aapt_bin" "$apksigner_bin" "$zipalign_bin"; do
    require_regular_file "$tool"
    [[ -x "$tool" ]] || {
        printf 'error: Android signing tool is not executable: %s\n' "$tool" >&2
        exit 1
    }
done
for tool in "$java_bin" "$jarsigner_bin" "$keytool_bin"; do
    require_regular_file "$tool"
    [[ -x "$tool" ]] || {
        printf 'error: reviewed Java signing tool is not executable: %s\n' "$tool" >&2
        exit 1
    }
done
for command in base64 grep openssl sed sha256sum unzip; do
    command -v "$command" >/dev/null 2>&1 || {
        printf 'error: Android signing command is unavailable: %s\n' "$command" >&2
        exit 1
    }
done
export JAVA_HOME=$ANDROID_JAVA_HOME

manifest=$ANDROID_UNSIGNED_HANDOFF_DIR/ANDROID-RELEASE-MANIFEST
require_regular_file "$manifest"
[[ "$(wc -l < "$manifest" | tr -d '[:space:]')" -eq 13 \
    && -z "$(grep $'\r' "$manifest" || true)" ]] || {
    printf 'error: Android handoff manifest has an invalid line format\n' >&2
    exit 1
}
[[ "$(manifest_field format "$manifest")" == openpencil-android-unsigned-v2 ]]
[[ "$(manifest_field application_id "$manifest")" == tech.zseven.openpencil ]]
[[ "$(manifest_field version "$manifest")" == "$ANDROID_RELEASE_VERSION" ]]
[[ "$(manifest_field target_sdk "$manifest")" == 36 ]]
[[ "$(manifest_field release_source_revision "$manifest")" \
    == "$OPENPENCIL_RELEASE_SOURCE_SHA" ]]
auth_matrix_sha=$(manifest_field auth_matrix_sha256 "$manifest")
[[ "$auth_matrix_sha" =~ ^[0-9a-f]{64}$ ]] || {
    printf 'error: Android handoff Auth matrix digest is malformed\n' >&2
    exit 1
}
version_code=$(manifest_field version_code "$manifest")
[[ "$version_code" =~ ^[1-9][0-9]*$ ]] || {
    printf 'error: Android handoff versionCode is malformed\n' >&2
    exit 1
}
apk_name=$(manifest_field apk "$manifest")
aab_name=$(manifest_field aab "$manifest")
[[ "$apk_name" == "OpenPencil-$ANDROID_RELEASE_VERSION-android-unsigned.apk" \
    && "$aab_name" == "OpenPencil-$ANDROID_RELEASE_VERSION-android-unsigned.aab" ]] || {
    printf 'error: Android handoff artifact names are not canonical\n' >&2
    exit 1
}
expected_files=$(printf '%s\n' ANDROID-RELEASE-MANIFEST "$apk_name" "$aab_name" \
    | LC_ALL=C sort)
actual_files=$(find "$ANDROID_UNSIGNED_HANDOFF_DIR" -mindepth 1 -maxdepth 1 -print \
    | sed 's#^.*/##' | LC_ALL=C sort)
[[ "$actual_files" == "$expected_files" ]] || {
    printf 'error: Android signing handoff has missing or unexpected files\n' >&2
    exit 1
}
unsigned_apk=$ANDROID_UNSIGNED_HANDOFF_DIR/$apk_name
unsigned_aab=$ANDROID_UNSIGNED_HANDOFF_DIR/$aab_name
require_regular_file "$unsigned_apk"
require_regular_file "$unsigned_aab"
[[ "$(sha256_file "$unsigned_apk")" == "$(manifest_field apk_sha256 "$manifest")" \
    && "$(sha256_file "$unsigned_aab")" == "$(manifest_field aab_sha256 "$manifest")" ]] || {
    printf 'error: Android unsigned handoff digest mismatch\n' >&2
    exit 1
}
if "$apksigner_bin" verify "$unsigned_apk" >/dev/null 2>&1; then
    printf 'error: signing handoff APK is already signed\n' >&2
    exit 1
fi
if unzip -Z1 "$unsigned_aab" | grep -Eq '^META-INF/[^/]+\.(RSA|DSA|EC|SF)$'; then
    printf 'error: signing handoff AAB is already signed\n' >&2
    exit 1
fi

work_root=${RUNNER_TEMP:-${TMPDIR:-/tmp}}
temp_dir=$(mktemp -d "$work_root/openpencil-android-sign.XXXXXX")
keystore_path=$temp_dir/release.keystore
store_password_file=$temp_dir/store-password
key_password_file=$temp_dir/key-password
certificate_der=$temp_dir/release-certificate.der
aab_certificate_pem=$temp_dir/aab-certificate.pem
aab_certificate_der=$temp_dir/aab-certificate.der
aligned_apk=$temp_dir/aligned.apk
cleanup() {
    rm -rf "$temp_dir"
}
trap cleanup EXIT

printf '%s' "$keystore_base64" | base64 --decode > "$keystore_path"
require_regular_file "$keystore_path"
[[ -s "$keystore_path" ]] || { printf 'error: decoded Android keystore is empty\n' >&2; exit 1; }
printf '%s' "$keystore_password" > "$store_password_file"
printf '%s' "$key_password" > "$key_password_file"
"$keytool_bin" -exportcert \
    -alias "$key_alias" \
    -keystore "$keystore_path" \
    -storepass:file "$store_password_file" \
    -file "$certificate_der"
require_regular_file "$certificate_der"
actual_certificate_sha256=$(sha256_file "$certificate_der")
[[ "$actual_certificate_sha256" == "$expected_certificate_sha256" ]] || {
    printf 'error: Android release certificate fingerprint does not match the protected value\n' >&2
    exit 1
}

mkdir -p "$ANDROID_SIGNED_OUTPUT_DIR"
signed_apk=$ANDROID_SIGNED_OUTPUT_DIR/OpenPencil-$ANDROID_RELEASE_VERSION-android.apk
signed_aab=$ANDROID_SIGNED_OUTPUT_DIR/OpenPencil-$ANDROID_RELEASE_VERSION-android.aab

"$zipalign_bin" -P 16 -f -v 4 "$unsigned_apk" "$aligned_apk" >/dev/null
"$apksigner_bin" sign \
    --ks "$keystore_path" \
    --ks-key-alias "$key_alias" \
    --ks-pass "file:$store_password_file" \
    --key-pass "file:$key_password_file" \
    --out "$signed_apk" \
    "$aligned_apk"
require_regular_file "$signed_apk"
apk_verification=$("$apksigner_bin" verify --verbose --print-certs "$signed_apk")
printf '%s\n' "$apk_verification"
signer_count=$(grep -Ec '^Signer #[0-9]+ certificate SHA-256 digest:' \
    <<< "$apk_verification")
[[ "$signer_count" -eq 1 ]] || {
    printf 'error: signed APK must have exactly one signer\n' >&2
    exit 1
}
apk_certificate_sha256=$(sed -n \
    's/^Signer #1 certificate SHA-256 digest: //p' <<< "$apk_verification")
apk_certificate_sha256=${apk_certificate_sha256//:/}
apk_certificate_sha256=${apk_certificate_sha256,,}
[[ "$apk_certificate_sha256" == "$expected_certificate_sha256" ]] || {
    printf 'error: APK signer fingerprint does not match the protected value\n' >&2
    exit 1
}
"$zipalign_bin" -c -P 16 -v 4 "$signed_apk" >/dev/null

"$jarsigner_bin" \
    -keystore "$keystore_path" \
    -storepass:file "$store_password_file" \
    -keypass:file "$key_password_file" \
    -digestalg SHA-256 \
    -signedjar "$signed_aab" \
    "$unsigned_aab" "$key_alias"
require_regular_file "$signed_aab"
"$jarsigner_bin" -verify -verbose -certs "$signed_aab" \
    | grep -Fq 'jar verified.' || {
    printf 'error: signed Android App Bundle failed JAR verification\n' >&2
    exit 1
}
"$keytool_bin" -printcert -rfc -jarfile "$signed_aab" > "$aab_certificate_pem"
openssl x509 -in "$aab_certificate_pem" -outform DER \
    -out "$aab_certificate_der"
[[ "$(sha256_file "$aab_certificate_der")" == "$expected_certificate_sha256" ]] || {
    printf 'error: AAB signer fingerprint does not match the protected value\n' >&2
    exit 1
}
"$java_bin" -jar "$BUNDLETOOL_JAR" validate --bundle="$signed_aab"
bundle_config=$("$java_bin" -jar "$BUNDLETOOL_JAR" dump config --bundle="$signed_aab")
grep -Fq PAGE_ALIGNMENT_16K <<< "$bundle_config" || {
    printf 'error: signed Android App Bundle lost 16 KB alignment\n' >&2
    exit 1
}

badging=$("$aapt_bin" dump badging "$signed_apk")
[[ "$badging" == *"package: name='tech.zseven.openpencil'"* \
    && "$badging" == *"versionName='$ANDROID_RELEASE_VERSION'"* \
    && "$badging" == *"versionCode='$version_code'"* \
    && "$badging" == *"targetSdkVersion:'36'"* ]] || {
    printf 'error: signed APK metadata differs from the verified handoff\n' >&2
    exit 1
}

arm64_sha=$(unzip -p "$signed_apk" lib/arm64-v8a/libop_engine_jni.so \
    | sha256sum | awk '{ print $1 }')
x86_64_sha=$(unzip -p "$signed_apk" lib/x86_64/libop_engine_jni.so \
    | sha256sum | awk '{ print $1 }')
[[ "$arm64_sha" == "$(manifest_field arm64_v8a_sha256 "$manifest")" \
    && "$x86_64_sha" == "$(manifest_field x86_64_sha256 "$manifest")" ]] || {
    printf 'error: signed APK native libraries differ from the verified build\n' >&2
    exit 1
}

(
    cd "$ANDROID_SIGNED_OUTPUT_DIR"
    sha256sum -- "$(basename "$signed_apk")" "$(basename "$signed_aab")" \
        > SHA256SUMS.android.txt
)
chmod 0644 "$ANDROID_SIGNED_OUTPUT_DIR"/*

printf 'sign-android-release.sh: signed and verified Android %s artifacts.\n' \
    "$ANDROID_RELEASE_VERSION"
