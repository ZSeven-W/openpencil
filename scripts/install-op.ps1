param(
  [string]$OpVersion = $env:OP_VERSION,
  [switch]$PreRelease,
  [string]$InstallDir = $(if ($env:INSTALL_DIR) { $env:INSTALL_DIR } else { Join-Path $env:USERPROFILE ".openpencil\bin" })
)

$ErrorActionPreference = "Stop"

$Owner = "ZSeven-W"
$Repo = "openpencil"
$DefaultOpVersion = ""
$DefaultShaWindowsAarch64 = ""
$DefaultShaWindowsX86_64 = ""

function Resolve-Version {
  if (-not [string]::IsNullOrWhiteSpace($OpVersion)) {
    return $OpVersion.TrimStart("v")
  }
  if (-not [string]::IsNullOrWhiteSpace($DefaultOpVersion)) {
    return $DefaultOpVersion.TrimStart("v")
  }

  $AllowPreRelease = $PreRelease.IsPresent -or $env:OP_PRERELEASE -in @("1", "true", "TRUE", "yes", "YES")
  if ($AllowPreRelease) {
    $Latest = Invoke-RestMethod -Uri "https://api.github.com/repos/$Owner/$Repo/releases" | Select-Object -First 1
  } else {
    $Latest = Invoke-RestMethod -Uri "https://api.github.com/repos/$Owner/$Repo/releases/latest"
  }
  if (-not $Latest.tag_name) {
    throw "install-op: could not resolve latest release tag; set OP_VERSION explicitly or OP_PRERELEASE=1 to allow pre-release tags"
  }
  return $Latest.tag_name.TrimStart("v")
}

switch ($env:PROCESSOR_ARCHITECTURE) {
  "AMD64" {
    $Label = "windows-x86_64"
    $ExpectedSha = $DefaultShaWindowsX86_64
  }
  "ARM64" {
    $Label = "windows-aarch64"
    $ExpectedSha = $DefaultShaWindowsAarch64
  }
  default {
    throw "install-op: unsupported Windows architecture $env:PROCESSOR_ARCHITECTURE"
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

  Write-Host "==> Done. Run 'op --version' to verify."
  & $Target --version
} finally {
  Remove-Item -Path $Temp -Recurse -Force -ErrorAction SilentlyContinue
}
