#!/usr/bin/env bash
# Secret-free negative coverage for unsigned-candidate promotion and signing.

set -euo pipefail

script_dir=$(CDPATH='' cd "$(dirname "$0")" && pwd)
# shellcheck source=op-auth-candidate-targets.sh
source "$script_dir/op-auth-candidate-targets.sh"

temp_dir=$(mktemp -d "${TMPDIR:-/tmp}/op-auth-promotion-test.XXXXXX")
cleanup() {
    chmod -R u+w "$temp_dir" 2>/dev/null || true
    rm -rf "$temp_dir"
}
trap cleanup EXIT HUP INT TERM

candidate_root=$temp_dir/candidates
signed_root=$temp_dir/signed
version=9.8.7
private_run_id=123456789
run_attempt=2
private_head_sha=1111111111111111111111111111111111111111
openpencil_sha=2222222222222222222222222222222222222222
build_id=promotion-test
mkdir -p "$candidate_root"

sha256_file() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | awk '{ print $1 }'
    else
        shasum -a 256 "$1" | awk '{ print $1 }'
    fi
}

sha256_stdin() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum | awk '{ print $1 }'
    else
        shasum -a 256 | awk '{ print $1 }'
    fi
}

artifact_records=$temp_dir/artifact-records
: > "$artifact_records"
index=100
while IFS= read -r target; do
    artifact=$(op_auth_candidate_artifact_name "$target")
    bundle=$(op_auth_candidate_bundle_name "$target")
    target_root=$candidate_root/$target
    mkdir -p "$target_root"
    printf 'unsigned fixture bytes for %s\n' "$target" > "$target_root/$artifact"
    artifact_sha=$(sha256_file "$target_root/$artifact")
    artifact_size=$(wc -c < "$target_root/$artifact" | tr -d '[:space:]')
    review_id=source-$private_head_sha
    protector_sha=3333333333333333333333333333333333333333333333333333333333333333
    review_binding=$(
        printf '%s\n' \
            op-auth-protector-binding-v1 \
            "target=$target" \
            "review_id=$review_id" \
            "protector_sha256=$protector_sha" \
            | sha256_stdin
    )
    platform=macos
    validation=native-link-run
    execution=passed
    minimum=native
    case "$target" in
        aarch64-apple-ios)
            platform=ios
            validation=cross-final-link
            execution=not-applicable
            minimum=15.0
            ;;
        aarch64-apple-ios-sim)
            platform=ios-simulator
            validation=cross-final-link
            execution=not-applicable
            minimum=15.0
            ;;
        *-linux-android)
            platform=android
            validation=cross-final-link
            execution=not-applicable
            minimum=21
            ;;
        *-unknown-linux-gnu) platform=linux ;;
        *-pc-windows-msvc) platform=windows ;;
    esac
    cat > "$target_root/HARDENING-ATTESTATION" <<EOF
format=3
mode=production
abi=3
target=$target
artifact=$artifact
artifact_sha256=$artifact_sha
artifact_size=$artifact_size
source_revision=$private_head_sha
source_tree_state=clean
source_date_epoch=1700000000
profile=private-ci-fat-lto
paths=remapped
debug=none
symbols=stripped
dead_code=eliminated
audit_profile=op-auth-hardened-v3
abi_allowlist_sha256=5555555555555555555555555555555555555555555555555555555555555555
global_defined_symbols_sha256=6666666666666666666666666666666666666666666666666666666666666666
section_inventory_sha256=7777777777777777777777777777777777777777777777777777777777777777
object_format_count=1
debug_section_count=0
link_validation=$validation
link_execution=$execution
target_platform=$platform
minimum_platform_version=$minimum
linked_binary_sha256=8888888888888888888888888888888888888888888888888888888888888888
obfuscation_review=$review_id
protection_tool_sha256=$protector_sha
review_binding_sha256=$review_binding
linux_glibc_baseline=none
linux_sysroot=none
zig_version=none
zig_sha256=none
zig_sysroot_sha256=none
cargo_zigbuild_version=none
cargo_zigbuild_sha256=none
toolchain=rustc-test
staging_format=1
release_build_id=$build_id
openpencil_revision=$openpencil_sha
EOF
    hardening_sha=$(sha256_file "$target_root/HARDENING-ATTESTATION")
    printf '%s\n' "$version" > "$target_root/VERSION"
    printf '3\n' > "$target_root/ABI_VERSION"
    printf '%s\n' "$artifact_sha" > "$target_root/SHA256"
    cat > "$target_root/CANDIDATE" <<EOF
