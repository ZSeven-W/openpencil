# Shared helpers for Linux packaging (sourced by package-deb.sh and
# package-appimage.sh — not executable on its own, so no shebang / `set`).
#
# Branding parity with the TS reference (apps/desktop/electron-builder.yml):
#   productName  OpenPencil
#   mime type    application/x-openpencil  (covers .op and .pen)
#   categories   Graphics;
# Divergences (intentional):
#   * Exec is `openpencil-desktop %U`, not electron's `AppRun --no-sandbox %U`
#     — the Rust shell has no Chromium sandbox flag.
#   * Icon ships at its source size (1024px) into hicolor + pixmaps; older
#     hicolor index files may not list 1024x1024, so usr/share/pixmaps is the
#     universal fallback. No resize step: imagemagick is not guaranteed on
#     runners and the spec tolerates the fallback path.

OP_DESKTOP_BIN_NAME="openpencil-desktop"
OP_CLI_BIN_NAME="op"

# op_write_desktop_file <dest-path>
op_write_desktop_file() {
  cat > "$1" <<'EOF'
[Desktop Entry]
Name=OpenPencil
Comment=Open-source AI-native vector design tool
Exec=openpencil-desktop %U
Terminal=false
Type=Application
Icon=openpencil
StartupWMClass=openpencil
Categories=Graphics;
MimeType=application/x-openpencil;
EOF
}

# op_write_mime_xml <dest-path>
# shared-mime-info definition registering the .op / .pen extensions.
op_write_mime_xml() {
  cat > "$1" <<'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<mime-info xmlns="http://www.freedesktop.org/standards/shared-mime-info">
  <mime-type type="application/x-openpencil">
    <comment>OpenPencil Document</comment>
    <glob pattern="*.op"/>
    <glob pattern="*.pen"/>
    <icon name="openpencil"/>
  </mime-type>
</mime-info>
EOF
}

# op_install_usr_tree <root> <desktop-bin> <cli-bin> <icon-png>
# Lays out the usr/ tree shared by the .deb payload and the AppImage AppDir:
#   usr/bin/openpencil-desktop, usr/bin/op
#   usr/share/applications/openpencil.desktop
#   usr/share/mime/packages/openpencil.xml
#   usr/share/icons/hicolor/1024x1024/apps/openpencil.png
#   usr/share/pixmaps/openpencil.png
# Uses mkdir/cp/chmod (not `install -D`) so the layout can be dry-run on
# macOS, whose BSD install(1) lacks -D.
op_install_usr_tree() {
  local root="$1" desktop_bin="$2" cli_bin="$3" icon_png="$4"

  mkdir -p \
    "$root/usr/bin" \
    "$root/usr/share/applications" \
    "$root/usr/share/mime/packages" \
    "$root/usr/share/icons/hicolor/1024x1024/apps" \
    "$root/usr/share/pixmaps"

  cp "$desktop_bin" "$root/usr/bin/$OP_DESKTOP_BIN_NAME"
  cp "$cli_bin" "$root/usr/bin/$OP_CLI_BIN_NAME"
  chmod 755 "$root/usr/bin/$OP_DESKTOP_BIN_NAME" "$root/usr/bin/$OP_CLI_BIN_NAME"

  op_write_desktop_file "$root/usr/share/applications/openpencil.desktop"
  op_write_mime_xml "$root/usr/share/mime/packages/openpencil.xml"

  cp "$icon_png" "$root/usr/share/icons/hicolor/1024x1024/apps/openpencil.png"
  cp "$icon_png" "$root/usr/share/pixmaps/openpencil.png"
  chmod 644 \
    "$root/usr/share/icons/hicolor/1024x1024/apps/openpencil.png" \
    "$root/usr/share/pixmaps/openpencil.png"
}

# op_artifact_arch <amd64|arm64|x86_64|aarch64>
# Maps a deb/appimage arch onto the electron-builder artifact arch token
# (x64 / arm64) so release filenames match the TS pipeline's
# `${productName}-${version}-${arch}-linux.${ext}` convention.
op_artifact_arch() {
  case "$1" in
    amd64 | x86_64) echo "x64" ;;
    arm64 | aarch64) echo "arm64" ;;
    *)
      echo "error: unmapped architecture '$1'" >&2
      return 1
      ;;
  esac
}
