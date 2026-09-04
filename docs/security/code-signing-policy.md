# Code Signing Policy

This document describes how OpenPencil release binaries are built and signed,
and the rules the project follows for code signing.

## What gets signed

- **Windows:** the NSIS installer `OpenPencil-<version>-<arch>-win-setup.exe`
  (x64 and arm64), containing the desktop app `openpencil-desktop.exe`, the
  CLI `op.exe`, the bundled ANGLE runtime DLLs, and an unmodified
  Microsoft-signed Visual C++ Redistributable prerequisite.
- **macOS:** the `.dmg` disk images and the app bundle inside them
  (Developer ID signing + notarization when release certificates are
  configured).

## How releases are built

Every release artifact is produced by the public GitHub Actions workflow
[`.github/workflows/rust-release.yml`](../../.github/workflows/rust-release.yml)
from a version tag on this repository. OpenPencil binaries are compiled from
the tagged source, packaged, checksummed into `SHA256SUMS.txt`, attested with a
signed SLSA build-provenance attestation, and uploaded to GitHub Releases
directly by CI. The Windows packaging job additionally downloads the Visual C++
Redistributable from a versioned Microsoft URL and verifies its pinned SHA-256
and Microsoft Authenticode signer before embedding it unchanged. The release
also fails before compiling Windows binaries if an installed MSVC toolset is
newer than that pinned Runtime. No human handles or modifies the binaries
between build and publication.

Anyone can verify an asset's origin:

```bash
gh attestation verify <downloaded-file> --repo ZSeven-W/openpencil
```

## Signing rules

- Free code signing is provided by [SignPath.io](https://signpath.io) and a
  free code signing certificate by the
  [SignPath Foundation](https://signpath.org).
- We only sign artifacts built by the release workflow of this repository
  from source code in this repository (including its vendored submodules,
  which the same team maintains). When present, the Windows installer's outer
  signature covers the complete installer, including the unmodified
  Microsoft-signed Visual C++ prerequisite; it does not replace Microsoft's
  nested signature or claim that prerequisite as OpenPencil code. We never
  apply an OpenPencil signature to a standalone third-party binary or to
  locally built artifacts.
- Signing is performed in CI as part of the release pipeline; signing
  credentials are never exported to developer machines.
- The team responsible for code signing is the same team that develops and
  maintains OpenPencil and owns this source repository.

## Privacy policy

OpenPencil is a local-first design tool. The application does not collect or
transmit personal data or telemetry. Network access happens only for features
the user explicitly invokes (e.g. optional AI providers, collaboration, or
image search), using endpoints the user configures or enables.

## Team

OpenPencil is developed and maintained by the ZSeven-W organization. The
maintainers listed on the [GitHub organization](https://github.com/ZSeven-W)
are the only people with commit access to this repository and control over the
release workflow.