format=op-auth-unsigned-candidate-v1
private_repository=ZSeven-W/op-platform
workflow_path=.github/workflows/prebuilt-production.yml
workflow_run_id=$private_run_id
workflow_run_attempt=$run_attempt
artifact_bundle=$bundle
target=$target
version=$version
abi=3
source_revision=$private_head_sha
openpencil_revision=$openpencil_sha
build_id=$build_id
artifact=$artifact
artifact_sha256=$artifact_sha
hardening_sha256=$hardening_sha
EOF
    archive_digest=$(printf '%064x' "$index")
    printf 'artifact_bundle=%s|artifact_id=%s|archive_sha256=%s\n' \
        "$bundle" "$index" "$archive_digest" >> "$artifact_records"
    index=$((index + 1))
done < <(op_auth_candidate_targets)

{
    printf 'format=op-auth-candidate-download-v1\n'
    printf 'private_repository=ZSeven-W/op-platform\n'
    printf 'workflow_path=.github/workflows/prebuilt-production.yml\n'
    printf 'workflow_id=322324691\n'
    printf 'workflow_run_id=%s\n' "$private_run_id"
    printf 'workflow_run_attempt=%s\n' "$run_attempt"
    printf 'private_head_sha=%s\n' "$private_head_sha"
    printf 'artifact_count=10\n'
    cat "$artifact_records"
} > "$candidate_root/DOWNLOAD-MANIFEST"

verify_candidates() {
    bash "$script_dir/verify-op-auth-candidates.sh" \
        --candidate-root "$1" \
        --private-run-id "$private_run_id" \
        --private-head-sha "$private_head_sha" \
        --openpencil-sha "$openpencil_sha" \
        --version "$version"
}

verify_candidates "$candidate_root" >/dev/null
candidate_tree_digest=$(python3 \
    "$script_dir/digest-op-auth-tree.py" --root "$candidate_root")
python3 "$script_dir/digest-op-auth-tree.py" \
    --root "$candidate_root" --expected "$candidate_tree_digest" >/dev/null

fixture_target=aarch64-apple-ios
fixture_zip=$temp_dir/candidate.zip
python3 - "$candidate_root/$fixture_target" "$fixture_target" "$fixture_zip" <<'PY'
import pathlib
import sys
import zipfile

source = pathlib.Path(sys.argv[1])
target = sys.argv[2]
archive = pathlib.Path(sys.argv[3])
with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as bundle:
    for path in sorted(source.iterdir()):
        bundle.write(path, f"{target}/{path.name}")
PY
python3 "$script_dir/extract-op-auth-candidate.py" \
    --zip "$fixture_zip" --target "$fixture_target" \
    --output-root "$temp_dir/extracted"
cmp -s \
    "$candidate_root/$fixture_target/CANDIDATE" \
    "$temp_dir/extracted/$fixture_target/CANDIDATE"

python3 - "$temp_dir/traversal.zip" <<'PY'
import sys
import zipfile

with zipfile.ZipFile(sys.argv[1], "w") as bundle:
    bundle.writestr("../escape", "unsafe")
PY
if python3 "$script_dir/extract-op-auth-candidate.py" \
    --zip "$temp_dir/traversal.zip" --target "$fixture_target" \
    --output-root "$temp_dir/traversal-output" >/dev/null 2>&1; then
    printf 'error: traversal candidate zip was accepted\n' >&2
    exit 1
