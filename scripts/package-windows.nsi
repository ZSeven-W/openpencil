; OpenPencil Windows installer (NSIS).
;
; Why NSIS (not Inno Setup): the TS reference pipeline
; (apps/desktop/electron-builder.yml) also targets `nsis`, so installer UX
; stays consistent between the Electron and Rust shells. The release workflow
; installs NSIS explicitly before invoking makensis.
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
;   makensis "/DVERSION=X.Y.Z" "/DARCH=x64" ^
;     "/DBIN_DIR=D:\w\target\x86_64-pc-windows-msvc\release" ^
;     "/DICON_FILE=D:\w\crates\op-host-desktop\assets\icon.ico" ^
;     "/DVC_REDIST_FILE=D:\w\VC_redist.x64.exe" ^
;     "/DOUT_FILE=D:\w\OpenPencil-X.Y.Z-x64-win-setup.exe" ^
;     scripts\package-windows.nsi
;
; The script is compile-tested with pinned NSIS 3.12 for x64 and ARM64,
; including pre-release VERSION values and the real pinned redistributable.
; Windows CI remains responsible for native install/runtime verification. For
; ARCH=arm64 the installer stub is x86 and runs under emulation on
; Windows-on-ARM; the payload binaries are native aarch64.
;
; VIProductVersion is intentionally omitted: it requires a strict 4-part
; numeric version and would break compiles for pre-release tags like
; X.Y.Z-beta.1.

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
  !define ICON_FILE "..\crates\op-host-desktop\assets\icon.ico"
!endif
!ifndef OUT_FILE
  !define OUT_FILE "OpenPencil-${VERSION}-${ARCH}-win-setup.exe"
!endif
!ifndef VC_REDIST_FILE
  !error "VC_REDIST_FILE must point to the staged Microsoft Visual C++ Redistributable"
!endif

; This command intentionally omits /noerrors, so a missing file fails the
; package build. An existing PE without readable version metadata produces
; empty defines, which the explicit checks below also reject. /packed exposes
; the ProductVersion as HIGH and LOW DWORD defines for the runtime code below.
!getdllversion /packed /productversion "${VC_REDIST_FILE}" VC_REDIST_VERSION_
!if "${VC_REDIST_VERSION_HIGH}" == ""
  !error "VC_REDIST_FILE does not contain readable ProductVersion metadata"
!endif
!if "${VC_REDIST_VERSION_LOW}" == ""
  !error "VC_REDIST_FILE does not contain readable ProductVersion metadata"
!endif

!define PRODUCT_NAME "OpenPencil"
!define EXE_NAME "openpencil-desktop.exe"
!define CLI_NAME "op.exe"
!define PROG_ID "OpenPencil.Document"
!define REG_APP_KEY "Software\${PRODUCT_NAME}"
!define UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}"
!define VC_RUNTIME_REG_KEY "SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\${ARCH}"
!define VC_REDIST_EXE "vc_redist.${ARCH}.exe"
!define VC_REDIST_LOG "$TEMP\OpenPencil-vc-redist-${ARCH}.log"

Var VCRuntimeCompatible
Var VCRuntimeInstalled
Var VCRuntimeInstalledMajor
Var VCRuntimeInstalledMinor
Var VCRuntimeInstalledBuild
Var VCRuntimeInstalledRevision
Var VCRuntimeInstalledVersion
Var VCRuntimeRequiredHigh
Var VCRuntimeRequiredLow
Var VCRuntimeRequiredMajor
Var VCRuntimeRequiredMinor
Var VCRuntimeRequiredBuild
Var VCRuntimeRequiredRevision
Var VCRuntimeRequiredVersion
Var VCRuntimeExitCode

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

