#!/usr/bin/env bash

set -euo pipefail

script_dir=$(CDPATH= cd "$(dirname "$0")" && pwd)
guard_source="$script_dir/check-version-sync.sh"
reader_source="$script_dir/../scripts/workspace-version.sh"
temp_root=$(mktemp -d "${TMPDIR:-/tmp}/check-version-sync.XXXXXX")
trap 'rm -rf "$temp_root"' EXIT HUP INT TERM

tests_run=0
case_status=0
case_output=

fail() {
    printf 'not ok - %s\n' "$1" >&2
    exit 1
}

pass() {
    tests_run=$((tests_run + 1))
    printf 'ok %s - %s\n' "$tests_run" "$1"
}

assert_status() {
    expected=$1
    label=$2
    if [[ "$case_status" -ne "$expected" ]]; then
        printf '%s\n' "$case_output" >&2
        fail "$label: expected status $expected, got $case_status"
    fi
}

assert_contains() {
    needle=$1
    label=$2
    case "$case_output" in
        *"$needle"*) ;;
        *)
            printf '%s\n' "$case_output" >&2
            fail "$label: missing output: $needle"
            ;;
    esac
}

assert_not_contains() {
    needle=$1
    label=$2
    case "$case_output" in
        *"$needle"*)
            printf '%s\n' "$case_output" >&2
            fail "$label: unexpected output: $needle"
            ;;
        *) ;;
    esac
}

assert_no_success_output() {
    label=$1
    assert_not_contains 'no ordinary Rust fixtures copy current product version' "$label"
    assert_not_contains 'skipping literal fixture drift scan' "$label"
}

cargo() {
    if [[ "$*" != 'metadata --no-deps --format-version 1 --locked' ]]; then
        printf 'unexpected cargo arguments: %s\n' "$*" >&2
        return 41
    fi
    repo_root=$PWD
    canonical=$("$repo_root/scripts/workspace-version.sh")
    package_version=$canonical
    if [[ -f "$repo_root/.fake-cargo-version" ]]; then
        package_version=$(sed -n '1p' "$repo_root/.fake-cargo-version")
    fi
    if [[ -e "$repo_root/.fake-cargo-warning" ]]; then
        printf 'warning: fake Cargo metadata warning\n' >&2
    fi
    printf '{"packages":[{"name":"op-example","version":"%s","manifest_path":"%s/crates/example/Cargo.toml"}]}\n' \
        "$package_version" "$repo_root"
}

bun() {
    repo_root=$(CDPATH= cd "$PWD/.." && pwd)
    if [[ "$PWD" != "$repo_root/packages" || "$*" != 'run sync-version:check' ]]; then
        printf 'unexpected bun invocation: cwd=%s args=%s\n' "$PWD" "$*" >&2
        return 42
    fi
    if [[ -e "$repo_root/.fake-bun-fail" ]]; then
        printf 'package versions are stale\n' >&2
        return 43
    fi
}

export -f cargo bun

