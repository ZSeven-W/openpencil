#!/usr/bin/env bash
# Produce a fully-wired macOS .app bundle for openpencil-desktop.
#
# `cargo-bundle` 0.10.0 ignores most of our `[package.metadata.bundle]`
# entries (CFBundleName / CFBundleIdentifier / CFBundleIconFile come
# out blank or set to the package name, Resources/ is never created,
# and the UTI-based file-association keys aren't emitted at all). On
# macOS 10.10+ the document-binding lookup is UTI-based (.fig files
# are tagged `com.figma.document`), so without `LSItemContentTypes` +
# an `UTImportedTypeDeclarations` claim the OS will not route .fig
# double-clicks to our bundle even if `CFBundleTypeExtensions`
# contains "fig".
#
# This script:
#   1. Builds the release binary.
#   2. Runs cargo-bundle.
#   3. Renames the produced `.app` to `OpenPencil.app`.
#   4. Backfills the Info.plist with CFBundleName / CFBundleIdentifier
#      / CFBundleIconFile / CFBundleShortVersionString /
#      CFBundleDocumentTypes (with UTI binding) /
#      UTImportedTypeDeclarations / local-network privacy metadata.
#   5. Copies the icon into Resources/.
#   6. Re-registers the bundle with LaunchServices so Finder picks up
#      the .fig binding.
#
# Idempotent: re-running on a refreshed binary overwrites the patched
# fields without compounding them (PlistBuddy `Set` replaces; `Add`
# would duplicate, so this script clears the entries it rewrites
# first).
#
# CI / cross-build controls (all optional, default = local behavior):
#   OPENPENCIL_VERSION     CFBundleShortVersionString (default: root Cargo
#                          workspace version; overrides must match it)
#   OPENPENCIL_TARGET      cargo target triple (e.g. x86_64-apple-darwin);
#                          builds + bundles for that triple and reads the
#                          bundle from target/<triple>/release/bundle/osx
#   OPENPENCIL_BINARY      path to a prebuilt release openpencil-desktop;
#                          skips this script's own cargo build and is copied
#                          over the bundled executable afterwards, so the
#                          shipped binary is EXACTLY the one CI built
#                          (cargo-bundle still runs `cargo build` internally,
#                          but with a warm target dir that is a no-op)
#   OPENPENCIL_CLI_BINARY  path to a prebuilt `op` CLI binary; embedded at
#                          Contents/MacOS/op so the .app/DMG ships the CLI
#   MACOS_SIGN_IDENTITY    codesign identity. Defaults to "-" (ad-hoc).
#                          When set to a Developer ID identity, the script
#                          signs binaries and the app bundle with hardened
#                          runtime + secure timestamp for notarization.

set -euo pipefail

WS_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=../tools/macos-local-network-plist.sh
source "$WS_ROOT/tools/macos-local-network-plist.sh"
CANONICAL_VERSION="$("$WS_ROOT/scripts/workspace-version.sh")"
APP_VERSION="${OPENPENCIL_VERSION:-$CANONICAL_VERSION}"
if [[ "$APP_VERSION" != "$CANONICAL_VERSION" ]]; then
  printf 'bundle-macos: error: OPENPENCIL_VERSION (%s) must match Cargo workspace version (%s)\n' \
    "$APP_VERSION" "$CANONICAL_VERSION" >&2
  exit 1
fi
if [[ "${OPENPENCIL_VALIDATE_VERSION_ONLY:-}" == 1 ]]; then
  printf '%s\n' "$APP_VERSION"
  exit 0
fi
TARGET_TRIPLE="${OPENPENCIL_TARGET:-}"

# Locate cargo-bundle. Tries PATH first, then a workspace-local
# fallback under `target/cargo-bundle-host/bin`. When neither
# resolves the script auto-installs into the fallback so a fresh CI
# checkout bootstraps without manual intervention. The install uses
# a script-local `CARGO_HOME` so the user's Cargo mirror config
# (the workspace pins a private USTC mirror) doesn't apply.
CARGO_BUNDLE_HOME="${CARGO_BUNDLE_HOME:-$WS_ROOT/target/cargo-bundle-host}"
if command -v cargo-bundle >/dev/null 2>&1; then
  CARGO_BUNDLE=cargo-bundle
elif [ -x "$CARGO_BUNDLE_HOME/bin/cargo-bundle" ]; then
  CARGO_BUNDLE="$CARGO_BUNDLE_HOME/bin/cargo-bundle"
