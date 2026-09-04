#!/usr/bin/env bash
# Build a Debian package for the OpenPencil Rust desktop shell + `op` CLI.
#
# Payload:
#   usr/bin/openpencil-desktop          editor binary
#   usr/bin/op                          CLI binary
#   usr/share/applications/openpencil.desktop
#       Desktop entry with MimeType=application/x-openpencil so .op/.pen
#       double-clicks route to the editor after update-desktop-database.
#   usr/share/mime/packages/openpencil.xml
#       shared-mime-info definition registering the .op/.pen globs.
#   usr/share/icons/hicolor/1024x1024/apps/openpencil.png (+ pixmaps fallback)
#   DEBIAN/control, postinst, postrm
#       postinst refreshes the mime / desktop / icon caches (guarded — the
#       tools are Recommends-level, absent on minimal containers).
#
# Usage:
#   package-deb.sh --desktop-bin PATH --cli-bin PATH --icon PATH \
#                  --version X.Y.Z --arch amd64|arm64 --out-dir DIR \
#                  [--layout-only]
#
#   --layout-only  stage and print the tree without invoking dpkg-deb, so the
#                  layout logic can be smoke-tested on machines without dpkg
#                  (e.g. macOS dev boxes). CI always does the full build.
#
# Output: <out-dir>/OpenPencil-<version>-<x64|arm64>-linux.deb
# (filename matches electron-builder's artifactName pattern
#  `${productName}-${version}-${arch}-linux.${ext}`; the Debian *control*
#  Architecture field still uses proper dpkg arch names amd64/arm64).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=package-linux-common.sh
source "$SCRIPT_DIR/package-linux-common.sh"

DESKTOP_BIN="" CLI_BIN="" ICON_PNG="" VERSION="" DEB_ARCH="" OUT_DIR=""
LAYOUT_ONLY=0

usage() {
  sed -n '2,28p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
  exit "${1:-1}"
}

while [ $# -gt 0 ]; do
  case "$1" in
    --desktop-bin) DESKTOP_BIN="$2"; shift 2 ;;
    --cli-bin)     CLI_BIN="$2"; shift 2 ;;
    --icon)        ICON_PNG="$2"; shift 2 ;;
    --version)     VERSION="$2"; shift 2 ;;
    --arch)        DEB_ARCH="$2"; shift 2 ;;
    --out-dir)     OUT_DIR="$2"; shift 2 ;;
    --layout-only) LAYOUT_ONLY=1; shift ;;
    -h|--help)     usage 0 ;;
    *) echo "error: unknown argument '$1'" >&2; usage ;;
  esac
done

for req in DESKTOP_BIN CLI_BIN ICON_PNG VERSION DEB_ARCH OUT_DIR; do
  if [ -z "${!req}" ]; then
    echo "error: --$(echo "$req" | tr '[:upper:]_' '[:lower:]-') is required" >&2
    usage
  fi
done
for f in "$DESKTOP_BIN" "$CLI_BIN" "$ICON_PNG"; do
  [ -f "$f" ] || { echo "error: input file not found: $f" >&2; exit 1; }
done
case "$DEB_ARCH" in
  amd64 | arm64) ;;
  *) echo "error: --arch must be amd64 or arm64 (got '$DEB_ARCH')" >&2; exit 1 ;;
esac

ARTIFACT_ARCH="$(op_artifact_arch "$DEB_ARCH")"
STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT

echo "==> staging payload tree"
op_install_usr_tree "$STAGE" "$DESKTOP_BIN" "$CLI_BIN" "$ICON_PNG"

echo "==> writing DEBIAN/ metadata"
mkdir -p "$STAGE/DEBIAN"

# Installed-Size is in KiB of the payload (excludes DEBIAN/).
INSTALLED_SIZE="$(du -sk "$STAGE/usr" | cut -f1)"

cat > "$STAGE/DEBIAN/control" <<EOF
Package: openpencil
Version: $VERSION
Section: graphics
Priority: optional
Architecture: $DEB_ARCH
Maintainer: ZSeven-W <fini.yang@gmail.com>
Installed-Size: $INSTALLED_SIZE
Depends: libc6, libssl3, libfontconfig1, libfreetype6, libegl1, libgles2, libgbm1, libxkbcommon0, libxkbcommon-x11-0, libwayland-client0
Recommends: shared-mime-info, desktop-file-utils
Homepage: https://github.com/ZSeven-W/openpencil
Description: OpenPencil — open-source AI-native vector design tool
 Native desktop editor (openpencil-desktop) built on the Rust shell, plus
 the op command-line client that drives the editor over MCP.
 Registers the application/x-openpencil MIME type for .op and .pen files.
EOF

cat > "$STAGE/DEBIAN/postinst" <<'EOF'
#!/bin/sh
set -e
# Refresh caches so the .op/.pen association is live immediately. Each tool
# is optional (Recommends), so guard and never fail the install on them.
if command -v update-mime-database >/dev/null 2>&1; then
  update-mime-database /usr/share/mime || true
fi
if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database /usr/share/applications || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -q /usr/share/icons/hicolor || true
fi
exit 0
EOF

cat > "$STAGE/DEBIAN/postrm" <<'EOF'
#!/bin/sh
set -e
if command -v update-mime-database >/dev/null 2>&1; then
  update-mime-database /usr/share/mime || true
fi
if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database /usr/share/applications || true
fi
exit 0
EOF

chmod 755 "$STAGE/DEBIAN/postinst" "$STAGE/DEBIAN/postrm"

if [ "$LAYOUT_ONLY" = 1 ]; then
  echo "==> layout-only mode: staged tree"
  (cd "$STAGE" && find . \( -type f -o -type l \) | sort)
  echo "==> DEBIAN/control"
  cat "$STAGE/DEBIAN/control"
  exit 0
fi

command -v dpkg-deb >/dev/null 2>&1 || {
  echo "error: dpkg-deb not found — run on a Debian/Ubuntu host or pass --layout-only" >&2
  exit 1
}

mkdir -p "$OUT_DIR"
DEB_PATH="$OUT_DIR/OpenPencil-$VERSION-$ARTIFACT_ARCH-linux.deb"
echo "==> dpkg-deb build"
# --root-owner-group: payload owned by root:root without needing fakeroot
# (dpkg-deb >= 1.19; Ubuntu 24.04 runners ship 1.22).
dpkg-deb --build --root-owner-group "$STAGE" "$DEB_PATH"
echo "Package ready: $DEB_PATH"
