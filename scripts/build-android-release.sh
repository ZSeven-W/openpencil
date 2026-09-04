#!/usr/bin/env bash
# Build an unsigned Android APK/AAB from trusted source plus the reviewed
# signed auth matrix. Release signing happens on a fresh runner.

set -euo pipefail

relay_bootstrap_cn=${OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_CN:-}
relay_bootstrap_global=${OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_GLOBAL:-}
export -n relay_bootstrap_cn relay_bootstrap_global
unset OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_CN
unset OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_GLOBAL
umask 077

script_dir=$(CDPATH='' cd "$(dirname "$0")" && pwd)
repo_root=$(CDPATH='' cd "$script_dir/.." && pwd)
cd "$repo_root"

usage() {
    printf '%s\n' \
        'usage: scripts/build-android-release.sh' \
        '' \
        'Required environment:' \
        '  OP_AUTH_ARTIFACT_ROOT' \
        '  OPENPENCIL_RELEASE_SOURCE_SHA' \
        '  ANDROID_NDK_HOME, ANDROID_HOME' \
        '  ANDROID_SKIA_AARCH64_BINARIES_URL' \
        '  ANDROID_SKIA_X86_64_BINARIES_URL' \
        '  ANDROID_UNSIGNED_OUTPUT_DIR' \
        '  BUNDLETOOL_JAR' \
        '  OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_CN' \
        '  OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_GLOBAL'
}

[[ "$#" -eq 0 ]] || { usage >&2; exit 2; }

require_env() {
    [[ -n "${!1:-}" ]] || {
        printf 'error: required environment variable is missing: %s\n' "$1" >&2
        exit 2
    }
}

require_regular_file() {
    [[ -f "$1" && ! -L "$1" ]] || {
        printf 'error: required regular non-symlink file is missing: %s\n' "$1" >&2
        exit 1
    }
}

require_command() {
    command -v "$1" >/dev/null 2>&1 || {
        printf 'error: required command is unavailable: %s\n' "$1" >&2
        exit 1
    }
}

sha256_file() {
    sha256sum "$1" | awk '{ print $1 }'
}

for name in \
    OP_AUTH_ARTIFACT_ROOT OPENPENCIL_RELEASE_SOURCE_SHA \
    ANDROID_NDK_HOME ANDROID_HOME \
    ANDROID_SKIA_AARCH64_BINARIES_URL \
    ANDROID_SKIA_X86_64_BINARIES_URL \
    ANDROID_UNSIGNED_OUTPUT_DIR BUNDLETOOL_JAR; do
    require_env "$name"
