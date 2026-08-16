[CmdletBinding()]
param(
  [switch]$SelfTest,
  [string]$Architecture,
  [string]$Target
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Assert-Sha256 {
  param([string]$Path, [string]$Expected)
  if ($Expected -cnotmatch '^[0-9a-f]{64}$') {
    throw 'Malformed pinned SHA-256'
  }
  $item = Get-Item -LiteralPath $Path -Force
  if ($item.PSIsContainer -or ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
    throw "Downloaded asset is not a regular file: $Path"
  }
  $actual = (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
  if ($actual -cne $Expected) {
    throw "SHA-256 mismatch for $Path"
  }
}

function Invoke-ChecksumSelfTest {
  $path = [IO.Path]::GetTempFileName()
  try {
    [IO.File]::WriteAllBytes($path, [Text.Encoding]::ASCII.GetBytes('test'))
    Assert-Sha256 $path '9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08'
    $rejected = $false
    try {
      Assert-Sha256 $path ('0' * 64)
    } catch {
      $rejected = $true
    }
    if (-not $rejected) {
      throw 'Checksum mismatch was accepted'
    }
  } finally {
    Remove-Item -LiteralPath $path -Force -ErrorAction SilentlyContinue
  }
  Write-Host 'stage-pinned-angle.ps1: checksum rejection self-test passed.'
}

if ($SelfTest) {
  Invoke-ChecksumSelfTest
  exit 0
}
if ([string]::IsNullOrWhiteSpace($Architecture) -or [string]::IsNullOrWhiteSpace($Target)) {
  throw 'Architecture and Target are required'
}

$assets = @{
  x64 = @{
    Target = 'x86_64-pc-windows-msvc'
    Sha256 = '52bbe826b5e9d0dc779321866043d310aa8072d44ef3c05d7cdd3c4a69228fa0'
  }
  arm64 = @{
    Target = 'aarch64-pc-windows-msvc'
    Sha256 = '781209a26586dcb1e545335dc451479424e94407f73cc25696f0035a31273323'
  }
}
if (-not $assets.ContainsKey($Architecture)) {
  throw "Unsupported ANGLE architecture: $Architecture"
}
$asset = $assets[$Architecture]
if ($Target -cne $asset.Target) {
  throw "ANGLE architecture/target mismatch: $Architecture / $Target"
}
if ([string]::IsNullOrWhiteSpace($env:RUNNER_TEMP) -or
    [string]::IsNullOrWhiteSpace($env:GITHUB_WORKSPACE)) {
  throw 'RUNNER_TEMP and GITHUB_WORKSPACE are required'
}

$version = 'v33.0.0'
$url = "https://github.com/electron/electron/releases/download/$version/electron-$version-win32-$Architecture.zip"
$temporary = Join-Path $env:RUNNER_TEMP ("openpencil-angle-" + [Guid]::NewGuid().ToString('N'))
$zipPath = Join-Path $temporary 'electron.zip'
$destinationRoot = Join-Path $env:GITHUB_WORKSPACE "target\$Target\release"
if (-not (Test-Path -LiteralPath $destinationRoot -PathType Container)) {
  throw "ANGLE destination does not exist: $destinationRoot"
}

New-Item -ItemType Directory -Path $temporary | Out-Null
try {
  Invoke-WebRequest -Uri $url -OutFile $zipPath
  Assert-Sha256 $zipPath $asset.Sha256
  $archive = [IO.Compression.ZipFile]::OpenRead($zipPath)
  try {
    foreach ($dll in @('libEGL.dll', 'libGLESv2.dll', 'd3dcompiler_47.dll')) {
      $entries = @($archive.Entries | Where-Object { $_.FullName -ceq $dll })
      if ($entries.Count -ne 1 -or $entries[0].Length -le 0) {
        throw "Verified Electron archive does not contain one regular $dll"
      }
      $destination = Join-Path $destinationRoot $dll
      [IO.Compression.ZipFileExtensions]::ExtractToFile($entries[0], $destination, $true)
      Write-Host "Staged $dll -> $destinationRoot"
    }
  } finally {
    $archive.Dispose()
  }
} finally {
  Remove-Item -LiteralPath $temporary -Recurse -Force -ErrorAction SilentlyContinue
}
