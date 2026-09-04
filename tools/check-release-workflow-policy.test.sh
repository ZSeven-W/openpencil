#!/usr/bin/env bash
# Mutation tests proving the release contracts reject policy regressions.

set -euo pipefail

script_dir=$(CDPATH='' cd "$(dirname "$0")" && pwd)
repo_root=$(CDPATH='' cd "$script_dir/.." && pwd)
temporary=$(mktemp -d)
trap 'rm -rf "$temporary"' EXIT

mutate() {
    python3 - "$1" "$2" "$3" <<'PYTHON'
import pathlib
import sys

source, destination, mode = map(pathlib.Path, sys.argv[1:])
text = source.read_text()
if str(mode) == "mutable-action":
    old = "actions/checkout@08eba0b27e820071cde6df949e0beb9ba4906955"
    new = "actions/checkout@v4"
elif str(mode) == "broad-permission":
    old = "permissions:\n  contents: read"
    new = "permissions:\n  contents: write"
elif str(mode) == "job-secret":
    marker = "  sdk-packages:\n"
    start = text.index(marker)
    end = text.index("\n  vsix:\n", start)
    section = text[start:end]
    old = "    env:\n"
    new = "    env:\n      NPM_TOKEN: ${{ secrets.NPM_TOKEN }}\n"
    if old not in section:
        raise SystemExit("mutation marker missing: sdk env")
    text = text[:start] + section.replace(old, new, 1) + text[end:]
    destination.write_text(text)
    raise SystemExit(0)
elif str(mode) == "mutable-package-assets":
    old = "tools/package-manager-handoff.sh download"
    new = 'gh release download "$GITHUB_REF_NAME"'
elif str(mode) == "mutable-apt-source":
    old = "https://snapshot.ubuntu.com/ubuntu/20260801T000000Z/"
    new = "http://ports.ubuntu.com/ubuntu-ports/"
elif str(mode) == "direct-cargo-cli":
    old = '''tools/pinned-release-tools.sh cargo-cli wasm-bindgen-cli \\
            "$RUNNER_TEMP/wasm-bindgen-cli-0.2.117"'''
    new = "cargo install wasm-bindgen-cli --version 0.2.117 --locked"
elif str(mode) == "cargo-cli-digest":
    old = "bb3601b2899d4887512bdcaad115074750be7c212b122fa7ed4faed6c919229e"
    new = "0" * 64
elif str(mode) == "ripgrep-digest":
    old = "33e15bcf1624b25cdd2a55813a47a2f95dbe126268203e76aa6a585d1e7b149c"
    new = "0" * 64
elif str(mode) == "version-sync-direct-ripgrep":
    old = 'tools/pinned-release-tools.sh ripgrep "$RUNNER_TEMP/ripgrep-15.2.0"'
    new = "sudo apt-get install --yes ripgrep"
elif str(mode) == "docker-direct-cargo-cli":
    old = "tools/pinned-release-tools.sh cargo-cli wasm-bindgen-cli " + "\\"
    new = 'cargo install wasm-bindgen-cli --version "$version" --locked #'
elif str(mode) == "ios-skia-profile":
    old = "tools/pinned-release-tools.sh skia ios aarch64-apple-ios"
    new = "tools/pinned-release-tools.sh skia web aarch64-apple-ios"
elif str(mode) == "ios-skia-digest":
    old = "4abbaea5e4e8934a6f19c5de44eaba9bf9238af4abbe57dbac5f2dc03923b182"
    new = "0" * 64
elif str(mode) == "ios-encryption-yes":
    old = '"INFOPLIST_KEY_ITSAppUsesNonExemptEncryption=NO"'
    new = '"INFOPLIST_KEY_ITSAppUsesNonExemptEncryption=YES"'
elif str(mode) == "ios-encryption-code":
    old = '"INFOPLIST_KEY_ITSAppUsesNonExemptEncryption=NO"'
    new = old + '\n    "INFOPLIST_KEY_ITSEncryptionExportComplianceCode=forbidden"'
elif str(mode) == "ios-encryption-env-override":
    old = "          IOS_MARKETING_VERSION: ${{ needs.verify.outputs.version }}"
    new = old + "\n          IOS_USES_NON_EXEMPT_ENCRYPTION: ${{ vars.IOS_USES_NON_EXEMPT_ENCRYPTION }}"
elif str(mode) == "rust-skia-force":
    old = "[[ -z ${FORCE_SKIA_BINARIES_DOWNLOAD:-} && ${SKIA_BINARIES_URL:-} == file://* ]]"
    new = "[[ ${FORCE_SKIA_BINARIES_DOWNLOAD:-} == 1 && ${SKIA_BINARIES_URL:-} == file://* ]]"
elif str(mode) == "ios-only-input-default":
    old = """      ios_app_store_only:
        description: Run only the iOS App Store / TestFlight lane
        required: false
        default: false
        type: boolean"""
    new = old.replace("default: false", "default: true")
elif str(mode) == "ios-only-ios-gate":
    old = "if: startsWith(github.ref, 'refs/tags/v') || (github.event_name == 'workflow_dispatch' && inputs.ios_app_store_only == true)"
    new = "if: startsWith(github.ref, 'refs/tags/v')"
elif str(mode) == "ios-only-build-bypass":
    old = "if: (github.event_name == 'workflow_dispatch' && inputs.ios_app_store_only == false) || (startsWith(github.ref, 'refs/tags/v') && github.event_name != 'workflow_dispatch')"
    new = "if: github.event_name == 'workflow_dispatch' || startsWith(github.ref, 'refs/tags/v')"
elif str(mode) == "ios-only-release-bypass":
    marker = "  web-docker:\n"
    start = text.index(marker)
    end = text.index("\n  sdk-packages:\n", start)
    section = text[start:end]
    old = "if: startsWith(github.ref, 'refs/tags/v') && (github.event_name != 'workflow_dispatch' || inputs.ios_app_store_only == false)"
    new = "if: startsWith(github.ref, 'refs/tags/v')"
    if old not in section:
        raise SystemExit("mutation marker missing: web-docker if")
    text = text[:start] + section.replace(old, new, 1) + text[end:]
    destination.write_text(text)
    raise SystemExit(0)
elif str(mode) == "vcredist-workflow-define":
    old = '"/DVC_REDIST_FILE=$env:VC_REDIST_FILE"'
    new = '"/DOPTIONAL_VC_REDIST_FILE=$env:VC_REDIST_FILE"'
elif str(mode) == "vcredist-toolset-gate":
    old = "& tools/stage-pinned-vcredist.ps1 -ValidateBuildToolset"
    new = "& tools/stage-pinned-vcredist.ps1 -SkipBuildToolsetValidation"
elif str(mode) == "vcredist-scoop-cli-bypass":
    marker = "scoop-bucket/bucket/op.json"
    marker_index = text.index(marker)
    call_index = text.rindex("write_scoop_manifest \\", 0, marker_index)
    text = text[:call_index] + text[call_index:].replace(
        "write_scoop_manifest \\", "write_scoop_manifest_without_runtime \\", 1
    )
    destination.write_text(text)
    raise SystemExit(0)
elif str(mode) == "vcredist-scoop-skip-postcheck":
    old = '  if ($null -eq $installedVersion -or $installedVersion -lt $requiredVersion) { throw \\"Microsoft Visual C++ Redistributable $requiredVersion or newer is required.\\" }'
    new = '  if ($false) { throw \\"Microsoft Visual C++ Redistributable $requiredVersion or newer is required.\\" }'
elif str(mode) == "vcredist-stager-digest":
    old = "843068991daaa1f73ad9f6239bce4d0f6a07a51f18c37ea2a867e9beca71295c"
    new = "0" * 64
elif str(mode) == "vcredist-nsis-soft-fail":
    old = 'Abort "Microsoft Visual C++ Redistributable verification failed."'
    new = 'DetailPrint "Microsoft Visual C++ Redistributable verification failed."'
elif str(mode) == "vcredist-cli-skip-postcheck":
    old = "  $InstalledVersion = Get-InstalledVcRedistVersion $RuntimeArch"
    replacement_index = text.rindex(old)
    text = text[:replacement_index] + text[replacement_index:].replace(
        old, "  $InstalledVersion = $VcRedistVersion", 1
    )
    destination.write_text(text)
    raise SystemExit(0)
else:
    raise SystemExit(f"unknown mutation: {mode}")
if old not in text:
    raise SystemExit(f"mutation marker missing: {mode}")
destination.write_text(text.replace(old, new, 1))
PYTHON
}