write_workflow_fixture() {
    repo=$1
    dependency_mode=$2
    publish_version_mode=$3
    template="$repo/.github/workflows/rust-release.yml.in"

    cat > "$template" <<'SCRIPT'
jobs:
  version:
    runs-on: ubuntu-latest
    outputs:
      version: ${{ steps.version.outputs.version }}
    steps:
      - uses: actions/checkout@v4
      - id: version
        shell: bash
        run: |
          cargo_version="$(scripts/workspace-version.sh)"
          if [[ "$GITHUB_REF" == refs/tags/v* ]]; then
            tag_version="${GITHUB_REF_NAME#v}"
            if [[ "$tag_version" != "$cargo_version" ]]; then
              exit 1
            fi
          fi
          echo "version=$cargo_version" >> "$GITHUB_OUTPUT"
  build:
    needs: version
    runs-on: ubuntu-latest
    env:
      OP_VERSION: ${{ needs.version.outputs.version }}
    steps:
      - run: echo build
  web-docker:
__WEB_NEEDS__
    runs-on: ubuntu-latest
    env:
      OP_VERSION: ${{ needs.version.outputs.version }}
    steps:
      - run: |
          version="__PUBLISH_VERSION__"
          echo "$version"
  sdk-packages:
__SDK_NEEDS__
    runs-on: ubuntu-latest
    env:
      OP_VERSION: ${{ needs.version.outputs.version }}
    steps:
      - run: bun run sync-version:check
      - run: |
          version="__PUBLISH_VERSION__"
          echo "$version"
  release-draft:
    needs: [version, build, web-docker, sdk-packages]
    runs-on: ubuntu-latest
    env:
      OP_VERSION: ${{ needs.version.outputs.version }}
    steps:
      - run: version="$OP_VERSION"
  package-managers:
    needs: [version, release-draft]
    runs-on: ubuntu-latest
    env:
      OP_VERSION: ${{ needs.version.outputs.version }}
    steps:
      - run: version="$OP_VERSION"
SCRIPT

    if [[ "$publish_version_mode" == independent ]]; then
        publish_version='${GITHUB_REF_NAME#v}'
    else
        publish_version='$OP_VERSION'
    fi

    awk -v dependency_mode="$dependency_mode" -v publish_version="$publish_version" '
        $0 == "__WEB_NEEDS__" {
            if (dependency_mode == "required") print "    needs: version"
            next
        }
        $0 == "__SDK_NEEDS__" {
            if (dependency_mode == "required") print "    needs: version"
            next
        }
        {
            gsub(/__PUBLISH_VERSION__/, publish_version)
            print
        }
    ' "$template" > "$repo/.github/workflows/rust-release.yml"
    rm "$template"
}