; Set VCRuntimeCompatible to 1 only when the native runtime for the payload
; architecture is installed and its four-part version is at least the bundled
; redistributable's ProductVersion. NSIS uses an x86 installer stub even for
; the ARM64 package, so SetRegView 64 is required to reach the native x64 or
; ARM64 view instead of the stub's redirected 32-bit view.
Function CheckVCRuntime
  StrCpy $VCRuntimeCompatible "0"
  StrCpy $VCRuntimeInstalledVersion "not installed"

  SetRegView 64
  ClearErrors
  ReadRegDWORD $VCRuntimeInstalled HKLM "${VC_RUNTIME_REG_KEY}" "Installed"
  IfErrors vc_runtime_check_done
  IntCmpU $VCRuntimeInstalled 1 vc_runtime_check_version \
    vc_runtime_check_done vc_runtime_check_done

  vc_runtime_check_version:
    ClearErrors
    ReadRegDWORD $VCRuntimeInstalledMajor HKLM "${VC_RUNTIME_REG_KEY}" "Major"
    IfErrors vc_runtime_check_done
    ReadRegDWORD $VCRuntimeInstalledMinor HKLM "${VC_RUNTIME_REG_KEY}" "Minor"
    IfErrors vc_runtime_check_done
    ReadRegDWORD $VCRuntimeInstalledBuild HKLM "${VC_RUNTIME_REG_KEY}" "Bld"
    IfErrors vc_runtime_check_done
    ReadRegDWORD $VCRuntimeInstalledRevision HKLM "${VC_RUNTIME_REG_KEY}" "Rbld"
    IfErrors vc_runtime_check_done

    StrCpy $VCRuntimeInstalledVersion \
      "$VCRuntimeInstalledMajor.$VCRuntimeInstalledMinor.$VCRuntimeInstalledBuild.$VCRuntimeInstalledRevision"

    ; Compare each component with unsigned ordering. This avoids signed DWORD
    ; surprises and does not rely on a lexicographic version-string comparison.
    IntCmpU $VCRuntimeInstalledMajor $VCRuntimeRequiredMajor \
      vc_runtime_check_minor vc_runtime_check_done vc_runtime_check_compatible
  vc_runtime_check_minor:
    IntCmpU $VCRuntimeInstalledMinor $VCRuntimeRequiredMinor \
      vc_runtime_check_build vc_runtime_check_done vc_runtime_check_compatible
  vc_runtime_check_build:
    IntCmpU $VCRuntimeInstalledBuild $VCRuntimeRequiredBuild \
      vc_runtime_check_revision vc_runtime_check_done vc_runtime_check_compatible
  vc_runtime_check_revision:
    IntCmpU $VCRuntimeInstalledRevision $VCRuntimeRequiredRevision \
      vc_runtime_check_compatible vc_runtime_check_done vc_runtime_check_compatible

  vc_runtime_check_compatible:
    StrCpy $VCRuntimeCompatible "1"

  vc_runtime_check_done:
    ; Restore the x86 stub's default view so the installer's existing registry
    ; writes keep their established location.
    SetRegView default
FunctionEnd

; Propagate the standard Windows success-with-reboot-required status to silent
; installers and callers that inspect the process exit code. The interactive
; MUI flow also uses the reboot flag to tell the user a restart is required.
Function .onInstSuccess
  IfRebootFlag 0 vc_runtime_no_reboot_exit
  SetErrorLevel 3010
  vc_runtime_no_reboot_exit:
FunctionEnd

