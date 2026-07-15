#!/usr/bin/env bash

set -euo pipefail

script_dir=$(CDPATH= cd "$(dirname "$0")" && pwd)
repo_root=$(CDPATH= cd "$script_dir/.." && pwd)
fixture_version=1.0.0
current_version=$(bash "$repo_root/scripts/workspace-version.sh")

cd "$repo_root"

errors=0
fixture_scan_skipped=0

report_missing() {
    file=$1
    message=$2
    printf '%s:1: error: %s\n' "$file" "$message" >&2
    errors=1
}

require_regex() {
    file=$1
    pattern=$2
    message=$3
    if [[ ! -f "$file" ]] || ! rg --quiet -- "$pattern" "$file"; then
        report_missing "$file" "$message"
    fi
}

require_statement() {
    file=$1
    statement_pattern=$2
    message=$3
    require_regex "$file" \
        "^[[:space:]]*${statement_pattern}[[:space:]]*(#.*)?$" \
        "$message"
}

reject_matches() {
    mode=$1
    file=$2
    pattern=$3
    message=$4
    rg_status=0
    if [[ "$mode" == fixed ]]; then
        matches=$(rg --fixed-strings --line-number --with-filename --color never -- \
            "$pattern" "$file") || rg_status=$?
    else
        matches=$(rg --line-number --with-filename --color never -- \
            "$pattern" "$file") || rg_status=$?
    fi

    if [[ "$rg_status" -gt 1 ]]; then
        printf '%s:1: error: failed to scan version policy (rg status %s)\n' \
            "$file" "$rg_status" >&2
        errors=1
        return
    fi

    if [[ -n "$matches" ]]; then
        while IFS=: read -r match_file match_line _; do
            printf '%s:%s: error: %s\n' "$match_file" "$match_line" "$message" >&2
        done <<< "$matches"
        errors=1
    fi
}

if [[ "$current_version" == "$fixture_version" ]]; then
    printf 'version-fixtures: current product version %s equals stable fixture version %s; skipping literal fixture drift scan because stable fixtures and product-version literals are indistinguishable\n' \
        "$current_version" "$fixture_version"
    fixture_scan_skipped=1
else
    rg_status=0
    matches=$(rg \
        --fixed-strings \
        --line-number \
        --with-filename \
        --color never \
        --glob '*.rs' \
        --glob '!**/op-host-desktop/src/update_check.rs' \
        "$current_version" \
        crates) || rg_status=$?

    if [[ "$rg_status" -gt 1 ]]; then
        printf 'version-fixtures: failed to scan Rust sources with rg (status %s)\n' "$rg_status" >&2
        exit "$rg_status"
    fi

    if [[ -n "$matches" ]]; then
        printf 'version-fixtures: ordinary Rust fixtures copy current product version %s:\n' \
            "$current_version" >&2
        printf '%s\n' "$matches" >&2
        printf 'version-fixtures: use stable %s test data unless a test explicitly covers compatibility, migration, or updates\n' \
            "$fixture_version" >&2
        errors=1
    fi
fi

