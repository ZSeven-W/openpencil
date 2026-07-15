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

printf '1..%s\n' "$tests_run"
