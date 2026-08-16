#!/usr/bin/env bash
# Derive a repository-independent, monotonically increasing CFBundleVersion
# from UTC Unix epoch minutes. Eight decimal minute digits map to Apple’s
# conservative 4.2.2 numeric segment limits and remain available until 2160.

set -euo pipefail

format_epoch_minutes() {
    local epoch_minutes=$1
    local major minor patch

    [[ "$epoch_minutes" =~ ^[1-9][0-9]{7}$ ]] || {
        printf 'error: UTC epoch minutes must contain exactly eight digits\n' >&2
        return 2
    }
    major=${epoch_minutes:0:4}
    minor=$((10#${epoch_minutes:4:2}))
    patch=$((10#${epoch_minutes:6:2}))
    printf '%s.%s.%s\n' "$major" "$minor" "$patch"
}

self_test() {
    local input expected actual
    while IFS=' ' read -r input expected; do
        actual=$(format_epoch_minutes "$input")
        [[ "$actual" == "$expected" ]] || {
            printf 'error: iOS build number fixture mismatch: %s != %s\n' \
                "$actual" "$expected" >&2
            return 1
        }
    done <<'EOF'
10000000 1000.0.0
29810105 2981.1.5
29812345 2981.23.45
99999999 9999.99.99
EOF

    for input in 9999999 100000000 02981234 not-a-number; do
        if format_epoch_minutes "$input" >/dev/null 2>&1; then
            printf 'error: invalid iOS build number fixture was accepted: %s\n' \
                "$input" >&2
            return 1
        fi
    done
    printf 'ios-build-number.sh: 4.2.2 CFBundleVersion fixtures passed.\n'
}

case ${1-} in
    --self-test)
        [[ $# -eq 1 ]] || {
            printf 'usage: %s [--self-test]\n' "$0" >&2
            exit 2
        }
        self_test
        exit
        ;;
    '')
        [[ $# -eq 0 ]] || exit 2
        ;;
    *)
        printf 'usage: %s [--self-test]\n' "$0" >&2
        exit 2
        ;;
esac

epoch_seconds=$(date -u +%s)
[[ "$epoch_seconds" =~ ^[1-9][0-9]*$ ]] || {
    printf 'error: failed to resolve the current UTC Unix epoch\n' >&2
    exit 1
}
epoch_minutes=$((10#$epoch_seconds / 60))
format_epoch_minutes "$epoch_minutes"
