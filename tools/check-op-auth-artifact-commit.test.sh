#!/usr/bin/env bash
# Secret-free Git fixture tests for an exact signed-matrix adoption transition.
# shellcheck disable=SC2016 # Generated fixture scripts must retain variables.

set -euo pipefail

script_dir=$(CDPATH='' cd "$(dirname "$0")" && pwd)
temp_root=$(mktemp -d "${TMPDIR:-/tmp}/op-auth-commit-test.XXXXXX")
fixture=$temp_root/repo

cleanup() {
    rm -rf "$temp_root"
}
trap cleanup EXIT

mkdir -p \
    "$fixture/tools" "$fixture/scripts" \
    "$fixture/crates/op-auth-bridge/prebuilt/aarch64-apple-ios"
cp "$script_dir/check-op-auth-artifact-commit.sh" "$fixture/tools/"
grep -Fq \
    'unexpected=$(LC_ALL=C comm -23 "$actual_changes" "$allowed_changes")' \
    "$fixture/tools/check-op-auth-artifact-commit.sh"
printf '#!/usr/bin/env bash\nprintf "1.2.3\\n"\n' \
    > "$fixture/scripts/workspace-version.sh"
printf '%s\n' \
    '#!/usr/bin/env bash' \
    'set -euo pipefail' \
    '[[ "$OP_AUTH_PREBUILT_ROOT" == "$PWD/crates/op-auth-bridge/prebuilt" ]]' \
    'if [[ -n "${OP_AUTH_RELEASE_WORKSPACE_VERSION:-}" || -n "${OP_AUTH_RELEASE_EXPECTED_OPENPENCIL_REVISION:-}" ]]; then' \
    '    [[ "$OP_AUTH_RELEASE_EXPECTED_OPENPENCIL_REVISION" == "$EXPECTED_FIXTURE_PARENT" ]]' \
    '    [[ "$OP_AUTH_RELEASE_WORKSPACE_VERSION" == 1.2.3 ]]' \
    '    printf "strict\n" >> "$MATRIX_CHECK_LOG"' \
    'else' \
    '    printf "consumer\n" >> "$MATRIX_CHECK_LOG"' \
    'fi' \
    > "$fixture/tools/check-op-auth-release-matrix.sh"
printf '%s\n' \
    '#!/usr/bin/env bash' \
    'set -euo pipefail' \
    '[[ "$1" == --require-hardened ]]' \
    '[[ "$OP_AUTH_PREBUILT_ROOT" == "$PWD/crates/op-auth-bridge/prebuilt" ]]' \
    > "$fixture/tools/check-op-auth-prebuilt.sh"
chmod +x "$fixture/tools/"*.sh "$fixture/scripts/workspace-version.sh"

git -C "$fixture" init -q
git -C "$fixture" config user.name 'OpenPencil fixture'
git -C "$fixture" config user.email 'fixture@openpencil.invalid'
printf 'trusted-key\n' \
    > "$fixture/crates/op-auth-bridge/prebuilt/PROVENANCE_PUBKEY"
printf 'old source adoption policy\n' \
    > "$fixture/crates/op-auth-bridge/AUTH-RELEASE-POLICY"
git -C "$fixture" add .
git -C "$fixture" commit -q -m 'fixture source S'
source_sha=$(git -C "$fixture" rev-parse HEAD)

printf 'manifest\n' \
    > "$fixture/crates/op-auth-bridge/prebuilt/RELEASE-MANIFEST"
printf 'signature\n' \
    > "$fixture/crates/op-auth-bridge/prebuilt/RELEASE-MANIFEST.sig"
printf 'ios archive\n' \
    > "$fixture/crates/op-auth-bridge/prebuilt/aarch64-apple-ios/libop_auth.a"
printf 'new matrix adoption policy\n' \
    > "$fixture/crates/op-auth-bridge/AUTH-RELEASE-POLICY"
