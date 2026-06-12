#!/usr/bin/env bash
# Assemble an AppImage for the OpenPencil Rust desktop shell + `op` CLI.
#
# AppDir layout (usr/ tree shared with the .deb via package-linux-common.sh):
#   AppRun -> usr/bin/openpencil-desktop   (symlink entrypoint)
#   openpencil.desktop, openpencil.png, .DirIcon   (AppImage root metadata)
#   usr/bin/{openpencil-desktop,op}
#   usr/share/{applications,mime/packages,icons,pixmaps}/...
#
# The actual packing tool (appimagetool) is downloaded pinned-by-URL by the
# workflow and passed in via --tool; this script only assembles the AppDir
# and invokes it. Without --tool (or with --layout-only) it stops after
# assembly and prints the tree — that is the only mode runnable on macOS,
# since appimagetool is a Linux ELF.
#
# NOTE: plain appimagetool does NOT bundle shared libraries (unlike
# linuxdeploy). The produced AppImage links against the build host's glibc /
# mesa / fontconfig, i.e. it runs on distros at least as new as the runner
# image (ubuntu-24.04). Acceptable for now; switch to linuxdeploy if older
# distro coverage becomes a requirement.
#
# Usage:
#   package-appimage.sh --desktop-bin PATH --cli-bin PATH --icon PATH \
#                       --version X.Y.Z --arch x86_64|aarch64 \
#                       --out-dir DIR [--tool PATH] [--layout-only]
#
# Output: <out-dir>/OpenPencil-<version>-<x64|arm64>-linux.AppImage
# (matches electron-builder's `${productName}-${version}-${arch}-linux.${ext}`)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=package-linux-common.sh
source "$SCRIPT_DIR/package-linux-common.sh"

DESKTOP_BIN="" CLI_BIN="" ICON_PNG="" VERSION="" AI_ARCH="" OUT_DIR="" TOOL=""
LAYOUT_ONLY=0

usage() {
  sed -n '2,29p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
  exit "${1:-1}"
}

while [ $# -gt 0 ]; do
  case "$1" in
    --desktop-bin) DESKTOP_BIN="$2"; shift 2 ;;
    --cli-bin)     CLI_BIN="$2"; shift 2 ;;
    --icon)        ICON_PNG="$2"; shift 2 ;;
    --version)     VERSION="$2"; shift 2 ;;
    --arch)        AI_ARCH="$2"; shift 2 ;;
    --out-dir)     OUT_DIR="$2"; shift 2 ;;
    --tool)        TOOL="$2"; shift 2 ;;
    --layout-only) LAYOUT_ONLY=1; shift ;;
    -h|--help)     usage 0 ;;
    *) echo "error: unknown argument '$1'" >&2; usage ;;
  esac
done

for req in DESKTOP_BIN CLI_BIN ICON_PNG VERSION AI_ARCH OUT_DIR; do
  if [ -z "${!req}" ]; then
    echo "error: --$(echo "$req" | tr '[:upper:]_' '[:lower:]-') is required" >&2
    usage
  fi
done
for f in "$DESKTOP_BIN" "$CLI_BIN" "$ICON_PNG"; do
  [ -f "$f" ] || { echo "error: input file not found: $f" >&2; exit 1; }
done
case "$AI_ARCH" in
  x86_64 | aarch64) ;;
  *) echo "error: --arch must be x86_64 or aarch64 (got '$AI_ARCH')" >&2; exit 1 ;;
esac

ARTIFACT_ARCH="$(op_artifact_arch "$AI_ARCH")"
STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT
APPDIR="$STAGE/OpenPencil.AppDir"
mkdir -p "$APPDIR"

echo "==> staging AppDir"
op_install_usr_tree "$APPDIR" "$DESKTOP_BIN" "$CLI_BIN" "$ICON_PNG"

# AppImage root metadata: desktop entry + icon must sit at AppDir root;
# .DirIcon is what most file managers display for the mounted image.
cp "$APPDIR/usr/share/applications/openpencil.desktop" "$APPDIR/openpencil.desktop"
cp "$ICON_PNG" "$APPDIR/openpencil.png"
cp "$ICON_PNG" "$APPDIR/.DirIcon"
ln -sf usr/bin/openpencil-desktop "$APPDIR/AppRun"

if [ "$LAYOUT_ONLY" = 1 ] || [ -z "$TOOL" ]; then
  echo "==> layout-only mode: assembled AppDir"
  (cd "$APPDIR" && find . \( -type f -o -type l \) | sort)
  if [ -z "$TOOL" ] && [ "$LAYOUT_ONLY" = 0 ]; then
    echo "note: no --tool given; skipped AppImage packing" >&2
  fi
  exit 0
fi

[ -x "$TOOL" ] || { echo "error: appimagetool not executable: $TOOL" >&2; exit 1; }

mkdir -p "$OUT_DIR"
OUT_PATH="$OUT_DIR/OpenPencil-$VERSION-$ARTIFACT_ARCH-linux.AppImage"
echo "==> packing AppImage ($AI_ARCH)"
# --appimage-extract-and-run: appimagetool itself ships as an AppImage and
# CI runners have no FUSE; this flag self-extracts instead of mounting.
# ARCH is required when the tool cannot infer the target architecture.
ARCH="$AI_ARCH" "$TOOL" --appimage-extract-and-run "$APPDIR" "$OUT_PATH"
echo "AppImage ready: $OUT_PATH"
