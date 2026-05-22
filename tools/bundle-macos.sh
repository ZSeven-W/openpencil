#!/bin/sh
# Wrap the built desktop binary in a minimal `OpenPencil.app` so
# macOS shows the proper Dock name + icon.
#
# A bare binary (`target/release/openpencil-desktop`) has no
# `Info.plist`, so the Dock falls back to the raw executable name
# and a generic icon. Running the binary from *inside* a `.app`
# directory makes macOS pick up the bundle's `CFBundleName` + icon
# even without `cargo bundle`.
#
# Usage: tools/bundle-macos.sh   (after `cargo build -p op-host-desktop --release`)
# Then run: target/release/OpenPencil.app/Contents/MacOS/openpencil-desktop
set -e

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="$ROOT/target/release/openpencil-desktop"
ICON="$ROOT/crates/op-host-desktop/assets/icon.icns"
APP="$ROOT/target/release/OpenPencil.app"

if [ ! -x "$BIN" ]; then
  echo "bundle-macos: $BIN not found — run 'cargo build -p op-host-desktop --release' first" >&2
  exit 1
fi

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/openpencil-desktop"
cp "$ICON" "$APP/Contents/Resources/icon.icns"

cat > "$APP/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>OpenPencil</string>
  <key>CFBundleDisplayName</key><string>OpenPencil</string>
  <key>CFBundleExecutable</key><string>openpencil-desktop</string>
  <key>CFBundleIdentifier</key><string>com.zseven-w.openpencil</string>
  <key>CFBundleIconFile</key><string>icon</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
  <key>CFBundleShortVersionString</key><string>0.8.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST

echo "bundle-macos: built $APP"
