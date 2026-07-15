#!/usr/bin/env bash

set -euo pipefail

script_dir=$(CDPATH= cd "$(dirname "$0")" && pwd)
guard_source="$script_dir/check-version-fixtures.sh"
reader_source="$script_dir/../scripts/workspace-version.sh"
temp_root=$(mktemp -d "${TMPDIR:-/tmp}/check-version-fixtures.XXXXXX")
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

new_repo() {
    name=$1
    version=$2
    repo="$temp_root/$name"

    mkdir -p \
        "$repo/.github/workflows" \
        "$repo/tools" \
        "$repo/scripts" \
        "$repo/crates/example/src" \
        "$repo/crates/op-host-desktop/src"
    git -C "$repo" init -q
    cp "$guard_source" "$repo/tools/check-version-fixtures.sh"
    cp "$reader_source" "$repo/scripts/workspace-version.sh"
    chmod +x "$repo/tools/check-version-fixtures.sh" "$repo/scripts/workspace-version.sh"
    printf '%s\n' \
        '[workspace]' \
        'members = []' \
        '' \
        '[workspace.package]' \
        "version = \"$version\"" \
        'edition = "2024"' > "$repo/Cargo.toml"

    cat > "$repo/scripts/bundle-macos.sh" <<'SCRIPT'
#!/usr/bin/env bash
WS_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CANONICAL_VERSION="$("$WS_ROOT/scripts/workspace-version.sh")"
APP_VERSION="${OPENPENCIL_VERSION:-$CANONICAL_VERSION}"
if [[ "$APP_VERSION" != "$CANONICAL_VERSION" ]]; then
    exit 1
fi
/usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $APP_VERSION" "$PLIST"
SCRIPT

    cat > "$repo/tools/bundle-macos.sh" <<'SCRIPT'
#!/bin/sh
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CANONICAL_VERSION="$("$ROOT/scripts/workspace-version.sh")"
APP_VERSION="${OPENPENCIL_VERSION:-$CANONICAL_VERSION}"
if [ "$APP_VERSION" != "$CANONICAL_VERSION" ]; then
    exit 1
fi
<key>CFBundleShortVersionString</key><string>${APP_VERSION}</string>
SCRIPT

    cat > "$repo/scripts/package-windows.nsi" <<'SCRIPT'
; makensis "/DVERSION=X.Y.Z" "/DOUT_FILE=OpenPencil-X.Y.Z-x64-win-setup.exe"
SCRIPT

    cat > "$repo/scripts/install-op.sh" <<'SCRIPT'
# OP_VERSION=X.Y.Z ./install-op.sh
# set OP_VERSION explicitly, e.g. OP_VERSION=X.Y.Z ./install-op.sh
SCRIPT

    cat > "$repo/.github/workflows/rust-release.yml" <<'SCRIPT'
- name: Compute release version
  shell: bash
  run: |
    cargo_version="$(scripts/workspace-version.sh)"
    if [[ "$GITHUB_REF" == refs/tags/v* ]]; then
      tag_version="${GITHUB_REF_NAME#v}"
      if [[ "$tag_version" != "$cargo_version" ]]; then
        exit 1
      fi
    fi
    echo "OP_VERSION=$cargo_version" >> "$GITHUB_ENV"
SCRIPT

    printf '%s\n' "$repo"
}

run_guard() {
    repo=$1
    case_status=0
    case_output=$(cd "$repo" && bash tools/check-version-fixtures.sh 2>&1) || case_status=$?
}

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

printf '1..%s\n' "$tests_run"
