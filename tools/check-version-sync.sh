#!/usr/bin/env bash

# The policy expressions below intentionally quote shell syntax as literal
# regular expressions; they must not expand in this guard process.
# shellcheck disable=SC2016

set -euo pipefail

script_dir=$(CDPATH='' cd "$(dirname "$0")" && pwd -P)
repo_root=$(CDPATH='' cd "$script_dir/.." && pwd -P)
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

validate_top_level_readmes() {
    semver_pattern='(?<![0-9A-Za-z.])v?(?:0|[1-9][0-9]*)[.](?:0|[1-9][0-9]*)[.](?:0|[1-9][0-9]*)(?:-[0-9A-Za-z-]+(?:[.][0-9A-Za-z-]+)*)?(?:[+][0-9A-Za-z-]+(?:[.][0-9A-Za-z-]+)*)?(?![0-9A-Za-z]|[.][0-9A-Za-z])'
    readme_files=(README*.md)

    for file in "${readme_files[@]}"; do
        rg_status=0
        matches=$(rg --pcre2 --only-matching --line-number --with-filename \
            --color never -- "$semver_pattern" "$file") || rg_status=$?
        if [[ "$rg_status" -gt 1 ]]; then
            printf '%s:1: error: failed to scan README version policy (rg status %s)\n' \
                "$file" "$rg_status" >&2
            errors=1
            continue
        fi

        if [[ -n "$matches" ]]; then
            while IFS=: read -r match_file match_line token; do
                if [[ "$token" == v0.7.5 ]]; then
                    continue
                fi
                printf '%s:%s: error: top-level READMEs must not contain active product SemVer releases; use vX.Y.Z or the workspace-version reader\n' \
                    "$match_file" "$match_line" >&2
                errors=1
            done <<< "$matches"
        fi
    done
}