new_repo() {
    name=$1
    version=$2
    repo="$temp_root/$name"

    mkdir -p \
        "$repo/.github/workflows" \
        "$repo/packages" \
        "$repo/tools" \
        "$repo/scripts" \
        "$repo/crates/op-cli/assets" \
        "$repo/crates/op-cli/src" \
        "$repo/crates/op-editor-core/src" \
        "$repo/crates/op-host-desktop" \
        "$repo/crates/example/src" \
        "$repo/crates/op-host-desktop/src"
    git -C "$repo" init -q
    cp "$guard_source" "$repo/tools/check-version-sync.sh"
    cp "$reader_source" "$repo/scripts/workspace-version.sh"
    chmod +x "$repo/tools/check-version-sync.sh" "$repo/scripts/workspace-version.sh"
    printf '%s\n' \
        '[workspace]' \
        'members = []' \
        '' \
        '[workspace.package]' \
        "version = \"$version\"" \
        'edition = "2024"' > "$repo/Cargo.toml"
    printf '%s\n' 'version = 4' > "$repo/Cargo.lock"
    printf '%s\n' '{"name":"fixture-packages"}' > "$repo/packages/package.json"

    cat > "$repo/crates/op-cli/assets/skill-bundle.json" <<'JSON'
{"one":"__OPENPENCIL_VERSION__","two":"__OPENPENCIL_VERSION__","three":"__OPENPENCIL_VERSION__","four":"__OPENPENCIL_VERSION__","five":"__OPENPENCIL_VERSION__","six":"__OPENPENCIL_VERSION__"}
JSON
    cat > "$repo/crates/op-editor-core/src/state.rs" <<'RUST'
version: env!("CARGO_PKG_VERSION").to_owned(),
RUST
    cat > "$repo/crates/op-editor-core/src/host_support.rs" <<'RUST'
let src = src.replace("__OPENPENCIL_VERSION__", env!("CARGO_PKG_VERSION"));
let src = src.replace("__OPENPENCIL_VERSION__", env!("CARGO_PKG_VERSION"));
let src = src.replace("__OPENPENCIL_VERSION__", env!("CARGO_PKG_VERSION"));
RUST
    cat > "$repo/crates/op-cli/src/app_control_cli.rs" <<'RUST'
const MINIMAL_DOCUMENT: &str = concat!(env!("CARGO_PKG_VERSION"));
RUST
    cat > "$repo/crates/op-host-desktop/Cargo.toml" <<'TOML'
op-host-native = { path = "../op-host-native", features = ["gl-host"] }
TOML

    cat > "$repo/scripts/bundle-macos.sh" <<'SCRIPT'
#!/usr/bin/env bash
WS_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CANONICAL_VERSION="$("$WS_ROOT/scripts/workspace-version.sh")"
APP_VERSION="${OPENPENCIL_VERSION:-$CANONICAL_VERSION}"
if [[ "$APP_VERSION" != "$CANONICAL_VERSION" ]]; then
    printf 'bundle-macos: error: OPENPENCIL_VERSION (%s) must match Cargo workspace version (%s)\n' \
        "$APP_VERSION" "$CANONICAL_VERSION" >&2
    exit 1
fi
if [[ "${OPENPENCIL_VALIDATE_VERSION_ONLY:-}" == 1 ]]; then
    printf '%s\n' "$APP_VERSION"
    exit 0
fi
/usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $APP_VERSION" "$PLIST"
touch "$WS_ROOT/packaging-side-effect"
SCRIPT

    cat > "$repo/tools/bundle-macos.sh" <<'SCRIPT'
#!/bin/sh
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CANONICAL_VERSION="$("$ROOT/scripts/workspace-version.sh")"
APP_VERSION="${OPENPENCIL_VERSION:-$CANONICAL_VERSION}"
if [ "$APP_VERSION" != "$CANONICAL_VERSION" ]; then
    printf 'bundle-macos: error: OPENPENCIL_VERSION (%s) must match Cargo workspace version (%s)\n' \
        "$APP_VERSION" "$CANONICAL_VERSION" >&2
    exit 1
fi
if [ "${OPENPENCIL_VALIDATE_VERSION_ONLY:-}" = 1 ]; then
    printf '%s\n' "$APP_VERSION"
    exit 0
fi
<key>CFBundleShortVersionString</key><string>${APP_VERSION}</string>
touch "$ROOT/packaging-side-effect"
SCRIPT

    cat > "$repo/scripts/package-windows.nsi" <<'SCRIPT'
; makensis "/DVERSION=X.Y.Z" "/DOUT_FILE=OpenPencil-X.Y.Z-x64-win-setup.exe"
!ifndef VERSION
  !define VERSION "0.0.0"
!endif
SCRIPT

    cat > "$repo/scripts/install-op.sh" <<'SCRIPT'
# OP_VERSION=X.Y.Z ./install-op.sh
# set OP_VERSION explicitly, e.g. OP_VERSION=X.Y.Z ./install-op.sh
SCRIPT

    write_workflow_fixture "$repo" required canonical

    printf '%s\n' "$repo"
}

run_guard() {
    repo=$1
    shift
    case_status=0
    case_output=$(cd "$repo" && env "$@" \
        bash tools/check-version-sync.sh 2>&1) || case_status=$?
}

repo_snapshot() {
    repo=$1
    find "$repo" -type f ! -path "$repo/.git/*" -exec shasum {} + |
        LC_ALL=C sort |
        shasum |
        awk '{print $1}'
}

repo=$(new_repo v_prefixed_stale_version 0.8.1)
printf '%s\n' '# pin v0.8.2' >> "$repo/scripts/install-op.sh"
run_guard "$repo"
assert_status 1 'v-prefixed stale version'
assert_contains 'scripts/install-op.sh:' 'v-prefixed stale version'
assert_contains 'error: version examples must use X.Y.Z or <version>, not a SemVer release' \
    'v-prefixed stale version'
assert_no_success_output 'v-prefixed stale version'
pass 'v-prefixed stale SemVer examples are rejected'

