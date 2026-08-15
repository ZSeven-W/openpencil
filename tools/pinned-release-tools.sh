#!/usr/bin/env bash
# Download release build tools only from immutable release URLs and verify their
# reviewed SHA-256 digests before extraction or execution.

set -euo pipefail

sha256_file() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | awk '{print $1}'
    else
        shasum -a 256 "$1" | awk '{print $1}'
    fi
}

verify_sha256() {
    local expected=$1 file=$2 actual
    [[ $expected =~ ^[0-9a-f]{64}$ ]] || {
        printf 'error: malformed pinned SHA-256\n' >&2
        return 1
    }
    [[ -f $file && ! -L $file ]] || {
        printf 'error: downloaded asset is not a regular file: %s\n' "$file" >&2
        return 1
    }
    actual=$(sha256_file "$file")
    [[ $actual == "$expected" ]] || {
        printf 'error: SHA-256 mismatch for %s\n' "$file" >&2
        return 1
    }
}

skia_file_url() {
    local platform=$1 path=$2
    case $platform in
        Windows)
            [[ $path =~ ^[A-Za-z]:/ ]] || {
                printf 'error: Windows Skia cache path is not drive-absolute\n' >&2
                return 1
            }
            # skia-bindings 0.97.2 strips file:// and calls fs::read directly.
            printf 'file://%s\n' "$path"
            ;;
        *)
            [[ $path == /* ]] || {
                printf 'error: Skia cache path is not absolute\n' >&2
                return 1
            }
            printf 'file://%s\n' "$path"
            ;;
    esac
}

reviewed_skia_archive() {
    local profile=$1 target=$2
    local repository_hash=da8fc6731fc439bc3b6a
    local features expected key
    case $profile in
        desktop)
            features=gl-jpegd-jpege-pdf-textlayout
            case $target in
                aarch64-apple-darwin) expected=c4c5d5059ab9226aaf3d5337a8fd42ef0e42e9fe3cbc3c8da4310b4a3a1e4254 ;;
                x86_64-apple-darwin) expected=fe92e66916947a4d666a24d0580434f42585853d221d2af006a52a72b55b283b ;;
                aarch64-unknown-linux-gnu) expected=2587dcaf11aab680ef8637d4192fc77a507c91e3a88bebb79d7993a4fefa1d1b ;;
                x86_64-unknown-linux-gnu) expected=ee77fbd0183e854e297276705e4e8685837c6c7d0304472c97145fcd8f7f2cfc ;;
                aarch64-pc-windows-msvc) expected=20ba7acf5e306b6d875863c838cb9d3c4a39a05792fb6256a3f03ddcbc1077a1 ;;
                x86_64-pc-windows-msvc) expected=6b61061c32fb7a72944e3dae63d97241271b1ac7bcaf3752cfa0c79ed37ee8b6 ;;
                *)
                    printf 'error: no reviewed desktop Skia archive for %s\n' "$target" >&2
                    return 1
                    ;;
            esac
            ;;
        web)
            [[ $target == x86_64-unknown-linux-gnu ]] || {
                printf 'error: the reviewed web Skia archive is Linux x86_64 only\n' >&2
                return 1
            }
            features=jpegd-jpege-pdf-textlayout
            expected=c066658b13e257d418f647447d06eb8a83cb060b037228da838589dd863bf053
            ;;
        ios)
            [[ $target == aarch64-apple-ios ]] || {
                printf 'error: the reviewed iOS Skia archive is device ARM64 only\n' >&2
                return 1
            }
            features=jpegd-jpege-metal-pdf-textlayout
            expected=4abbaea5e4e8934a6f19c5de44eaba9bf9238af4abbe57dbac5f2dc03923b182
            ;;
        android)
            features=gl-jpegd-jpege-pdf-textlayout
            case $target in
                aarch64-linux-android) expected=82ca6dd1720bbe8b105c12c4d0c78786d2c792e9d2a7f2102ab66bb24dafa9d0 ;;
                x86_64-linux-android) expected=ca217df6ffced17381cbea4df044969a493a46bddc757ee844e2fbaf54fa1257 ;;
                *)
                    printf 'error: no reviewed Android Skia archive for %s\n' "$target" >&2
                    return 1
                    ;;
            esac
            ;;
        *)
            printf 'error: unsupported Skia release profile: %s\n' "$profile" >&2
            return 1
            ;;
    esac
    key=$repository_hash-$target-$features
    printf '%s %s\n' "$key" "$expected"
}

verify_staged_skia_archive() {
    local profile=$1 target=$2 url_template=$3
    local metadata key expected archive_suffix remainder archive_url archive
    [[ $profile == desktop || $profile == android ]] || {
        printf 'error: staged release verification requires the desktop or Android Skia profile\n' >&2
        return 1
    }
    [[ -z ${FORCE_SKIA_BINARIES_DOWNLOAD+x} ]] || {
        printf 'error: FORCE_SKIA_BINARIES_DOWNLOAD must be unset for a staged release archive\n' >&2
        return 1
    }
    metadata=$(reviewed_skia_archive "$profile" "$target") || return 1
    read -r key expected <<< "$metadata"

    archive_suffix='/skia-binaries-{key}.tar.gz'
    [[ $url_template == *'{key}'* ]] || {
        printf 'error: staged Skia URL must contain the {key} placeholder\n' >&2
        return 1
    }
    remainder=${url_template#*'{key}'}
    [[ $remainder != *'{key}'* && $url_template == *"$archive_suffix" ]] || {
        printf 'error: staged Skia URL does not have the reviewed archive template shape\n' >&2
        return 1
    }
    archive_url=${url_template/\{key\}/$key}
    case ${RUNNER_OS:-} in
        Windows)
            [[ $archive_url =~ ^file://[A-Za-z]:/ ]] || {
                printf 'error: staged Windows Skia URL must use file://C:/... form\n' >&2
                return 1
            }
            ;;
        *)
            [[ $archive_url == file:///* ]] || {
                printf 'error: staged Skia URL must use an absolute file:///... path\n' >&2
                return 1
            }
            ;;
    esac
    # skia-bindings 0.97.2 strips this exact prefix and passes the remainder to
    # std::fs::read, including a Windows drive path such as C:/runner/cache/....
    archive=${archive_url#file://}
    verify_sha256 "$expected" "$archive"
}

download_verified() {
    local url=$1 expected=$2 output=$3 output_dir temporary
    case $url in
        https://github.com/*|https://dl.google.com/android/repository/*) ;;
        *)
            printf 'error: release tool URL is not on an approved HTTPS origin\n' >&2
            return 1
            ;;
    esac
    output_dir=${output%/*}
    [[ $output_dir != "$output" ]] || output_dir=.
    mkdir -p "$output_dir"
    temporary=$(mktemp "$output_dir/.release-tool.XXXXXX")
    if ! curl --fail --location --proto '=https' --tlsv1.2 \
        --retry 3 --silent --show-error "$url" --output "$temporary"; then
        rm -f "$temporary"
        return 1
    fi
    if ! verify_sha256 "$expected" "$temporary"; then
        rm -f "$temporary"
        return 1
    fi
    mv -f "$temporary" "$output"
}

expected_cargo_cli_version() {
    case $1 in
        cargo-bundle) printf 'cargo-bundle v0.10.0\n' ;;
        wasm-bindgen-cli) printf 'wasm-bindgen 0.2.117\n' ;;
        *) return 1 ;;
    esac
}

verify_pinned_cargo_cli_binary() {
    local tool=$1 binary=$2 expected actual
    expected=$(expected_cargo_cli_version "$tool") || return 1
    actual=$("$binary" --version) || return 1
    [[ $actual == "$expected" ]] || {
        printf 'error: installed %s reports an unexpected version: %s\n' \
            "$tool" "$actual" >&2
        return 1
    }
}

verify_pinned_ripgrep_binary() {
    local binary=$1 actual first_line
    actual=$("$binary" --version) || return 1
    first_line=${actual%%$'\n'*}
    [[ $first_line == 'ripgrep 15.2.0' ]] || {
        printf 'error: installed ripgrep reports an unexpected version: %s\n' \
            "$first_line" >&2
        return 1
    }
}

install_pinned_cargo_cli() {
    local tool=$1 install_root=$2
    local crate_name version expected binary
    local temporary archive source_dir member listing
    case $tool in
        cargo-bundle)
            crate_name=cargo-bundle
            version=0.10.0
            expected=41058f8f2967385b2799764c2c281fd143392ef82221d5ffde0481a1cdbfc40e
            binary=cargo-bundle
            ;;
        wasm-bindgen-cli)
            crate_name=wasm-bindgen-cli
            version=0.2.117
            expected=bb3601b2899d4887512bdcaad115074750be7c212b122fa7ed4faed6c919229e
            binary=wasm-bindgen
            ;;
        *)
            printf 'error: unsupported pinned Cargo CLI: %s\n' "$tool" >&2
            exit 1
            ;;
    esac
    [[ ! -e $install_root && ! -L $install_root ]] || {
        printf 'error: refusing to replace existing Cargo CLI root: %s\n' "$install_root" >&2
        exit 1
    }
    command -v cargo >/dev/null 2>&1 || {
        printf 'error: cargo is required to install %s\n' "$tool" >&2
        exit 1
    }

    temporary=$(mktemp -d)
    archive=$temporary/$crate_name.crate
    source_dir=$temporary/$crate_name-$version
    if ! curl --fail --location --proto '=https' --tlsv1.2 --retry 3 \
        --silent --show-error \
        "https://static.crates.io/crates/$crate_name/$crate_name-$version.crate" \
        --output "$archive"; then
        rm -rf "$temporary"
        exit 1
    fi
    if ! verify_sha256 "$expected" "$archive"; then
        rm -rf "$temporary"
        exit 1
    fi
    while IFS= read -r member; do
        [[ $member == "$crate_name-$version/"* \
            && $member != *'../'* && $member != /* && $member != *"\\"* ]] || {
            printf 'error: unsafe %s crate member: %s\n' "$tool" "$member" >&2
            rm -rf "$temporary"
            exit 1
        }
    done < <(tar -tzf "$archive")
    while IFS= read -r listing; do
        case ${listing:0:1} in
            -|d) ;;
            *)
                printf 'error: %s crate contains a non-regular member\n' "$tool" >&2
                rm -rf "$temporary"
                exit 1
                ;;
        esac
    done < <(tar -tvzf "$archive")
    tar -xzf "$archive" -C "$temporary"
    [[ -f $source_dir/Cargo.toml && ! -L $source_dir/Cargo.toml \
        && -f $source_dir/Cargo.lock && ! -L $source_dir/Cargo.lock ]] || {
        printf 'error: verified %s crate lacks its locked manifest\n' "$tool" >&2
        rm -rf "$temporary"
        exit 1
    }
    mkdir -p "$install_root"
    if ! CARGO_HOME=$temporary/cargo-home cargo install \
        --path "$source_dir" --locked --root "$install_root" --force; then
        rm -rf "$install_root" "$temporary"
        exit 1
    fi
    [[ -f $install_root/bin/$binary && ! -L $install_root/bin/$binary \
        && -x $install_root/bin/$binary ]] || {
        printf 'error: verified %s installation lacks its primary binary\n' "$tool" >&2
        rm -rf "$install_root" "$temporary"
        exit 1
    }
    if ! verify_pinned_cargo_cli_binary "$tool" "$install_root/bin/$binary"; then
        rm -rf "$install_root" "$temporary"
        exit 1
    fi
    rm -rf "$temporary"
    if [[ -n ${GITHUB_PATH:-} ]]; then
        printf '%s\n' "$install_root/bin" >> "$GITHUB_PATH"
        if [[ $tool == cargo-bundle && -n ${GITHUB_ENV:-} ]]; then
            printf 'CARGO_BUNDLE_HOME=%s\n' "$install_root" >> "$GITHUB_ENV"
        fi
    else
        printf '%s\n' "$install_root/bin"
    fi
}

self_test() {
    local test_dir test_file good_cli bad_cli good_rg bad_rg
    test_dir=$(mktemp -d)
    test_file=$test_dir/payload
    printf test > "$test_file"
    verify_sha256 \
        9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08 \
        "$test_file"
    if verify_sha256 \
        0000000000000000000000000000000000000000000000000000000000000000 \
        "$test_file" 2>/dev/null; then
        printf 'error: checksum mismatch was accepted\n' >&2
        exit 1
    fi
    [[ $(skia_file_url Windows C:/runner/temp) == file://C:/runner/temp ]]
    [[ $(skia_file_url Linux /runner/temp) == file:///runner/temp ]]
    if skia_file_url Windows /C:/runner/temp >/dev/null 2>&1; then
        printf 'error: invalid Windows Skia file URL path was accepted\n' >&2
        exit 1
    fi
    good_cli=$test_dir/good-cargo-bundle
    bad_cli=$test_dir/bad-cargo-bundle
    good_rg=$test_dir/good-rg
    bad_rg=$test_dir/bad-rg
    printf '#!/bin/sh\nprintf "cargo-bundle v0.10.0\\n"\n' > "$good_cli"
    printf '#!/bin/sh\nprintf "cargo-bundle 0.10.0\\n"\n' > "$bad_cli"
    printf '#!/bin/sh\nprintf "ripgrep 15.2.0\\nfeatures:+pcre2\\n"\n' > "$good_rg"
    printf '#!/bin/sh\nprintf "ripgrep 15.1.0\\nfeatures:+pcre2\\n"\n' > "$bad_rg"
    chmod 0755 "$good_cli" "$bad_cli" "$good_rg" "$bad_rg"
    verify_pinned_cargo_cli_binary cargo-bundle "$good_cli"
    if verify_pinned_cargo_cli_binary cargo-bundle "$bad_cli" >/dev/null 2>&1; then
        printf 'error: cargo-bundle version output without v was accepted\n' >&2
        exit 1
    fi
    verify_pinned_ripgrep_binary "$good_rg"
    if verify_pinned_ripgrep_binary "$bad_rg" >/dev/null 2>&1; then
        printf 'error: an unexpected ripgrep version was accepted\n' >&2
        exit 1
    fi
    rm -rf "$test_dir"
    printf 'pinned-release-tools.sh: checksum rejection self-test passed.\n'
}

install_binaryen() {
    local install_root=$1 version=version_123
    local expected=e959f2170af4c20c552e9de3a0253704d6a9d2766e8fdb88e4d6ac4bae9388fe
    local temporary archive destination member
    [[ -d $install_root && ! -L $install_root ]] || {
        printf 'error: Binaryen install root must be an existing regular directory\n' >&2
        exit 1
    }
    temporary=$(mktemp -d)
    archive=$temporary/binaryen.tar.gz
    destination=$install_root/binaryen-$version
    [[ ! -e $destination && ! -L $destination ]] || {
        printf 'error: refusing to replace existing Binaryen directory: %s\n' "$destination" >&2
        exit 1
    }
    download_verified \
        "https://github.com/WebAssembly/binaryen/releases/download/$version/binaryen-$version-x86_64-linux.tar.gz" \
        "$expected" "$archive"
    while IFS= read -r member; do
        [[ $member == "binaryen-$version/"* && $member != *'../'* && $member != /* ]] || {
            printf 'error: unsafe Binaryen archive member: %s\n' "$member" >&2
            exit 1
        }
    done < <(tar -tzf "$archive")
    tar -xzf "$archive" -C "$install_root"
    [[ -x $destination/bin/wasm-opt && ! -L $destination/bin/wasm-opt ]] || {
        printf 'error: verified Binaryen archive lacks wasm-opt\n' >&2
        exit 1
    }
    rm -rf "$temporary"
    "$destination/bin/wasm-opt" --version
    if [[ -n ${GITHUB_PATH:-} ]]; then
        printf '%s\n' "$destination/bin" >> "$GITHUB_PATH"
    else
        printf '%s\n' "$destination/bin"
    fi
}

install_bundletool() {
    local install_root=$1
    local version=1.18.3
    local expected=a099cfa1543f55593bc2ed16a70a7c67fe54b1747bb7301f37fdfd6d91028e29
    local archive actual
    [[ ! -e $install_root && ! -L $install_root ]] || {
        printf 'error: refusing to replace existing bundletool root: %s\n' "$install_root" >&2
        exit 1
    }
    command -v java >/dev/null 2>&1 || {
        printf 'error: Java is required to validate bundletool\n' >&2
        exit 1
    }
    archive=$install_root/bundletool-all-$version.jar
    download_verified \
        "https://github.com/google/bundletool/releases/download/$version/bundletool-all-$version.jar" \
        "$expected" "$archive"
    actual=$(java -jar "$archive" version)
    [[ $actual == "$version" ]] || {
        printf 'error: bundletool reports an unexpected version: %s\n' "$actual" >&2
        exit 1
    }
    printf '%s\n' "$archive"
}

install_android_signing_tools() {
    local install_root=$1 temporary build_tools_archive jdk_archive
    local build_tools_root jdk_root bundletool_jar actual member resolved
    local build_tools_sha=5d9ac77fb6ff43d9da518a337b4fcf8f9097113df531d99ccefe80ef7ce8250b
    local jdk_sha=f2dc5418092c43003db8f9005c4a286e1c0104fea96ccdd49e8ebd037cac9219
    local bundletool_sha=a099cfa1543f55593bc2ed16a70a7c67fe54b1747bb7301f37fdfd6d91028e29
    [[ $(uname -s) == Linux && $(uname -m) == x86_64 ]] || {
        printf 'error: Android signing tools are pinned for Linux x86_64 only\n' >&2
        exit 1
    }
    [[ ! -e $install_root && ! -L $install_root ]] || {
        printf 'error: refusing to replace existing Android signing tools: %s\n' \
            "$install_root" >&2
        exit 1
    }
    temporary=$(mktemp -d)
    build_tools_archive=$temporary/build-tools.zip
    jdk_archive=$temporary/jdk.tar.gz
    download_verified \
        https://dl.google.com/android/repository/build-tools_r36_linux.zip \
        "$build_tools_sha" "$build_tools_archive"
    download_verified \
        'https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.8%2B9/OpenJDK21U-jdk_x64_linux_hotspot_21.0.8_9.tar.gz' \
        "$jdk_sha" "$jdk_archive"
    while IFS= read -r member; do
        [[ $member == android-16/* && $member != *'../'* \
            && $member != /* && $member != *"\\"* ]] || {
            printf 'error: unsafe Android Build-Tools archive member: %s\n' "$member" >&2
            exit 1
        }
    done < <(unzip -Z1 "$build_tools_archive")
    while IFS= read -r member; do
        [[ $member == jdk-21.0.8+9/* && $member != *'../'* \
            && $member != /* && $member != *"\\"* ]] || {
            printf 'error: unsafe Temurin archive member: %s\n' "$member" >&2
            exit 1
        }
    done < <(tar -tzf "$jdk_archive")
    mkdir -p "$install_root/build-tools" "$install_root/bundletool"
    unzip -q "$build_tools_archive" -d "$temporary/build-tools"
    mv "$temporary/build-tools/android-16" "$install_root/build-tools/36.0.0"
    tar -xzf "$jdk_archive" -C "$install_root"
    build_tools_root=$install_root/build-tools/36.0.0
    jdk_root=$install_root/jdk-21.0.8+9
    while IFS= read -r member; do
        resolved=$(readlink -f "$member")
        [[ $resolved == "$install_root/"* ]] || {
            printf 'error: verified Android signing tool symlink escapes its root\n' >&2
            exit 1
        }
    done < <(find "$install_root" -type l -print)
    for member in aapt apksigner zipalign; do
        [[ -f $build_tools_root/$member && ! -L $build_tools_root/$member \
            && -x $build_tools_root/$member ]] || {
            printf 'error: Android Build-Tools archive lacks %s\n' "$member" >&2
            exit 1
        }
    done
    for member in java jarsigner keytool; do
        [[ -f $jdk_root/bin/$member && ! -L $jdk_root/bin/$member \
            && -x $jdk_root/bin/$member ]] || {
            printf 'error: Temurin archive lacks %s\n' "$member" >&2
            exit 1
        }
    done
    grep -Eq '^Pkg\.Revision[[:space:]]*=[[:space:]]*36\.0\.0$' \
        "$build_tools_root/source.properties"
    actual=$("$jdk_root/bin/java" -version 2>&1 | head -n 1)
    [[ $actual == 'openjdk version "21.0.8"'* ]] || {
        printf 'error: verified Temurin reports an unexpected version: %s\n' "$actual" >&2
        exit 1
    }
    bundletool_jar=$install_root/bundletool/bundletool-all-1.18.3.jar
    download_verified \
        https://github.com/google/bundletool/releases/download/1.18.3/bundletool-all-1.18.3.jar \
        "$bundletool_sha" "$bundletool_jar"
    [[ $("$jdk_root/bin/java" -jar "$bundletool_jar" version) == 1.18.3 ]]
    {
        printf 'android_build_tools_sha256=%s\n' "$build_tools_sha"
        printf 'temurin_jdk_sha256=%s\n' "$jdk_sha"
        printf 'bundletool_sha256=%s\n' "$bundletool_sha"
    } > "$install_root/VERIFIED-DIGESTS"
    chmod 0444 "$install_root/VERIFIED-DIGESTS"
    rm -rf "$temporary"
    if [[ -n ${GITHUB_ENV:-} ]]; then
        {
            printf 'ANDROID_SIGNING_TOOLS_ROOT=%s\n' "$install_root"
            printf 'ANDROID_BUILD_TOOLS_DIR=%s\n' "$build_tools_root"
            printf 'ANDROID_JAVA_HOME=%s\n' "$jdk_root"
            printf 'BUNDLETOOL_JAR=%s\n' "$bundletool_jar"
        } >> "$GITHUB_ENV"
    else
        printf '%s\n' "$install_root"
    fi
}

download_appimage_tools() {
    local architecture=$1 tool_output=$2 runtime_output=$3
    local tool_sha runtime_sha
    case $architecture in
        x86_64)
            tool_sha=ed4ce84f0d9caff66f50bcca6ff6f35aae54ce8135408b3fa33abfc3cb384eb0
            runtime_sha=2fca8b443c92510f1483a883f60061ad09b46b978b2631c807cd873a47ec260d
            ;;
        aarch64)
            tool_sha=f0837e7448a0c1e4e650a93bb3e85802546e60654ef287576f46c71c126a9158
            runtime_sha=00cbdfcf917cc6c0ff6d3347d59e0ca1f7f45a6df1a428a0d6d8a78664d87444
            ;;
        *)
            printf 'error: unsupported AppImage architecture: %s\n' "$architecture" >&2
            exit 1
            ;;
    esac
    [[ $tool_output != "$runtime_output" ]] || {
        printf 'error: AppImage tool and runtime outputs must differ\n' >&2
        exit 1
    }
    download_verified \
        "https://github.com/AppImage/appimagetool/releases/download/1.9.1/appimagetool-$architecture.AppImage" \
        "$tool_sha" "$tool_output"
    download_verified \
        "https://github.com/AppImage/type2-runtime/releases/download/20251108/runtime-$architecture" \
        "$runtime_sha" "$runtime_output"
    chmod 0755 "$tool_output"
    chmod 0644 "$runtime_output"
}

install_buildx() {
    local output=$1
    local expected=48af8a397ebd60178778bf63611dbcebe5f5e7a9be90eb9147b24b9587455778
    [[ $(uname -s) == Linux && $(uname -m) == x86_64 ]] || {
        printf 'error: the pinned Buildx asset is only for Linux x86_64\n' >&2
        exit 1
    }
    download_verified \
        https://github.com/docker/buildx/releases/download/v0.36.1/buildx-v0.36.1.linux-amd64 \
        "$expected" "$output"
    chmod 0755 "$output"
    [[ $("$output" version) == "github.com/docker/buildx v0.36.1 "* ]] || {
        printf 'error: downloaded Buildx reports an unexpected version\n' >&2
        exit 1
    }
}

install_bun() {
    local install_root=$1
    local expected=951ee2aee855f08595aeec6225226a298d3fea83a3dcd6465c09cbccdf7e848f
    local temporary archive binary member_count
    [[ ! -e $install_root && ! -L $install_root ]] || {
        printf 'error: refusing to replace existing Bun directory: %s\n' "$install_root" >&2
        exit 1
    }
    temporary=$(mktemp -d)
    archive=$temporary/bun.zip
    download_verified \
        https://github.com/oven-sh/bun/releases/download/bun-v1.3.14/bun-linux-x64.zip \
        "$expected" "$archive"
    member_count=$(unzip -Z1 "$archive" | wc -l | tr -d '[:space:]')
    if [[ $member_count != 2 ]] \
        || ! unzip -Z1 "$archive" | grep -Fxq 'bun-linux-x64/' \
        || ! unzip -Z1 "$archive" | grep -Fxq 'bun-linux-x64/bun'; then
        printf 'error: verified Bun archive shape is unexpected\n' >&2
        exit 1
    fi
    mkdir -p "$install_root"
    binary=$install_root/bun
    unzip -p "$archive" bun-linux-x64/bun > "$binary"
    chmod 0755 "$binary"
    [[ $("$binary" --version) == 1.3.14 ]] || {
        printf 'error: downloaded Bun reports an unexpected version\n' >&2
        exit 1
    }
    rm -rf "$temporary"
    if [[ -n ${GITHUB_PATH:-} ]]; then
        printf '%s\n' "$install_root" >> "$GITHUB_PATH"
    else
        printf '%s\n' "$install_root"
    fi
}

install_ripgrep() {
    local install_root=$1 version=15.2.0
    local expected=33e15bcf1624b25cdd2a55813a47a2f95dbe126268203e76aa6a585d1e7b149c
    local asset=ripgrep-$version-x86_64-unknown-linux-musl
    local temporary archive source_dir binary member listing
    [[ ! -e $install_root && ! -L $install_root ]] || {
        printf 'error: refusing to replace existing ripgrep directory: %s\n' \
            "$install_root" >&2
        exit 1
    }
    [[ $(uname -s) == Linux && $(uname -m) == x86_64 ]] || {
        printf 'error: the pinned ripgrep asset is only for Linux x86_64\n' >&2
        exit 1
    }
    temporary=$(mktemp -d)
    archive=$temporary/ripgrep.tar.gz
    source_dir=$temporary/$asset
    download_verified \
        "https://github.com/BurntSushi/ripgrep/releases/download/$version/$asset.tar.gz" \
        "$expected" "$archive"
    while IFS= read -r member; do
        [[ $member == "$asset/"* && $member != *'../'* \
            && $member != /* && $member != *"\\"* ]] || {
            printf 'error: unsafe ripgrep archive member: %s\n' "$member" >&2
            rm -rf "$temporary"
            exit 1
        }
    done < <(tar -tzf "$archive")
    while IFS= read -r listing; do
        case ${listing:0:1} in
            -|d) ;;
            *)
                printf 'error: ripgrep archive contains a non-regular member\n' >&2
                rm -rf "$temporary"
                exit 1
                ;;
        esac
    done < <(tar -tvzf "$archive")
    tar -xzf "$archive" -C "$temporary"
    binary=$source_dir/rg
    [[ -f $binary && ! -L $binary && -x $binary ]] || {
        printf 'error: verified ripgrep archive lacks a regular rg binary\n' >&2
        rm -rf "$temporary"
        exit 1
    }
    verify_pinned_ripgrep_binary "$binary" || {
        rm -rf "$temporary"
        exit 1
    }
    if ! printf 'version 0.8.5\n' \
        | "$binary" --pcre2 --quiet '(?<=version )[0-9]+\.[0-9]+\.[0-9]+'; then
        printf 'error: pinned ripgrep lacks required PCRE2 support\n' >&2
        rm -rf "$temporary"
        exit 1
    fi
    mkdir -p "$install_root"
    cp "$binary" "$install_root/rg"
    chmod 0755 "$install_root/rg"
    rm -rf "$temporary"
    if [[ -n ${GITHUB_PATH:-} ]]; then
        printf '%s\n' "$install_root" >> "$GITHUB_PATH"
    else
        printf '%s\n' "$install_root"
    fi
}

install_node() {
    local install_root=$1
    local expected=cc47fc6eb872f905f48c85397856e8a097b2020bb65b394e728af606bfb6e1a9
    local temporary archive member
    [[ ! -e $install_root && ! -L $install_root ]] || {
        printf 'error: refusing to replace existing Node directory: %s\n' "$install_root" >&2
        exit 1
    }
    temporary=$(mktemp -d)
    archive=$temporary/node.tar.gz
    download_verified \
        https://github.com/actions/node-versions/releases/download/20.20.2-23521894959/node-20.20.2-linux-x64.tar.gz \
        "$expected" "$archive"
    while IFS= read -r member; do
        [[ $member == ./* && $member != *'../'* ]] || {
            printf 'error: unsafe Node archive member: %s\n' "$member" >&2
            exit 1
        }
    done < <(tar -tzf "$archive")
    mkdir -p "$install_root"
    tar -xzf "$archive" -C "$install_root"
    [[ -x $install_root/bin/node && ! -L $install_root/bin/node ]] || {
        printf 'error: verified Node archive lacks a regular node binary\n' >&2
        exit 1
    }
    [[ $("$install_root/bin/node" --version) == v20.20.2 ]] || {
        printf 'error: downloaded Node reports an unexpected version\n' >&2
        exit 1
    }
    rm -rf "$temporary"
    if [[ -n ${GITHUB_PATH:-} ]]; then
        printf '%s\n' "$install_root/bin" >> "$GITHUB_PATH"
    else
        printf '%s\n' "$install_root/bin"
    fi
}

install_skia_archive() {
    local profile=$1 target=$2 cache_dir=$3
    local metadata key expected archive cache_url
    metadata=$(reviewed_skia_archive "$profile" "$target") || exit 1
    read -r key expected <<< "$metadata"
    [[ ! -e $cache_dir && ! -L $cache_dir ]] || {
        printf 'error: refusing to replace existing Skia cache: %s\n' "$cache_dir" >&2
        exit 1
    }
    mkdir -p "$cache_dir"
    archive=$cache_dir/skia-binaries-$key.tar.gz
    download_verified \
        "https://github.com/rust-skia/skia-binaries/releases/download/0.97.2/skia-binaries-$key.tar.gz" \
        "$expected" "$archive"
    if [[ ${RUNNER_OS:-} == Windows ]]; then
        command -v cygpath >/dev/null 2>&1 || {
            printf 'error: cygpath is required for a Windows file URL\n' >&2
            exit 1
        }
        cache_url=$(cygpath -am "$cache_dir")
        cache_url=$(skia_file_url Windows "$cache_url")
    else
        cache_url=$(skia_file_url "${RUNNER_OS:-Linux}" "$(CDPATH='' cd "$cache_dir" && pwd)")
    fi
    if [[ -n ${GITHUB_ENV:-} ]]; then
        {
            printf 'SKIA_BINARIES_URL=%s/skia-binaries-{key}.tar.gz\n' "$cache_url"
        } >> "$GITHUB_ENV"
    else
        printf 'SKIA_BINARIES_URL=%s/skia-binaries-{key}.tar.gz\n' "$cache_url"
    fi
}

case ${1-} in
    --self-test)
        [[ $# -eq 1 ]] || { printf 'usage: %s --self-test\n' "$0" >&2; exit 2; }
        self_test
        ;;
    binaryen)
        [[ $# -eq 2 ]] || { printf 'usage: %s binaryen INSTALL_ROOT\n' "$0" >&2; exit 2; }
        install_binaryen "$2"
        ;;
    bundletool)
        [[ $# -eq 2 ]] || { printf 'usage: %s bundletool INSTALL_ROOT\n' "$0" >&2; exit 2; }
        install_bundletool "$2"
        ;;
    android-signing-tools)
        [[ $# -eq 2 ]] || { printf 'usage: %s android-signing-tools INSTALL_ROOT\n' "$0" >&2; exit 2; }
        install_android_signing_tools "$2"
        ;;
    appimage)
        [[ $# -eq 4 ]] || {
            printf 'usage: %s appimage ARCH TOOL_OUTPUT RUNTIME_OUTPUT\n' "$0" >&2
            exit 2
        }
        download_appimage_tools "$2" "$3" "$4"
        ;;
    buildx)
        [[ $# -eq 2 ]] || { printf 'usage: %s buildx OUTPUT\n' "$0" >&2; exit 2; }
        install_buildx "$2"
        ;;
    bun)
        [[ $# -eq 2 ]] || { printf 'usage: %s bun INSTALL_ROOT\n' "$0" >&2; exit 2; }
        install_bun "$2"
        ;;
    ripgrep)
        [[ $# -eq 2 ]] || { printf 'usage: %s ripgrep INSTALL_ROOT\n' "$0" >&2; exit 2; }
        install_ripgrep "$2"
        ;;
    node)
        [[ $# -eq 2 ]] || { printf 'usage: %s node INSTALL_ROOT\n' "$0" >&2; exit 2; }
        install_node "$2"
        ;;
    cargo-cli)
        [[ $# -eq 3 ]] || {
            printf 'usage: %s cargo-cli {cargo-bundle|wasm-bindgen-cli} INSTALL_ROOT\n' "$0" >&2
            exit 2
        }
        install_pinned_cargo_cli "$2" "$3"
        ;;
    skia)
        [[ $# -eq 4 ]] || { printf 'usage: %s skia {desktop|web|ios|android} TARGET CACHE_DIR\n' "$0" >&2; exit 2; }
        install_skia_archive "$2" "$3" "$4"
        ;;
    verify-skia)
        [[ $# -eq 4 ]] || {
            printf 'usage: %s verify-skia {desktop|android} TARGET SKIA_BINARIES_URL\n' "$0" >&2
            exit 2
        }
        verify_staged_skia_archive "$2" "$3" "$4"
        ;;
    *)
        printf 'usage: %s {--self-test|binaryen|bundletool|android-signing-tools|appimage|buildx|bun|ripgrep|node|cargo-cli|skia|verify-skia} ...\n' "$0" >&2
        exit 2
        ;;
esac
