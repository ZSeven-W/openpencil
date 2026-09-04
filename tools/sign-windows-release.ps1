param(
    [switch]$SelfTest,
    [string]$CertificateBase64,
    [string]$CertificatePassword,
    [string]$ExpectedPfxSha256,
    [string]$ExpectedCertificateSha1,
    [string]$Executable
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Normalize-Hex {
    param([string]$Value, [int]$Length, [string]$Label)
    $normalized = ($Value -replace '[\s:-]', '').ToLowerInvariant()
    if ($normalized -notmatch "^[0-9a-f]{$Length}$") {
        throw "$Label must be exactly $Length hexadecimal characters"
    }
    return $normalized
}

function Convert-CertificatePayload {
    param([string]$Payload)
    if ([string]::IsNullOrWhiteSpace($Payload)) {
        throw 'certificate payload is empty'
    }
    if ($Payload -match '^[a-zA-Z][a-zA-Z0-9+.-]*:') {
        if ($Payload -notmatch '^data:application/(?:x-pkcs12|pkcs12|octet-stream);base64,([A-Za-z0-9+/=\r\n]+)$') {
            throw 'certificate payload must not be a URL; use raw base64 or an approved PKCS#12 data URL'
        }
        $Payload = $Matches[1]
    }
    $Payload = $Payload -replace '\s', ''
    try {
        return [Convert]::FromBase64String($Payload)
    } catch {
        throw 'certificate payload is not valid base64'
    }
}

function Assert-Rejected {
    param([scriptblock]$Action, [string]$Label)
    try {
        & $Action | Out-Null
    } catch {
        return
    }
    throw "negative self-test unexpectedly accepted $Label"
}

if ($SelfTest) {
    Assert-Rejected { Convert-CertificatePayload 'https://example.invalid/codesign.pfx' } 'HTTPS input'
    Assert-Rejected { Convert-CertificatePayload 'http://example.invalid/codesign.pfx' } 'HTTP input'
    Assert-Rejected { Convert-CertificatePayload 'not-base64!' } 'malformed base64'
    Assert-Rejected { Normalize-Hex '1234' 64 'PFX SHA-256' } 'short PFX digest'
    Assert-Rejected { Normalize-Hex ('a' * 64) 40 'certificate SHA-1' } 'long certificate thumbprint'
    [void](Convert-CertificatePayload 'data:application/x-pkcs12;base64,dGVzdA==')
    Write-Host 'sign-windows-release.ps1: input-boundary self-test passed.'
    exit 0
}

$pfxSha256 = Normalize-Hex $ExpectedPfxSha256 64 'PFX SHA-256'
$certificateSha1 = Normalize-Hex $ExpectedCertificateSha1 40 'certificate SHA-1 thumbprint'
if ([string]::IsNullOrWhiteSpace($CertificatePassword)) {
    throw 'certificate password is required'
}
if ([string]::IsNullOrWhiteSpace($Executable)) {
    throw 'executable path is required'
}
$executableItem = Get-Item -LiteralPath $Executable
if ($executableItem.PSIsContainer -or ($executableItem.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
    throw 'executable must be a regular, non-reparse-point file'
}

$pfxBytes = Convert-CertificatePayload $CertificateBase64
$CertificateBase64 = $null
$env:WIN_CSC_LINK = $null
$env:WIN_CSC_KEY_PASSWORD = $null
$env:WIN_CSC_PFX_SHA256 = $null
$env:WIN_CSC_CERTIFICATE_SHA1 = $null

$pfx = Join-Path $env:RUNNER_TEMP ("openpencil-codesign-{0}.pfx" -f [Guid]::NewGuid())
try {
    [IO.File]::WriteAllBytes($pfx, $pfxBytes)
    [Array]::Clear($pfxBytes, 0, $pfxBytes.Length)
    $actualPfxSha256 = (Get-FileHash -LiteralPath $pfx -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actualPfxSha256 -ne $pfxSha256) {
        throw 'PFX SHA-256 does not match the independently configured digest'
    }

    $flags = [Security.Cryptography.X509Certificates.X509KeyStorageFlags]::EphemeralKeySet
    $certificate = [Security.Cryptography.X509Certificates.X509Certificate2]::new(
        $pfx, $CertificatePassword, $flags
    )
    try {
        if (-not $certificate.HasPrivateKey) {
            throw 'PFX does not contain a private key'
        }
        $actualCertificateSha1 = Normalize-Hex $certificate.Thumbprint 40 'PFX certificate SHA-1 thumbprint'
        if ($actualCertificateSha1 -ne $certificateSha1) {
            throw 'PFX signer certificate thumbprint does not match the reviewed value'
        }
    } finally {
        $certificate.Dispose()
    }

    $signtool = (Get-ChildItem 'C:\Program Files (x86)\Windows Kits\10\bin\*\x64\signtool.exe' |
        Sort-Object FullName -Descending | Select-Object -First 1).FullName
    if ([string]::IsNullOrWhiteSpace($signtool)) {
        throw 'signtool.exe was not found in the Windows SDK'
    }
    & $signtool sign /f $pfx /p $CertificatePassword /fd SHA256 `
        /tr https://timestamp.digicert.com /td SHA256 $Executable
    if ($LASTEXITCODE -ne 0) {
        throw "signtool sign failed with exit code $LASTEXITCODE"
    }
    & $signtool verify /pa /all /v $Executable
    if ($LASTEXITCODE -ne 0) {
        throw "signtool verify failed with exit code $LASTEXITCODE"
    }
    $signature = Get-AuthenticodeSignature -LiteralPath $Executable
    if ($signature.Status -ne [Management.Automation.SignatureStatus]::Valid) {
        throw "signed executable has invalid Authenticode status: $($signature.Status)"
    }
    $actualSignerSha1 = Normalize-Hex $signature.SignerCertificate.Thumbprint 40 'signed executable certificate SHA-1 thumbprint'
    if ($actualSignerSha1 -ne $certificateSha1) {
        throw 'signed executable signer thumbprint does not match the reviewed value'
    }
} finally {
    $CertificatePassword = $null
    if (Test-Path -LiteralPath $pfx) {
        Remove-Item -LiteralPath $pfx -Force
    }
}