repo=$(new_repo sentence_final_stale_version 0.8.1)
printf '%s\n' '# pin 0.8.2.' >> "$repo/scripts/install-op.sh"
run_guard "$repo"
assert_status 1 'sentence-final stale version'
assert_contains 'scripts/install-op.sh:' 'sentence-final stale version'
assert_contains 'error: version examples must use X.Y.Z or <version>, not a SemVer release' \
    'sentence-final stale version'
assert_no_success_output 'sentence-final stale version'
pass 'sentence-final stale SemVer examples are rejected'

repo=$(new_repo prerelease_build_stale_version 0.8.1)
printf '%s\n' '# pin v0.8.2-beta.1+build.5.' >> "$repo/scripts/install-op.sh"
run_guard "$repo"
assert_status 1 'pre-release/build stale version'
assert_contains 'scripts/install-op.sh:' 'pre-release/build stale version'
assert_contains 'error: version examples must use X.Y.Z or <version>, not a SemVer release' \
    'pre-release/build stale version'
assert_no_success_output 'pre-release/build stale version'
pass 'pre-release and build metadata remain part of the rejected token'

repo=$(new_repo ordinary_current 0.8.1)
printf '%s\n' 'const DOC: &str = r#"{"version":"0.8.1","children":[]}"#;' \
    > "$repo/crates/example/src/lib.rs"
run_guard "$repo"
assert_status 1 'ordinary current-version fixture'
assert_contains 'crates/example/src/lib.rs:1:' 'ordinary current-version fixture'
assert_contains 'use stable 1.0.0 test data' 'ordinary current-version fixture'
assert_no_success_output 'ordinary current-version fixture'
pass 'ordinary current-version fixture is rejected with guidance'

repo=$(new_repo updater_compatibility 0.8.1)
printf '%s\n' 'assert!(is_newer("0.8.1", "0.8.0"));' \
    > "$repo/crates/op-host-desktop/src/update_check.rs"
run_guard "$repo"
assert_status 0 'updater compatibility literal'
assert_contains 'no ordinary Rust fixtures copy current product version 0.8.1' \
    'updater compatibility literal'
pass 'updater compatibility literal is allowed'

repo=$(new_repo legitimate_stable_constant 0.8.1)
printf '%s\n' 'pub const FORMAT_VERSION: &str = "1.0.0";' \
    > "$repo/crates/example/src/lib.rs"
run_guard "$repo"
assert_status 0 'legitimate stable production constant'
assert_contains 'no ordinary Rust fixtures copy current product version 0.8.1' \
    'legitimate stable production constant'
pass 'unrelated stable production constant is allowed'

repo=$(new_repo reader_failure 0.8.1)
printf '%s\n' \
    '[workspace.package]' \
    'version = "not-semver"' > "$repo/Cargo.toml"
run_guard "$repo"
assert_status 1 'canonical reader failure'
assert_contains 'workspace-version: invalid version' 'canonical reader failure'
assert_no_success_output 'canonical reader failure'
pass 'canonical reader failure has no misleading success output'

repo=$(new_repo fixture_version_collision 1.0.0)
printf '%s\n' \
    'const DOC: &str = r#"{"version":"1.0.0","children":[]}"#;' \
    'pub const FORMAT_VERSION: &str = "1.0.0";' \
    > "$repo/crates/example/src/lib.rs"
run_guard "$repo"
assert_status 0 'fixture-version collision'
assert_contains 'current product version 1.0.0 equals stable fixture version 1.0.0' \
    'fixture-version collision'
assert_contains 'skipping literal fixture drift scan' 'fixture-version collision'
assert_contains 'stable fixtures and product-version literals are indistinguishable' \
    'fixture-version collision'
assert_not_contains 'no ordinary Rust fixtures copy current product version' \
    'fixture-version collision'
pass 'fixture-version collision is documented and skipped'

repo=$(new_repo hardcoded_macos_version 0.8.1)
printf '%s\n' 'APP_VERSION="${OPENPENCIL_VERSION:-0.8.1}"' \
    >> "$repo/scripts/bundle-macos.sh"
