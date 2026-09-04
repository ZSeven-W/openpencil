#!/usr/bin/env bash

set -euo pipefail

script_dir=$(CDPATH='' cd "$(dirname "$0")" && pwd)
tool=$script_dir/pinned-release-tools.sh
release_builder=$script_dir/../scripts/build-rust-release-host.sh

[[ -f $tool && ! -L $tool ]] || {
    printf 'error: missing pinned release tool downloader\n' >&2
    exit 1
}
[[ -f $release_builder && ! -L $release_builder ]] || {
    printf 'error: missing Rust release host builder\n' >&2
    exit 1
}
bash -n "$tool"
bash -n "$release_builder"
"$tool" --self-test

temporary=$(mktemp -d)
trap 'rm -rf "$temporary"' EXIT
mkdir "$temporary/existing"
if "$tool" skia desktop x86_64-unknown-linux-gnu "$temporary/existing" >/dev/null 2>&1; then
    printf 'error: an existing Skia cache destination was accepted\n' >&2
    exit 1
fi
if "$tool" skia desktop unsupported-target "$temporary/new" >/dev/null 2>&1; then
    printf 'error: an unreviewed Skia target was accepted\n' >&2
    exit 1
fi
if "$tool" skia ios aarch64-apple-ios-sim "$temporary/ios" >/dev/null 2>&1; then
    printf 'error: the device-only iOS Skia profile accepted a simulator\n' >&2
    exit 1
fi
if "$tool" skia android armv7-linux-androideabi "$temporary/android" >/dev/null 2>&1; then
    printf 'error: the Android Skia profile accepted an unreviewed ABI\n' >&2
    exit 1
fi
mkdir "$temporary/existing-cargo-cli"
if "$tool" cargo-cli cargo-bundle "$temporary/existing-cargo-cli" >/dev/null 2>&1; then
    printf 'error: an existing Cargo CLI destination was accepted\n' >&2
    exit 1
fi
if "$tool" cargo-cli unreviewed-cli "$temporary/unreviewed-cli" >/dev/null 2>&1; then
    printf 'error: an unreviewed Cargo CLI was accepted\n' >&2
    exit 1
fi
mkdir "$temporary/existing-bundletool"
if "$tool" bundletool "$temporary/existing-bundletool" >/dev/null 2>&1; then
    printf 'error: an existing bundletool destination was accepted\n' >&2
    exit 1
fi
mkdir "$temporary/existing-android-signing-tools"
if "$tool" android-signing-tools \
    "$temporary/existing-android-signing-tools" >/dev/null 2>&1; then
    printf 'error: an existing Android signing-tools destination was accepted\n' >&2
    exit 1
fi
mkdir "$temporary/existing-ripgrep"
if "$tool" ripgrep "$temporary/existing-ripgrep" >/dev/null 2>&1; then
    printf 'error: an existing ripgrep destination was accepted\n' >&2
    exit 1
fi

skia_cache=$temporary/skia-cache
skia_url="file://$skia_cache/skia-binaries-{key}.tar.gz"
mkdir "$skia_cache"
for desktop_target in \
    aarch64-apple-darwin \
    x86_64-apple-darwin \
    aarch64-unknown-linux-gnu \
    x86_64-unknown-linux-gnu \
    aarch64-pc-windows-msvc \
    x86_64-pc-windows-msvc; do
    skia_key=da8fc6731fc439bc3b6a-$desktop_target-gl-jpegd-jpege-pdf-textlayout
    skia_archive=$skia_cache/skia-binaries-$skia_key.tar.gz
    printf 'tampered archive for %s\n' "$desktop_target" > "$skia_archive"
    if "$tool" verify-skia desktop "$desktop_target" "$skia_url" \
        >"$temporary/tampered-$desktop_target.log" 2>&1; then
        printf 'error: a tampered staged Skia archive was accepted: %s\n' \
            "$desktop_target" >&2
        exit 1
    fi
    grep -Fq 'SHA-256 mismatch' "$temporary/tampered-$desktop_target.log" || {
        printf 'error: %s did not resolve its exact reviewed Skia key and digest\n' \
            "$desktop_target" >&2
        exit 1
    }
    rm "$skia_archive"
done
if "$tool" verify-skia desktop unsupported-target "$skia_url" \
    >"$temporary/wrong-target.log" 2>&1; then
    printf 'error: staged Skia verification accepted an unreviewed target\n' >&2
    exit 1
fi
grep -Fq 'no reviewed desktop Skia archive' "$temporary/wrong-target.log" || {
    printf 'error: the wrong-target test did not reach the reviewed target gate\n' >&2
    exit 1
}
if "$tool" verify-skia web x86_64-unknown-linux-gnu "$skia_url" \
    >"$temporary/wrong-profile.log" 2>&1; then
    printf 'error: staged Skia verification accepted a non-desktop profile\n' >&2
    exit 1