fi

cp -R "$candidate_root" "$temp_dir/incomplete"
rm -rf "$temp_dir/incomplete/aarch64-apple-ios"
if verify_candidates "$temp_dir/incomplete" >/dev/null 2>&1; then
    printf 'error: incomplete unsigned candidate matrix was accepted\n' >&2
    exit 1
fi

cp -R "$candidate_root" "$temp_dir/tampered"
printf 'tampered\n' >> "$temp_dir/tampered/aarch64-apple-ios/libop_auth.a"
if verify_candidates "$temp_dir/tampered" >/dev/null 2>&1; then
    printf 'error: tampered unsigned archive was accepted\n' >&2
    exit 1
fi
if python3 "$script_dir/digest-op-auth-tree.py" \
    --root "$temp_dir/tampered" --expected "$candidate_tree_digest" \
    >/dev/null 2>&1; then
    printf 'error: tampered candidate handoff retained its tree digest\n' >&2
    exit 1
fi

cp -R "$candidate_root" "$temp_dir/wrong-run"
sed -i.bak 's/^workflow_run_id=.*/workflow_run_id=999/' \
    "$temp_dir/wrong-run/aarch64-apple-darwin/CANDIDATE"
rm "$temp_dir/wrong-run/aarch64-apple-darwin/CANDIDATE.bak"
if verify_candidates "$temp_dir/wrong-run" >/dev/null 2>&1; then
    printf 'error: candidate with a mismatched workflow run was accepted\n' >&2
    exit 1
fi

private_key=$temp_dir/private.pem
forged_key=$temp_dir/forged.pem
trusted_public_key=$temp_dir/PROVENANCE_PUBKEY
forged_public_key=$temp_dir/FORGED_PUBKEY
openssl genpkey -algorithm ED25519 -out "$private_key" >/dev/null 2>&1
openssl genpkey -algorithm ED25519 -out "$forged_key" >/dev/null 2>&1
openssl pkey -in "$private_key" -pubout -outform DER \
    | tail -c 32 | xxd -p -c 256 > "$trusted_public_key"
openssl pkey -in "$forged_key" -pubout -outform DER \
    | tail -c 32 | xxd -p -c 256 > "$forged_public_key"

bash "$script_dir/sign-op-auth-candidates.sh" \
    --candidate-root "$candidate_root" \
    --trusted-public-key "$trusted_public_key" \
    --signing-key "$private_key" \
    --output-root "$signed_root" \
    --version "$version" \
    --private-head-sha "$private_head_sha" \
    --openpencil-sha "$openpencil_sha" \
    --build-id "$build_id" >/dev/null
bash "$script_dir/verify-signed-op-auth-matrix.sh" \
    --signed-root "$signed_root" \
    --trusted-public-key "$trusted_public_key" \
    --version "$version" \
    --private-head-sha "$private_head_sha" \
    --openpencil-sha "$openpencil_sha" \
    --build-id "$build_id" >/dev/null
signed_tree_digest=$(python3 \
    "$script_dir/digest-op-auth-tree.py" --root "$signed_root")
python3 "$script_dir/digest-op-auth-tree.py" \
    --root "$signed_root" --expected "$signed_tree_digest" >/dev/null

promotion_repo=$temp_dir/promotion-repo
prebuilt=$promotion_repo/crates/op-auth-bridge/prebuilt
policy=$promotion_repo/crates/op-auth-bridge/AUTH-RELEASE-POLICY
mkdir -p "$prebuilt" "$promotion_repo/tools"
git -C "$temp_dir" init -q -b main promotion-repo
git -C "$promotion_repo" config user.name fixture
git -C "$promotion_repo" config user.email fixture@example.invalid
cp "$script_dir/check-op-auth-release-matrix.sh" "$promotion_repo/tools/"
chmod +x "$promotion_repo/tools/check-op-auth-release-matrix.sh"
cp "$trusted_public_key" "$prebuilt/PROVENANCE_PUBKEY"
printf 'previous source adoption policy\n' > "$policy"
git -C "$promotion_repo" add \
    tools/check-op-auth-release-matrix.sh \
    crates/op-auth-bridge/prebuilt/PROVENANCE_PUBKEY \
    crates/op-auth-bridge/AUTH-RELEASE-POLICY