run_guard "$repo"
assert_status 1 'hardcoded macOS version'
assert_contains 'scripts/bundle-macos.sh:' 'hardcoded macOS version'
assert_contains 'error: OPENPENCIL_VERSION must fall back to the Cargo workspace version' \
    'hardcoded macOS version'
assert_no_success_output 'hardcoded macOS version'
pass 'hardcoded macOS package version is rejected with file and line guidance'

repo=$(new_repo macos_reader_comment_only 0.8.1)
cat > "$repo/scripts/bundle-macos.sh" <<'SCRIPT'
#!/usr/bin/env bash
# scripts/workspace-version.sh
APP_VERSION="${OPENPENCIL_VERSION:-$CANONICAL_VERSION}"
if [[ "$APP_VERSION" != "$CANONICAL_VERSION" ]]; then
    exit 1
fi
/usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $OTHER_VERSION" "$PLIST"
SCRIPT
run_guard "$repo"
assert_status 1 'macOS reader comment only'
assert_contains 'scripts/bundle-macos.sh:1:' 'macOS reader comment only'
assert_contains 'error: macOS packaging must assign CANONICAL_VERSION from scripts/workspace-version.sh' \
    'macOS reader comment only'
assert_contains 'error: CFBundleShortVersionString must use APP_VERSION' \
    'macOS reader comment only'
assert_no_success_output 'macOS reader comment only'
pass 'macOS packaging must invoke the reader and use the resolved version'

repo=$(new_repo release_missing_tag_equality 0.8.1)
cat > "$repo/.github/workflows/rust-release.yml" <<'SCRIPT'
- name: Compute release version
  shell: bash
  run: |
    cargo_version="$(scripts/workspace-version.sh)"
    if [[ "$GITHUB_REF" == refs/tags/v* ]]; then
      tag_version="${GITHUB_REF_NAME#v}"
    fi
    echo "OP_VERSION=$cargo_version" >> "$GITHUB_ENV"
SCRIPT
run_guard "$repo"
assert_status 1 'release missing tag equality'
assert_contains '.github/workflows/rust-release.yml:1:' 'release missing tag equality'
assert_contains 'error: release tags must be compared with the Cargo workspace version' \
    'release missing tag equality'
assert_no_success_output 'release missing tag equality'
pass 'release workflow must reject tags that differ from Cargo'

repo=$(new_repo macos_readers_commented_out 0.8.1)
cat > "$repo/scripts/bundle-macos.sh" <<'SCRIPT'
#!/usr/bin/env bash
WS_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CANONICAL_VERSION="0.0.0"
# CANONICAL_VERSION="$("$WS_ROOT/scripts/workspace-version.sh")"
APP_VERSION="${OPENPENCIL_VERSION:-$CANONICAL_VERSION}"
if [[ "$APP_VERSION" != "$CANONICAL_VERSION" ]]; then
    exit 1
fi
/usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $APP_VERSION" "$PLIST"
SCRIPT
cat > "$repo/tools/bundle-macos.sh" <<'SCRIPT'
#!/bin/sh
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CANONICAL_VERSION="0.0.0"
# CANONICAL_VERSION="$("$ROOT/scripts/workspace-version.sh")"
APP_VERSION="${OPENPENCIL_VERSION:-$CANONICAL_VERSION}"
if [ "$APP_VERSION" != "$CANONICAL_VERSION" ]; then
    exit 1
fi
<key>CFBundleShortVersionString</key><string>${APP_VERSION}</string>
SCRIPT
run_guard "$repo"
assert_status 1 'commented macOS readers'
assert_contains 'scripts/bundle-macos.sh:1:' 'commented macOS readers'
assert_contains 'tools/bundle-macos.sh:1:' 'commented macOS readers'
assert_contains 'error: macOS packaging must assign CANONICAL_VERSION from scripts/workspace-version.sh' \
    'commented macOS readers'