done
[[ -n "$relay_bootstrap_cn" ]] || {
    printf 'error: required secret is missing: OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_CN\n' >&2
    exit 2
}
[[ -n "$relay_bootstrap_global" ]] || {
    printf 'error: required secret is missing: OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_GLOBAL\n' >&2
    exit 2
}
[[ "$OPENPENCIL_RELEASE_SOURCE_SHA" =~ ^[0-9a-f]{40}$ ]] || {
    printf 'error: Android release source must be a full lowercase SHA\n' >&2
    exit 2
}
for absolute in \
    "$OP_AUTH_ARTIFACT_ROOT" "$ANDROID_NDK_HOME" "$ANDROID_HOME" \
    "$ANDROID_UNSIGNED_OUTPUT_DIR" "$BUNDLETOOL_JAR"; do
    [[ "$absolute" == /* ]] || {
        printf 'error: Android release paths must be absolute: %s\n' "$absolute" >&2
        exit 2
    }
done
[[ -d "$OP_AUTH_ARTIFACT_ROOT" && ! -L "$OP_AUTH_ARTIFACT_ROOT" ]] || {
    printf 'error: auth artifact root must be a non-symlink directory\n' >&2
    exit 1
}
[[ ! -e "$ANDROID_UNSIGNED_OUTPUT_DIR" && ! -L "$ANDROID_UNSIGNED_OUTPUT_DIR" ]] || {
    printf 'error: unsigned output directory must not already exist\n' >&2
    exit 1
}
require_regular_file "$BUNDLETOOL_JAR"
[[ -d "$ANDROID_HOME" && ! -L "$ANDROID_HOME" \
    && "$ANDROID_NDK_HOME" == "$ANDROID_HOME/ndk/28.2.13676358" \
    && -d "$ANDROID_NDK_HOME" && ! -L "$ANDROID_NDK_HOME" ]] || {
    printf 'error: Android release must use the pinned SDK and NDK roots\n' >&2
    exit 1
}
sdk_digests=$ANDROID_HOME/VERIFIED-DIGESTS
require_regular_file "$sdk_digests"
expected_sdk_digests=$'android_build_tools_sha256=5d9ac77fb6ff43d9da518a337b4fcf8f9097113df531d99ccefe80ef7ce8250b\nandroid_platform_36_sha256=37607369a28c5b640b3a7998868d45898ebcb777565a0e85f9acf36f29631d2e\nandroid_ndk_r28c_sha256=dfb20d396df28ca02a8c708314b814a4d961dc9074f9a161932746f815aa552f'
[[ "$(< "$sdk_digests")" == "$expected_sdk_digests" ]] || {
    printf 'error: Android SDK digest receipt is invalid\n' >&2
    exit 1
}
require_regular_file "$ANDROID_HOME/platforms/android-36/source.properties"
grep -Eq '^AndroidVersion\.ApiLevel=36$' \
    "$ANDROID_HOME/platforms/android-36/source.properties" || {
    printf 'error: Android Release requires the reviewed API 36 platform\n' >&2
    exit 1
}
[[ -z ${FORCE_SKIA_BINARIES_DOWNLOAD+x} ]] || {
    printf 'error: FORCE_SKIA_BINARIES_DOWNLOAD must be unset\n' >&2
    exit 2
}

for command in \
    awk cargo find grep java python3 readlink ruby rustup sed sha256sum unzip xxd; do
    require_command "$command"
done
[[ "$(uname -s)" == Linux && "$(uname -m)" == x86_64 ]] || {
    printf 'error: production Android builds require the reviewed Linux x86_64 runner\n' >&2
    exit 2
}

printf '%s\0%s\0' "$relay_bootstrap_cn" "$relay_bootstrap_global" \
    | python3 "$repo_root/tools/check-collab-bootstrap-urls.py"

source_properties=$ANDROID_NDK_HOME/source.properties
require_regular_file "$source_properties"
grep -Eq '^Pkg\.Revision[[:space:]]*=[[:space:]]*28\.2\.13676358$' \
    "$source_properties" || {
    printf 'error: Android Release requires NDK 28.2.13676358\n' >&2
    exit 1
}
ndk_tools=$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin
readelf_bin=$ndk_tools/llvm-readelf

require_ndk_tool() {
    local tool=$1 resolved
    [[ -x "$tool" ]] || {
        printf 'error: required NDK tool is not executable: %s\n' "$tool" >&2
        exit 1
    }
    resolved=$(readlink -f "$tool")
    [[ "$resolved" == "$ndk_tools/"* ]] || {
        printf 'error: NDK tool resolves outside the pinned toolchain: %s\n' "$tool" >&2
        exit 1
    }
}

require_ndk_tool "$readelf_bin"
build_tools=$ANDROID_HOME/build-tools/36.0.0
aapt_bin=$build_tools/aapt
apksigner_bin=$build_tools/apksigner
zipalign_bin=$build_tools/zipalign
for tool in "$aapt_bin" "$apksigner_bin" "$zipalign_bin"; do
    require_regular_file "$tool"
    [[ -x "$tool" ]] || { printf 'error: Android build tool is not executable: %s\n' "$tool" >&2; exit 1; }
done

"$repo_root/tools/pinned-release-tools.sh" verify-skia android \
    aarch64-linux-android "$ANDROID_SKIA_AARCH64_BINARIES_URL"
"$repo_root/tools/pinned-release-tools.sh" verify-skia android \
    x86_64-linux-android "$ANDROID_SKIA_X86_64_BINARIES_URL"

canonical_prebuilt=$repo_root/crates/op-auth-bridge/prebuilt
release_jni=$repo_root/packaging/android/app/src/release/jniLibs
[[ ! -e "$release_jni" && ! -L "$release_jni" ]] || {
    printf 'error: refusing to replace an existing Android Release jniLibs directory\n' >&2
    exit 1
}
work_root=${RUNNER_TEMP:-${TMPDIR:-/tmp}}
temp_dir=$(mktemp -d "$work_root/openpencil-android-build.XXXXXX")
staged_prebuilt=$temp_dir/prebuilt
canonical_backups=()
canonical_installed=()

cleanup() {
    if [[ -d "$release_jni" && ! -L "$release_jni" ]]; then
        rm -rf "$release_jni"
    fi
    local index target canonical backup
    for ((index=${#canonical_installed[@]}-1; index>=0; index--)); do
        target=${canonical_installed[$index]}
        canonical=$canonical_prebuilt/$target
        rm -rf "$canonical"
    done
    for target in "${canonical_backups[@]}"; do
        canonical=$canonical_prebuilt/$target
        backup=$temp_dir/original-$target
        mv "$backup" "$canonical"
    done
    rm -rf "$temp_dir"
}
trap cleanup EXIT

source "$repo_root/tools/op-auth-candidate-targets.sh"
mkdir -p "$staged_prebuilt"
trusted_public_key=$canonical_prebuilt/PROVENANCE_PUBKEY
require_regular_file "$trusted_public_key"
cp -p "$trusted_public_key" "$staged_prebuilt/PROVENANCE_PUBKEY"
for matrix_file in RELEASE-MANIFEST RELEASE-MANIFEST.sig; do
    require_regular_file "$OP_AUTH_ARTIFACT_ROOT/$matrix_file"
    cp -p "$OP_AUTH_ARTIFACT_ROOT/$matrix_file" "$staged_prebuilt/$matrix_file"
done

while IFS= read -r target; do
    source_dir=$OP_AUTH_ARTIFACT_ROOT/$target
    destination=$staged_prebuilt/$target
    [[ -d "$source_dir" && ! -L "$source_dir" ]] || {
        printf 'error: auth artifact target is missing: %s\n' "$target" >&2
        exit 1
    }
    artifact_name=$(op_auth_candidate_artifact_name "$target")
    expected_files=$(printf '%s\n' \
        ABI_VERSION HARDENING-ATTESTATION PROVENANCE PROVENANCE.sig \
        SHA256 VERSION "$artifact_name" | LC_ALL=C sort)
    actual_files=$(find "$source_dir" -mindepth 1 -maxdepth 1 -print \
        | sed 's#^.*/##' | LC_ALL=C sort)
    [[ "$actual_files" == "$expected_files" ]] || {
        printf 'error: auth target contains missing or unexpected files: %s\n' "$target" >&2
        exit 1
    }
    mkdir -p "$destination"
    while IFS= read -r file; do
        require_regular_file "$source_dir/$file"
        cp -p "$source_dir/$file" "$destination/$file"
    done <<< "$expected_files"
done < <(op_auth_candidate_targets)

workspace_version=$("$repo_root/scripts/workspace-version.sh")
OP_AUTH_PREBUILT_ROOT=$staged_prebuilt \
    "$repo_root/tools/check-op-auth-release-matrix.sh"
OP_AUTH_PREBUILT_ROOT=$staged_prebuilt \
    "$repo_root/tools/check-op-auth-prebuilt.sh" --require-hardened

for target in aarch64-linux-android x86_64-linux-android; do
    canonical=$canonical_prebuilt/$target
    if [[ -e "$canonical" || -L "$canonical" ]]; then
        [[ -d "$canonical" && ! -L "$canonical" ]] || {
            printf 'error: canonical Android auth target is not a regular directory\n' >&2
            exit 1
        }
        mv "$canonical" "$temp_dir/original-$target"
        canonical_backups+=("$target")
    fi
    cp -R "$staged_prebuilt/$target" "$canonical"
    canonical_installed+=("$target")
    CONFIGURATION=Release \
    OP_AUTH_ARCHIVE=$canonical/libop_auth.a \
    OP_AUTH_TARGET=$target \
        "$repo_root/tools/check-mobile-auth-link-input.sh"
done

mkdir -p "$release_jni"

verify_elf_16k() {
    local binary=$1 count=0 alignment
    while IFS= read -r alignment; do
        [[ "$alignment" =~ ^0x[0-9a-fA-F]+$ ]] || {
            printf 'error: could not parse ELF LOAD alignment for %s\n' "$binary" >&2
            exit 1
        }
        ((count += 1))
        (( alignment >= 0x4000 )) || {
            printf 'error: ELF LOAD segment is not 16 KB aligned: %s (%s)\n' \
                "$binary" "$alignment" >&2
            exit 1
        }
    done < <("$readelf_bin" -lW "$binary" | awk '$1 == "LOAD" { print $NF }')
    (( count > 0 )) || {
        printf 'error: Android JNI library has no ELF LOAD segments: %s\n' "$binary" >&2
        exit 1
    }
}

build_target() {
    local target=$1 abi=$2 skia_url=$3 clang_prefix=$4
    local env_target=${target//-/_}
    local cargo_env_target=${env_target^^}
    local cc=$ndk_tools/${clang_prefix}26-clang
    local cxx=$ndk_tools/${clang_prefix}26-clang++
    local so=$repo_root/target/$target/release/libop_engine_jni.so
    require_ndk_tool "$cc"
    require_ndk_tool "$cxx"
    require_ndk_tool "$ndk_tools/llvm-ar"
    cargo clean -p skia-bindings --target "$target" --release
    env \
        "CC_$env_target=$cc" \
        "CXX_$env_target=$cxx" \
        "AR_$env_target=$ndk_tools/llvm-ar" \
        "CARGO_TARGET_${cargo_env_target}_LINKER=$cc" \
        "CARGO_TARGET_${cargo_env_target}_AR=$ndk_tools/llvm-ar" \
        "BINDGEN_EXTRA_CLANG_ARGS_$env_target=--target=${clang_prefix}26 --sysroot=$ndk_tools/../sysroot" \
        RUSTUP_TOOLCHAIN=1.94 \
        SKIA_BINARIES_URL="$skia_url" \
        OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_CN="$relay_bootstrap_cn" \
        OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_GLOBAL="$relay_bootstrap_global" \
        cargo build --locked --release -p op-engine-jni --target "$target" \
            --features gl,editor,pinned-skia-binaries
    OP_AUTH_CARGO_TARGET=$target \
        "$repo_root/tools/check-op-auth-cargo-build.sh"
    require_regular_file "$so"
    verify_elf_16k "$so"
    LC_ALL=C grep -aFq "$relay_bootstrap_cn" "$so" || {
        printf 'error: CN collaboration bootstrap URL is missing from %s\n' "$target" >&2
        exit 1
    }
    LC_ALL=C grep -aFq "$relay_bootstrap_global" "$so" || {
        printf 'error: global collaboration bootstrap URL is missing from %s\n' "$target" >&2
        exit 1
    }
    mkdir -p "$release_jni/$abi"
    install -m 0755 "$so" "$release_jni/$abi/libop_engine_jni.so"
}

build_target \
    aarch64-linux-android arm64-v8a \
    "$ANDROID_SKIA_AARCH64_BINARIES_URL" aarch64-linux-android
build_target \
    x86_64-linux-android x86_64 \
    "$ANDROID_SKIA_X86_64_BINARIES_URL" x86_64-linux-android

cd "$repo_root"
for test_file in packaging/android/Tests/*.rb; do
    ruby "$test_file"
done
(
    cd packaging/android
    ./gradlew --no-daemon --console=plain --dependency-verification strict \
        :app:testDebugUnitTest :app:lintRelease \
        :app:assembleRelease :app:bundleRelease
)

unsigned_apk=$repo_root/packaging/android/app/build/outputs/apk/release/app-release-unsigned.apk
unsigned_aab=$repo_root/packaging/android/app/build/outputs/bundle/release/app-release.aab
require_regular_file "$unsigned_apk"
require_regular_file "$unsigned_aab"
if "$apksigner_bin" verify "$unsigned_apk" >/dev/null 2>&1; then
    printf 'error: build runner unexpectedly produced a signed APK\n' >&2
    exit 1
fi
if unzip -Z1 "$unsigned_aab" | grep -Eq '^META-INF/[^/]+\.(RSA|DSA|EC|SF)$'; then
    printf 'error: build runner unexpectedly produced a signed AAB\n' >&2
    exit 1
fi
"$zipalign_bin" -c -P 16 -v 4 "$unsigned_apk" >/dev/null
java -jar "$BUNDLETOOL_JAR" validate --bundle="$unsigned_aab"
bundle_config=$(java -jar "$BUNDLETOOL_JAR" dump config --bundle="$unsigned_aab")
grep -Fq PAGE_ALIGNMENT_16K <<< "$bundle_config" || {
    printf 'error: Android App Bundle does not request 16 KB page alignment\n' >&2
    exit 1
}

badging=$("$aapt_bin" dump badging "$unsigned_apk")
grep -Fq "package: name='tech.zseven.openpencil'" <<< "$badging" || {
    printf 'error: Android package id does not match the production application\n' >&2
    exit 1
}
grep -Fq "targetSdkVersion:'36'" <<< "$badging" || {
    printf 'error: Android release must target API 36\n' >&2
    exit 1
}
version_metadata=$("$repo_root/scripts/android-version.sh")
version_code=$(sed -n 's/^versionCode=//p' <<< "$version_metadata")
[[ "$badging" == *"versionName='$workspace_version'"* \
    && "$badging" == *"versionCode='$version_code'"* ]] || {
    printf 'error: packaged Android version does not match the workspace\n' >&2
    exit 1
}

mkdir -p "$ANDROID_UNSIGNED_OUTPUT_DIR"
apk_name=OpenPencil-$workspace_version-android-unsigned.apk
aab_name=OpenPencil-$workspace_version-android-unsigned.aab
install -m 0644 "$unsigned_apk" "$ANDROID_UNSIGNED_OUTPUT_DIR/$apk_name"
install -m 0644 "$unsigned_aab" "$ANDROID_UNSIGNED_OUTPUT_DIR/$aab_name"
apk_sha=$(sha256_file "$ANDROID_UNSIGNED_OUTPUT_DIR/$apk_name")
aab_sha=$(sha256_file "$ANDROID_UNSIGNED_OUTPUT_DIR/$aab_name")
arm64_sha=$(sha256_file "$release_jni/arm64-v8a/libop_engine_jni.so")
x86_64_sha=$(sha256_file "$release_jni/x86_64/libop_engine_jni.so")
auth_matrix_sha=$(sha256_file "$staged_prebuilt/RELEASE-MANIFEST")
cat > "$ANDROID_UNSIGNED_OUTPUT_DIR/ANDROID-RELEASE-MANIFEST" <<EOF
format=openpencil-android-unsigned-v2
application_id=tech.zseven.openpencil
version=$workspace_version
version_code=$version_code
target_sdk=36
release_source_revision=$OPENPENCIL_RELEASE_SOURCE_SHA
auth_matrix_sha256=$auth_matrix_sha
apk=$apk_name
apk_sha256=$apk_sha
aab=$aab_name
aab_sha256=$aab_sha
arm64_v8a_sha256=$arm64_sha
x86_64_sha256=$x86_64_sha
EOF
chmod 0644 "$ANDROID_UNSIGNED_OUTPUT_DIR/ANDROID-RELEASE-MANIFEST"

printf 'build-android-release.sh: created verified unsigned handoff for %s.\n' \
    "$workspace_version"
