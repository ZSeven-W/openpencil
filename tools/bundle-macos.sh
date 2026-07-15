#!/bin/sh
# Wrap the built desktop binary in `OpenPencil.app` and code-sign it
# with the project's Developer ID so it carries the SAME signing
# identity as the shipped TS app (`dev.openpencil.app`, Team
# CYHUA8SZF6).
#
# Why the identity matters: macOS TCC keys folder-access grants
# (Desktop / Documents / Downloads) on an app's *designated
# requirement* — bundle id + signing team — not on the exact binary.
# A bare/ad-hoc binary has no durable identity, so it is denied; a
# bundle signed with the same Developer ID + bundle id as an
# already-granted app inherits that grant. This is why the TS app can
# commit a `.op` file on the Desktop and an unsigned dev build cannot.
#
# Usage: tools/bundle-macos.sh   (after `cargo build -p op-host-desktop --release`)
# Then launch via LaunchServices so the app is its own TCC responsible
# process:   open target/release/OpenPencil.app
#
# Signing identity is configurable:
#   OPENPENCIL_SIGN_ID="Developer ID Application: ... (TEAMID)"  (default below)
#   OPENPENCIL_SIGN_ID=-      ad-hoc sign (no durable identity / no grant)
# When the identity is not in the keychain the script warns and leaves
# the bundle linker-signed so it still runs locally.
set -e

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CANONICAL_VERSION="$("$ROOT/scripts/workspace-version.sh")"
APP_VERSION="${OPENPENCIL_VERSION:-$CANONICAL_VERSION}"
if [ "$APP_VERSION" != "$CANONICAL_VERSION" ]; then
  printf 'bundle-macos: error: OPENPENCIL_VERSION (%s) must match Cargo workspace version (%s)\n' \
    "$APP_VERSION" "$CANONICAL_VERSION" >&2
  exit 1
fi
BIN="$ROOT/target/release/openpencil-desktop"
ICON="$ROOT/crates/op-host-desktop/assets/icon.icns"
ENTITLEMENTS="$ROOT/crates/op-host-desktop/entitlements.plist"
APP="$ROOT/target/release/OpenPencil.app"

# Match the TS app exactly so the two share one TCC designated requirement.
BUNDLE_ID="dev.openpencil.app"
SIGN_ID="${OPENPENCIL_SIGN_ID:-Developer ID Application: Shanghai Yixing Intelligence Technology Co., Ltd. (CYHUA8SZF6)}"

if [ ! -x "$BIN" ]; then
  echo "bundle-macos: $BIN not found — run 'cargo build -p op-host-desktop --release' first" >&2
  exit 1
fi

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/openpencil-desktop"
cp "$ICON" "$APP/Contents/Resources/icon.icns"

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>OpenPencil</string>
  <key>CFBundleDisplayName</key><string>OpenPencil</string>
  <key>CFBundleExecutable</key><string>openpencil-desktop</string>
  <key>CFBundleIdentifier</key><string>${BUNDLE_ID}</string>
  <key>CFBundleAllowMixedLocalizations</key><true/>
  <key>CFBundleDevelopmentRegion</key><string>en</string>
  <key>CFBundleLocalizations</key>
  <array>
    <string>en</string>
    <string>zh-Hans</string>
    <string>zh-Hant</string>
    <string>ja</string>
    <string>ko</string>
    <string>fr</string>
    <string>es</string>
    <string>de</string>
    <string>pt</string>
    <string>ru</string>
    <string>hi</string>
    <string>tr</string>
    <string>th</string>
    <string>vi</string>
    <string>id</string>
  </array>
  <key>CFBundleIconFile</key><string>icon</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
  <key>CFBundleShortVersionString</key><string>${APP_VERSION}</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST

echo "bundle-macos: assembled $APP (bundle id $BUNDLE_ID)"

# --- Code-sign so the bundle's identity matches the TS app -----------
if [ "$SIGN_ID" = "-" ]; then
  echo "bundle-macos: ad-hoc signing (OPENPENCIL_SIGN_ID=-) — no TCC grant inheritance"
  codesign --force --options runtime --entitlements "$ENTITLEMENTS" --sign - "$APP"
elif security find-identity -v -p codesigning | grep -qF "$SIGN_ID"; then
  echo "bundle-macos: signing with \"$SIGN_ID\""
  # --options runtime → hardened runtime (matches TS flags=runtime).
  # entitlements relax library validation for the Homebrew OpenSSL dylibs.
  codesign --force --options runtime --entitlements "$ENTITLEMENTS" \
    --sign "$SIGN_ID" --timestamp=none "$APP"
  echo "bundle-macos: verifying signature"
  codesign --verify --strict --verbose=2 "$APP"
  codesign -dvv "$APP" 2>&1 | grep -E 'Identifier|Authority|TeamIdentifier|flags' || true
else
  echo "bundle-macos: WARNING signing identity not found in keychain:" >&2
  echo "  \"$SIGN_ID\"" >&2
  echo "  leaving bundle linker-signed (runs locally, but won't inherit the TS TCC grant)." >&2
fi

echo "bundle-macos: done — launch with: open \"$APP\""
