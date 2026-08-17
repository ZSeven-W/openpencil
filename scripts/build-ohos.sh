#!/usr/bin/env bash
# Build the OpenHarmony engine module (`libopenpencil.so`) from op-engine-napi.
#
# Usage:
#   OHOS_NDK_HOME=/path/to/openharmony scripts/build-ohos.sh [extra cargo args]
#
# Environment:
#   OHOS_NDK_HOME   REQUIRED. The OpenHarmony SDK toolchain root. Either the
#                   `native` directory or its parent works — both this script
#                   and skia-bindings probe for the sysroot. skia-bindings
#                   itself appends `native/`, so PREFER the parent, e.g.
#                   ~/command-line-tools/sdk/default/openharmony
#   OHOS_TARGET     Rust target triple. Default aarch64-unknown-linux-ohos;
#                   x86_64-unknown-linux-ohos is the emulator lane.
#   OHOS_FEATURES   Cargo features. Default "gl,editor".
#   OHOS_PROFILE    "release" (default) or "debug".
#
# See scripts/ohos/README.md for the skia cross-compile story and for every
# step still marked UNVERIFIED-UNTIL-NDK.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

target="${OHOS_TARGET:-aarch64-unknown-linux-ohos}"
features="${OHOS_FEATURES:-gl,editor}"
profile="${OHOS_PROFILE:-release}"

# ---- 1. Toolchain preconditions -----------------------------------------

if [[ -z "${OHOS_NDK_HOME:-}" ]]; then
  cat >&2 <<'MSG'
build-ohos.sh: OHOS_NDK_HOME is not set.

Install the OpenHarmony command-line tools, then point OHOS_NDK_HOME at the
SDK's toolchain root (the directory CONTAINING `native/`):

  export OHOS_NDK_HOME="$HOME/command-line-tools/sdk/default/openharmony"

skia-bindings reads the same variable and appends `native/` itself.
MSG
  exit 1
fi

if [[ -d "$OHOS_NDK_HOME/native/sysroot" ]]; then
  ndk_root="$OHOS_NDK_HOME/native"
elif [[ -d "$OHOS_NDK_HOME/sysroot" ]]; then
  ndk_root="$OHOS_NDK_HOME"
  echo "build-ohos.sh: warning — OHOS_NDK_HOME points at the 'native' dir." >&2
  echo "  skia-bindings expects its PARENT and will append 'native/'; export" >&2
  echo "  ohos_sdk_native=\"$OHOS_NDK_HOME\" as well, or move the variable up." >&2
else
  echo "build-ohos.sh: no sysroot under \$OHOS_NDK_HOME ($OHOS_NDK_HOME)" >&2
  exit 1
fi
llvm_bin="$ndk_root/llvm/bin"

for tool in clang clang++ llvm-ar llvm-ranlib; do
  if [[ ! -x "$llvm_bin/$tool" ]]; then
    echo "build-ohos.sh: $llvm_bin/$tool not found" >&2
    exit 1
  fi
done

# Skia is built from source for OHOS (rust-skia publishes no prebuilt archive
# for these triples), and its GN/ninja bootstrap needs python3.
if ! command -v python3 >/dev/null 2>&1; then
  echo "build-ohos.sh: python3 is required to build Skia from source" >&2
  exit 1
fi

rustup target add "$target"

# ---- 2. Cross-compilation environment ------------------------------------

case "$target" in
  aarch64-unknown-linux-ohos) clang_target="aarch64-linux-ohos" ;;
  x86_64-unknown-linux-ohos) clang_target="x86_64-linux-ohos" ;;
  armv7-unknown-linux-ohos) clang_target="arm-linux-ohos" ;;
  *)
    echo "build-ohos.sh: unsupported target '$target'" >&2
    exit 1
    ;;
esac

# The `cc` crate keys on the UNDERSCORED target triple.
env_target="${target//-/_}"
env_target_upper="$(echo "$env_target" | tr '[:lower:]' '[:upper:]')"

# skia-bindings parses `--target=` out of CC to decide what it is building for
# (build_support/skia/config.rs), so the flag must be part of the command
# string, not of CFLAGS.
export CC="$llvm_bin/clang --target=$clang_target --sysroot=$ndk_root/sysroot"
export CXX="$llvm_bin/clang++ --target=$clang_target --sysroot=$ndk_root/sysroot"
# CLANGCC/CLANGCXX take precedence over CC/CXX inside skia-bindings; set both
# so the choice is unambiguous no matter which branch it takes.
export CLANGCC="$CC"
export CLANGCXX="$CXX"
export AR="$llvm_bin/llvm-ar"
export RANLIB="$llvm_bin/llvm-ranlib"
export "CC_${env_target}=$CC"
export "CXX_${env_target}=$CXX"
export "AR_${env_target}=$AR"
export "CFLAGS_${env_target}=--target=$clang_target --sysroot=$ndk_root/sysroot"
export "CXXFLAGS_${env_target}=--target=$clang_target --sysroot=$ndk_root/sysroot"

# Absolute linker override so the build works from any cwd (the repo's
# .cargo/config.toml carries a repo-root-relative fallback).
export "CARGO_TARGET_${env_target_upper}_LINKER=$repo_root/scripts/ohos/ohos-clang.sh"
export OHOS_CLANG_TARGET="$clang_target"

# skia-bindings' OpenHarmony platform reads this; keep it pointing at the
# parent of `native/` regardless of how OHOS_NDK_HOME was given.
export ohos_sdk_native="$ndk_root"

# ---- 3. Build ------------------------------------------------------------

# Built as one never-empty array: macOS still ships bash 3.2, where expanding
# an EMPTY array under `set -u` aborts the script.
cargo_args=(build -p op-engine-napi --target "$target" --features "$features")
if [[ "$profile" == "release" ]]; then
  cargo_args+=(--release)
fi

set -x
cargo "${cargo_args[@]}" "$@"
set +x

artifact="target/$target/$profile/libopenpencil.so"
if [[ -f "$artifact" ]]; then
  echo
  echo "built: $artifact"
  echo "copy it into the ArkTS app's libs/<abi>/ directory (packaging/harmony)."
else
  echo "build-ohos.sh: expected $artifact but it is missing" >&2
  exit 1
fi