assert_no_success_output 'commented macOS readers'
pass 'commented macOS reader assignments do not satisfy the guard'

repo=$(new_repo release_checks_commented_out 0.8.1)
cat > "$repo/.github/workflows/rust-release.yml" <<'SCRIPT'
- name: Compute release version
  shell: bash
  run: |
    cargo_version="0.0.0"
    # cargo_version="$(scripts/workspace-version.sh)"
    if [[ "$GITHUB_REF" == refs/tags/v* ]]; then
      tag_version="${GITHUB_REF_NAME#v}"
      if [[ "$tag_version" == "$cargo_version" ]]; then
        :
      fi
      # if [[ "$tag_version" != "$cargo_version" ]]; then
    fi
    echo "OP_VERSION=$cargo_version" >> "$GITHUB_ENV"
SCRIPT
run_guard "$repo"
assert_status 1 'commented release checks'
assert_contains '.github/workflows/rust-release.yml:1:' 'commented release checks'
assert_contains 'error: release version computation must invoke scripts/workspace-version.sh' \
    'commented release checks'
assert_contains 'error: release tags must be compared with the Cargo workspace version' \
    'commented release checks'
assert_no_success_output 'commented release checks'
pass 'commented release derivation and comparison do not satisfy the guard'

repo=$(new_repo macos_noop_mismatch_and_reassignment 0.8.1)
cat > "$repo/scripts/bundle-macos.sh" <<'SCRIPT'
#!/usr/bin/env bash
WS_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CANONICAL_VERSION="$("$WS_ROOT/scripts/workspace-version.sh")"
APP_VERSION="${OPENPENCIL_VERSION:-$CANONICAL_VERSION}"
if [[ "$APP_VERSION" != "$CANONICAL_VERSION" ]]; then
    :
fi
if [[ "${OPENPENCIL_VALIDATE_VERSION_ONLY:-}" == 1 ]]; then
    printf '%s\n' "$APP_VERSION"
    exit 0
fi
APP_VERSION="0.8.2"
/usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $APP_VERSION" "$PLIST"
touch "$WS_ROOT/packaging-side-effect"
SCRIPT
run_guard "$repo"
assert_status 1 'macOS no-op mismatch and reassignment'
assert_contains 'scripts/bundle-macos.sh:1:' 'macOS no-op mismatch and reassignment'
assert_contains 'error: expected exactly one active APP_VERSION assignment' \
    'macOS no-op mismatch and reassignment'
assert_contains 'error: mismatched OPENPENCIL_VERSION must fail validation with actionable error' \
    'macOS no-op mismatch and reassignment'
if [[ -e "$repo/packaging-side-effect" ]]; then
    fail 'macOS no-op mismatch and reassignment: validation executed packaging side effects'
fi
assert_no_success_output 'macOS no-op mismatch and reassignment'
pass 'macOS validation rejects no-op mismatch bodies and APP_VERSION reassignment'

repo=$(new_repo arbitrary_stale_example_version 0.8.1)
printf '%s\n' '# stale example: OP_VERSION=0.8.2' >> "$repo/scripts/install-op.sh"
run_guard "$repo"
assert_status 1 'arbitrary stale example version'
assert_contains 'scripts/install-op.sh:' 'arbitrary stale example version'
assert_contains 'error: version examples must use X.Y.Z or <version>, not a SemVer release' \
    'arbitrary stale example version'
assert_no_success_output 'arbitrary stale example version'
pass 'arbitrary stale SemVer examples are rejected'

repo=$(new_repo embedded_version_like_substring 0.8.1)
printf '%s\n' '# identifier build0.8.2candidate is not a version token' \
    >> "$repo/scripts/install-op.sh"
run_guard "$repo"
assert_status 0 'embedded version-like substring'
assert_contains 'all managed versions derive from Cargo workspace version 0.8.1' \
    'embedded version-like substring'
pass 'version-like substrings inside larger identifiers are allowed'

