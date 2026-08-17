#!/usr/bin/env bash
# `aarch64-unknown-linux-ohos` linker driver (see ohos-clang.sh).
set -euo pipefail
export OHOS_CLANG_TARGET="aarch64-linux-ohos"
exec "$(dirname "${BASH_SOURCE[0]}")/ohos-clang.sh" "$@"
