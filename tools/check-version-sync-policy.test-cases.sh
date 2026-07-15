repo=$(new_repo ordinary_dynamic_rust_fixture 0.8.1)
printf '%s\n' \
    'let src = src.replace("__OPENPENCIL_VERSION__", env!("CARGO_PKG_VERSION"));' \
    >> "$repo/crates/op-editor-core/src/host_support.rs"
run_guard "$repo"
assert_status 1 'ordinary dynamic Rust fixture'
assert_contains 'crates/op-editor-core/src/host_support.rs:1: error: ordinary test fixtures must use stable 1.0.0 instead of CARGO_PKG_VERSION' \
    'ordinary dynamic Rust fixture'
assert_no_success_output 'ordinary dynamic Rust fixture'
pass 'ordinary Rust fixtures cannot masquerade as product-version producers'

repo=$(new_repo dynamic_rust_fixture_balanced_count 0.8.1)
cat > "$repo/crates/op-editor-core/src/host_support.rs" <<'RUST'
pub fn sample() {
let src = src.replace("__OPENPENCIL_VERSION__", env!("CARGO_PKG_VERSION"));
}
pub fn starter() {
}
#[cfg(test)]
mod tests {
let src = src.replace("__OPENPENCIL_VERSION__", env!("CARGO_PKG_VERSION"));
}
RUST
run_guard "$repo"
assert_status 1 'dynamic Rust fixture with balanced count'
assert_contains 'crates/op-editor-core/src/host_support.rs:1: error: ordinary test fixtures must use stable 1.0.0 instead of CARGO_PKG_VERSION' \
    'dynamic Rust fixture with balanced count'
assert_no_success_output 'dynamic Rust fixture with balanced count'
pass 'a dynamic test fixture cannot replace one of the two production producers'

repo=$(new_repo cargo_manifest_literal_workspace_version 0.8.1)
sed -i.bak 's/version[.]workspace = true/version = "0.8.1"/' \
    "$repo/crates/example/Cargo.toml"
rm "$repo/crates/example/Cargo.toml.bak"
run_guard "$repo"
assert_status 1 'Cargo manifest literal workspace version'
assert_contains 'crates/example/Cargo.toml:3: error: local op-* package must declare exactly one active version.workspace = true in [package]' \
    'Cargo manifest literal workspace version'
assert_no_success_output 'Cargo manifest literal workspace version'
pass 'metadata equality cannot hide a literal local package version'

repo=$(new_repo cargo_manifest_missing_workspace_version 0.8.1)
sed -i.bak '/version[.]workspace = true/d' "$repo/crates/example/Cargo.toml"
rm "$repo/crates/example/Cargo.toml.bak"
run_guard "$repo"
assert_status 1 'Cargo manifest missing workspace version'
assert_contains 'crates/example/Cargo.toml:1: error: local op-* package must declare exactly one active version.workspace = true in [package]' \
    'Cargo manifest missing workspace version'
assert_no_success_output 'Cargo manifest missing workspace version'
pass 'local op-* manifests must inherit the workspace version'

repo=$(new_repo cargo_manifest_commented_workspace_version 0.8.1)
sed -i.bak 's/^version[.]workspace = true/# version.workspace = true/' \
    "$repo/crates/example/Cargo.toml"
rm "$repo/crates/example/Cargo.toml.bak"
run_guard "$repo"
assert_status 1 'Cargo manifest commented workspace version'
assert_contains 'crates/example/Cargo.toml:1: error: local op-* package must declare exactly one active version.workspace = true in [package]' \
    'Cargo manifest commented workspace version'
assert_no_success_output 'Cargo manifest commented workspace version'
pass 'commented inheritance declarations do not satisfy the manifest guard'

repo=$(new_repo readme_numeric_docker_tag 0.8.1)
printf '%s\n' 'docker pull ghcr.io/zseven-w/openpencil-web:v0.8.2' >> "$repo/README.md"
run_guard "$repo"
assert_status 1 'README numeric Docker tag'
assert_contains 'README.md:' 'README numeric Docker tag'
assert_contains 'error: top-level READMEs must not contain active product SemVer releases; use vX.Y.Z or the workspace-version reader' \
    'README numeric Docker tag'
assert_no_success_output 'README numeric Docker tag'
pass 'top-level README Docker tags must stay version-neutral'

repo=$(new_repo readme_active_status_version 0.8.1)
printf '%s\n' 'The Rust v0.8.2 release is under active development.' >> "$repo/README.md"
run_guard "$repo"
assert_status 1 'README active status version'
assert_contains 'README.md:' 'README active status version'
assert_contains 'error: top-level READMEs must not contain active product SemVer releases; use vX.Y.Z or the workspace-version reader' \
    'README active status version'
assert_no_success_output 'README active status version'
pass 'top-level README status text cannot pin an active product version'

repo=$(new_repo readme_version_like_substring 0.8.1)
printf '%s\n' 'The identifier build0.8.2candidate is not a release.' >> "$repo/README.md"
run_guard "$repo"
assert_status 0 'README version-like substring'
pass 'README version-like substrings inside larger identifiers are allowed'

repo=$(new_repo readme_historical_typescript_version 0.8.1)
printf '%s\n' 'The retired TypeScript line ended at v0.7.5.' >> "$repo/README.md"
run_guard "$repo"
assert_status 0 'README historical TypeScript version'
pass 'the explicit retired TypeScript v0.7.5 history remains allowed'

repo=$(new_repo version_sync_ci_missing_readmes 0.8.1)
sed -i.bak '/README[*][.]md/d' "$repo/.github/workflows/version-sync.yml"
rm "$repo/.github/workflows/version-sync.yml.bak"
run_guard "$repo"
assert_status 1 'version-sync CI missing README paths'
assert_contains '.github/workflows/version-sync.yml:1: error: version-sync CI must run for top-level README changes in pull requests and pushes' \
    'version-sync CI missing README paths'
assert_no_success_output 'version-sync CI missing README paths'
pass 'version-sync CI covers every top-level README input'