else
  echo "==> bootstrapping cargo-bundle into $CARGO_BUNDLE_HOME"
  mkdir -p "$CARGO_BUNDLE_HOME"
  if ! CARGO_HOME="$CARGO_BUNDLE_HOME" cargo install cargo-bundle --locked >&2; then
    echo "error: failed to install cargo-bundle into $CARGO_BUNDLE_HOME" >&2
    echo "       try a hermetic CARGO_HOME by hand:" >&2
    echo "         CARGO_HOME=$CARGO_BUNDLE_HOME cargo install cargo-bundle --locked" >&2
    exit 1
  fi
  CARGO_BUNDLE="$CARGO_BUNDLE_HOME/bin/cargo-bundle"
fi

if [ -n "${OPENPENCIL_BINARY:-}" ]; then
  echo "==> skipping cargo build (prebuilt binary: $OPENPENCIL_BINARY)"
else
  echo "==> building release binary"
  ( cd "$WS_ROOT" && cargo build --release --bin openpencil-desktop \
      ${TARGET_TRIPLE:+--target "$TARGET_TRIPLE"} )
fi

echo "==> running cargo-bundle"
( cd "$WS_ROOT/crates/op-host-desktop" \
    && "$CARGO_BUNDLE" bundle --bin openpencil-desktop --release --format osx \
         ${TARGET_TRIPLE:+--target "$TARGET_TRIPLE"} )

if [ -n "$TARGET_TRIPLE" ]; then
  OUT_DIR="$WS_ROOT/target/$TARGET_TRIPLE/release/bundle/osx"
else
  OUT_DIR="$WS_ROOT/target/release/bundle/osx"
fi
RAW_APP="$OUT_DIR/op-host-desktop.app"
APP="$OUT_DIR/OpenPencil.app"

if [ ! -d "$RAW_APP" ]; then
  echo "error: cargo-bundle did not produce $RAW_APP" >&2
  exit 1
fi

echo "==> renaming to OpenPencil.app"
rm -rf "$APP"
mv "$RAW_APP" "$APP"

PLIST="$APP/Contents/Info.plist"

echo "==> patching Info.plist (name, identifier, icon, doc-types, UTI)"
# Set known fields (Set is idempotent).
/usr/libexec/PlistBuddy -c "Set :CFBundleName OpenPencil"                         "$PLIST"
/usr/libexec/PlistBuddy -c "Set :CFBundleDisplayName OpenPencil"                  "$PLIST"
/usr/libexec/PlistBuddy -c "Set :CFBundleIdentifier com.zseven-w.openpencil"      "$PLIST"
/usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $APP_VERSION"         "$PLIST"

# Clear-and-add the entries cargo-bundle leaves blank / wrong. Suppress
# stderr on Delete because the keys may or may not already exist.
/usr/libexec/PlistBuddy -c "Delete :CFBundleIconFile"               "$PLIST" 2>/dev/null || true
/usr/libexec/PlistBuddy -c "Add    :CFBundleIconFile string icon.icns" "$PLIST"

/usr/libexec/PlistBuddy -c "Delete :CFBundleDocumentTypes"         "$PLIST" 2>/dev/null || true
/usr/libexec/PlistBuddy -c "Add    :CFBundleDocumentTypes array"   "$PLIST"

# Group 0: .op / .pen (canonical OpenPencil documents).
/usr/libexec/PlistBuddy \
  -c "Add :CFBundleDocumentTypes:0 dict" \
  -c "Add :CFBundleDocumentTypes:0:CFBundleTypeName string 'OpenPencil Document'" \
  -c "Add :CFBundleDocumentTypes:0:CFBundleTypeRole string Editor" \
  -c "Add :CFBundleDocumentTypes:0:CFBundleTypeExtensions array" \
  -c "Add :CFBundleDocumentTypes:0:CFBundleTypeExtensions:0 string op" \
  -c "Add :CFBundleDocumentTypes:0:CFBundleTypeExtensions:1 string pen" \
  -c "Add :CFBundleDocumentTypes:0:LSHandlerRank string Owner" \
  "$PLIST"

# Group 1: .fig (Figma export) — UTI-based binding so Finder routes
# double-clicks even though the OS-assigned UTI is com.figma.document.
/usr/libexec/PlistBuddy \
  -c "Add :CFBundleDocumentTypes:1 dict" \
  -c "Add :CFBundleDocumentTypes:1:CFBundleTypeName string 'Figma Document'" \
  -c "Add :CFBundleDocumentTypes:1:CFBundleTypeRole string Editor" \
  -c "Add :CFBundleDocumentTypes:1:CFBundleTypeExtensions array" \
  -c "Add :CFBundleDocumentTypes:1:CFBundleTypeExtensions:0 string fig" \
  -c "Add :CFBundleDocumentTypes:1:LSItemContentTypes array" \
  -c "Add :CFBundleDocumentTypes:1:LSItemContentTypes:0 string com.figma.document" \
  -c "Add :CFBundleDocumentTypes:1:LSHandlerRank string Alternate" \
  "$PLIST"