git -C "$fixture" add \
    crates/op-auth-bridge/prebuilt \
    crates/op-auth-bridge/AUTH-RELEASE-POLICY
git -C "$fixture" commit -q -m 'fixture auth-only A'
artifact_sha=$(git -C "$fixture" rev-parse HEAD)

output=$temp_root/output
matrix_log=$temp_root/matrix-log
: > "$output"
: > "$matrix_log"
(
    cd "$fixture"
    LC_ALL=C \
    EXPECTED_FIXTURE_PARENT=$source_sha \
    MATRIX_CHECK_LOG=$matrix_log \
    OP_AUTH_ARTIFACT_COMMIT=$artifact_sha \
    OP_AUTH_ARTIFACT_SELECTED_COMMIT=$artifact_sha \
    OP_AUTH_ARTIFACT_REF=refs/tags/v1.2.3 \
    OP_AUTH_ARTIFACT_OUTPUT=$output \
        tools/check-op-auth-artifact-commit.sh >/dev/null
)
grep -Fxq "artifact_sha=$artifact_sha" "$output"
grep -Fxq "base_sha=$source_sha" "$output"
grep -Fxq 'version=1.2.3' "$output"
[[ "$(cat "$matrix_log")" == $'strict\nconsumer' ]]

# A valid iOS archive used to be reported as unexpected when the caller's
# collation differed from the C locale used to sort the comm inputs.
non_c_locale=
for candidate in \
    en_US.UTF-8 en_US.utf8 \
    de_DE.UTF-8 de_DE.utf8 \
    fr_FR.UTF-8 fr_FR.utf8; do
    if LC_ALL="$candidate" locale charmap >/dev/null 2>&1; then
        non_c_locale=$candidate
        break
    fi
done
if [[ -n "$non_c_locale" ]]; then
    locale_output=$temp_root/locale-output
    locale_matrix_log=$temp_root/locale-matrix-log
    : > "$locale_output"
    : > "$locale_matrix_log"
    (
        cd "$fixture"
        LC_ALL="$non_c_locale" \
        EXPECTED_FIXTURE_PARENT=$source_sha \
        MATRIX_CHECK_LOG=$locale_matrix_log \
        OP_AUTH_ARTIFACT_COMMIT=$artifact_sha \
        OP_AUTH_ARTIFACT_SELECTED_COMMIT=$artifact_sha \
        OP_AUTH_ARTIFACT_REF=refs/tags/v1.2.3 \
        OP_AUTH_ARTIFACT_OUTPUT=$locale_output \
            tools/check-op-auth-artifact-commit.sh >/dev/null
    )
    grep -Fxq "artifact_sha=$artifact_sha" "$locale_output"
    grep -Fxq "base_sha=$source_sha" "$locale_output"
    grep -Fxq 'version=1.2.3' "$locale_output"
    [[ "$(cat "$locale_matrix_log")" == $'strict\nconsumer' ]]
fi

# A complete matrix replacement without the source-owned policy update must
# not be promotable, even though every historical matrix path changed.
git -C "$fixture" checkout -q --detach "$source_sha"
mkdir -p "$fixture/crates/op-auth-bridge/prebuilt/aarch64-apple-ios"
printf 'manifest without adoption\n' \
    > "$fixture/crates/op-auth-bridge/prebuilt/RELEASE-MANIFEST"
printf 'signature without adoption\n' \
    > "$fixture/crates/op-auth-bridge/prebuilt/RELEASE-MANIFEST.sig"
printf 'archive without adoption\n' \
    > "$fixture/crates/op-auth-bridge/prebuilt/aarch64-apple-ios/libop_auth.a"
