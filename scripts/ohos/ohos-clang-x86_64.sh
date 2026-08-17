#!/usr/bin/env bash
# `x86_64-unknown-linux-ohos` linker driver (see ohos-clang.sh).
set -euo pipefail
export OHOS_CLANG_TARGET="x86_64-linux-ohos"
exec "$(dirname "${BASH_SOURCE[0]}")/ohos-clang.sh" "$@"
