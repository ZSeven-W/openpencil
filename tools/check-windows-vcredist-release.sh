#!/usr/bin/env bash
# Structural checks for the Windows VC++ Runtime release contract.
# shellcheck disable=SC2016 # Source literals intentionally stay literal.

set -euo pipefail

script_dir=$(CDPATH='' cd "$(dirname "$0")" && pwd)
repo_root=$(CDPATH='' cd "$script_dir/.." && pwd)
vcredist_stager=${OPENPENCIL_VC_REDIST_STAGER:-$repo_root/tools/stage-pinned-vcredist.ps1}
windows_installer=${OPENPENCIL_WINDOWS_NSIS_INSTALLER:-$repo_root/scripts/package-windows.nsi}
cli_installer=${OPENPENCIL_WINDOWS_CLI_INSTALLER:-$repo_root/scripts/install-op.ps1}

for file in "$vcredist_stager" "$windows_installer" "$cli_installer"; do
    [[ -f "$file" && ! -L "$file" ]] || {
        printf 'error: missing Windows VC++ Runtime contract file: %s\n' "$file" >&2
        exit 1
    }
done

require_literal() {
    local literal=$1 file=$2
    grep -Fq -- "$literal" "$file" || {
        printf 'error: %s lacks required literal: %s\n' "$file" "$literal" >&2
        exit 1
    }
}

reject_literal() {
    local literal=$1 file=$2
    if grep -Fq -- "$literal" "$file"; then
        printf 'error: %s contains forbidden literal: %s\n' "$file" "$literal" >&2
        exit 1
    fi
}

require_count() {
    local expected=$1 literal=$2 file=$3 actual
    actual=$(grep -Fc -- "$literal" "$file" || true)
    [[ $actual -eq $expected ]] || {
        printf 'error: %s must contain %s copies of %s (found %s)\n' \
            "$file" "$expected" "$literal" "$actual" >&2
        exit 1
    }
}

require_literal '[switch]$SelfTest' "$vcredist_stager"
require_literal '[switch]$ValidateBuildToolset' "$vcredist_stager"
require_literal '[string]$Destination' "$vcredist_stager"
require_literal "throw 'Destination is required'" "$vcredist_stager"
require_literal 'Assert-Sha256' "$vcredist_stager"
require_literal 'Get-AuthenticodeSignature' "$vcredist_stager"
require_count 3 'Assert-VcRedistAsset' "$vcredist_stager"
require_literal '843068991daaa1f73ad9f6239bce4d0f6a07a51f18c37ea2a867e9beca71295c' "$vcredist_stager"
require_literal "\$PinnedFileVersion = '14.51.36247.0'" "$vcredist_stager"
require_literal "\$PinnedProductVersion = '14.51.36247.0'" "$vcredist_stager"
require_literal 'function Assert-FileVersionValue' "$vcredist_stager"
require_literal "Assert-Rejected -Name 'FileVersion mismatch'" "$vcredist_stager"
require_count 2 '-ExpectedFileVersion $PinnedFileVersion' "$vcredist_stager"
require_count 2 '-ExpectedProductVersion $PinnedProductVersion' "$vcredist_stager"
require_literal 'https://download.visualstudio.microsoft.com/download/pr/' "$vcredist_stager"
require_literal '-MaximumRedirection 0' "$vcredist_stager"
require_literal "\$Status -cne 'Valid'" "$vcredist_stager"
require_literal "\$SimpleName -cne 'Microsoft Corporation'" "$vcredist_stager"
require_literal 'Assert-Rejected -Name '\''unsigned asset'\''' "$vcredist_stager"
require_literal 'function Assert-ToolsetCoveredByRedist' "$vcredist_stager"
require_literal 'function Assert-BuildToolsetsCovered' "$vcredist_stager"
require_literal '$ToolsetVersion -gt $RedistVersion' "$vcredist_stager"
require_literal '$redistVersion = [version]$PinnedFileVersion' "$vcredist_stager"
require_literal 'is newer than pinned Visual C++ Redistributable' "$vcredist_stager"
require_literal "Assert-Rejected -Name 'newer MSVC toolset'" "$vcredist_stager"
require_literal "-ToolsetVersion ([version]'14.52.0')" "$vcredist_stager"
require_literal "throw 'vswhere.exe is required to validate the MSVC build toolset'" "$vcredist_stager"
require_literal "throw 'No installed MSVC toolset versions were found'" "$vcredist_stager"
require_literal 'Get-ChildItem -LiteralPath $toolsetRoot -Directory -Force' "$vcredist_stager"
require_literal 'if ($ValidateBuildToolset)' "$vcredist_stager"
require_count 2 'Assert-BuildToolsetsCovered' "$vcredist_stager"
require_literal 'SelfTest and ValidateBuildToolset modes are mutually exclusive' "$vcredist_stager"
reject_literal 'https://aka.ms/' "$vcredist_stager"