git -C "$fixture" add crates/op-auth-bridge/prebuilt
git -C "$fixture" commit -q -m 'fixture matrix without adoption policy'
missing_policy_sha=$(git -C "$fixture" rev-parse HEAD)
if (
    cd "$fixture"
    EXPECTED_FIXTURE_PARENT=$source_sha \
    OP_AUTH_ARTIFACT_COMMIT=$missing_policy_sha \
    OP_AUTH_ARTIFACT_SELECTED_COMMIT=$missing_policy_sha \
    OP_AUTH_ARTIFACT_REF=refs/tags/v1.2.3 \
        tools/check-op-auth-artifact-commit.sh >/dev/null 2>&1
); then
    printf 'error: complete matrix without adoption policy was accepted\n' >&2
    exit 1
fi
git -C "$fixture" checkout -q --detach "$artifact_sha"

if (
    cd "$fixture"
    EXPECTED_FIXTURE_PARENT=$source_sha \
    OP_AUTH_ARTIFACT_COMMIT=$artifact_sha \
    OP_AUTH_ARTIFACT_SELECTED_COMMIT=3333333333333333333333333333333333333333 \
    OP_AUTH_ARTIFACT_REF=refs/tags/v1.2.3 \
        tools/check-op-auth-artifact-commit.sh >/dev/null 2>&1
); then
    printf 'error: mismatched selected artifact commit was accepted\n' >&2
    exit 1
fi

if (
    cd "$fixture"
    EXPECTED_FIXTURE_PARENT=$source_sha \
    OP_AUTH_ARTIFACT_COMMIT=$artifact_sha \
    OP_AUTH_ARTIFACT_SELECTED_COMMIT=$artifact_sha \
    OP_AUTH_ARTIFACT_REF=refs/tags/v1.2.4 \
        tools/check-op-auth-artifact-commit.sh >/dev/null 2>&1
); then
    printf 'error: mismatched release ref version was accepted\n' >&2
    exit 1
fi

printf 'missing required archive change\n' \
    > "$fixture/crates/op-auth-bridge/prebuilt/RELEASE-MANIFEST"
git -C "$fixture" add crates/op-auth-bridge/prebuilt/RELEASE-MANIFEST
git -C "$fixture" commit -q -m 'fixture incomplete auth transition'
incomplete_sha=$(git -C "$fixture" rev-parse HEAD)
if (
    cd "$fixture"
    EXPECTED_FIXTURE_PARENT=$artifact_sha \
    OP_AUTH_ARTIFACT_COMMIT=$incomplete_sha \
    OP_AUTH_ARTIFACT_SELECTED_COMMIT=$incomplete_sha \
    OP_AUTH_ARTIFACT_REF=refs/heads/main \
        tools/check-op-auth-artifact-commit.sh >/dev/null 2>&1
); then
    printf 'error: artifact commit missing required changes was accepted\n' >&2
    exit 1
fi

printf 'changed manifest\n' \
    > "$fixture/crates/op-auth-bridge/prebuilt/RELEASE-MANIFEST"
printf 'changed signature\n' \
    > "$fixture/crates/op-auth-bridge/prebuilt/RELEASE-MANIFEST.sig"
printf 'changed iOS archive\n' \
    > "$fixture/crates/op-auth-bridge/prebuilt/aarch64-apple-ios/libop_auth.a"
printf 'forged key\n' \
    > "$fixture/crates/op-auth-bridge/prebuilt/PROVENANCE_PUBKEY"
git -C "$fixture" add .
git -C "$fixture" commit -q -m 'fixture forged-key transition'
forbidden_sha=$(git -C "$fixture" rev-parse HEAD)
if (
    cd "$fixture"
    EXPECTED_FIXTURE_PARENT=$incomplete_sha \
    OP_AUTH_ARTIFACT_COMMIT=$forbidden_sha \
    OP_AUTH_ARTIFACT_SELECTED_COMMIT=$forbidden_sha \
    OP_AUTH_ARTIFACT_REF=refs/heads/main \
        tools/check-op-auth-artifact-commit.sh >/dev/null 2>&1
); then
    printf 'error: artifact commit replacing the trusted key was accepted\n' >&2
    exit 1
fi

printf 'check-op-auth-artifact-commit.test.sh: promotion adoption gates passed.\n'
