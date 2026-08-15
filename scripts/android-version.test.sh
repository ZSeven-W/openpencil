#!/usr/bin/env bash

set -euo pipefail

script_dir=$(CDPATH='' cd "$(dirname "$0")" && pwd -P)
reader="$script_dir/android-version.sh"
tmp_dir=$(mktemp -d "${TMPDIR:-/tmp}/android-version.XXXXXX")
trap 'rm -rf "$tmp_dir"' EXIT HUP INT TERM

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

write_manifest() {
    version=$1
    manifest=$2
    printf '%s\n' \
        '[workspace]' \
        'members = []' \
        '' \
        '[workspace.package]' \
        "version = \"$version\"" > "$manifest"
}

run_version() {
    version=$1
    manifest="$tmp_dir/Cargo-$tests_run.toml"
    write_manifest "$version" "$manifest"
    case_status=0
    case_output=$(cd / && "$reader" "$manifest" 2>&1) || case_status=$?
}

assert_success() {
    version=$1
    expected_code=$2
    label=$3
    run_version "$version"
    if [[ "$case_status" -ne 0 ]]; then
        printf '%s\n' "$case_output" >&2
        fail "$label: expected success, got status $case_status"
    fi
    expected=$(printf 'versionName=%s\nversionCode=%s' "$version" "$expected_code")
    if [[ "$case_output" != "$expected" ]]; then
        printf 'expected:\n%s\nactual:\n%s\n' "$expected" "$case_output" >&2
        fail "$label: unexpected metadata"
    fi
    pass "$label"
}

assert_failure() {
    version=$1
    needle=$2
    label=$3
    run_version "$version"
    if [[ "$case_status" -ne 1 || "$case_output" != *"$needle"* ]]; then
        printf '%s\n' "$case_output" >&2
        fail "$label: expected status 1 containing: $needle"
    fi
    pass "$label"
}

canonical_version=$("$script_dir/workspace-version.sh")
canonical_output=$("$reader")
if [[ "$canonical_output" != "versionName=$canonical_version"$'\n'versionCode=* ]]; then
    printf '%s\n' "$canonical_output" >&2
    fail 'default invocation did not derive versionName from the canonical workspace manifest'
fi
pass 'default invocation derives versionName from the canonical workspace manifest'

assert_success 0.0.1 1 'the first publishable 0.x patch has a positive versionCode'
assert_success 0.8.4 8004 '0.x minor and patch components map deterministically'
assert_success 0.8.5 8005 'a 0.x patch bump strictly increases versionCode'
assert_success 0.9.0 9000 'a 0.x minor bump stays above the preceding supported patch range'
assert_success 0.999.999 999999 'the largest supported 0.x version remains below the major boundary'
assert_success 1.0.0 1000000 'the first stable major stays above the entire supported 0.x range'
assert_success 2100.0.0 2100000000 'the Android versionCode ceiling is accepted exactly'

assert_failure 0.0.0 'outside Android range' 'zero is never emitted as an Android versionCode'
assert_failure 0.8.1000 'components exceed the supported' 'patch overflow is rejected instead of colliding'
assert_failure 0.1000.0 'components exceed the supported' 'minor overflow is rejected instead of colliding'
assert_failure 2100.0.1 'outside Android range' 'values above the Android versionCode ceiling are rejected'
assert_failure 0.8.4-beta.1 'require a stable X.Y.Z' 'pre-release versions cannot reuse a stable versionCode'
assert_failure 0.8.4+build.1 'require a stable X.Y.Z' 'build metadata cannot reuse a stable versionCode'

case_status=0
case_output=$("$reader" one two 2>&1) || case_status=$?
if [[ "$case_status" -ne 2 || "$case_output" != *'usage: scripts/android-version.sh [Cargo.toml]'* ]]; then
    printf '%s\n' "$case_output" >&2
    fail 'argument rejection is not actionable'
fi
pass 'unexpected arguments are rejected with usage'

printf '1..%s\n' "$tests_run"
