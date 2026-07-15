#!/usr/bin/env bash

set -euo pipefail

script_dir=$(CDPATH= cd "$(dirname "$0")" && pwd)
repo_root=$(CDPATH= cd "$script_dir/.." && pwd)
fixture_version=1.0.0

for required_command in cargo jq bun rg; do
    if ! command -v "$required_command" >/dev/null 2>&1; then
        printf 'version-sync: required command not found: %s\n' "$required_command" >&2
        exit 1
    fi
done

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

require_single_assignment() {
    file=$1
    variable=$2
    assignment_count=$(rg --count-matches \
        "^[[:space:]]*${variable}[[:space:]]*=" "$file" || true)
    if [[ "$assignment_count" != 1 ]]; then
        report_missing "$file" \
            "expected exactly one active ${variable} assignment (found ${assignment_count:-0})"
    fi
}

workflow_job_block() {
    job=$1
    awk -v job="$job" '
        $0 == "  " job ":" {
            in_job = 1
        }
        in_job && $0 ~ /^  [0-9A-Za-z_-]+:$/ && $0 != "  " job ":" {
            exit
        }
        in_job {
            print
        }
    ' "$release_workflow"
}

require_workflow_job_regex() {
    job=$1
    pattern=$2
    message=$3
    job_block=$(workflow_job_block "$job")
    if [[ -z "$job_block" ]] || ! rg --quiet -- "$pattern" <<< "$job_block"; then
        report_missing "$release_workflow" "$message"
    fi
}

validate_macos_version_behavior() {
    file=$1
    interpreter=$2
    validation_pattern=$3

    if ! rg --quiet -- "$validation_pattern" "$file"; then
        return
    fi

    validation_status=0
    validation_output=$(env -u OPENPENCIL_VERSION OPENPENCIL_VALIDATE_VERSION_ONLY=1 \
        "$interpreter" "$file" 2>&1) || validation_status=$?
    if [[ "$validation_status" -ne 0 || "$validation_output" != "$current_version" ]]; then
        report_missing "$file" \
            'validate-only mode without an override must print the canonical version and exit 0'
    fi

    validation_status=0
    validation_output=$(OPENPENCIL_VERSION="$current_version" \
        OPENPENCIL_VALIDATE_VERSION_ONLY=1 "$interpreter" "$file" 2>&1) || \
        validation_status=$?
    if [[ "$validation_status" -ne 0 || "$validation_output" != "$current_version" ]]; then
        report_missing "$file" \
            'validate-only mode with a matching override must print the canonical version and exit 0'
    fi

    mismatch_version=9.9.9
    if [[ "$current_version" == "$mismatch_version" ]]; then
        mismatch_version=0.0.0
    fi
    expected_mismatch="bundle-macos: error: OPENPENCIL_VERSION (${mismatch_version}) must match Cargo workspace version (${current_version})"
    validation_status=0
    validation_output=$(OPENPENCIL_VERSION="$mismatch_version" \
        OPENPENCIL_VALIDATE_VERSION_ONLY=1 "$interpreter" "$file" 2>&1) || \
        validation_status=$?
    if [[ "$validation_status" -ne 1 || "$validation_output" != "$expected_mismatch" ]]; then
        report_missing "$file" \
            'mismatched OPENPENCIL_VERSION must fail validation with actionable error'
    fi
}

