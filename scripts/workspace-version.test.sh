#!/bin/sh

set -eu

script_dir=$(CDPATH= cd "$(dirname "$0")" && pwd)
reader="$script_dir/workspace-version.sh"
tmp_dir=$(mktemp -d "${TMPDIR:-/tmp}/workspace-version.XXXXXX")
trap 'rm -rf "$tmp_dir"' 0 HUP INT TERM

test_count=0

fail() {
    printf 'not ok %s - %s\n' "$test_count" "$1" >&2
    exit 1
}

write_manifest() {
    name=$1
    content=$2
    printf '%s\n' "$content" > "$tmp_dir/$name"
}

assert_success() {
    name=$1
    label=$2
    expected=$3
    test_count=$((test_count + 1))

    if ! "$reader" "$tmp_dir/$name" > "$tmp_dir/stdout" 2> "$tmp_dir/stderr"; then
        sed 's/^/  /' "$tmp_dir/stderr" >&2
        fail "$label unexpectedly failed"
    fi

    printf '%s\n' "$expected" > "$tmp_dir/expected"
    if ! cmp -s "$tmp_dir/expected" "$tmp_dir/stdout"; then
        fail "$label did not print exactly '$expected'"
    fi
    if [ -s "$tmp_dir/stderr" ]; then
        fail "$label wrote unexpected stderr"
    fi

    printf 'ok %s - %s\n' "$test_count" "$label"
}

assert_relative_success() {
    name=$1
    label=$2
    expected=$3
    test_count=$((test_count + 1))

    if ! (cd "$tmp_dir" && "$reader" "$name" </dev/null) > "$tmp_dir/stdout" 2> "$tmp_dir/stderr"; then
        sed 's/^/  /' "$tmp_dir/stderr" >&2
        fail "$label unexpectedly failed"
    fi

    printf '%s\n' "$expected" > "$tmp_dir/expected"
    if ! cmp -s "$tmp_dir/expected" "$tmp_dir/stdout"; then
        fail "$label did not print exactly '$expected'"
    fi
    if [ -s "$tmp_dir/stderr" ]; then
        fail "$label wrote unexpected stderr"
    fi

    printf 'ok %s - %s\n' "$test_count" "$label"
}

assert_failure() {
    name=$1
    label=$2
    expected_error=$3
    test_count=$((test_count + 1))

    if "$reader" "$tmp_dir/$name" > "$tmp_dir/stdout" 2> "$tmp_dir/stderr"; then
        fail "$label unexpectedly succeeded"
    fi
    if [ -s "$tmp_dir/stdout" ]; then
        fail "$label wrote unexpected stdout"
    fi
    printf 'workspace-version: %s in %s\n' "$expected_error" "$tmp_dir/$name" > "$tmp_dir/expected"
    if ! cmp -s "$tmp_dir/expected" "$tmp_dir/stderr"; then
        sed 's/^/  /' "$tmp_dir/stderr" >&2
        fail "$label stderr did not match the expected diagnostic"
    fi

    printf 'ok %s - %s\n' "$test_count" "$label"
}

write_manifest case01.toml '[workspace]
members = []

[workspace.package]
version = "0.8.1"'

write_manifest case02.toml '[workspace.package]
version = "1.2.3-rc.1+build.4"'

write_manifest case03.toml '[workspace.package]
version = "1.2.3+build.004"'

write_manifest 'version=fixture' '[workspace.package]
version = "0.8.1"'

write_manifest case04.toml '[workspace]
members = []

[package]
name = "decoy"
version = "9.9.9"'

write_manifest case05.toml '[workspace.package]
edition = "2021"

[workspace.dependencies]
serde = { version = "1.0", features = ["derive"] }'

write_manifest case06.toml '[workspace.package]
version = "0.8.1"

[workspace.dependencies]
serde = "1"

[workspace.package]
version = "0.8.2"'

write_manifest case07.toml '[workspace.package]
version = "0.8.1"
version = "0.8.2"'

write_manifest case08.toml '[workspace.package]
version = "v0.8.1"'

write_manifest case09.toml '[workspace.package]
version = "0.8"'

write_manifest case10.toml '[workspace.package]
version = "latest"'

write_manifest case11.toml '[workspace.package]
version = "01.2.3"'

write_manifest case12.toml '[workspace.package]
version = "1.02.3"'

write_manifest case13.toml '[workspace.package]
version = "1.2.03"'

write_manifest case14.toml '[workspace.package]
version = "1.2.3-01"'

write_manifest case15.toml '[workspace.package]
version = "1.2.3-"'

write_manifest case16.toml '[workspace.package]
version = "1.2.3+"'

write_manifest case17.toml '[workspace.package]
version = "1.2.3-rc_1"'

write_manifest case18.toml '[workspace.package]
version = "1.2.3-rc..1"'

write_manifest case19.toml '[workspace.package]
version = "1.2.3+build..4"'

write_manifest case20.toml '[workspace.package]
version = 0.8.1'

write_manifest case21.toml "[workspace.package]
version = '0.8.1'"

write_manifest case22.toml '[workspace.package]
version = "0.8.1'

write_manifest case23.toml '[workspace.package]
version = "0.8.1" trailing'

assert_success case01.toml stable '0.8.1'
assert_success case02.toml prerelease_build '1.2.3-rc.1+build.4'
assert_success case03.toml build_with_leading_zero '1.2.3+build.004'
assert_relative_success 'version=fixture' assignment_shaped_relative_path '0.8.1'
assert_failure case04.toml missing_workspace_package 'missing [workspace.package] section'
assert_failure case05.toml missing_version 'missing version in [workspace.package]'
assert_failure case06.toml duplicate_sections 'expected exactly one [workspace.package] section'
assert_failure case07.toml duplicate_versions 'expected exactly one version in [workspace.package]'
assert_failure case08.toml malformed_v_prefix 'invalid version in [workspace.package]'
assert_failure case09.toml malformed_short 'invalid version in [workspace.package]'
assert_failure case10.toml malformed_latest 'invalid version in [workspace.package]'
assert_failure case11.toml leading_zero_major 'invalid version in [workspace.package]'
assert_failure case12.toml leading_zero_minor 'invalid version in [workspace.package]'
assert_failure case13.toml leading_zero_patch 'invalid version in [workspace.package]'
assert_failure case14.toml leading_zero_numeric_prerelease 'invalid version in [workspace.package]'
assert_failure case15.toml empty_prerelease 'invalid version in [workspace.package]'
assert_failure case16.toml empty_build 'invalid version in [workspace.package]'
assert_failure case17.toml illegal_identifier_character 'invalid version in [workspace.package]'
assert_failure case18.toml repeated_prerelease_separator 'invalid version in [workspace.package]'
assert_failure case19.toml repeated_build_separator 'invalid version in [workspace.package]'
assert_failure case20.toml unquoted_assignment 'invalid version in [workspace.package]'
assert_failure case21.toml single_quoted_assignment 'invalid version in [workspace.package]'
assert_failure case22.toml unterminated_quoted_assignment 'invalid version in [workspace.package]'
assert_failure case23.toml trailing_assignment_content 'invalid version in [workspace.package]'

printf '1..%s\n' "$test_count"