validate_version_sync_ci_readme_paths() {
    ci_workflow=.github/workflows/version-sync.yml
    if [[ ! -f "$ci_workflow" ]]; then
        report_missing "$ci_workflow" \
            'version-sync CI must run for top-level README changes in pull requests and pushes'
        return
    fi

    read -r pull_request_readmes push_readmes < <(
        awk '
            /^  pull_request:[[:space:]]*$/ { event = "pull_request"; next }
            /^  push:[[:space:]]*$/ { event = "push"; next }
            /^[^[:space:]]/ { event = "" }
            /^[[:space:]]*-[[:space:]]+"README[*][.]md"[[:space:]]*$/ {
                if (event == "pull_request") pull_request_count++
                if (event == "push") push_count++
            }
            END { print pull_request_count + 0, push_count + 0 }
        ' "$ci_workflow"
    )
    if [[ "$pull_request_readmes" != 1 || "$push_readmes" != 1 ]]; then
        report_missing "$ci_workflow" \
            'version-sync CI must run for top-level README changes in pull requests and pushes'
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

validate_manifest_workspace_version() {
    manifest=$1
    relative_manifest=$2

    if [[ ! -f "$manifest" ]]; then
        report_missing "$relative_manifest" \
            'local op-* package manifest returned by cargo metadata does not exist'
        return
    fi

    manifest_result=$(awk '
        BEGIN {
            in_package = 0
            inheritance_count = 0
            package_header_line = 1
            declaration_line = 0
        }
        {
            line = $0
            sub(/\r$/, "", line)

            if (line ~ /^[[:space:]]*\[[^]]+\][[:space:]]*(#.*)?$/) {
                header = line
                sub(/[[:space:]]*#.*/, "", header)
                gsub(/[[:space:]]/, "", header)
                in_package = (header == "[package]")
                if (in_package && package_header_line == 1) {
                    package_header_line = NR
                }
                next
            }

            if (!in_package || line ~ /^[[:space:]]*#/) {
                next
            }

            code = line
            sub(/[[:space:]]*#.*/, "", code)
            if (code ~ /^[[:space:]]*version[.]workspace[[:space:]]*=[[:space:]]*true[[:space:]]*$/) {
                inheritance_count++
                if (declaration_line == 0) declaration_line = NR
                next
            }
            if (declaration_line == 0 &&
                    code ~ /^[[:space:]]*version([.]workspace)?[[:space:]]*=/) {
                declaration_line = NR
            }
        }
        END {
            diagnostic_line = declaration_line == 0 ? package_header_line : declaration_line
            printf "%d\t%d\n", inheritance_count, diagnostic_line
        }
    ' "$manifest")
    IFS=$'\t' read -r inheritance_count diagnostic_line <<< "$manifest_result"

    if [[ "$inheritance_count" != 1 ]]; then
        printf '%s:%s: error: local op-* package must declare exactly one active version.workspace = true in [package] (found %s)\n' \
            "$relative_manifest" "$diagnostic_line" "$inheritance_count" >&2
        errors=1
    fi
}

validate_workspace_package_versions() {
    if ! metadata=$(cargo metadata --no-deps --format-version 1 --locked); then
        report_missing Cargo.lock \
            'cargo metadata --locked failed; run scripts/sync-version.sh to refresh the lockfile'
        return
    fi

    package_rows_status=0
    package_rows=$(printf '%s\n' "$metadata" | jq -r \
        --arg crates_prefix "$repo_root/crates/" \
        '
            .packages[]?
            | select(.name | startswith("op-"))
            | select(.manifest_path | startswith($crates_prefix))
            | [.manifest_path, .name, .version]
            | @tsv
        ' 2>&1) || package_rows_status=$?
    if [[ "$package_rows_status" -ne 0 ]]; then
        printf '%s\n' "$package_rows" >&2
        report_missing Cargo.toml 'failed to inspect cargo metadata with jq'
        return
    fi

    if [[ -z "$package_rows" ]]; then
        report_missing Cargo.toml \
            'cargo metadata found no local op-* workspace packages under crates; verify workspace members and repository path resolution'
        return
    fi

    while IFS=$'\t' read -r manifest package package_version; do
        relative_manifest=${manifest#"$repo_root"/}
        validate_manifest_workspace_version "$manifest" "$relative_manifest"
        if [[ "$package_version" != "$current_version" ]]; then
            printf '%s:1: error: workspace package %s has version %s; expected %s\n' \
                "$relative_manifest" "$package" "$package_version" "$current_version" >&2
            errors=1
        fi
    done <<< "$package_rows"
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

validate_chrome_extension_manifest_version() {
    manifest=packages/op-chrome-extension/manifest.json
    if [[ ! -f "$manifest" ]]; then
        report_missing "$manifest" \
            'Chrome extension manifest is missing; it must exist and carry the Cargo workspace version'
        return
    fi
    manifest_version=$(jq -r '.version' "$manifest")
    if [[ "$manifest_version" != "$current_version" ]]; then
        report_missing "$manifest" \
            "Chrome extension manifest version ${manifest_version} must match Cargo workspace version ${current_version}; edit the \"version\" field (scripts/sync-version.sh does not rewrite this file — see packages/op-chrome-extension/README.md)"
    fi
}

validate_android_version_metadata() {
    android_script=scripts/android-version.sh
    android_gradle=packaging/android/app/build.gradle.kts

    if [[ ! -x "$android_script" ]]; then
        report_missing "$android_script" \
            'Android version resolver must exist and be executable'
        return
    fi

    android_status=0
    android_output=$("$android_script" "$repo_root/Cargo.toml" 2>&1) || android_status=$?
    if [[ "$android_status" -ne 0 ]]; then
        printf '%s\n' "$android_output" >&2
        report_missing "$android_script" \
            'failed to derive Android version metadata from the Cargo workspace version'
        return
    fi

    android_name=$(printf '%s\n' "$android_output" | sed -n 's/^versionName=//p')
    android_code=$(printf '%s\n' "$android_output" | sed -n 's/^versionCode=//p')
    IFS=. read -r android_major android_minor android_patch <<< "$current_version"
    expected_android_code=$((android_major * 1000000 + android_minor * 1000 + android_patch))
    expected_android_output=$(printf 'versionName=%s\nversionCode=%s' \
        "$current_version" "$expected_android_code")
    if [[ "$android_name" != "$current_version" || \
            "$android_code" != "$expected_android_code" || \
            "$android_output" != "$expected_android_output" || \
            ! "$android_code" =~ ^[1-9][0-9]*$ || \
            "${#android_code}" -gt 10 || \
            "$android_code" -gt 2100000000 ]]; then
        report_missing "$android_script" \
            'Android version resolver must emit exactly the canonical versionName and one valid versionCode'
    fi

    require_single_assignment "$android_gradle" versionName
    require_single_assignment "$android_gradle" versionCode
    require_statement "$android_gradle" \
        'versionName[[:space:]]*=[[:space:]]*canonicalVersionName' \
        'Android versionName must use canonicalVersionName'
    require_statement "$android_gradle" \
        'versionCode[[:space:]]*=[[:space:]]*canonicalVersionCode' \
        'Android versionCode must use canonicalVersionCode'
    require_regex "$android_gradle" \
        '^[[:space:]]*val[[:space:]]+androidVersionOutput[[:space:]]*=[[:space:]]*providers[.]exec[[:space:]]*\{' \
        'Android Gradle configuration must execute the canonical version resolver'
    require_statement "$android_gradle" \
        'repositoryRoot[.]file\(\"scripts/android-version[.]sh\"\)[.]asFile[.]absolutePath,' \
        'Android Gradle configuration must invoke scripts/android-version.sh'
    require_statement "$android_gradle" \
        'repositoryRoot[.]file\(\"Cargo[.]toml\"\)[.]asFile[.]absolutePath,' \
        'Android Gradle configuration must pass the root Cargo.toml explicitly'
    require_regex "$android_gradle" \
        '^[[:space:]]*val[[:space:]]+canonicalVersionName[[:space:]]*=[[:space:]]*Regex\(\"\"\"versionName=' \
        'canonicalVersionName must be parsed from strict resolver metadata'
    require_regex "$android_gradle" \
        '^[[:space:]]*[.]matchEntire\(androidVersionLines\[0\]\)' \
        'canonicalVersionName must consume the resolver versionName line'
    require_regex "$android_gradle" \
        '^[[:space:]]*val[[:space:]]+canonicalVersionCode[[:space:]]*=[[:space:]]*Regex\(\"\"\"versionCode=' \
        'canonicalVersionCode must be parsed from strict resolver metadata'
    require_regex "$android_gradle" \
        '^[[:space:]]*[.]matchEntire\(androidVersionLines\[1\]\)' \
        'canonicalVersionCode must consume the resolver versionCode line'
    reject_matches regex "$android_gradle" \
        '^[[:space:]]*val[[:space:]]+canonicalVersion(Name|Code)[[:space:]]*=[[:space:]]*[\"0-9]' \
        'canonical Android version values must not be hard-coded'
    reject_matches regex "$android_gradle" \
        '^[[:space:]]*versionName[[:space:]]*=[[:space:]]*\"[0-9]' \
        'Android versionName must not be hard-coded'
    reject_matches regex "$android_gradle" \
        '^[[:space:]]*versionCode[[:space:]]*=[[:space:]]*[0-9]' \
        'Android versionCode must not be hard-coded'
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
    if [[ "${sentinel_count:-0}" != 5 ]]; then
        report_missing "$bundle" \
            "expected exactly 5 version sentinels ${sentinel} (found ${sentinel_count:-0})"
    fi
    reject_matches fixed "$bundle" "$current_version" \
        'embedded CLI bundle must not contain the canonical version literal'
}

validate_rust_product_version_producers() {
    require_statement crates/op-editor-core/src/state.rs \
        'version:[[:space:]]*env!\("CARGO_PKG_VERSION"\)[.]to_owned\(\),' \
        'empty documents must derive their version from CARGO_PKG_VERSION'

    host_support=crates/op-editor-core/src/host_support.rs
    read -r host_production_version_count host_test_version_count < <(
        awk \
            -v needle='src.replace("__OPENPENCIL_VERSION__", env!("CARGO_PKG_VERSION"))' \
            '
                /^#[[:space:]]*\[cfg\(test\)\][[:space:]]*$/ { in_tests = 1 }
                index($0, needle) {
                    if (in_tests) test_count++
                    else production_count++
                }
                END { print production_count + 0, test_count + 0 }
            ' "$host_support"
    )
    if [[ "$host_production_version_count" != 2 ]]; then
        report_missing "$host_support" \
            "expected exactly 2 production document templates to derive from CARGO_PKG_VERSION (found ${host_production_version_count:-0})"
    fi
    if [[ "$host_test_version_count" != 0 ]]; then
        report_missing "$host_support" \
            'ordinary test fixtures must use stable 1.0.0 instead of CARGO_PKG_VERSION'
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
validate_chrome_extension_manifest_version
validate_android_version_metadata
validate_release_tag
validate_cli_bundle_version_template
validate_rust_product_version_producers
validate_top_level_readmes
validate_version_sync_ci_readme_paths

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
    '^[[:space:]]*(-[[:space:]]+)?uses:[[:space:]]+actions/checkout@[0-9a-f]{40}([[:space:]]*#.*)?$' \
    'version preflight must use an immutable repository checkout'
require_workflow_job_regex version \
    '^[[:space:]]*version:[[:space:]]*\$\{\{[[:space:]]*steps[.]version[.]outputs[.]version[[:space:]]*\}\}[[:space:]]*$' \
    'version preflight must expose the canonical version as a job output'
require_workflow_job_regex version \
    '^[[:space:]]*(-[[:space:]]+)?id:[[:space:]]+version[[:space:]]*$' \
    'version preflight must identify the canonical version step'
require_workflow_job_regex version \
    '^[[:space:]]*version=\$\(scripts/workspace-version[.]sh\)[[:space:]]*$' \
    'release version computation must use the canonical workspace reader'
require_workflow_job_regex version \
    '^[[:space:]]*tools/check-op-auth-release-matrix[.]sh[[:space:]]*$' \
    'release preflight must validate the adopted signed Auth matrix'
require_workflow_job_regex version \
    '^[[:space:]]*tools/check-op-auth-prebuilt[.]sh[[:space:]]+--require-hardened[[:space:]]*$' \
    'release preflight must reject an incomplete or development Auth matrix'
require_workflow_job_regex version \
    'refs/heads/v\$version.*refs/tags/v\$version' \
    'release preflight must validate the exact version branch or tag ref'
require_workflow_job_regex version \
    'git rev-parse HEAD.*GITHUB_SHA' \
    'release preflight must bind the checkout to the event source SHA'
require_workflow_job_regex version \
    '^[[:space:]]*echo[[:space:]]+"version=\$version"[[:space:]]+>>[[:space:]]+"\$GITHUB_OUTPUT"[[:space:]]*$' \
    'release preflight must write the canonical version to GITHUB_OUTPUT'
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
    '^[[:space:]]*needs:[[:space:]]*\[version,[[:space:]]*android-release,[[:space:]]*build,[[:space:]]*web-docker,[[:space:]]*sdk-packages,[[:space:]]*vsix\][[:space:]]*$' \
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
