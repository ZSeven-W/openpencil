#!/usr/bin/env bash
# Build one desktop release target with required production relay and ABI 3
# auth inputs, then prove Cargo did not silently select the auth stub.

set -euo pipefail

relay_cn=${OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_CN:-}
relay_global=${OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_GLOBAL:-}
release_target=${OPENPENCIL_RELEASE_TARGET:-}
export -n relay_cn relay_global release_target
unset OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_CN
unset OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_GLOBAL

script_dir=$(CDPATH='' cd "$(dirname "$0")" && pwd)
repo_root=$(CDPATH='' cd "$script_dir/.." && pwd)

if [[ "$#" -ne 0 ]]; then
    printf 'usage: OPENPENCIL_RELEASE_TARGET=<triple> %s\n' "$0" >&2
    exit 2
fi
[[ -n "$release_target" ]] || {
    printf 'error: OPENPENCIL_RELEASE_TARGET is required\n' >&2
    exit 2
}
[[ -z ${FORCE_SKIA_BINARIES_DOWNLOAD:-} && ${SKIA_BINARIES_URL:-} == file://* ]] || {
    printf 'error: release builds require the verified local Skia binary cache\n' >&2
    exit 2
}
[[ -z ${FORCE_SKIA_BINARIES_DOWNLOAD+x} ]] || {
    printf 'error: FORCE_SKIA_BINARIES_DOWNLOAD must be unset for release builds\n' >&2
    exit 2
}
command -v cargo >/dev/null 2>&1 || {
    printf 'error: cargo is unavailable\n' >&2
    exit 1
}
python_command=python3
[[ "${RUNNER_OS:-}" == Windows ]] && python_command=python
command -v "$python_command" >/dev/null 2>&1 || {
    printf 'error: %s is unavailable\n' "$python_command" >&2
    exit 1
}

cd "$repo_root"
printf '%s\0%s\0' "$relay_cn" "$relay_global" \
    | "$python_command" tools/check-collab-bootstrap-urls.py
cargo clean -p op-auth-bridge --target "$release_target" --release
cargo clean -p skia-bindings --target "$release_target" --release
"$repo_root/tools/pinned-release-tools.sh" verify-skia \
    desktop "$release_target" "$SKIA_BINARIES_URL"
OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_CN=$relay_cn \
OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_GLOBAL=$relay_global \
    cargo build \
        -p op-host-desktop -p op-cli -p op-host-web-server \
        --target "$release_target" --release --locked \
        --features \
          op-host-desktop/pinned-skia-binaries,op-host-web-server/pinned-skia-binaries
OP_AUTH_CARGO_TARGET=$release_target tools/check-op-auth-cargo-build.sh
