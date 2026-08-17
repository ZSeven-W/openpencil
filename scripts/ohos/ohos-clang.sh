#!/usr/bin/env bash
# OpenHarmony NDK clang wrapper.
#
# Cargo's `[target.<triple>] linker` key takes a bare program path and cannot
# carry flags, but the OHOS NDK's clang needs BOTH `--target` and `--sysroot`
# on every invocation. This wrapper supplies them and execs the real clang.
#
# Target resolution, first match wins:
#   1. $OHOS_CLANG_TARGET                        (set by the per-arch wrappers)
#   2. the `-<arch>` suffix on this script's own name
#   3. aarch64-linux-ohos
#
# Sysroot resolution: OHOS_NDK_HOME may point either at the SDK's `native`
# directory or at its PARENT. Both layouts are in the wild (skia-bindings
# expects the parent and appends `native/` itself), so probe for whichever
# actually contains a sysroot instead of guessing.
set -euo pipefail

if [[ -z "${OHOS_NDK_HOME:-}" ]]; then
  echo "ohos-clang.sh: OHOS_NDK_HOME is not set" >&2
  echo "  point it at the OpenHarmony SDK native toolchain, e.g." >&2
  echo "  export OHOS_NDK_HOME=\"\$HOME/command-line-tools/sdk/default/openharmony\"" >&2
  exit 1
fi

target="${OHOS_CLANG_TARGET:-}"
if [[ -z "$target" ]]; then
  case "$(basename "$0")" in
    *-x86_64.sh) target="x86_64-linux-ohos" ;;
    *-armv7.sh) target="arm-linux-ohos" ;;
    *) target="aarch64-linux-ohos" ;;
  esac
fi

if [[ -d "$OHOS_NDK_HOME/native/sysroot" ]]; then
  ndk_root="$OHOS_NDK_HOME/native"
elif [[ -d "$OHOS_NDK_HOME/sysroot" ]]; then
  ndk_root="$OHOS_NDK_HOME"
else
  echo "ohos-clang.sh: no sysroot under \$OHOS_NDK_HOME ($OHOS_NDK_HOME)" >&2
  echo "  expected either \$OHOS_NDK_HOME/native/sysroot or \$OHOS_NDK_HOME/sysroot" >&2
  exit 1
fi

# `clang` vs `clang++` is chosen by this wrapper's own name so a single file
# can back both the linker and the C++ driver.
case "$(basename "$0")" in
  *clang++*) driver="clang++" ;;
  *) driver="clang" ;;
esac

clang="$ndk_root/llvm/bin/$driver"
if [[ ! -x "$clang" ]]; then
  echo "ohos-clang.sh: $clang not found or not executable" >&2
  exit 1
fi

exec "$clang" --target="$target" --sysroot="$ndk_root/sysroot" "$@"
