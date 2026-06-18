#!/usr/bin/env bash
# Install the OpenPencil `op` CLI from a GitHub Release.
#
# Detects the host OS + architecture, downloads the matching standalone
# `op-cli-<label>` tarball published by .github/workflows/rust-release.yml,
# and installs the `op` binary into a bin directory on PATH.
#
# Usage:
#   ./install-op.sh                 # install the latest release
#   OP_VERSION=0.8.0 ./install-op.sh   # pin a specific version
#   INSTALL_DIR=$HOME/.local/bin ./install-op.sh   # custom install dir
#
# Environment overrides:
#   OP_VERSION    release version WITHOUT the leading "v" (default: latest)
#   INSTALL_DIR   install target directory   (default: /usr/local/bin)

set -euo pipefail

OWNER="ZSeven-W"
REPO="openpencil"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"

# Resolve the asset "label" token rust-release.yml uses in
# op-cli-<label>.tar.gz: "<os>-<arch>" where os is macos/linux and arch is
# x86_64/aarch64 (the cargo target arch, NOT the cask's x64/arm64 token).
detect_label() {
  local os arch
  case "$(uname -s)" in
    Darwin) os="macos" ;;
    Linux)  os="linux" ;;
    *)
      echo "error: unsupported OS '$(uname -s)' (only macOS and Linux are packaged)" >&2
      exit 1
      ;;
  esac
  case "$(uname -m)" in
    x86_64 | amd64)   arch="x86_64" ;;
    arm64 | aarch64)  arch="aarch64" ;;
    *)
      echo "error: unsupported architecture '$(uname -m)'" >&2
      exit 1
      ;;
  esac
  printf '%s-%s' "$os" "$arch"
}

# Resolve the release version. Honors OP_VERSION; otherwise queries the
# GitHub API for the latest release tag and strips the leading "v".
resolve_version() {
  if [ -n "${OP_VERSION:-}" ]; then
    printf '%s' "$OP_VERSION"
    return
  fi
  local api tag
  api="https://api.github.com/repos/${OWNER}/${REPO}/releases/latest"
  # Pull "tag_name": "vX.Y.Z" out of the JSON without a JSON parser.
  tag="$(curl -fsSL "$api" | grep -o '"tag_name"[[:space:]]*:[[:space:]]*"[^"]*"' | head -n1 | sed 's/.*"\(v\{0,1\}[^"]*\)"$/\1/')"
  if [ -z "$tag" ]; then
    echo "error: could not resolve the latest release tag from GitHub" >&2
    echo "       set OP_VERSION explicitly, e.g. OP_VERSION=0.8.0 $0" >&2
    exit 1
  fi
  printf '%s' "${tag#v}"
}

main() {
  local label version asset url tmp
  label="$(detect_label)"
  version="$(resolve_version)"
  asset="op-cli-${label}.tar.gz"
  url="https://github.com/${OWNER}/${REPO}/releases/download/v${version}/${asset}"

  echo "==> Installing op ${version} (${label})"
  echo "    from ${url}"

  tmp="$(mktemp -d)"
  # Always clean up the scratch dir, even on early exit.
  trap 'rm -rf "$tmp"' EXIT

  # Download and unpack. The tarball contains a single bare `op` binary.
  curl -fsSL --retry 3 -o "$tmp/${asset}" "$url"
  tar -xzf "$tmp/${asset}" -C "$tmp"
  if [ ! -f "$tmp/op" ]; then
    echo "error: ${asset} did not contain an 'op' binary" >&2
    exit 1
  fi
  chmod +x "$tmp/op"

  # Install — use sudo automatically only when the target dir is not
  # writable by the current user (e.g. the default /usr/local/bin).
  echo "==> Installing to ${INSTALL_DIR}/op"
  # Create the dir first (best effort) so the writability test below is
  # meaningful for a not-yet-existing custom INSTALL_DIR.
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