fi
grep -Fq 'requires the desktop or Android Skia profile' "$temporary/wrong-profile.log" || {
    printf 'error: the wrong-profile test did not reach the desktop profile gate\n' >&2
    exit 1
}

for android_target in aarch64-linux-android x86_64-linux-android; do
    android_key=da8fc6731fc439bc3b6a-$android_target-gl-jpegd-jpege-pdf-textlayout
    android_archive=$skia_cache/skia-binaries-$android_key.tar.gz
    printf 'tampered Android archive for %s\n' "$android_target" > "$android_archive"
    if "$tool" verify-skia android "$android_target" "$skia_url" \
        >"$temporary/tampered-$android_target.log" 2>&1; then
        printf 'error: a tampered staged Android Skia archive was accepted: %s\n' \
            "$android_target" >&2
        exit 1
    fi
    grep -Fq 'SHA-256 mismatch' "$temporary/tampered-$android_target.log" || {
        printf 'error: %s did not resolve its exact reviewed Android Skia digest\n' \
            "$android_target" >&2
        exit 1
    }
    rm "$android_archive"
done
if env FORCE_SKIA_BINARIES_DOWNLOAD='' "$tool" verify-skia \
    desktop x86_64-unknown-linux-gnu "$skia_url" \
    >"$temporary/forced.log" 2>&1; then
    printf 'error: staged Skia verification accepted the FORCE override\n' >&2
    exit 1
fi
grep -Fq 'FORCE_SKIA_BINARIES_DOWNLOAD must be unset' "$temporary/forced.log" || {
    printf 'error: the FORCE test did not reach the environment gate\n' >&2
    exit 1
}
skia_key=da8fc6731fc439bc3b6a-x86_64-unknown-linux-gnu-gl-jpegd-jpege-pdf-textlayout
skia_archive=$skia_cache/skia-binaries-$skia_key.tar.gz
printf 'symlink target\n' > "$temporary/skia-symlink-target"
ln -s "$temporary/skia-symlink-target" "$skia_archive"
if "$tool" verify-skia desktop x86_64-unknown-linux-gnu "$skia_url" \
    >"$temporary/symlink.log" 2>&1; then
    printf 'error: staged Skia verification accepted a symlink archive\n' >&2
    exit 1
fi
grep -Fq 'not a regular file' "$temporary/symlink.log" || {
    printf 'error: the symlink test did not reach the regular-file gate\n' >&2
    exit 1
}

windows_key=da8fc6731fc439bc3b6a-x86_64-pc-windows-msvc-gl-jpegd-jpege-pdf-textlayout
windows_cache=$temporary/C:/cache
windows_archive=$windows_cache/skia-binaries-$windows_key.tar.gz
mkdir -p "$windows_cache"
printf 'tampered Windows archive\n' > "$windows_archive"
if (cd "$temporary" && RUNNER_OS=Windows "$tool" verify-skia \
    desktop x86_64-pc-windows-msvc \
    'file://C:/cache/skia-binaries-{key}.tar.gz') \
    >"$temporary/windows-url.log" 2>&1; then
    printf 'error: a tampered Windows staged Skia archive was accepted\n' >&2
    exit 1
fi
grep -Fq 'SHA-256 mismatch' "$temporary/windows-url.log" || {
    printf 'error: file://C:/... did not resolve like skia-bindings on Windows\n' >&2
    exit 1
}
if RUNNER_OS=Windows "$tool" verify-skia desktop x86_64-pc-windows-msvc \
    'file:///C:/cache/skia-binaries-{key}.tar.gz' \
    >"$temporary/windows-leading-slash.log" 2>&1; then
    printf 'error: an upstream-incompatible Windows file URL was accepted\n' >&2
    exit 1
fi
grep -Fq 'must use file://C:/... form' "$temporary/windows-leading-slash.log" || {
    printf 'error: malformed Windows URL did not reach the file URL gate\n' >&2
    exit 1
}

verify_pattern="\"\$repo_root/tools/pinned-release-tools.sh\" verify-skia"
verify_args="    desktop \"\$release_target\" \"\$SKIA_BINARIES_URL\""
build_pattern="    cargo build \\"
[[ $(grep -Fc "$verify_pattern" "$release_builder") -eq 1 ]] || {
    printf 'error: Rust release build must invoke staged Skia verification once\n' >&2
    exit 1
}
grep -Fxq "$verify_args" "$release_builder" || {
    printf 'error: Rust release build does not verify its exact target and Skia URL\n' >&2
    exit 1
}
verify_line=$(grep -nF "$verify_pattern" "$release_builder" | cut -d: -f1)
build_line=$(grep -nF "$build_pattern" "$release_builder" | cut -d: -f1)
[[ -n $verify_line && -n $build_line && $((build_line - verify_line)) -eq 4 ]] || {
    printf 'error: Rust release build does not verify staged Skia before Cargo\n' >&2
    exit 1
}