repo=$(new_repo nsis_defensive_fallback 0.8.1)
run_guard "$repo"
assert_status 0 'NSIS defensive fallback'
assert_contains 'all managed versions derive from Cargo workspace version 0.8.1' \
    'NSIS defensive fallback'
pass 'NSIS 0.0.0 defensive fallback remains allowed'

repo=$(new_repo publish_jobs_without_version_dependency 0.8.1)
write_workflow_fixture "$repo" missing canonical
run_guard "$repo"
assert_status 1 'publish jobs without version dependency'
assert_contains '.github/workflows/rust-release.yml:1:' \
    'publish jobs without version dependency'
assert_contains 'error: web-docker must depend on the version preflight job' \
    'publish jobs without version dependency'
assert_contains 'error: sdk-packages must depend on the version preflight job' \
    'publish jobs without version dependency'
assert_no_success_output 'publish jobs without version dependency'
pass 'web Docker and SDK publishing cannot start before version preflight'

repo=$(new_repo independent_publish_tag_versions 0.8.1)
write_workflow_fixture "$repo" required independent
run_guard "$repo"
assert_status 1 'independent publish tag versions'
assert_contains '.github/workflows/rust-release.yml:' 'independent publish tag versions'
assert_contains 'error: publish paths must consume the canonical version job output' \
    'independent publish tag versions'
assert_no_success_output 'independent publish tag versions'
pass 'publish paths cannot derive independent versions from the tag'

repo=$(new_repo collision_still_checks_packaging 1.0.0)
printf '%s\n' 'APP_VERSION="${OPENPENCIL_VERSION:-0.8.1}"' \
    >> "$repo/tools/bundle-macos.sh"
run_guard "$repo"
assert_status 1 'fixture-version collision packaging check'
assert_contains 'skipping literal fixture drift scan' \
    'fixture-version collision packaging check'
assert_contains 'tools/bundle-macos.sh:' 'fixture-version collision packaging check'
assert_contains 'error: OPENPENCIL_VERSION must fall back to the Cargo workspace version' \
    'fixture-version collision packaging check'
assert_not_contains 'no ordinary Rust fixtures copy current product version' \
    'fixture-version collision packaging check'
pass 'fixture-version collision still runs packaging checks'

repo=$(new_repo cargo_metadata_version_mismatch 0.8.1)
printf '%s\n' '0.8.0' > "$repo/.fake-cargo-version"
run_guard "$repo"
assert_status 1 'Cargo metadata version mismatch'
assert_contains 'crates/example/Cargo.toml:1: error: workspace package op-example has version 0.8.0; expected 0.8.1' \
    'Cargo metadata version mismatch'
assert_no_success_output 'Cargo metadata version mismatch'
pass 'workspace op-* packages under crates must match the canonical Cargo version'

repo=$(new_repo cargo_metadata_warning 0.8.1)
touch "$repo/.fake-cargo-warning"
run_guard "$repo"
assert_status 0 'Cargo metadata warning'
assert_contains 'warning: fake Cargo metadata warning' 'Cargo metadata warning'
pass 'Cargo warnings do not corrupt the metadata JSON passed to jq'

repo=$(new_repo package_version_drift 0.8.1)
touch "$repo/.fake-bun-fail"
run_guard "$repo"
assert_status 1 'package version drift'
assert_contains 'packages:1: error: bun run sync-version:check failed' 'package version drift'
assert_no_success_output 'package version drift'
pass 'web SDK package drift is reported by the read-only guard'

repo=$(new_repo matching_release_tag 0.8.1)
run_guard "$repo" GITHUB_REF=refs/tags/v0.8.1 GITHUB_REF_NAME=v0.8.1
assert_status 0 'matching release tag'
pass 'matching v* release tags pass the repository guard'

repo=$(new_repo mismatched_release_tag 0.8.1)
run_guard "$repo" GITHUB_REF=refs/tags/v0.8.2 GITHUB_REF_NAME=v0.8.2
assert_status 1 'mismatched release tag'
assert_contains 'environment:1: error: release tag v0.8.2 does not match Cargo workspace version 0.8.1' \
    'mismatched release tag'
