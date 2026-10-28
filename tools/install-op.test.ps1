# Run with: pwsh -NoProfile -File tools/install-op.test.ps1
# Extract only the resolver: no downloads, registry access, or installation runs.
$ErrorActionPreference = "Stop"
$Installer = Join-Path (Split-Path $PSScriptRoot -Parent) "scripts/install-op.ps1"
$Tokens = $null
$ParseErrors = $null
$Ast = [System.Management.Automation.Language.Parser]::ParseFile(
  $Installer, [ref]$Tokens, [ref]$ParseErrors
)
if ($ParseErrors.Count -ne 0) {
  throw "Installer parse failed: $($ParseErrors -join '; ')"
}
$Resolver = $Ast.Find({
  param($Node)
  $Node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and
    $Node.Name -eq "Resolve-Version"
}, $true)
if ($null -eq $Resolver) {
  throw "Resolve-Version was not found in the installer"
}
$Definition = [scriptblock]::Create($Resolver.Extent.Text)

class InstallerLookupHttpException : System.Exception {
  [object]$Response
  InstallerLookupHttpException([int]$Status) : base("HTTP $Status") {
    $this.Response = [pscustomobject]@{ StatusCode = $Status }
  }
}

$script:CaseCount = 0
function Assert-Equal {
  param($Actual, $Expected, [string]$Context)
  if ($Actual -cne $Expected) {
    throw "${Context}: expected '$Expected', got '$Actual'"
  }
}

function Test-ResolveVersion {
  param(
    [string]$Name,
    [object]$Payload = ([pscustomobject]@{ tag_name = "v1.2.3" }),
    [int]$HttpStatus = 200,
    [switch]$NetworkError,
    [string]$Version = "",
    [string]$EmbeddedVersion = "",
    [switch]$AllowPreRelease,
    [string]$PreReleaseEnv = "",
    [string]$ExpectedVersion = "",
    [string[]]$ExpectedErrors = @(),
    [string]$ExpectedEndpoint = "latest"
  )

  $Calls = [System.Collections.Generic.List[string]]::new()
  $SavedPreRelease = $env:OP_PRERELEASE
  try {
    $env:OP_PRERELEASE = $PreReleaseEnv
    # A child scope keeps the mock and extracted function local to this case.
    $Result = & {
      $Owner = "ZSeven-W"
      $Repo = "openpencil"
      $OpVersion = $Version
      $DefaultOpVersion = $EmbeddedVersion
      $PreRelease = [switch]$AllowPreRelease

      function Invoke-RestMethod {
        param([string]$Uri)
        $Calls.Add($Uri)
        if ($NetworkError) {
          throw [System.Exception]::new("Connection refused")
        }
        if ($HttpStatus -ne 200) {
          throw [InstallerLookupHttpException]::new($HttpStatus)
        }
        # Real Invoke-RestMethod returns a release array as one pipeline object.
        Write-Output -NoEnumerate $Payload
      }

      . $Definition
      try {
        [pscustomobject]@{ Value = @(Resolve-Version); Error = $null }
      } catch {
        [pscustomobject]@{ Value = @(); Error = $_.Exception.Message }
      }
    }
  } finally {
    $env:OP_PRERELEASE = $SavedPreRelease
  }

  if ($ExpectedErrors.Count -gt 0) {
    if ([string]::IsNullOrEmpty($Result.Error)) {
      throw "${Name}: expected resolver failure, got '$($Result.Value)'"
    }
    foreach ($Text in $ExpectedErrors) {
      if (-not $Result.Error.Contains($Text)) {
        throw "${Name}: expected error containing '$Text', got '$($Result.Error)'"
      }
    }
    Assert-Equal $Result.Value.Count 0 "$Name output count"
  } else {
    Assert-Equal $Result.Error $null "$Name error"
    Assert-Equal $Result.Value.Count 1 "$Name output count"
    Assert-Equal $Result.Value[0] $ExpectedVersion "$Name version"
  }

  if ($ExpectedEndpoint -eq "none") {
    Assert-Equal $Calls.Count 0 "$Name request count"
  } else {
    Assert-Equal $Calls.Count 1 "$Name request count (no fallback)"
    $Url = "https://api.github.com/repos/ZSeven-W/openpencil/releases"
    if ($ExpectedEndpoint -eq "latest") {
      $Url += "/latest"
    }
    Assert-Equal $Calls[0] $Url "$Name request URL"
  }
  $script:CaseCount++
  Write-Host "PASS $Name"
}

Test-ResolveVersion -Name "stable release" -ExpectedVersion "1.2.3"
Test-ResolveVersion -Name "stable 404 explains prerelease opt-in" -HttpStatus 404 `
  -ExpectedErrors @("no stable OpenPencil release", "OP_PRERELEASE", "OP_VERSION")
foreach ($Status in @(403, 429, 500, 503)) {
  Test-ResolveVersion -Name "HTTP $Status stays an API failure" -HttpStatus $Status `
    -ExpectedErrors @("could not query GitHub releases", "HTTP $Status")
}
Test-ResolveVersion -Name "network failure does not fall back" -NetworkError `
  -ExpectedErrors @("could not query GitHub releases", "Connection refused")

$Releases = @(
  [pscustomobject]@{ tag_name = "v2.0.0-rc.2" },
  [pscustomobject]@{ tag_name = "v1.2.3" }
)
Test-ResolveVersion -Name "prerelease switch selects first array entry" `
  -Payload $Releases -AllowPreRelease -ExpectedVersion "2.0.0-rc.2" -ExpectedEndpoint "releases"
foreach ($Value in @("1", "true", "TRUE", "yes", "YES")) {
  Test-ResolveVersion -Name "prerelease environment $Value" -Payload $Releases `
    -PreReleaseEnv $Value -ExpectedVersion "2.0.0-rc.2" -ExpectedEndpoint "releases"
}
Test-ResolveVersion -Name "single prerelease entry" -Payload @($Releases[0]) `
  -AllowPreRelease -ExpectedVersion "2.0.0-rc.2" -ExpectedEndpoint "releases"
Test-ResolveVersion -Name "empty prerelease list" -Payload @() -AllowPreRelease `
  -ExpectedErrors @("no release tag", "OP_VERSION") -ExpectedEndpoint "releases"
Test-ResolveVersion -Name "release response missing tag" -Payload ([pscustomobject]@{}) `
  -ExpectedErrors @("no release tag", "OP_PRERELEASE")
Test-ResolveVersion -Name "prerelease 404 remains API error" -HttpStatus 404 `
  -AllowPreRelease -ExpectedErrors @("could not query GitHub releases", "HTTP 404") `
  -ExpectedEndpoint "releases"
Test-ResolveVersion -Name "explicit version skips lookup" -Version "v3.2.1" `
  -NetworkError -ExpectedVersion "3.2.1" -ExpectedEndpoint "none"
Test-ResolveVersion -Name "embedded version skips lookup" -EmbeddedVersion "v4.3.2" `
  -AllowPreRelease -NetworkError -ExpectedVersion "4.3.2" -ExpectedEndpoint "none"
Test-ResolveVersion -Name "explicit version wins over embedded version" -Version "v3.2.1" `
  -EmbeddedVersion "v4.3.2" -NetworkError -ExpectedVersion "3.2.1" -ExpectedEndpoint "none"

Write-Host "install-op.test.ps1: $script:CaseCount resolver cases passed."
