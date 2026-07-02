; OpenPencil Windows installer (NSIS).
;
; Why NSIS (not Inno Setup): makensis is preinstalled on the GitHub
; windows-latest runner image, and the TS reference pipeline
; (apps/desktop/electron-builder.yml) also targets `nsis`, so installer UX
; stays consistent between the Electron and Rust shells.
;
; Installs:
;   openpencil-desktop.exe  editor binary
;   op.exe                  CLI binary (also shipped standalone as
;                           op-cli-<target>.zip in the release; $INSTDIR is
;                           NOT added to PATH — doing so reliably needs the
;                           non-stock EnVar plugin, deferred)
;   openpencil.ico          icon used by shortcuts + the .op/.pen ProgID
;   Uninstall.exe           uninstaller (registered in Add/Remove Programs)
;
; File association: ProgID "OpenPencil.Document" under HKCR with DefaultIcon
; and an open command, claimed by .op and .pen. Writing the machine hive
; requires elevation, hence RequestExecutionLevel admin + $PROGRAMFILES64 —
; an intentional divergence from electron-builder's per-user install
; (perMachine: false): a per-user HKCU\Software\Classes claim silently loses
; to any pre-existing machine-level registration. `.fig` is deliberately not
; claimed on Windows (macOS-only association, parity with electron-builder's
; fileAssociations list which only covers .op).
;
; Compile (relative paths resolve against this script's directory, so the
; workflow passes absolute /D defines):
;   makensis "/DVERSION=0.8.0" "/DARCH=x64" ^
;     "/DBIN_DIR=D:\w\target\x86_64-pc-windows-msvc\release" ^
;     "/DICON_FILE=D:\w\apps\desktop\build\icon.ico" ^
;     "/DOUT_FILE=D:\w\OpenPencil-0.8.0-x64-win-setup.exe" ^
;     scripts\package-windows.nsi
;
; NOT compiled locally (no makensis on the macOS dev machine) — first real
; verification is the tag-push CI run. For ARCH=arm64 the installer stub is
; x86 and runs under emulation on Windows-on-ARM; the payload binaries are
; native aarch64.
;
; VIProductVersion is intentionally omitted: it requires a strict 4-part
; numeric version and would break compiles for pre-release tags like
; 0.9.0-beta.1.

Unicode true

!include "MUI2.nsh"

!ifndef VERSION
  !define VERSION "0.0.0"
!endif
!ifndef ARCH
  !define ARCH "x64"
!endif
!ifndef BIN_DIR
  !define BIN_DIR "..\target\release"
!endif
!ifndef ICON_FILE
  !define ICON_FILE "..\apps\desktop\build\icon.ico"
!endif
!ifndef OUT_FILE
  !define OUT_FILE "OpenPencil-${VERSION}-${ARCH}-win-setup.exe"
!endif

!define PRODUCT_NAME "OpenPencil"
!define EXE_NAME "openpencil-desktop.exe"
!define CLI_NAME "op.exe"
!define PROG_ID "OpenPencil.Document"
!define REG_APP_KEY "Software\${PRODUCT_NAME}"
!define UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}"

Name "${PRODUCT_NAME}"
OutFile "${OUT_FILE}"
InstallDir "$PROGRAMFILES64\${PRODUCT_NAME}"
InstallDirRegKey HKLM "${REG_APP_KEY}" "InstallDir"
RequestExecutionLevel admin
SetCompressor /SOLID lzma

!define MUI_ICON "${ICON_FILE}"
!define MUI_UNICON "${ICON_FILE}"
!define MUI_ABORTWARNING

!insertmacro MUI_PAGE_WELCOME
; allowToChangeInstallationDirectory parity with electron-builder.yml
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"