for target_digest in \
    aarch64-apple-darwin=c4c5d5059ab9226aaf3d5337a8fd42ef0e42e9fe3cbc3c8da4310b4a3a1e4254 \
    x86_64-apple-darwin=fe92e66916947a4d666a24d0580434f42585853d221d2af006a52a72b55b283b \
    aarch64-unknown-linux-gnu=2587dcaf11aab680ef8637d4192fc77a507c91e3a88bebb79d7993a4fefa1d1b \
    x86_64-unknown-linux-gnu=ee77fbd0183e854e297276705e4e8685837c6c7d0304472c97145fcd8f7f2cfc \
    aarch64-pc-windows-msvc=20ba7acf5e306b6d875863c838cb9d3c4a39a05792fb6256a3f03ddcbc1077a1 \
    x86_64-pc-windows-msvc=6b61061c32fb7a72944e3dae63d97241271b1ac7bcaf3752cfa0c79ed37ee8b6; do
    desktop_target=${target_digest%%=*}
    digest=${target_digest#*=}
    grep -Fq "$desktop_target) expected=$digest" "$tool" || {
        printf 'error: desktop Skia target is not bound to its reviewed digest: %s\n' \
            "$desktop_target" >&2
        exit 1
    }
done

for digest in \
    e959f2170af4c20c552e9de3a0253704d6a9d2766e8fdb88e4d6ac4bae9388fe \
    48af8a397ebd60178778bf63611dbcebe5f5e7a9be90eb9147b24b9587455778 \
    951ee2aee855f08595aeec6225226a298d3fea83a3dcd6465c09cbccdf7e848f \
    cc47fc6eb872f905f48c85397856e8a097b2020bb65b394e728af606bfb6e1a9 \
    ed4ce84f0d9caff66f50bcca6ff6f35aae54ce8135408b3fa33abfc3cb384eb0 \
    f0837e7448a0c1e4e650a93bb3e85802546e60654ef287576f46c71c126a9158 \
    2fca8b443c92510f1483a883f60061ad09b46b978b2631c807cd873a47ec260d \
    00cbdfcf917cc6c0ff6d3347d59e0ca1f7f45a6df1a428a0d6d8a78664d87444 \
    41058f8f2967385b2799764c2c281fd143392ef82221d5ffde0481a1cdbfc40e \
    bb3601b2899d4887512bdcaad115074750be7c212b122fa7ed4faed6c919229e \
    33e15bcf1624b25cdd2a55813a47a2f95dbe126268203e76aa6a585d1e7b149c \
    c4c5d5059ab9226aaf3d5337a8fd42ef0e42e9fe3cbc3c8da4310b4a3a1e4254 \
    fe92e66916947a4d666a24d0580434f42585853d221d2af006a52a72b55b283b \
    2587dcaf11aab680ef8637d4192fc77a507c91e3a88bebb79d7993a4fefa1d1b \
    ee77fbd0183e854e297276705e4e8685837c6c7d0304472c97145fcd8f7f2cfc \
    20ba7acf5e306b6d875863c838cb9d3c4a39a05792fb6256a3f03ddcbc1077a1 \
    6b61061c32fb7a72944e3dae63d97241271b1ac7bcaf3752cfa0c79ed37ee8b6 \
    c066658b13e257d418f647447d06eb8a83cb060b037228da838589dd863bf053 \
    4abbaea5e4e8934a6f19c5de44eaba9bf9238af4abbe57dbac5f2dc03923b182 \
    82ca6dd1720bbe8b105c12c4d0c78786d2c792e9d2a7f2102ab66bb24dafa9d0 \
    ca217df6ffced17381cbea4df044969a493a46bddc757ee844e2fbaf54fa1257 \
    5d9ac77fb6ff43d9da518a337b4fcf8f9097113df531d99ccefe80ef7ce8250b \
    f2dc5418092c43003db8f9005c4a286e1c0104fea96ccdd49e8ebd037cac9219; do
    [[ $(grep -Fc "$digest" "$tool") -eq 1 ]] || {
        printf 'error: pinned release digest is missing or duplicated: %s\n' "$digest" >&2
        exit 1
    }
done

[[ $(grep -Fc a099cfa1543f55593bc2ed16a70a7c67fe54b1747bb7301f37fdfd6d91028e29 "$tool") -eq 2 ]] || {
    printf 'error: the reviewed bundletool digest is missing or duplicated\n' >&2
    exit 1
}

printf 'pinned-release-tools.test.sh: immutable tool contracts passed.\n'
