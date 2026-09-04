[CmdletBinding()]
param(
  [switch]$SelfTest,
  [switch]$ValidateBuildToolset,
  [string]$Destination
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# Microsoft documents that the x64 bundle contains both x64 and ARM64 payloads.
$PinnedUrl = 'https://download.visualstudio.microsoft.com/download/pr/ebdab8e5-1d7b-4d9f-a11b-cbb1720c3b12/843068991DAAA1F73AD9F6239BCE4D0F6A07A51F18C37EA2A867E9BECA71295C/VC_redist.x64.exe'
$PinnedSha256 = '843068991daaa1f73ad9f6239bce4d0f6a07a51f18c37ea2a867e9beca71295c'
$PinnedFileVersion = '14.51.36247.0'
$PinnedProductVersion = '14.51.36247.0'

function Assert-RegularFile {
  param([string]$Path)

  $item = Get-Item -LiteralPath $Path -Force -ErrorAction Stop
  if ($item.PSIsContainer -or
      ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -or
      $item.Length -le 0) {
    throw "Asset is not a non-empty regular file: $Path"
  }
}

function Assert-Sha256 {
  param(
    [string]$Path,
    [string]$Expected
  )

  if ($Expected -cnotmatch '^[0-9a-f]{64}$') {
    throw 'Malformed pinned SHA-256'
  }
  Assert-RegularFile -Path $Path
  $actual = (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
  if ($actual -cne $Expected) {
    throw "SHA-256 mismatch for $Path"
  }
}

function Assert-ProductVersionValue {
  param(
    [AllowEmptyString()][string]$Actual,
    [string]$Expected
  )

  if ([string]::IsNullOrWhiteSpace($Expected)) {
    throw 'Pinned ProductVersion is required'
  }
  if ($Actual -cne $Expected) {
    throw "ProductVersion mismatch: expected $Expected, got $Actual"
  }
}

function Assert-FileVersionValue {
  param(
    [AllowEmptyString()][string]$Actual,
    [string]$Expected
  )

  if ([string]::IsNullOrWhiteSpace($Expected)) {
    throw 'Pinned FileVersion is required'
  }
  if ($Actual -cne $Expected) {
    throw "FileVersion mismatch: expected $Expected, got $Actual"
  }
}

function Assert-MicrosoftSignatureIdentity {
  param(
    [AllowEmptyString()][string]$Status,
    [AllowEmptyString()][string]$Subject,
    [AllowEmptyString()][string]$SimpleName
  )

  if ($Status -cne 'Valid') {
    throw "Authenticode signature is not valid: $Status"
  }
  if ($SimpleName -cne 'Microsoft Corporation' -or
      $Subject -cnotmatch '(?:^|,\s*)O=Microsoft Corporation(?:,|$)') {
    throw "Unexpected Authenticode signer: $Subject"
  }
}

function Assert-AuthenticodeSignature {
  param([string]$Path)

  $signature = Get-AuthenticodeSignature -LiteralPath $Path -ErrorAction Stop
  if ($null -eq $signature.SignerCertificate) {
    throw "Authenticode signer certificate is missing: $Path"
  }
  $simpleName = $signature.SignerCertificate.GetNameInfo(
    [Security.Cryptography.X509Certificates.X509NameType]::SimpleName,
    $false
  )
  Assert-MicrosoftSignatureIdentity `
    -Status ([string]$signature.Status) `
    -Subject ([string]$signature.SignerCertificate.Subject) `
    -SimpleName $simpleName
}

function Assert-VcRedistAsset {
  param(
    [string]$Path,
    [string]$ExpectedSha256,
    [string]$ExpectedFileVersion,
    [string]$ExpectedProductVersion
  )

  Assert-Sha256 -Path $Path -Expected $ExpectedSha256
  Assert-AuthenticodeSignature -Path $Path
  $versionInfo = [Diagnostics.FileVersionInfo]::GetVersionInfo($Path)
  Assert-FileVersionValue `
    -Actual $versionInfo.FileVersion `
    -Expected $ExpectedFileVersion
  Assert-ProductVersionValue `
    -Actual $versionInfo.ProductVersion `
    -Expected $ExpectedProductVersion
}

function Assert-Rejected {
  param(
    [string]$Name,
    [scriptblock]$Action
  )

  $rejected = $false
  try {
    & $Action
  } catch {
    $rejected = $true
  }
  if (-not $rejected) {
    throw "Self-test rejection failed: $Name"
  }
}

function Assert-ToolsetCoveredByRedist {
  param(
    [version]$ToolsetVersion,
    [version]$RedistVersion
  )

  if ($ToolsetVersion -gt $RedistVersion) {
    throw "MSVC toolset $ToolsetVersion is newer than pinned Visual C++ Redistributable $RedistVersion"
  }
}

function Assert-BuildToolsetsCovered {
  $vswherePath = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
  if (-not (Test-Path -LiteralPath $vswherePath -PathType Leaf)) {
    $vswhereCommand = Get-Command 'vswhere.exe' -CommandType Application -ErrorAction SilentlyContinue |
      Select-Object -First 1
    if ($null -eq $vswhereCommand) {
      throw 'vswhere.exe is required to validate the MSVC build toolset'
    }
    $vswherePath = $vswhereCommand.Source
  }

  $vswhereArgs = @(
    '-products', '*',
    '-requires', 'Microsoft.VisualStudio.Component.VC.Tools.x86.x64',
    '-property', 'installationPath'
  )
  $installations = @(& $vswherePath @vswhereArgs | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
  if ($LASTEXITCODE -ne 0 -or $installations.Count -eq 0) {
    throw 'No Visual Studio installation with the MSVC x86/x64 toolset was found'
  }

  $toolsets = @()
  foreach ($installation in $installations) {
    $toolsetRoot = Join-Path ($installation.Trim()) 'VC\Tools\MSVC'
    if (-not (Test-Path -LiteralPath $toolsetRoot -PathType Container)) {
      throw "Visual Studio MSVC toolset directory is missing: $toolsetRoot"
    }
    foreach ($directory in @(Get-ChildItem -LiteralPath $toolsetRoot -Directory -Force)) {
      try {
        $toolsetVersion = [version]$directory.Name
      } catch {
        throw "Unparseable MSVC toolset version directory: $($directory.FullName)"
      }
      $toolsets += [pscustomobject]@{
        Version = $toolsetVersion
        Path = $directory.FullName
      }
    }
  }
  if ($toolsets.Count -eq 0) {
    throw 'No installed MSVC toolset versions were found'
  }

  $redistVersion = [version]$PinnedFileVersion
  foreach ($toolset in $toolsets) {
    Assert-ToolsetCoveredByRedist `
      -ToolsetVersion $toolset.Version `
      -RedistVersion $redistVersion
  }
  $newest = $toolsets | Sort-Object -Property Version -Descending | Select-Object -First 1
  Write-Host "Pinned Visual C++ Redistributable $redistVersion covers newest installed MSVC toolset $($newest.Version)."
}

function Invoke-SelfTest {
  $temporary = Join-Path ([IO.Path]::GetTempPath()) (
    'openpencil-vcredist-selftest-' + [Guid]::NewGuid().ToString('N')
  )
  $testFile = Join-Path $temporary 'unsigned.exe'
  New-Item -ItemType Directory -Path $temporary | Out-Null
  try {
    [IO.File]::WriteAllBytes($testFile, [Text.Encoding]::ASCII.GetBytes('test'))
    $testSha256 = '9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08'
    Assert-Sha256 -Path $testFile -Expected $testSha256

    Assert-Rejected -Name 'malformed SHA-256' -Action {
      Assert-Sha256 -Path $testFile -Expected 'not-a-sha256'
    }
    Assert-Rejected -Name 'SHA-256 mismatch' -Action {
      Assert-Sha256 -Path $testFile -Expected ('0' * 64)
    }
    Assert-Rejected -Name 'directory accepted as a file' -Action {
      Assert-Sha256 -Path $temporary -Expected $testSha256
    }
    Assert-Rejected -Name 'unsigned asset' -Action {
      Assert-AuthenticodeSignature -Path $testFile
    }

    Assert-MicrosoftSignatureIdentity `
      -Status 'Valid' `
      -Subject 'CN=Microsoft Corporation, O=Microsoft Corporation, L=Redmond, S=Washington, C=US' `
      -SimpleName 'Microsoft Corporation'
    Assert-Rejected -Name 'invalid signature status' -Action {
      Assert-MicrosoftSignatureIdentity `
        -Status 'HashMismatch' `
        -Subject 'CN=Microsoft Corporation, O=Microsoft Corporation' `
        -SimpleName 'Microsoft Corporation'
    }
    Assert-Rejected -Name 'unexpected signature subject' -Action {
      Assert-MicrosoftSignatureIdentity `
        -Status 'Valid' `
        -Subject 'CN=Microsoft Corporation, O=Contoso Ltd' `
        -SimpleName 'Microsoft Corporation'
    }
    Assert-Rejected -Name 'unexpected signature common name' -Action {
      Assert-MicrosoftSignatureIdentity `
        -Status 'Valid' `
        -Subject 'CN=Contoso Ltd, O=Microsoft Corporation' `
        -SimpleName 'Contoso Ltd'
    }

    Assert-ProductVersionValue -Actual $PinnedProductVersion -Expected $PinnedProductVersion
    Assert-Rejected -Name 'ProductVersion mismatch' -Action {
      Assert-ProductVersionValue -Actual '0.0.0.0' -Expected $PinnedProductVersion
    }
    Assert-FileVersionValue -Actual $PinnedFileVersion -Expected $PinnedFileVersion
    Assert-Rejected -Name 'FileVersion mismatch' -Action {
      Assert-FileVersionValue -Actual '0.0.0.0' -Expected $PinnedFileVersion
    }
    Assert-ToolsetCoveredByRedist `
      -ToolsetVersion ([version]'14.51.36247') `
      -RedistVersion ([version]$PinnedProductVersion)
    Assert-Rejected -Name 'newer MSVC toolset' -Action {
      Assert-ToolsetCoveredByRedist `
        -ToolsetVersion ([version]'14.52.0') `
        -RedistVersion ([version]$PinnedProductVersion)
    }
  } finally {
    Remove-Item -LiteralPath $temporary -Recurse -Force -ErrorAction SilentlyContinue
  }
  Write-Output 'stage-pinned-vcredist.ps1: rejection self-tests passed.'
}

if ($env:OS -cne 'Windows_NT') {
  throw 'stage-pinned-vcredist.ps1 requires Windows'
}
if ($SelfTest -and $ValidateBuildToolset) {
  throw 'SelfTest and ValidateBuildToolset modes are mutually exclusive'
}
if (($SelfTest -or $ValidateBuildToolset) -and -not [string]::IsNullOrWhiteSpace($Destination)) {
  throw 'SelfTest, ValidateBuildToolset, and Destination modes are mutually exclusive'
}
if ($SelfTest) {
  Invoke-SelfTest
  exit 0
}
if ($ValidateBuildToolset) {
  Assert-BuildToolsetsCovered
  exit 0
}
if ([string]::IsNullOrWhiteSpace($Destination)) {
  throw 'Destination is required'
}

$destinationPath = [IO.Path]::GetFullPath($Destination)
$destinationParent = [IO.Path]::GetDirectoryName($destinationPath)
if ([string]::IsNullOrWhiteSpace($destinationParent) -or
    -not (Test-Path -LiteralPath $destinationParent -PathType Container)) {
  throw "Destination directory does not exist: $destinationParent"
}
if (Test-Path -LiteralPath $destinationPath) {
  $destinationItem = Get-Item -LiteralPath $destinationPath -Force
  if ($destinationItem.PSIsContainer -or
      ($destinationItem.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
    throw "Destination is not a regular file path: $destinationPath"
  }
}

$temporary = Join-Path ([IO.Path]::GetTempPath()) (
  'openpencil-vcredist-' + [Guid]::NewGuid().ToString('N')
)
$downloadPath = Join-Path $temporary 'VC_redist.x64.exe'
New-Item -ItemType Directory -Path $temporary | Out-Null
try {
  [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
  Invoke-WebRequest `
    -Uri $PinnedUrl `
    -OutFile $downloadPath `
    -MaximumRedirection 0 `
    -UseBasicParsing
  Assert-VcRedistAsset `
    -Path $downloadPath `
    -ExpectedSha256 $PinnedSha256 `
    -ExpectedFileVersion $PinnedFileVersion `
    -ExpectedProductVersion $PinnedProductVersion

  [IO.File]::Copy($downloadPath, $destinationPath, $true)
  Assert-VcRedistAsset `
    -Path $destinationPath `
    -ExpectedSha256 $PinnedSha256 `
    -ExpectedFileVersion $PinnedFileVersion `
    -ExpectedProductVersion $PinnedProductVersion
  Write-Output "Staged Microsoft Visual C++ Redistributable $PinnedProductVersion -> $destinationPath"
} finally {
  Remove-Item -LiteralPath $temporary -Recurse -Force -ErrorAction SilentlyContinue
}