git -C "$promotion_repo" commit -q -m 'test: base trust root'
parent=$(git -C "$promotion_repo" rev-parse HEAD)
cp -R "$signed_root"/. "$prebuilt"/
manifest_sha=$(sha256_file "$prebuilt/RELEASE-MANIFEST")
public_key=$(tr -d '[:space:]' < "$prebuilt/PROVENANCE_PUBKEY")
cat > "$policy" <<EOF
format=op-auth-release-policy-v1
abi=3
public_key=$public_key
release_manifest_sha256=$manifest_sha
source_revision=$private_head_sha
build_id=$build_id
EOF
valid_policy=$temp_dir/valid-auth-release-policy
cp "$policy" "$valid_policy"
git -C "$promotion_repo" add -A \
    crates/op-auth-bridge/prebuilt \
    crates/op-auth-bridge/AUTH-RELEASE-POLICY
bash "$script_dir/verify-op-auth-promotion-diff.sh" \
    --repo "$promotion_repo" --expected-parent "$parent" >/dev/null

expect_policy_rejection() {
    local field=$1
    local value=$2
    sed "s#^$field=.*#$field=$value#" "$valid_policy" > "$policy"
    git -C "$promotion_repo" add crates/op-auth-bridge/AUTH-RELEASE-POLICY
    if bash "$script_dir/verify-op-auth-promotion-diff.sh" \
        --repo "$promotion_repo" --expected-parent "$parent" >/dev/null 2>&1; then
        printf 'error: promotion policy with wrong %s was accepted\n' "$field" >&2
        exit 1
    fi
}
expect_policy_rejection abi 2
expect_policy_rejection public_key \
    9999999999999999999999999999999999999999999999999999999999999999
expect_policy_rejection release_manifest_sha256 \
    0000000000000000000000000000000000000000000000000000000000000000
expect_policy_rejection source_revision \
    9999999999999999999999999999999999999999
expect_policy_rejection build_id wrong-build
cp "$valid_policy" "$policy"
git -C "$promotion_repo" add crates/op-auth-bridge/AUTH-RELEASE-POLICY

printf 'outside auth matrix\n' > "$promotion_repo/OUTSIDE"
git -C "$promotion_repo" add OUTSIDE
if bash "$script_dir/verify-op-auth-promotion-diff.sh" \
    --repo "$promotion_repo" --expected-parent "$parent" >/dev/null 2>&1; then
    printf 'error: staged path outside the auth matrix was accepted\n' >&2
    exit 1
fi

if bash "$script_dir/verify-signed-op-auth-matrix.sh" \
    --signed-root "$signed_root" \
    --trusted-public-key "$forged_public_key" \
    --version "$version" \
    --private-head-sha "$private_head_sha" \
    --openpencil-sha "$openpencil_sha" \
    --build-id "$build_id" >/dev/null 2>&1; then
    printf 'error: forged public release root was accepted\n' >&2
    exit 1
fi

chmod u+w "$signed_root/aarch64-apple-ios/libop_auth.a"
printf 'tampered\n' >> "$signed_root/aarch64-apple-ios/libop_auth.a"
if bash "$script_dir/verify-signed-op-auth-matrix.sh" \
    --signed-root "$signed_root" \
    --trusted-public-key "$trusted_public_key" \
    --version "$version" \
    --private-head-sha "$private_head_sha" \
    --openpencil-sha "$openpencil_sha" \
    --build-id "$build_id" >/dev/null 2>&1; then
    printf 'error: tampered signed archive was accepted\n' >&2
    exit 1
fi

printf '%s\n' \
    'check-op-auth-promotion.test: candidate, digest, trust-root, and tamper gates passed.'