Section "OpenPencil" SecMain
  SectionIn RO

  ; Embed the bundled redistributable's ProductVersion as two DWORDs, unpack it
  ; into the registry's Major/Minor/Bld/Rbld shape, and stage the prerequisite
  ; in the automatically cleaned NSIS plug-in directory. This all happens
  ; before any OpenPencil application file is written to $INSTDIR.
  StrCpy $VCRuntimeRequiredHigh "${VC_REDIST_VERSION_HIGH}"
  StrCpy $VCRuntimeRequiredLow "${VC_REDIST_VERSION_LOW}"
  IntOp $VCRuntimeRequiredMajor $VCRuntimeRequiredHigh >>> 16
  IntOp $VCRuntimeRequiredMinor $VCRuntimeRequiredHigh & 0x0000FFFF
  IntOp $VCRuntimeRequiredBuild $VCRuntimeRequiredLow >>> 16
  IntOp $VCRuntimeRequiredRevision $VCRuntimeRequiredLow & 0x0000FFFF
  StrCpy $VCRuntimeRequiredVersion \
    "$VCRuntimeRequiredMajor.$VCRuntimeRequiredMinor.$VCRuntimeRequiredBuild.$VCRuntimeRequiredRevision"

  InitPluginsDir
  SetOutPath "$PLUGINSDIR"
  File "/oname=${VC_REDIST_EXE}" "${VC_REDIST_FILE}"

  DetailPrint \
    "Checking Microsoft Visual C++ runtime ${ARCH} (required $VCRuntimeRequiredVersion)"
  Call CheckVCRuntime
  StrCmp $VCRuntimeCompatible "1" vc_runtime_ready

  DetailPrint \
    "Installing Microsoft Visual C++ Redistributable ${ARCH} (found $VCRuntimeInstalledVersion)"
  ClearErrors
  ExecWait \
    '"$PLUGINSDIR\${VC_REDIST_EXE}" /install /passive /norestart /log "${VC_REDIST_LOG}"' \
    $VCRuntimeExitCode
  IfErrors vc_runtime_launch_failed

  ; 0 is success, 3010 is success with a required reboot, and 1638 means a
  ; different product version is already installed. Every nominal success is
  ; verified against the registry; in particular, 1638 is never accepted on
  ; its exit code alone.
  StrCmp $VCRuntimeExitCode "0" vc_runtime_verify_success
  StrCmp $VCRuntimeExitCode "3010" vc_runtime_verify_reboot
  StrCmp $VCRuntimeExitCode "1638" vc_runtime_verify_existing
  Goto vc_runtime_install_failed

  vc_runtime_verify_success:
    Call CheckVCRuntime
    StrCmp $VCRuntimeCompatible "1" vc_runtime_install_succeeded \
      vc_runtime_verification_failed

  vc_runtime_verify_reboot:
    Call CheckVCRuntime
    StrCmp $VCRuntimeCompatible "1" 0 vc_runtime_verification_failed
    SetRebootFlag true
    DetailPrint \
      "Microsoft Visual C++ Redistributable installed; Windows restart required"
    Goto vc_runtime_ready

  vc_runtime_verify_existing:
    Call CheckVCRuntime
    StrCmp $VCRuntimeCompatible "1" vc_runtime_existing_compatible \
      vc_runtime_install_failed

  vc_runtime_install_succeeded:
    DetailPrint \
      "Microsoft Visual C++ runtime $VCRuntimeInstalledVersion installed successfully"
    Goto vc_runtime_ready

  vc_runtime_existing_compatible:
    DetailPrint \
      "Existing Microsoft Visual C++ runtime $VCRuntimeInstalledVersion is compatible"
    Goto vc_runtime_ready

  vc_runtime_launch_failed:
    MessageBox MB_OK|MB_ICONSTOP \
      "The Microsoft Visual C++ Redistributable could not be started.$\r$\n$\r$\nOpenPencil was not installed. See the log for details:$\r$\n${VC_REDIST_LOG}" \
      /SD IDOK
    Abort "Microsoft Visual C++ Redistributable could not be started."

  vc_runtime_verification_failed:
    MessageBox MB_OK|MB_ICONSTOP \
      "The Microsoft Visual C++ Redistributable returned exit code $VCRuntimeExitCode, but runtime $VCRuntimeRequiredVersion was not detected.$\r$\n$\r$\nOpenPencil was not installed. See the log for details:$\r$\n${VC_REDIST_LOG}" \
      /SD IDOK
    Abort "Microsoft Visual C++ Redistributable verification failed."

  vc_runtime_install_failed:
    MessageBox MB_OK|MB_ICONSTOP \
      "The Microsoft Visual C++ Redistributable installation failed with exit code $VCRuntimeExitCode.$\r$\n$\r$\nOpenPencil was not installed. See the log for details:$\r$\n${VC_REDIST_LOG}" \
      /SD IDOK
    Abort "Microsoft Visual C++ Redistributable installation failed."

  vc_runtime_ready:
    DetailPrint \
      "Microsoft Visual C++ runtime $VCRuntimeInstalledVersion satisfies $VCRuntimeRequiredVersion"

  SetOutPath "$INSTDIR"

  File "${BIN_DIR}\${EXE_NAME}"
  File "${BIN_DIR}\${CLI_NAME}"
  File "/oname=openpencil.ico" "${ICON_FILE}"

  ; ANGLE fallback DLLs (libEGL.dll + libGLESv2.dll, optionally
  ; d3dcompiler_47.dll). Installed next to the exe so glutin's EGL path
  ; loads them when the native WGL OpenGL context can't drive Skia — the
  ; machines that were flash-exiting on startup (no/old GPU driver,
  ; software-only OpenGL, VMs, RDP). See
  ; `SharedSkiaContext::new_desktop` for the fallback wiring.
  ;
  ; `/nonfatal`: the release/CI build must stage these DLLs into
  ; ${BIN_DIR} (matching the target arch) before running makensis. Until
  ; that step exists the installer still builds (just without the
  ; fallback), so packaging never hard-breaks on a missing DLL.
  File /nonfatal "${BIN_DIR}\libEGL.dll"
  File /nonfatal "${BIN_DIR}\libGLESv2.dll"
  File /nonfatal "${BIN_DIR}\d3dcompiler_47.dll"

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
