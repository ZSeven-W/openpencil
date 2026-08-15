[CmdletBinding()]
param([switch]$SelfTest)

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
  Write-Host 'install-pinned-nsis.ps1: checksum rejection self-test passed.'
}

if ($SelfTest) {
  Invoke-ChecksumSelfTest
  exit 0
}
if ([string]::IsNullOrWhiteSpace($env:RUNNER_TEMP)) {
  throw 'RUNNER_TEMP is required'
}

$packageVersion = '3.12.0'
$packageUrl = "https://community.chocolatey.org/api/v2/package/nsis.install/$packageVersion"
$packageSha256 = '4a1bbf9987e5b9b6bda4c2433af62bb79f2d9d3bd67b392f29a069ecda8c5f64'
$installerSha256 = '3bc2b06253a7e4957111be152ac6a536e0c7478a706e19da814038db5d706495'
$temporary = Join-Path $env:RUNNER_TEMP ("openpencil-nsis-" + [Guid]::NewGuid().ToString('N'))
$packagePath = Join-Path $temporary 'nsis.install.nupkg'
$installerPath = Join-Path $temporary 'nsis-3.12-setup.exe'

New-Item -ItemType Directory -Path $temporary | Out-Null
try {
  Invoke-WebRequest -Uri $packageUrl -OutFile $packagePath
  Assert-Sha256 $packagePath $packageSha256
  $archive = [IO.Compression.ZipFile]::OpenRead($packagePath)
  try {
    $entries = @($archive.Entries | Where-Object {
      $_.FullName -ceq 'tools/nsis-3.12-setup.exe'
    })
    if ($entries.Count -ne 1 -or $entries[0].Length -le 0) {
      throw 'Verified NSIS package does not contain one installer'
    }
    [IO.Compression.ZipFileExtensions]::ExtractToFile($entries[0], $installerPath, $false)
  } finally {
    $archive.Dispose()
  }
  Assert-Sha256 $installerPath $installerSha256
  $process = Start-Process -FilePath $installerPath -ArgumentList '/S' -Wait -PassThru
  if ($process.ExitCode -notin @(0, 3010)) {
    throw "NSIS installer exited with $($process.ExitCode)"
  }
} finally {
  Remove-Item -LiteralPath $temporary -Recurse -Force -ErrorAction SilentlyContinue
}

$roots = @("${env:ProgramFiles(x86)}\NSIS", "$env:ProgramFiles\NSIS")
$makensis = $null
foreach ($root in $roots) {
  $candidate = Join-Path $root 'makensis.exe'
  if (Test-Path -LiteralPath $candidate -PathType Leaf) {
    $makensis = $candidate
    break
  }
}
if (-not $makensis) {
  throw 'makensis.exe not found after the pinned NSIS install'
}
$installedVersion = (& $makensis /VERSION | Out-String).Trim()
if ($installedVersion -cnotmatch '^v3\.12(?:\D|$)') {
  throw "Unexpected makensis version: $installedVersion"
}
Split-Path $makensis | Out-File -FilePath $env:GITHUB_PATH -Append
Write-Host $installedVersion
