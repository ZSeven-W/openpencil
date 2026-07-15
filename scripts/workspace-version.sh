#!/bin/sh

set -eu

if [ "$#" -gt 1 ]; then
    printf 'workspace-version: expected zero or one manifest path argument\n' >&2
    exit 1
fi

script_dir=$(CDPATH= cd "$(dirname "$0")" && pwd)
manifest=${1-"$script_dir/../Cargo.toml"}

if [ ! -f "$manifest" ] || [ ! -r "$manifest" ]; then
    printf 'workspace-version: cannot read manifest: %s\n' "$manifest" >&2
    exit 1
fi

if version=$(awk '
function valid_core_number(value) {
    return value == "0" || value ~ /^[1-9][0-9]*$/
}

function valid_identifiers(value, reject_leading_zero, count, i, identifiers) {
    if (value == "") {
        return 0
    }

    count = split(value, identifiers, "[.]")
    for (i = 1; i <= count; i++) {
        if (identifiers[i] !~ /^[0-9A-Za-z-]+$/) {
            return 0
        }
        if (reject_leading_zero && identifiers[i] ~ /^[0-9]+$/ &&
                length(identifiers[i]) > 1 && substr(identifiers[i], 1, 1) == "0") {
            return 0
        }
    }
    return 1
}

function valid_semver(value, base, core, prerelease, build, plus, dash, count, parts) {
    base = value
    plus = index(base, "+")
    if (plus > 0) {
        build = substr(base, plus + 1)
        if (!valid_identifiers(build, 0)) {
            return 0
        }
        base = substr(base, 1, plus - 1)
    }

    dash = index(base, "-")
    if (dash > 0) {
        prerelease = substr(base, dash + 1)
        if (!valid_identifiers(prerelease, 1)) {
            return 0
        }
        core = substr(base, 1, dash - 1)
    } else {
        core = base
    }

    count = split(core, parts, "[.]")
    return count == 3 && valid_core_number(parts[1]) &&
        valid_core_number(parts[2]) && valid_core_number(parts[3])
}

/^[[:space:]]*\[[[:space:]]*workspace[[:space:]]*[.][[:space:]]*package[[:space:]]*\][[:space:]]*(#.*)?$/ {
    section_count++
    in_workspace_package = 1
    next
}

/^[[:space:]]*\[/ {
    in_workspace_package = 0
    next
}

in_workspace_package && /^[[:space:]]*version[[:space:]]*=/ {
    version_count++
    if ($0 !~ /^[[:space:]]*version[[:space:]]*=[[:space:]]*"[^"]*"[[:space:]]*(#.*)?$/) {
        invalid_assignment = 1
        next
    }

    value = $0
    sub(/^[^"]*"/, "", value)
    sub(/".*$/, "", value)
    workspace_version = value
}

END {
    if (section_count == 0) {
        exit 10
    }
    if (section_count > 1) {
        exit 11
    }
    if (version_count == 0) {
        exit 12
    }
    if (version_count > 1) {
        exit 13
    }
    if (invalid_assignment || !valid_semver(workspace_version)) {
        exit 14
    }

    print workspace_version
}
' < "$manifest"); then
    printf '%s\n' "$version"
else
    status=$?
    case "$status" in
        10)
            printf 'workspace-version: missing [workspace.package] section in %s\n' "$manifest" >&2
            ;;
        11)
            printf 'workspace-version: expected exactly one [workspace.package] section in %s\n' "$manifest" >&2
            ;;
        12)
            printf 'workspace-version: missing version in [workspace.package] in %s\n' "$manifest" >&2
            ;;
        13)
            printf 'workspace-version: expected exactly one version in [workspace.package] in %s\n' "$manifest" >&2
            ;;
        14)
            printf 'workspace-version: invalid version in [workspace.package] in %s\n' "$manifest" >&2
            ;;
        *)
            printf 'workspace-version: failed to parse manifest: %s\n' "$manifest" >&2
            ;;
    esac
    exit 1
fi
