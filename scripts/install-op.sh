#!/usr/bin/env bash
# Install the OpenPencil `op` CLI from a GitHub Release.
#
# Detects the host OS + architecture, downloads the matching standalone
# `op-cli-<label>` tarball published by .github/workflows/rust-release.yml,
# and installs the `op` binary into a bin directory on PATH.
#
# Usage:
#   ./install-op.sh                         # install the latest stable release
#   OP_VERSION=0.8.1 ./install-op.sh        # pin a specific version
#   OP_PRERELEASE=1 ./install-op.sh         # allow the newest pre-release
#   INSTALL_DIR=$HOME/.local/bin ./install-op.sh
#
# The release workflow stamps DEFAULT_OP_VERSION and DEFAULT_SHA_* in the copy
# uploaded to GitHub Releases, so the release asset installs that exact tag and
# verifies the CLI archive checksum.

set -euo pipefail

OWNER="ZSeven-W"
REPO="openpencil"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"

DEFAULT_OP_VERSION=""
DEFAULT_SHA_MACOS_AARCH64=""
DEFAULT_SHA_MACOS_X86_64=""
DEFAULT_SHA_LINUX_AARCH64=""
DEFAULT_SHA_LINUX_X86_64=""
INSTALL_TMP=""

detect_label() {
  local os arch
  case "$(uname -s)" in
    Darwin) os="macos" ;;
    Linux) os="linux" ;;
    *)
      echo "error: unsupported OS '$(uname -s)' (only macOS and Linux are packaged)" >&2
      exit 1
      ;;
  esac
  case "$(uname -m)" in
    x86_64 | amd64) arch="x86_64" ;;
    arm64 | aarch64) arch="aarch64" ;;
    *)
      echo "error: unsupported architecture '$(uname -m)'" >&2
      exit 1
      ;;
  esac
  printf '%s-%s' "$os" "$arch"
}

resolve_version() {
  if [ -n "${OP_VERSION:-}" ]; then
    printf '%s' "$OP_VERSION"
    return
  fi
  if [ -n "$DEFAULT_OP_VERSION" ]; then
    printf '%s' "$DEFAULT_OP_VERSION"
    return
  fi

  local api tag
  if [ "${OP_PRERELEASE:-}" = "1" ] || [ "${OP_PRERELEASE:-}" = "true" ]; then
    api="https://api.github.com/repos/${OWNER}/${REPO}/releases"
  else
    api="https://api.github.com/repos/${OWNER}/${REPO}/releases/latest"
  fi
  tag="$(curl -fsSL "$api" | grep -o '"tag_name"[[:space:]]*:[[:space:]]*"[^"]*"' | head -n1 | sed 's/.*"\(v\{0,1\}[^"]*\)"$/\1/')"
  if [ -z "$tag" ]; then
    echo "error: could not resolve the latest release tag from GitHub" >&2
    echo "       set OP_VERSION explicitly, e.g. OP_VERSION=0.8.1 $0" >&2
    echo "       or set OP_PRERELEASE=1 to allow pre-release tags" >&2
    exit 1
  fi
  printf '%s' "${tag#v}"
}

expected_sha_for_label() {
  case "$1" in
    macos-aarch64) printf '%s' "$DEFAULT_SHA_MACOS_AARCH64" ;;
    macos-x86_64) printf '%s' "$DEFAULT_SHA_MACOS_X86_64" ;;
    linux-aarch64) printf '%s' "$DEFAULT_SHA_LINUX_AARCH64" ;;
    linux-x86_64) printf '%s' "$DEFAULT_SHA_LINUX_X86_64" ;;
    *) printf '' ;;
  esac
}

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    echo "error: sha256sum or shasum is required for checksum verification" >&2
    exit 1
  fi
}

main() {
  local label version asset url tmp expected_sha actual_sha
  label="$(detect_label)"
  version="$(resolve_version)"
  asset="op-cli-${label}.tar.gz"
  url="https://github.com/${OWNER}/${REPO}/releases/download/v${version}/${asset}"
  expected_sha="$(expected_sha_for_label "$label")"

  echo "==> Installing op ${version} (${label})"
  echo "    from ${url}"

  tmp="$(mktemp -d)"
  INSTALL_TMP="$tmp"
  trap 'rm -rf "$INSTALL_TMP"' EXIT

  curl -fsSL --retry 3 -o "$tmp/${asset}" "$url"
  if [ -n "$expected_sha" ]; then
    actual_sha="$(sha256_file "$tmp/${asset}")"
    if [ "$actual_sha" != "$expected_sha" ]; then
      echo "error: checksum mismatch for ${asset}" >&2
      echo "       expected: $expected_sha" >&2
      echo "       actual:   $actual_sha" >&2
      exit 1
    fi
  fi

  tar -xzf "$tmp/${asset}" -C "$tmp"
  if [ ! -f "$tmp/op" ]; then
    echo "error: ${asset} did not contain an 'op' binary" >&2
    exit 1
  fi
  chmod +x "$tmp/op"

  echo "==> Installing to ${INSTALL_DIR}/op"
  mkdir -p "$INSTALL_DIR" 2>/dev/null || true
  if [ -w "$INSTALL_DIR" ]; then
    install -m 0755 "$tmp/op" "$INSTALL_DIR/op"
  else
    echo "    (need elevated permissions for ${INSTALL_DIR})"
    sudo install -m 0755 "$tmp/op" "$INSTALL_DIR/op"
  fi

  echo "==> Done. Run 'op --version' to verify."
}

main "$@"