# UTImportedTypeDeclarations — declare we KNOW about com.figma.document
# without forcing Figma.app to be installed. Required so the
# LSItemContentTypes claim above resolves on systems without Figma.
/usr/libexec/PlistBuddy -c "Delete :UTImportedTypeDeclarations" "$PLIST" 2>/dev/null || true
/usr/libexec/PlistBuddy \
  -c "Add :UTImportedTypeDeclarations array" \
  -c "Add :UTImportedTypeDeclarations:0 dict" \
  -c "Add :UTImportedTypeDeclarations:0:UTTypeIdentifier string com.figma.document" \
  -c "Add :UTImportedTypeDeclarations:0:UTTypeDescription string 'Figma Document'" \
  -c "Add :UTImportedTypeDeclarations:0:UTTypeConformsTo array" \
  -c "Add :UTImportedTypeDeclarations:0:UTTypeConformsTo:0 string public.data" \
  -c "Add :UTImportedTypeDeclarations:0:UTTypeTagSpecification dict" \
  -c "Add :UTImportedTypeDeclarations:0:UTTypeTagSpecification:public.filename-extension array" \
  -c "Add :UTImportedTypeDeclarations:0:UTTypeTagSpecification:public.filename-extension:0 string fig" \
  -c "Add :UTImportedTypeDeclarations:0:UTTypeTagSpecification:public.mime-type array" \
  -c "Add :UTImportedTypeDeclarations:0:UTTypeTagSpecification:public.mime-type:0 string application/x-figma" \
  "$PLIST"

echo "==> patching local-network privacy and Bonjour service metadata"
openpencil_apply_macos_local_network_plist "$PLIST"
bash "$WS_ROOT/tools/check-macos-bundle-plist.sh" "$PLIST"

echo "==> copying icon into Resources/"
mkdir -p "$APP/Contents/Resources"
cp "$WS_ROOT/crates/op-host-desktop/assets/icon.icns" "$APP/Contents/Resources/icon.icns"

if [ -n "${OPENPENCIL_BINARY:-}" ]; then
  echo "==> installing prebuilt binary into Contents/MacOS/"
  cp "$OPENPENCIL_BINARY" "$APP/Contents/MacOS/openpencil-desktop"
  chmod 755 "$APP/Contents/MacOS/openpencil-desktop"
fi

if [ -n "${OPENPENCIL_CLI_BINARY:-}" ]; then
  echo "==> embedding op CLI into Contents/MacOS/"
  cp "$OPENPENCIL_CLI_BINARY" "$APP/Contents/MacOS/op"
  chmod 755 "$APP/Contents/MacOS/op"
fi

# Sign the bundle. Ad-hoc ("-") by default — enough for local launch and
# for arm64 Macs which refuse fully unsigned code; Gatekeeper still warns
# on downloaded DMGs until real Developer ID signing + notarization land
# (set MACOS_SIGN_IDENTITY when the cert exists). Nested Mach-Os are
# signed first so we don't need the deprecated `--deep`.
SIGN_IDENTITY="${MACOS_SIGN_IDENTITY:--}"
echo "==> codesigning (identity: $SIGN_IDENTITY)"
sign_macos_code() {
  local path="$1"
  if [ "$SIGN_IDENTITY" = "-" ]; then
    codesign --force --sign "$SIGN_IDENTITY" "$path"
  else
    codesign --force --timestamp --options runtime --sign "$SIGN_IDENTITY" "$path"
  fi
}
if [ -f "$APP/Contents/MacOS/op" ]; then
  sign_macos_code "$APP/Contents/MacOS/op"
fi
sign_macos_code "$APP/Contents/MacOS/openpencil-desktop"
sign_macos_code "$APP"

echo "==> registering with LaunchServices"
LSREG=/System/Library/Frameworks/CoreServices.framework/Versions/A/Frameworks/LaunchServices.framework/Versions/A/Support/lsregister
"$LSREG" -u "$APP" 2>/dev/null || true
"$LSREG" -f "$APP" 2>/dev/null || true

echo ""
echo "Bundle ready: $APP"
echo "  open '$APP' to launch"
echo "  open -a '$APP' /path/to/file.fig  # routes through async Figma import"