reject_example_semver_tokens() {
    file=$1
    semver_pattern='(^|[^0-9A-Za-z.])(v?(0|[1-9][0-9]*)[.](0|[1-9][0-9]*)[.](0|[1-9][0-9]*)(-[0-9A-Za-z-]+([.][0-9A-Za-z-]+)*)?([+][0-9A-Za-z-]+([.][0-9A-Za-z-]+)*)?)([^0-9A-Za-z.]|[.]+([^0-9A-Za-z.]|$)|$)'
    rg_status=0
    matches=$(rg --line-number --with-filename --color never -- \
        "$semver_pattern" "$file") || rg_status=$?

    if [[ "$rg_status" -gt 1 ]]; then
        printf '%s:1: error: failed to scan version examples (rg status %s)\n' \
            "$file" "$rg_status" >&2
        errors=1
        return
    fi

    if [[ -n "$matches" ]]; then
        while IFS=: read -r match_file match_line match_text; do
            if [[ "$match_file" == scripts/package-windows.nsi && \
                    "$match_text" == '  !define VERSION "0.0.0"' ]]; then
                continue
            fi
            printf '%s:%s: error: version examples must use X.Y.Z or <version>, not a SemVer release\n' \
                "$match_file" "$match_line" >&2
            errors=1
        done <<< "$matches"
    fi
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

validate_workspace_package_versions() {
    if ! metadata=$(cargo metadata --no-deps --format-version 1 --locked); then
        report_missing Cargo.lock \
            'cargo metadata --locked failed; run scripts/sync-version.sh to refresh the lockfile'
        return
    fi

    mismatch_status=0
    mismatches=$(printf '%s\n' "$metadata" | jq -r \
        --arg crates_prefix "$repo_root/crates/" \
        --arg expected "$current_version" '
            .packages[]?
            | select(.name | startswith("op-"))
            | select(.manifest_path | startswith($crates_prefix))
            | select(.version != $expected)
            | [.manifest_path, .name, .version]
            | @tsv
        ' 2>&1) || mismatch_status=$?
    if [[ "$mismatch_status" -ne 0 ]]; then
        printf '%s\n' "$mismatches" >&2
        report_missing Cargo.toml 'failed to inspect cargo metadata with jq'
        return
    fi

    if [[ -n "$mismatches" ]]; then
        while IFS=$'\t' read -r manifest package package_version; do
            relative_manifest=${manifest#"$repo_root"/}
            printf '%s:1: error: workspace package %s has version %s; expected %s\n' \
                "$relative_manifest" "$package" "$package_version" "$current_version" >&2
            errors=1
        done <<< "$mismatches"
    fi
}

validate_package_versions() {
    package_status=0
    package_output=$(cd packages && bun run sync-version:check 2>&1) || package_status=$?
    if [[ "$package_status" -ne 0 ]]; then
        printf '%s\n' "$package_output" >&2
        report_missing packages \
            'bun run sync-version:check failed; run scripts/sync-version.sh to repair package drift'
    fi
}

validate_release_tag() {
    tag_name=
    if [[ "${GITHUB_REF:-}" == refs/tags/v* ]]; then
        tag_name=${GITHUB_REF#refs/tags/}
    elif [[ -z "${GITHUB_REF:-}" && "${GITHUB_REF_NAME:-}" == v* ]]; then
        tag_name=$GITHUB_REF_NAME
    fi

    if [[ -n "$tag_name" && "${tag_name#v}" != "$current_version" ]]; then
        report_missing environment \
            "release tag ${tag_name} does not match Cargo workspace version ${current_version}"
    fi
}

validate_cli_bundle_version_template() {
    bundle=crates/op-cli/assets/skill-bundle.json
    sentinel=__OPENPENCIL_VERSION__
    sentinel_count=$(rg --fixed-strings --count-matches -- "$sentinel" "$bundle" || true)
    if [[ "${sentinel_count:-0}" != 6 ]]; then
        report_missing "$bundle" \
            "expected exactly 6 version sentinels ${sentinel} (found ${sentinel_count:-0})"
    fi
    reject_matches fixed "$bundle" "$current_version" \
        'embedded CLI bundle must not contain the canonical version literal'
}

validate_rust_product_version_producers() {
    require_statement crates/op-editor-core/src/state.rs \
        'version:[[:space:]]*env!\("CARGO_PKG_VERSION"\)[.]to_owned\(\),' \
        'empty documents must derive their version from CARGO_PKG_VERSION'

    host_support=crates/op-editor-core/src/host_support.rs
    host_version_count=$(rg --fixed-strings --count-matches -- \
        'src.replace("__OPENPENCIL_VERSION__", env!("CARGO_PKG_VERSION"))' \
        "$host_support" || true)
    if [[ "$host_version_count" != 3 ]]; then
        report_missing "$host_support" \
            "expected 3 document templates to derive from CARGO_PKG_VERSION (found ${host_version_count:-0})"
    fi

    cli_source=crates/op-cli/src/app_control_cli.rs
    require_regex "$cli_source" 'env!\("CARGO_PKG_VERSION"\)' \
        'CLI starter documents must derive their version from CARGO_PKG_VERSION'
    reject_matches regex "$cli_source" \
        '"version"[^[:cntrl:]]*"[0-9]+[.][0-9]+[.][0-9]+' \
        'CLI starter documents must not hard-code a product version'

    reject_matches regex crates/op-host-desktop/Cargo.toml \
        '^[[:space:]]*op-host-native[[:space:]]*=.*path[[:space:]]*=.*version[[:space:]]*=' \
        'local op-host-native dependency must not duplicate the product version'
}

validate_workspace_package_versions
validate_package_versions
validate_release_tag
validate_cli_bundle_version_template
validate_rust_product_version_producers

if [[ "$current_version" == "$fixture_version" ]]; then
    printf 'version-sync: current product version %s equals stable fixture version %s; skipping literal fixture drift scan because stable fixtures and product-version literals are indistinguishable\n' \
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
        printf 'version-sync: failed to scan Rust sources with rg (status %s)\n' "$rg_status" >&2
        exit "$rg_status"
    fi

    if [[ -n "$matches" ]]; then
        printf 'version-sync: ordinary Rust fixtures copy current product version %s:\n' \
            "$current_version" >&2
        printf '%s\n' "$matches" >&2
        printf 'version-sync: use stable %s test data unless a test explicitly covers compatibility, migration, or updates\n' \
            "$fixture_version" >&2
        errors=1
    fi
fi

for macos_script in scripts/bundle-macos.sh tools/bundle-macos.sh; do
    require_single_assignment "$macos_script" CANONICAL_VERSION
    require_single_assignment "$macos_script" APP_VERSION
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
        validation_pattern='^[[:space:]]*if[[:space:]]+\[\[[[:space:]]*"\$\{OPENPENCIL_VALIDATE_VERSION_ONLY:-\}"[[:space:]]*==[[:space:]]*1[[:space:]]*\]\][[:space:]]*;[[:space:]]*then[[:space:]]*$'
        require_regex "$macos_script" "$validation_pattern" \
            'macOS packaging must support OPENPENCIL_VALIDATE_VERSION_ONLY immediately after version validation'
        validate_macos_version_behavior "$macos_script" bash "$validation_pattern"
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
        validation_pattern='^[[:space:]]*if[[:space:]]+\[[[:space:]]*"\$\{OPENPENCIL_VALIDATE_VERSION_ONLY:-\}"[[:space:]]*=[[:space:]]*1[[:space:]]*\][[:space:]]*;[[:space:]]*then[[:space:]]*$'
        require_regex "$macos_script" "$validation_pattern" \
            'macOS packaging must support OPENPENCIL_VALIDATE_VERSION_ONLY immediately after version validation'
        validate_macos_version_behavior "$macos_script" sh "$validation_pattern"
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

require_regex scripts/package-windows.nsi \
    '^[[:space:]]*;[[:space:]]*makensis[[:space:]]+"/DVERSION=X[.]Y[.]Z"' \
    'NSIS compile example must use /DVERSION=X.Y.Z'
require_regex scripts/install-op.sh \
    '^[[:space:]]*#[[:space:]]*OP_VERSION=(X[.]Y[.]Z|<version>)[[:space:]]+[.]/install-op[.]sh' \
    'install usage example must use OP_VERSION=X.Y.Z or OP_VERSION=<version>'
reject_example_semver_tokens scripts/package-windows.nsi
reject_example_semver_tokens scripts/install-op.sh

release_workflow=.github/workflows/rust-release.yml
require_workflow_job_regex version \
    '^[[:space:]]*-[[:space:]]+uses:[[:space:]]+actions/checkout@v4[[:space:]]*$' \
    'version preflight must check out the repository'
require_workflow_job_regex version \
    '^[[:space:]]*version:[[:space:]]*\$\{\{[[:space:]]*steps[.]version[.]outputs[.]version[[:space:]]*\}\}[[:space:]]*$' \
    'version preflight must expose the canonical version as a job output'
require_workflow_job_regex version \
    '^[[:space:]]*(-[[:space:]]+)?id:[[:space:]]+version[[:space:]]*$' \
    'version preflight must identify the canonical version step'
require_workflow_job_regex version \
    '^[[:space:]]*cargo_version[[:space:]]*=[[:space:]]*"\$\(scripts/workspace-version[.]sh\)"[[:space:]]*$' \
    'release version computation must invoke scripts/workspace-version.sh'
require_workflow_job_regex version \
    '^[[:space:]]*tag_version[[:space:]]*=[[:space:]]*"\$\{GITHUB_REF_NAME#v\}"[[:space:]]*$' \
    'release version computation must derive the version from v* tags'
require_workflow_job_regex version \
    '^[[:space:]]*if[[:space:]]+\[\[[[:space:]]*"\$tag_version"[[:space:]]*!=[[:space:]]*"\$cargo_version"[[:space:]]*\]\][[:space:]]*;[[:space:]]*then[[:space:]]*$' \
    'release tags must be compared with the Cargo workspace version'
require_workflow_job_regex version \
    '^[[:space:]]*echo[[:space:]]+"version=\$cargo_version"[[:space:]]*>>[[:space:]]*"\$GITHUB_OUTPUT"[[:space:]]*$' \
    'version preflight must write the canonical version to GITHUB_OUTPUT'
require_single_assignment "$release_workflow" cargo_version
require_single_assignment "$release_workflow" tag_version
require_workflow_job_regex build \
    '^[[:space:]]*needs:[[:space:]]*version[[:space:]]*$' \
    'build must depend on the version preflight job'
require_workflow_job_regex web-docker \
    '^[[:space:]]*needs:[[:space:]]*version[[:space:]]*$' \
    'web-docker must depend on the version preflight job'
require_workflow_job_regex sdk-packages \
    '^[[:space:]]*needs:[[:space:]]*version[[:space:]]*$' \
    'sdk-packages must depend on the version preflight job'
require_workflow_job_regex release-draft \
    '^[[:space:]]*needs:[[:space:]]*\[version,[[:space:]]*build,[[:space:]]*web-docker,[[:space:]]*sdk-packages\][[:space:]]*$' \
    'release-draft must preserve artifact dependencies and depend on version preflight'
require_workflow_job_regex package-managers \
    '^[[:space:]]*needs:[[:space:]]*\[version,[[:space:]]*release-draft\][[:space:]]*$' \
    'package-managers must preserve release dependency and depend on version preflight'
for version_consumer in build web-docker sdk-packages release-draft package-managers; do
    require_workflow_job_regex "$version_consumer" \
        'needs[.]version[.]outputs[.]version' \
        "${version_consumer} must consume the canonical version job output"
done
require_workflow_job_regex sdk-packages \
    'bun[[:space:]]+run[[:space:]]+sync-version:check' \
    'sdk-packages must verify package versions after installing dependencies'
reject_matches regex "$release_workflow" \
    '^[[:space:]]*version[[:space:]]*=.*GITHUB_REF_NAME#v' \
    'publish paths must consume the canonical version job output'
reject_matches regex "$release_workflow" \
    '^[[:space:]]*tag[[:space:]]*=[[:space:]]*"\$GITHUB_REF_NAME"' \
    'publish paths must not begin an independent two-step tag derivation'
reject_matches regex "$release_workflow" \
    '^[[:space:]]*version[[:space:]]*=[[:space:]]*"\$\{tag#v\}"' \
    'publish paths must consume the canonical version job output'
reject_matches fixed "$release_workflow" 'echo "OP_VERSION=${GITHUB_REF_NAME#v}"' \
    'OP_VERSION must not be written directly from the release tag'
reject_matches fixed "$release_workflow" 'echo "OP_VERSION=${ver:-0.0.0}"' \
    'OP_VERSION must not use an independent manifest parser or fallback'

if [[ "$errors" -ne 0 ]]; then
    exit 1
fi

if [[ "$fixture_scan_skipped" -eq 0 ]]; then
    printf 'version-sync: no ordinary Rust fixtures copy current product version %s\n' \
        "$current_version"
fi
printf 'version-sync: all managed versions derive from Cargo workspace version %s\n' \
    "$current_version"