require_literal '!ifndef VC_REDIST_FILE' "$windows_installer"
require_literal '!error' "$windows_installer"
require_literal '!getdllversion /packed /productversion "${VC_REDIST_FILE}" VC_REDIST_VERSION_' "$windows_installer"
reject_literal '!getdllversion /noerrors' "$windows_installer"
require_count 2 'VC_REDIST_FILE does not contain readable ProductVersion metadata' "$windows_installer"
require_literal 'StrCpy $VCRuntimeRequiredHigh "${VC_REDIST_VERSION_HIGH}"' "$windows_installer"
require_literal 'StrCpy $VCRuntimeRequiredLow "${VC_REDIST_VERSION_LOW}"' "$windows_installer"
require_literal 'SetRegView 64' "$windows_installer"
require_literal 'SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\${ARCH}' "$windows_installer"
require_literal 'File "/oname=${VC_REDIST_EXE}" "${VC_REDIST_FILE}"' "$windows_installer"
reject_literal 'File /nonfatal "/oname=${VC_REDIST_EXE}"' "$windows_installer"
require_literal '/install /passive /norestart' "$windows_installer"
require_literal 'StrCmp $VCRuntimeExitCode "3010" vc_runtime_verify_reboot' "$windows_installer"
require_literal 'SetRebootFlag true' "$windows_installer"
require_literal 'Function .onInstSuccess' "$windows_installer"
require_literal 'IfRebootFlag 0 vc_runtime_no_reboot_exit' "$windows_installer"
require_literal 'SetErrorLevel 3010' "$windows_installer"
require_literal 'StrCmp $VCRuntimeExitCode "1638" vc_runtime_verify_existing' "$windows_installer"
require_count 4 'Call CheckVCRuntime' "$windows_installer"
require_literal 'IfErrors vc_runtime_launch_failed' "$windows_installer"
require_literal 'MB_ICONSTOP' "$windows_installer"
require_count 3 'Abort "Microsoft Visual C++ Redistributable' "$windows_installer"

require_literal '$VcRedistVersion = [version]"14.51.36247.0"' "$cli_installer"
require_literal '$VcRedistUrl = "https://aka.ms/vs/18/release/14.51.36247/VC_redist.x64.exe"' "$cli_installer"
require_literal '$VcRedistSha256 = "843068991daaa1f73ad9f6239bce4d0f6a07a51f18c37ea2a867e9beca71295c"' "$cli_installer"
require_literal 'function Get-InstalledVcRedistVersion' "$cli_installer"
require_literal '[Microsoft.Win32.RegistryView]::Registry64' "$cli_installer"
require_literal 'SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\$RuntimeArch' "$cli_installer"
require_literal '$env:PROCESSOR_ARCHITEW6432' "$cli_installer"
require_literal '[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12' "$cli_installer"
require_literal '$InstallerItem.PSIsContainer' "$cli_installer"
require_literal '[System.IO.FileAttributes]::ReparsePoint' "$cli_installer"
require_literal '$InstallerItem.Length -le 0' "$cli_installer"
require_literal '$FileVersion = ConvertTo-Version $VersionInfo.FileVersion' "$cli_installer"
require_literal '$ProductVersion = ConvertTo-Version $VersionInfo.ProductVersion' "$cli_installer"
require_literal '$null -eq $FileVersion -or $FileVersion -ne $VcRedistVersion' "$cli_installer"
require_literal '$null -eq $ProductVersion -or $ProductVersion -ne $VcRedistVersion' "$cli_installer"
require_literal 'Get-AuthenticodeSignature' "$cli_installer"
require_literal '[System.Management.Automation.SignatureStatus]::Valid' "$cli_installer"
require_literal 'GetNameInfo' "$cli_installer"
require_literal 'X509NameType]::SimpleName' "$cli_installer"
require_literal '$SignerName -cne "Microsoft Corporation"' "$cli_installer"
require_literal 'O=Microsoft Corporation' "$cli_installer"
require_literal '-Verb RunAs -Wait -PassThru' "$cli_installer"
require_literal '/install /passive /norestart /log' "$cli_installer"
require_literal '@(0, 3010, 1638)' "$cli_installer"
require_literal '$null -eq $InstalledVersion -or $InstalledVersion -lt $VcRedistVersion' "$cli_installer"
require_literal '$RebootRequired = $Process.ExitCode -eq 3010' "$cli_installer"
require_literal '$VcRedistRebootRequired = Install-VcRedistIfRequired' "$cli_installer"
require_literal 'if ($VcRedistRebootRequired)' "$cli_installer"
require_literal 'restart Windows before running op' "$cli_installer"
require_count 3 'Get-InstalledVcRedistVersion' "$cli_installer"

ruby - "$cli_installer" "$vcredist_stager" <<'RUBY'
source = File.read(ARGV.fetch(0))
service_start = source.index("function Install-VcRedistIfRequired")
service_end = source.index("function Resolve-Version", service_start)
service = source[service_start...service_end]
precheck = service.index("Get-InstalledVcRedistVersion")
launch = service.index("Start-Process", precheck)
postcheck = service.index("Get-InstalledVcRedistVersion", precheck + 1)
unless precheck < launch && launch < postcheck
  raise "CLI VC++ Runtime pre/install/post checks are out of order"
end
service_call = source.index('$VcRedistRebootRequired = Install-VcRedistIfRequired')
archive_download = source.index("Invoke-WebRequest -Uri $Url -OutFile $Archive")
unless service_call && archive_download && service_call < archive_download
  raise "CLI archive can install before its VC++ Runtime prerequisite"
end
reboot_branch = source.rindex('if ($VcRedistRebootRequired)')
otherwise = source.index("} else {", reboot_branch)
immediate_verify = source.index('& $Target --version', reboot_branch)
unless reboot_branch && otherwise && immediate_verify && otherwise < immediate_verify
  raise "CLI must not execute op immediately when the VC++ Runtime requires a reboot"
end

stager = File.read(ARGV.fetch(1))
self_test = stager.index('if ($SelfTest)')
toolset_gate = stager.index('if ($ValidateBuildToolset)')
download = stager.index("Invoke-WebRequest")
unless self_test && toolset_gate && download && self_test < download && toolset_gate < download
  raise "VC++ Runtime self-test/toolset validation modes must exit before production download"
end
RUBY

printf 'check-windows-vcredist-release.sh: Windows VC++ Runtime contracts passed.\n'
