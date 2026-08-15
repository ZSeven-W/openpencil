#!/usr/bin/env bash

# Workflow fixtures intentionally preserve unexpanded shell variables.
# shellcheck disable=SC2016

set -euo pipefail

script_dir=$(CDPATH='' cd "$(dirname "$0")" && pwd)
repo_root=$(CDPATH='' cd "$script_dir/.." && pwd)
sync_source="$script_dir/sync-version.sh"
reader_source="$script_dir/workspace-version.sh"
android_reader_source="$script_dir/android-version.sh"
tmp_base=${TMPDIR:-/tmp}
tmp_dir=$(mktemp -d "${tmp_base%/}/sync-version.XXXXXX")
trap 'rm -rf "$tmp_dir"' 0 HUP INT TERM

test_count=0
case_status=0
case_output=

fail() {
    printf 'not ok %s - %s\n' "$test_count" "$1" >&2
    exit 1
}

pass() {
    test_count=$((test_count + 1))
    printf 'ok %s - %s\n' "$test_count" "$1"
}

assert_status() {
    expected=$1
    label=$2
    if [ "$case_status" -ne "$expected" ]; then
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

assert_file_contains() {
    file=$1
    needle=$2
    label=$3
    if ! grep -Fq -- "$needle" "$file"; then
        fail "$label: $file does not contain: $needle"
    fi
}

assert_file_not_matches() {
    file=$1
    pattern=$2
    label=$3
    if grep -Eq -- "$pattern" "$file"; then
        fail "$label: $file unexpectedly matches: $pattern"
    fi
}

cargo() {
    printf 'cargo:%s:%s\n' "$PWD" "$*" >> "$SYNC_TEST_LOG"
    if [ -e "$PWD/.fail-cargo" ]; then
        printf 'fake cargo failure\n' >&2
        return 31
    fi
    version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$PWD/Cargo.toml")
    printf 'lock-version=%s\n' "$version" > "$PWD/Cargo.lock"
    printf '%s\n' '{"packages":[]}'
}

bun() {
    printf 'bun:%s:%s\n' "$PWD" "$*" >> "$SYNC_TEST_LOG"
    repo_root=$(CDPATH='' cd "$(dirname "$PWD")" && pwd)
    if [ -e "$repo_root/.fail-bun" ]; then
        printf 'fake bun failure\n' >&2
        return 32
    fi
    version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$repo_root/Cargo.toml")
    printf '{"version":"%s"}\n' "$version" > "$PWD/package.json"
}

export -f cargo bun

new_repo() {
    name=$1
    repo="$tmp_dir/$name"
    mkdir -p "$repo/scripts" "$repo/tools" "$repo/packages" "$repo/outside"
    ln -s "$sync_source" "$repo/scripts/sync-version.sh"
    ln -s "$reader_source" "$repo/scripts/workspace-version.sh"
    ln -s "$android_reader_source" "$repo/scripts/android-version.sh"
    printf '%s\n' \
        '[workspace]' \
        'members = []' \
        '' \
        '[workspace.package]' \
        'version = "2.3.4"' > "$repo/Cargo.toml"
    printf '%s\n' 'lock-version=1.0.0' > "$repo/Cargo.lock"
    printf '%s\n' '{"version":"1.0.0"}' > "$repo/packages/package.json"

    cat > "$repo/tools/check-version-sync.sh" <<'SCRIPT'
#!/bin/sh
set -eu
repo_root=$(CDPATH= cd "$(dirname "$0")/.." && pwd)
printf 'guard:%s\n' "$PWD" >> "$SYNC_TEST_LOG"
if [ -e "$repo_root/.fail-guard" ]; then
    printf 'fake guard failure\n' >&2
    exit 33
fi
SCRIPT
    chmod +x "$repo/tools/check-version-sync.sh"
    printf '%s\n' "$repo"
}

run_sync() {
    repo=$1
    shift
    case_status=0
    case_output=$(cd "$repo/outside" && env SYNC_TEST_LOG="$repo/calls.log" \
        bash "$repo/scripts/sync-version.sh" "$@" 2>&1) || case_status=$?
}

case_status=0
case_output=$("$sync_source" 2.3.4 2>&1) || case_status=$?
assert_status 2 'argument rejection'
assert_contains 'usage: scripts/sync-version.sh' 'argument rejection'
pass 'sync entrypoint rejects positional versions with actionable usage'

repo=$(new_repo reader_failure)
printf '%s\n' \
    '[workspace]' \
    'members = []' \
    '' \
    '[workspace.package]' \
    'version = "not-semver"' > "$repo/Cargo.toml"
run_sync "$repo"
assert_status 1 'canonical reader failure'
assert_contains 'sync-version: canonical version read failed; set a valid SemVer at [workspace.package].version in Cargo.toml' \
    'canonical reader failure'
if [[ -e "$repo/calls.log" ]]; then
    printf '%s\n' "$(cat "$repo/calls.log")" >&2
    fail 'canonical reader failure invoked Cargo, Bun, or the version guard'
fi
pass 'canonical reader failures stop before Cargo, Bun, and the version guard'

repo=$(new_repo external_cwd)
run_sync "$repo"
assert_status 0 'external cwd'
assert_file_contains "$repo/calls.log" "cargo:$repo:update --workspace --offline" \
    'external cwd'
assert_file_contains "$repo/calls.log" "cargo:$repo:metadata --locked --no-deps --format-version 1" \
    'external cwd'
assert_file_contains "$repo/calls.log" "bun:$repo/packages:run sync-version" 'external cwd'
assert_file_contains "$repo/calls.log" "guard:$repo" 'external cwd'
assert_contains 'sync-version: Android package version is 2.3.4 (versionCode 2003004)' \
    'external cwd'
assert_file_contains "$repo/Cargo.lock" 'lock-version=2.3.4' 'external cwd'
assert_file_contains "$repo/packages/package.json" '"version":"2.3.4"' 'external cwd'
pass 'sync entrypoint locates the repository and runs every stage from an external cwd'

first_hash=$(shasum "$repo/Cargo.lock" "$repo/packages/package.json")
run_sync "$repo"
assert_status 0 'idempotent rerun'
second_hash=$(shasum "$repo/Cargo.lock" "$repo/packages/package.json")
if [ "$first_hash" != "$second_hash" ]; then
    fail 'idempotent rerun changed managed file hashes'
fi
pass 'running synchronization twice leaves managed file hashes unchanged'

repo=$(new_repo cargo_failure)
touch "$repo/.fail-cargo"
run_sync "$repo"
assert_status 1 'Cargo stage failure'
assert_contains 'sync-version: Cargo lock refresh failed' 'Cargo stage failure'
pass 'Cargo refresh failures identify the failing synchronization stage'

repo=$(new_repo android_version_failure)
sed -i.bak 's/version = "2.3.4"/version = "0.0.0"/' "$repo/Cargo.toml"
rm "$repo/Cargo.toml.bak"
run_sync "$repo"
assert_status 1 'Android version stage failure'
assert_contains 'sync-version: Android version metadata validation failed' \
    'Android version stage failure'
if [[ -f "$repo/calls.log" ]] && rg --quiet '^bun:|^guard:' "$repo/calls.log"; then
    printf '%s\n' "$(cat "$repo/calls.log")" >&2
    fail 'Android version stage failure continued into Bun or the version guard'
fi
pass 'invalid Android version metadata stops synchronization before later stages'

repo=$(new_repo bun_failure)
touch "$repo/.fail-bun"
run_sync "$repo"
assert_status 1 'package stage failure'
assert_contains 'sync-version: package version synchronization failed' 'package stage failure'
pass 'package synchronization failures identify the failing stage'

repo=$(new_repo guard_failure)
touch "$repo/.fail-guard"
run_sync "$repo"
assert_status 1 'guard stage failure'
assert_contains 'sync-version: version consistency check failed' 'guard stage failure'
pass 'guard failures identify the final verification stage'

hook="$repo_root/.githooks/pre-commit"
assert_file_contains "$hook" 'tools/check-version-sync.sh' 'pre-commit guard'
assert_file_not_matches "$hook" 'rev-parse --abbrev-ref|grep -oE.*[0-9]|git add|jq --arg|\.tmp.*mv' \
    'pre-commit detect-only policy'
guard_line=$(grep -n 'tools/check-version-sync.sh' "$hook" | head -n 1 | cut -d: -f1)
fmt_line=$(grep -n 'cargo fmt' "$hook" | head -n 1 | cut -d: -f1)
if [ -z "$guard_line" ] || [ -z "$fmt_line" ] || [ "$guard_line" -ge "$fmt_line" ]; then
    fail 'pre-commit guard must run before Rust format and lint checks'
fi
pass 'pre-commit is detect-only and runs the version guard before Rust checks'

workflow="$repo_root/.github/workflows/version-sync.yml"
assert_file_contains "$workflow" 'pull_request:' 'version sync workflow trigger'
assert_file_contains "$workflow" 'push:' 'version sync workflow trigger'
assert_file_contains "$workflow" 'submodules: recursive' 'version sync checkout'
assert_file_contains "$workflow" \
    'actions/checkout@08eba0b27e820071cde6df949e0beb9ba4906955' \
    'version sync immutable checkout'
assert_file_contains "$workflow" \
    'dtolnay/rust-toolchain@4360b52568e2003a75bf9bc1d59f33a8e3fc893c' \
    'version sync immutable Rust setup'
assert_file_contains "$workflow" "toolchain: '1.94'" 'version sync exact Rust toolchain'
assert_file_contains "$workflow" 'tools/pinned-release-tools.test.sh' \
    'version sync pinned tool contract tests'
assert_file_contains "$workflow" \
    'tools/pinned-release-tools.sh bun "$RUNNER_TEMP/bun-1.3.14"' \
    'version sync digest-pinned Bun setup'
assert_file_contains "$workflow" \
    'tools/pinned-release-tools.sh ripgrep "$RUNNER_TEMP/ripgrep-15.2.0"' \
    'version sync digest-pinned ripgrep setup'
assert_file_contains "$workflow" 'run: rg --version' 'version sync pinned ripgrep gate'
assert_file_not_matches "$workflow" \
    'uses:[[:space:]]+[^@[:space:]]+@(v[0-9]+|stable|main|master|latest)|setup-bun|apt-get' \
    'version sync immutable tools policy'
assert_file_contains "$workflow" 'bun install --frozen-lockfile' 'version sync dependency install'
assert_file_contains "$workflow" 'packages/bun.lock' 'version sync managed paths'
assert_file_contains "$workflow" 'packages/scripts/sync-version.mjs' 'version sync implementation paths'
assert_file_contains "$workflow" 'scripts/android-version.sh' 'Android version implementation paths'
assert_file_contains "$workflow" 'packaging/android-player/app/build.gradle.kts' \
    'Android Gradle version consumer paths'
assert_file_not_matches "$workflow" 'packages/\*\*' 'version sync focused path filters'
for trigger in pull_request push; do
    trigger_block=$(awk -v trigger="$trigger" '
        $0 == "  " trigger ":" { in_trigger = 1; next }
        in_trigger && ($0 ~ /^  [a-z_]+:$/ || $0 ~ /^[^ ]/) { exit }
        in_trigger { print }
    ' "$workflow")
    for required_path in \
        'crates/**/*.rs' \
        'tools/pinned-release-tools.sh' \
        'tools/pinned-release-tools.test.sh'; do
        path_filter_count=$(grep -Fc -- "      - \"$required_path\"" \
            <<< "$trigger_block" || true)
        if [[ "$path_filter_count" -ne 1 ]]; then
            fail "version sync workflow $trigger paths must include $required_path exactly once (found $path_filter_count)"
        fi
    done
done
assert_file_contains "$workflow" 'scripts/workspace-version.test.sh' 'version reader tests'
assert_file_contains "$workflow" 'scripts/android-version.test.sh' 'Android version policy tests'
assert_file_contains "$workflow" 'scripts/sync-version.test.sh' 'version sync tests'
assert_file_contains "$workflow" 'tools/check-version-sync.test.sh' 'version guard tests'
assert_file_contains "$workflow" 'tools/check-version-sync.sh' 'version guard'
pass 'dedicated CI installs prerequisites and runs the focused version checks'

old_guard_name='check-version-'"fixtures"
if rg -n "$old_guard_name" "$repo_root" --hidden \
    --glob '!.git/**' --glob '!vendor/**' > "$tmp_dir/old-paths"; then
    sed 's/^/  /' "$tmp_dir/old-paths" >&2
    fail 'old version guard paths remain in the repository'
fi
pass 'repository no longer references the retired fixture-only guard name'

printf '1..%s\n' "$test_count"