expect_rejected() {
    local label=$1 env_name=$2 checker=$3 fixture=$4
    if env "$env_name=$fixture" bash "$checker" >"$temporary/$label.log" 2>&1; then
        printf 'error: release policy mutation was accepted: %s\n' "$label" >&2
        exit 1
    fi
}

case ${1-} in
    rust)
        for mutation in \
            mutable-action broad-permission job-secret mutable-package-assets \
            mutable-apt-source direct-cargo-cli ios-only-input-default \
            ios-only-ios-gate ios-only-build-bypass ios-only-release-bypass \
            vcredist-workflow-define vcredist-toolset-gate \
            vcredist-scoop-cli-bypass vcredist-scoop-skip-postcheck; do
            fixture=$temporary/rust-$mutation.yml
            mutate "$repo_root/.github/workflows/rust-release.yml" "$fixture" "$mutation"
            expect_rejected "$mutation" OPENPENCIL_RUST_RELEASE_WORKFLOW \
                "$repo_root/tools/check-rust-release-auth-workflow.sh" "$fixture"
        done
        fixture=$temporary/build-rust-release-host.sh
        mutate "$repo_root/scripts/build-rust-release-host.sh" "$fixture" rust-skia-force
        expect_rejected rust-skia-force OPENPENCIL_RUST_RELEASE_BUILDER \
            "$repo_root/tools/check-rust-release-auth-workflow.sh" "$fixture"
        fixture=$temporary/pinned-release-tools.sh
        mutate "$repo_root/tools/pinned-release-tools.sh" "$fixture" cargo-cli-digest
        expect_rejected cargo-cli-digest OPENPENCIL_PINNED_RELEASE_TOOLS \
            "$repo_root/tools/check-rust-release-auth-workflow.sh" "$fixture"
        fixture=$temporary/pinned-release-tools-ripgrep.sh
        mutate "$repo_root/tools/pinned-release-tools.sh" "$fixture" ripgrep-digest
        expect_rejected ripgrep-digest OPENPENCIL_PINNED_RELEASE_TOOLS \
            "$repo_root/tools/check-rust-release-auth-workflow.sh" "$fixture"
        fixture=$temporary/version-sync-direct-ripgrep.yml
        mutate "$repo_root/.github/workflows/version-sync.yml" "$fixture" \
            version-sync-direct-ripgrep
        expect_rejected version-sync-direct-ripgrep OPENPENCIL_VERSION_SYNC_WORKFLOW \
            "$repo_root/tools/check-rust-release-auth-workflow.sh" "$fixture"
        fixture=$temporary/Dockerfile.web-rust
        mutate "$repo_root/Dockerfile.web-rust" "$fixture" docker-direct-cargo-cli
        expect_rejected docker-direct-cargo-cli OPENPENCIL_WEB_DOCKERFILE \
            "$repo_root/tools/check-rust-release-auth-workflow.sh" "$fixture"
        fixture=$temporary/stage-pinned-vcredist.ps1
        mutate "$repo_root/tools/stage-pinned-vcredist.ps1" "$fixture" \
            vcredist-stager-digest
        expect_rejected vcredist-stager-digest OPENPENCIL_VC_REDIST_STAGER \
            "$repo_root/tools/check-rust-release-auth-workflow.sh" "$fixture"
        fixture=$temporary/package-windows.nsi
        mutate "$repo_root/scripts/package-windows.nsi" "$fixture" \
            vcredist-nsis-soft-fail
        expect_rejected vcredist-nsis-soft-fail OPENPENCIL_WINDOWS_NSIS_INSTALLER \
            "$repo_root/tools/check-rust-release-auth-workflow.sh" "$fixture"
        fixture=$temporary/install-op.ps1
        mutate "$repo_root/scripts/install-op.ps1" "$fixture" \
            vcredist-cli-skip-postcheck
        expect_rejected vcredist-cli-skip-postcheck OPENPENCIL_WINDOWS_CLI_INSTALLER \
            "$repo_root/tools/check-rust-release-auth-workflow.sh" "$fixture"
        ;;
    ios)
        fixture=$temporary/ios-skia-profile.yml
        mutate "$repo_root/.github/workflows/ios-app-store.yml" "$fixture" ios-skia-profile
        expect_rejected ios-skia-profile OPENPENCIL_IOS_APP_STORE_WORKFLOW \
            "$repo_root/tools/check-ios-app-store-workflow.sh" "$fixture"
        fixture=$temporary/publish-ios-testflight.sh
        mutate "$repo_root/scripts/publish-ios-testflight.sh" "$fixture" ios-skia-digest
        expect_rejected ios-skia-digest OPENPENCIL_IOS_TESTFLIGHT_PUBLISHER \
            "$repo_root/tools/check-ios-app-store-workflow.sh" "$fixture"
        for mutation in ios-encryption-yes ios-encryption-code; do
            fixture=$temporary/$mutation-publisher.sh
            mutate "$repo_root/scripts/publish-ios-testflight.sh" "$fixture" "$mutation"
            expect_rejected "$mutation" OPENPENCIL_IOS_TESTFLIGHT_PUBLISHER \
                "$repo_root/tools/check-ios-app-store-workflow.sh" "$fixture"
        done
        fixture=$temporary/ios-encryption-env-override.yml
        mutate "$repo_root/.github/workflows/ios-app-store.yml" "$fixture" \
            ios-encryption-env-override
        expect_rejected ios-encryption-env-override OPENPENCIL_IOS_APP_STORE_WORKFLOW \
            "$repo_root/tools/check-ios-app-store-workflow.sh" "$fixture"
        ;;
    *)
        printf 'usage: %s {rust|ios}\n' "$0" >&2
        exit 2
        ;;
esac

printf 'check-release-workflow-policy.test.sh: %s mutations were rejected.\n' "$1"