Section "OpenPencil" SecMain
  SectionIn RO
  SetOutPath "$INSTDIR"

  File "${BIN_DIR}\${EXE_NAME}"
  File "${BIN_DIR}\${CLI_NAME}"
  File "/oname=openpencil.ico" "${ICON_FILE}"

  WriteUninstaller "$INSTDIR\Uninstall.exe"
  WriteRegStr HKLM "${REG_APP_KEY}" "InstallDir" "$INSTDIR"

  ; Shortcuts — createDesktopShortcut / createStartMenuShortcut parity.
  CreateDirectory "$SMPROGRAMS\${PRODUCT_NAME}"
  CreateShortcut "$SMPROGRAMS\${PRODUCT_NAME}\${PRODUCT_NAME}.lnk" \
    "$INSTDIR\${EXE_NAME}" "" "$INSTDIR\openpencil.ico"
  CreateShortcut "$DESKTOP\${PRODUCT_NAME}.lnk" \
    "$INSTDIR\${EXE_NAME}" "" "$INSTDIR\openpencil.ico"

  ; Add/Remove Programs entry.
  WriteRegStr HKLM "${UNINST_KEY}" "DisplayName" "${PRODUCT_NAME}"
  WriteRegStr HKLM "${UNINST_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKLM "${UNINST_KEY}" "DisplayIcon" "$INSTDIR\openpencil.ico"
  WriteRegStr HKLM "${UNINST_KEY}" "Publisher" "OpenPencil contributors"
  WriteRegStr HKLM "${UNINST_KEY}" "URLInfoAbout" "https://github.com/ZSeven-W/openpencil"
  WriteRegStr HKLM "${UNINST_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKLM "${UNINST_KEY}" "UninstallString" '"$INSTDIR\Uninstall.exe"'
  WriteRegDWORD HKLM "${UNINST_KEY}" "NoModify" 1
  WriteRegDWORD HKLM "${UNINST_KEY}" "NoRepair" 1

  ; File association: one ProgID claimed by both OpenPencil extensions.
  WriteRegStr HKCR "${PROG_ID}" "" "OpenPencil Document"
  WriteRegStr HKCR "${PROG_ID}\DefaultIcon" "" "$INSTDIR\openpencil.ico"
  WriteRegStr HKCR "${PROG_ID}\shell" "" "open"
  WriteRegStr HKCR "${PROG_ID}\shell\open\command" "" '"$INSTDIR\${EXE_NAME}" "%1"'

  WriteRegStr HKCR ".op" "" "${PROG_ID}"
  WriteRegStr HKCR ".op" "Content Type" "application/x-openpencil"
  WriteRegStr HKCR ".pen" "" "${PROG_ID}"
  WriteRegStr HKCR ".pen" "Content Type" "application/x-openpencil"

  ; SHCNE_ASSOCCHANGED — tell the shell to refresh icon/association caches.
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, i 0, i 0)'
SectionEnd

Section "Uninstall"
  Delete "$INSTDIR\${EXE_NAME}"
  Delete "$INSTDIR\${CLI_NAME}"
  Delete "$INSTDIR\openpencil.ico"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"

  Delete "$SMPROGRAMS\${PRODUCT_NAME}\${PRODUCT_NAME}.lnk"
  RMDir "$SMPROGRAMS\${PRODUCT_NAME}"
  Delete "$DESKTOP\${PRODUCT_NAME}.lnk"

  ; Only unclaim the extensions if they still point at our ProgID — never
  ; clobber an association another app took over after us.
  ReadRegStr $0 HKCR ".op" ""
  StrCmp $0 "${PROG_ID}" 0 +2
    DeleteRegKey HKCR ".op"
  ReadRegStr $0 HKCR ".pen" ""
  StrCmp $0 "${PROG_ID}" 0 +2
    DeleteRegKey HKCR ".pen"
  DeleteRegKey HKCR "${PROG_ID}"

  DeleteRegKey HKLM "${UNINST_KEY}"
  DeleteRegKey HKLM "${REG_APP_KEY}"

  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, i 0, i 0)'
SectionEnd
