#!/usr/bin/env bash
# Install the OpenPencil `op` CLI from a GitHub Release.
#
# Detects the host OS + architecture, downloads the matching standalone
# `op-cli-<label>` tarball published by .github/workflows/rust-release.yml,
# and installs the `op` binary into a bin directory on PATH.
#
# Usage:
#   ./install-op.sh                         # install the latest stable release
#   OP_VERSION=X.Y.Z ./install-op.sh        # pin a specific version
#   OP_PRERELEASE=1 ./install-op.sh         # allow the newest pre-release
#   INSTALL_DIR=$HOME/.local/bin ./install-op.sh
#
# Current releases are pre-releases; use OP_PRERELEASE=1 unless pinning a version.
# Without that opt-in, the source installer only selects stable releases.
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

version_help() {
  echo "       set OP_PRERELEASE=1 to allow pre-release tags, or set OP_VERSION=X.Y.Z to pin a version" >&2
  echo "       curl -fsSL https://raw.githubusercontent.com/${OWNER}/${REPO}/main/scripts/install-op.sh | OP_PRERELEASE=1 bash" >&2
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

  local api tag response status body allow_prerelease=false
  if [ "${OP_PRERELEASE:-}" = "1" ] || [ "${OP_PRERELEASE:-}" = "true" ]; then
    allow_prerelease=true
    api="https://api.github.com/repos/${OWNER}/${REPO}/releases"
  else
    api="https://api.github.com/repos/${OWNER}/${REPO}/releases/latest"
  fi
  if ! response="$(curl -sSL --write-out '\n%{http_code}' "$api")"; then
    echo "error: could not query GitHub releases; check your network connection and retry" >&2
    return 1
  fi
  status=${response##*$'\n'}
  body=${response%$'\n'*}
  if [ "$status" != "200" ]; then
    if [ "$status" = "404" ] && [ "$allow_prerelease" = false ]; then
      echo "error: no stable OpenPencil release is available; current releases are pre-releases" >&2
      version_help
    else
      echo "error: GitHub releases request failed (HTTP ${status}): ${api}" >&2
    fi
    return 1
  fi
  # Consume every match to avoid a SIGPIPE from `head` on a long release list.
  tag="$(printf '%s\n' "$body" | grep -o '"tag_name"[[:space:]]*:[[:space:]]*"[^"]*"' | sed -n '1s/.*"\([^"]*\)"$/\1/p' || true)"
  if [ -z "$tag" ]; then
    echo "error: GitHub returned no release tag" >&2
    version_help
    return 1
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