for macos_script in scripts/bundle-macos.sh tools/bundle-macos.sh; do
    if [[ "$macos_script" == scripts/bundle-macos.sh ]]; then
        require_statement "$macos_script" \
            'CANONICAL_VERSION[[:space:]]*=[[:space:]]*"\$\("\$WS_ROOT/scripts/workspace-version[.]sh"\)"' \
            'macOS packaging must assign CANONICAL_VERSION from scripts/workspace-version.sh'
        require_statement "$macos_script" \
            '/usr/libexec/PlistBuddy[[:space:]]+-c[[:space:]]+"Set :CFBundleShortVersionString \$APP_VERSION"[[:space:]]+"\$PLIST"' \
            'CFBundleShortVersionString must use APP_VERSION'
        require_statement "$macos_script" \
            'if[[:space:]]+\[\[[[:space:]]*"\$APP_VERSION"[[:space:]]*!=[[:space:]]*"\$CANONICAL_VERSION"[[:space:]]*\]\][[:space:]]*;[[:space:]]*then' \
            'OPENPENCIL_VERSION overrides must be rejected when they differ from Cargo'
    else
        require_statement "$macos_script" \
            'CANONICAL_VERSION[[:space:]]*=[[:space:]]*"\$\("\$ROOT/scripts/workspace-version[.]sh"\)"' \
            'macOS packaging must assign CANONICAL_VERSION from scripts/workspace-version.sh'
        require_statement "$macos_script" \
            '<key>CFBundleShortVersionString</key><string>\$\{APP_VERSION\}</string>' \
            'CFBundleShortVersionString must use APP_VERSION'
        require_statement "$macos_script" \
            'if[[:space:]]+\[[[:space:]]*"\$APP_VERSION"[[:space:]]*!=[[:space:]]*"\$CANONICAL_VERSION"[[:space:]]*\][[:space:]]*;[[:space:]]*then' \
            'OPENPENCIL_VERSION overrides must be rejected when they differ from Cargo'
    fi
    require_statement "$macos_script" \
        'APP_VERSION[[:space:]]*=[[:space:]]*"\$\{OPENPENCIL_VERSION:-\$CANONICAL_VERSION\}"' \
        'OPENPENCIL_VERSION must default to the Cargo workspace version'
    reject_matches regex "$macos_script" \
        'OPENPENCIL_VERSION:-[0-9]+[.][0-9]+[.][0-9]+' \
        'OPENPENCIL_VERSION must fall back to the Cargo workspace version'
    reject_matches regex "$macos_script" \
        'CFBundleShortVersionString[^[:cntrl:]]*[0-9]+[.][0-9]+[.][0-9]+' \
        'CFBundleShortVersionString must use the resolved Cargo workspace version'
done

for example_file in scripts/package-windows.nsi scripts/install-op.sh; do
    require_regex "$example_file" 'X[.]Y[.]Z|<version>' \
        'version examples must use X.Y.Z or <version>'
    reject_matches fixed "$example_file" '0.8.1' \
        'version examples must use X.Y.Z or <version>, not a product release'
    if [[ "$current_version" != 0.8.1 ]]; then
        reject_matches fixed "$example_file" "$current_version" \
            'version examples must use X.Y.Z or <version>, not the current Cargo version'
    fi
done

release_workflow=.github/workflows/rust-release.yml
require_statement "$release_workflow" \
    'cargo_version[[:space:]]*=[[:space:]]*"\$\(scripts/workspace-version[.]sh\)"' \
    'release version computation must invoke scripts/workspace-version.sh'
require_statement "$release_workflow" \
    'tag_version[[:space:]]*=[[:space:]]*"\$\{GITHUB_REF_NAME#v\}"' \
    'release version computation must derive the version from v* tags'
require_statement "$release_workflow" \
    'if[[:space:]]+\[\[[[:space:]]*"\$tag_version"[[:space:]]*!=[[:space:]]*"\$cargo_version"[[:space:]]*\]\][[:space:]]*;[[:space:]]*then' \
    'release tags must be compared with the Cargo workspace version'
require_statement "$release_workflow" \
    'echo[[:space:]]+"OP_VERSION=\$cargo_version"[[:space:]]*>>[[:space:]]*"\$GITHUB_ENV"' \
    'OP_VERSION must always be written from the Cargo workspace version'
reject_matches fixed "$release_workflow" 'echo "OP_VERSION=${GITHUB_REF_NAME#v}"' \
    'OP_VERSION must not be written directly from the release tag'
reject_matches fixed "$release_workflow" 'echo "OP_VERSION=${ver:-0.0.0}"' \
    'OP_VERSION must not use an independent manifest parser or fallback'

if [[ "$errors" -ne 0 ]]; then
    exit 1
fi

if [[ "$fixture_scan_skipped" -eq 0 ]]; then
    printf 'version-fixtures: no ordinary Rust fixtures copy current product version %s\n' \
        "$current_version"
fi
printf 'version-fixtures: packaging and release versions derive from Cargo workspace version %s\n' \
    "$current_version"