assert_no_success_output 'mismatched release tag'
pass 'mismatched v* release tags are rejected outside the release workflow too'

repo=$(new_repo cli_bundle_wrong_sentinel_count 0.8.1)
sed -i.bak 's/,"six":"__OPENPENCIL_VERSION__"//' \
    "$repo/crates/op-cli/assets/skill-bundle.json"
rm "$repo/crates/op-cli/assets/skill-bundle.json.bak"
run_guard "$repo"
assert_status 1 'CLI bundle sentinel count'
assert_contains 'crates/op-cli/assets/skill-bundle.json:1: error: expected exactly 6 version sentinels' \
    'CLI bundle sentinel count'
assert_no_success_output 'CLI bundle sentinel count'
pass 'embedded CLI bundle retains exactly six version sentinels'

repo=$(new_repo cli_bundle_without_sentinels 0.8.1)
sed -i.bak 's/__OPENPENCIL_VERSION__/__MISSING_VERSION__/g' \
    "$repo/crates/op-cli/assets/skill-bundle.json"
rm "$repo/crates/op-cli/assets/skill-bundle.json.bak"
run_guard "$repo"
assert_status 1 'CLI bundle missing sentinels'
assert_contains 'crates/op-cli/assets/skill-bundle.json:1: error: expected exactly 6 version sentinels' \
    'CLI bundle missing sentinels'
assert_contains '(found 0)' 'CLI bundle missing sentinels'
assert_no_success_output 'CLI bundle missing sentinels'
pass 'embedded CLI bundle reports zero missing version sentinels actionably'

repo=$(new_repo cli_bundle_numeric_product_version 0.8.1)
printf '%s\n' '{"stale":"0.8.1"}' >> "$repo/crates/op-cli/assets/skill-bundle.json"
run_guard "$repo"
assert_status 1 'CLI bundle numeric product version'
assert_contains 'crates/op-cli/assets/skill-bundle.json:' 'CLI bundle numeric product version'
assert_contains 'error: embedded CLI bundle must not contain the canonical version literal' \
    'CLI bundle numeric product version'
assert_no_success_output 'CLI bundle numeric product version'
pass 'embedded CLI bundle cannot duplicate the numeric product version'

repo=$(new_repo hardcoded_rust_product_version 0.8.1)
printf '%s\n' 'version: "0.8.1".to_owned(),' > "$repo/crates/op-editor-core/src/state.rs"
run_guard "$repo"
assert_status 1 'hardcoded Rust product version'
assert_contains 'crates/op-editor-core/src/state.rs:1: error: empty documents must derive their version from CARGO_PKG_VERSION' \
    'hardcoded Rust product version'
assert_no_success_output 'hardcoded Rust product version'
pass 'Rust product-version producers remain derived from Cargo metadata'

repo=$(new_repo versioned_local_product_dependency 0.8.1)
printf '%s\n' \
    'op-host-native = { path = "../op-host-native", version = "0.8.1", features = ["gl-host"] }' \
    > "$repo/crates/op-host-desktop/Cargo.toml"
run_guard "$repo"
assert_status 1 'versioned local product dependency'
assert_contains 'crates/op-host-desktop/Cargo.toml:1: error: local op-host-native dependency must not duplicate the product version' \
    'versioned local product dependency'
assert_no_success_output 'versioned local product dependency'
pass 'local product dependencies do not repeat the workspace version'

repo=$(new_repo guard_is_read_only 0.8.1)
before_snapshot=$(repo_snapshot "$repo")
run_guard "$repo"
assert_status 0 'read-only guard'
after_snapshot=$(repo_snapshot "$repo")
if [[ "$before_snapshot" != "$after_snapshot" ]]; then
    fail 'read-only guard changed repository file contents'
fi
pass 'repository-wide version guard does not write files'

printf '1..%s\n' "$tests_run"
