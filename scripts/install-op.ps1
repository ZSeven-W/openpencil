param(
  [string]$OpVersion = $env:OP_VERSION,
  [switch]$PreRelease,
  [string]$InstallDir = $(if ($env:INSTALL_DIR) { $env:INSTALL_DIR } else { Join-Path $env:USERPROFILE ".openpencil\bin" })
)

$ErrorActionPreference = "Stop"
[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

$Owner = "ZSeven-W"
$Repo = "openpencil"
$DefaultOpVersion = ""
$DefaultShaWindowsAarch64 = ""
$DefaultShaWindowsX86_64 = ""
$VcRedistVersion = [version]"14.51.36247.0"
$VcRedistUrl = "https://aka.ms/vs/18/release/14.51.36247/VC_redist.x64.exe"
$VcRedistSha256 = "843068991daaa1f73ad9f6239bce4d0f6a07a51f18c37ea2a867e9beca71295c"

function ConvertTo-Version {
  param([object]$Value)

  if ($null -eq $Value) {
    return $null
  }

  $Text = ([string]$Value).Trim().TrimStart("v")
  try {
    return [version]$Text
  } catch {
    return $null
  }
}

function Get-InstalledVcRedistVersion {
  param([string]$RuntimeArch)

  $BaseKey = $null
  $RuntimeKey = $null
  try {
    $BaseKey = [Microsoft.Win32.RegistryKey]::OpenBaseKey(
      [Microsoft.Win32.RegistryHive]::LocalMachine,
      [Microsoft.Win32.RegistryView]::Registry64
    )
    $RuntimeKey = $BaseKey.OpenSubKey("SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\$RuntimeArch")
    if ($null -eq $RuntimeKey -or [int]$RuntimeKey.GetValue("Installed", 0) -ne 1) {
      return $null
    }

    $Version = ConvertTo-Version $RuntimeKey.GetValue("Version", $null)
    if ($null -ne $Version) {
      return $Version
    }

    $Major = [int]$RuntimeKey.GetValue("Major", 0)
    $Minor = [int]$RuntimeKey.GetValue("Minor", 0)
    $Build = [int]$RuntimeKey.GetValue("Bld", 0)
    $Revision = [int]$RuntimeKey.GetValue("Rbld", 0)
    if ($Major -gt 0) {
      return [version]::new($Major, $Minor, $Build, $Revision)
    }
    return $null
  } finally {
    if ($null -ne $RuntimeKey) {
      $RuntimeKey.Dispose()
    }
    if ($null -ne $BaseKey) {
      $BaseKey.Dispose()
    }
  }
}

function Install-VcRedistIfRequired {
  param(
    [string]$RuntimeArch,
    [string]$WorkingDirectory
  )

  $InstalledVersion = Get-InstalledVcRedistVersion $RuntimeArch
  if ($null -ne $InstalledVersion -and $InstalledVersion -ge $VcRedistVersion) {
    Write-Host "==> Microsoft Visual C++ runtime $InstalledVersion is already installed."
    return $false
  }

  if ($null -eq $InstalledVersion) {
    Write-Host "==> Microsoft Visual C++ runtime is missing; installing $VcRedistVersion."
  } else {
    Write-Host "==> Microsoft Visual C++ runtime $InstalledVersion is outdated; installing $VcRedistVersion."
  }

  $Installer = Join-Path $WorkingDirectory "VC_redist.x64.exe"
  $LogPath = Join-Path ([System.IO.Path]::GetTempPath()) ("openpencil-vc-redist-" + [System.Guid]::NewGuid().ToString("N") + ".log")
  Invoke-WebRequest -Uri $VcRedistUrl -OutFile $Installer -UseBasicParsing

  $InstallerItem = Get-Item -LiteralPath $Installer -Force -ErrorAction Stop
  if ($InstallerItem.PSIsContainer -or
      ($InstallerItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -or
      $InstallerItem.Length -le 0) {
    throw "install-op: Visual C++ runtime download did not produce a regular file"
  }

  $ActualSha = (Get-FileHash -Algorithm SHA256 -LiteralPath $Installer).Hash.ToLowerInvariant()
  if ($ActualSha -ne $VcRedistSha256) {
    throw "install-op: checksum mismatch for Visual C++ runtime. Expected $VcRedistSha256, got $ActualSha"
  }

  $VersionInfo = [System.Diagnostics.FileVersionInfo]::GetVersionInfo($Installer)
  $FileVersion = ConvertTo-Version $VersionInfo.FileVersion
  $ProductVersion = ConvertTo-Version $VersionInfo.ProductVersion
  if ($null -eq $FileVersion -or $FileVersion -ne $VcRedistVersion -or
      $null -eq $ProductVersion -or $ProductVersion -ne $VcRedistVersion) {
    throw "install-op: unexpected Visual C++ runtime version file=$FileVersion product=$ProductVersion (expected $VcRedistVersion)"
  }

  $Signature = Get-AuthenticodeSignature -LiteralPath $Installer
  $SignerName = if ($null -ne $Signature.SignerCertificate) {
    $Signature.SignerCertificate.GetNameInfo(
      [System.Security.Cryptography.X509Certificates.X509NameType]::SimpleName,
      $false
    )
  } else {
    ""
  }
  if ($Signature.Status -ne [System.Management.Automation.SignatureStatus]::Valid -or
      $null -eq $Signature.SignerCertificate -or
      $SignerName -cne "Microsoft Corporation" -or
      $Signature.SignerCertificate.Subject -notmatch '(^|,\s*)O=Microsoft Corporation(,|$)') {
    throw "install-op: Visual C++ runtime does not have a valid Microsoft Authenticode signature"
  }

  $Arguments = '/install /passive /norestart /log "{0}"' -f $LogPath
  try {
    $Process = Start-Process -FilePath $Installer -ArgumentList $Arguments -Verb RunAs -Wait -PassThru
  } catch {
    throw "install-op: Visual C++ runtime installation could not start: $($_.Exception.Message)"
  }

  if ($Process.ExitCode -notin @(0, 3010, 1638)) {
    throw "install-op: Visual C++ runtime installer exited with code $($Process.ExitCode). See $LogPath"
  }
  $RebootRequired = $Process.ExitCode -eq 3010

  $InstalledVersion = Get-InstalledVcRedistVersion $RuntimeArch
  if ($null -eq $InstalledVersion -or $InstalledVersion -lt $VcRedistVersion) {
    throw "install-op: Visual C++ runtime $VcRedistVersion was not registered after installation. See $LogPath"
  }

  Remove-Item -LiteralPath $LogPath -Force -ErrorAction SilentlyContinue
  if ($RebootRequired) {
    Write-Warning "Microsoft Visual C++ runtime $InstalledVersion was installed; restart Windows before running op."
    return $true
  }
  Write-Host "==> Microsoft Visual C++ runtime $InstalledVersion is ready."
  return $false
}

function Resolve-Version {
  if (-not [string]::IsNullOrWhiteSpace($OpVersion)) {
    return $OpVersion.TrimStart("v")
  }
  if (-not [string]::IsNullOrWhiteSpace($DefaultOpVersion)) {
    return $DefaultOpVersion.TrimStart("v")
  }

  $AllowPreRelease = $PreRelease.IsPresent -or $env:OP_PRERELEASE -in @("1", "true", "TRUE", "yes", "YES")
  if ($AllowPreRelease) {
    # Invoke-RestMethod emits a JSON array as a single pipeline object, so
    # `| Select-Object -First 1` yields the whole array and `.tag_name` then
    # member-enumerates every tag. Assign first, then index.
    $Releases = Invoke-RestMethod -Uri "https://api.github.com/repos/$Owner/$Repo/releases"
    $Latest = @($Releases)[0]
  } else {
    $Latest = Invoke-RestMethod -Uri "https://api.github.com/repos/$Owner/$Repo/releases/latest"
  }
  if (-not $Latest.tag_name) {
    throw "install-op: could not resolve latest release tag; set OP_VERSION explicitly or OP_PRERELEASE=1 to allow pre-release tags"
  }
  return $Latest.tag_name.TrimStart("v")
}

$NativeProcessorArchitecture = if ($env:PROCESSOR_ARCHITEW6432) {
  $env:PROCESSOR_ARCHITEW6432
} else {
  $env:PROCESSOR_ARCHITECTURE
}

switch ($NativeProcessorArchitecture) {
  "AMD64" {
    $Label = "windows-x86_64"
    $ExpectedSha = $DefaultShaWindowsX86_64
    $VcRuntimeArch = "x64"
  }
  "ARM64" {
    $Label = "windows-aarch64"
    $ExpectedSha = $DefaultShaWindowsAarch64
    $VcRuntimeArch = "arm64"
  }
  default {
    throw "install-op: unsupported Windows architecture $NativeProcessorArchitecture"
  }
}

$Version = Resolve-Version
$Asset = "op-cli-$Label.zip"
$Url = "https://github.com/$Owner/$Repo/releases/download/v$Version/$Asset"

Write-Host "==> Installing op $Version ($Label)"
Write-Host "    from $Url"

$Temp = Join-Path ([System.IO.Path]::GetTempPath()) ("openpencil-op-install-" + [System.Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $Temp | Out-Null
try {
  $VcRedistRebootRequired = Install-VcRedistIfRequired `
    -RuntimeArch $VcRuntimeArch `
    -WorkingDirectory $Temp

  $Archive = Join-Path $Temp $Asset
  Invoke-WebRequest -Uri $Url -OutFile $Archive -UseBasicParsing

  if (-not [string]::IsNullOrWhiteSpace($ExpectedSha)) {
    $ActualSha = (Get-FileHash -Algorithm SHA256 $Archive).Hash.ToLowerInvariant()
    if ($ActualSha -ne $ExpectedSha) {
      throw "install-op: checksum mismatch for $Asset. Expected $ExpectedSha, got $ActualSha"
    }
  }

  Expand-Archive -Path $Archive -DestinationPath $Temp -Force
  $Source = Get-ChildItem -Path $Temp -Filter "op.exe" -Recurse | Select-Object -First 1
  if (-not $Source) {
    throw "install-op: op.exe was not found in $Asset"
  }

  New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
  $Target = Join-Path $InstallDir "op.exe"
  Copy-Item -Path $Source.FullName -Destination $Target -Force

  $UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
  $PathEntries = @($UserPath -split ";" | Where-Object { $_ })
  if ($PathEntries -notcontains $InstallDir) {
    [Environment]::SetEnvironmentVariable("Path", (($PathEntries + $InstallDir) -join ";"), "User")
    Write-Host "Added $InstallDir to the user PATH. Restart the shell to use op globally."
  }

  if ($VcRedistRebootRequired) {
    Write-Host "==> op $Version is installed. Restart Windows, then run 'op --version' to verify."
  } else {
    Write-Host "==> Done. Run 'op --version' to verify."
    & $Target --version
  }
} finally {
  Remove-Item -Path $Temp -Recurse -Force -ErrorAction SilentlyContinue
}
