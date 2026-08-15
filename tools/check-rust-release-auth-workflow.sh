#!/usr/bin/env bash
# Secret-free structural checks for the production-auth Rust release gate.
# shellcheck disable=SC2016 # Source workflow literals intentionally stay literal.

set -euo pipefail

script_dir=$(CDPATH='' cd "$(dirname "$0")" && pwd)
repo_root=$(CDPATH='' cd "$script_dir/.." && pwd)
workflow=${OPENPENCIL_RUST_RELEASE_WORKFLOW:-$repo_root/.github/workflows/rust-release.yml}
matrix_verifier=$repo_root/tools/check-op-auth-release-matrix.sh
prebuilt_verifier=$repo_root/tools/check-op-auth-prebuilt.sh
release_builder=${OPENPENCIL_RUST_RELEASE_BUILDER:-$repo_root/scripts/build-rust-release-host.sh}
pinned_tools=${OPENPENCIL_PINNED_RELEASE_TOOLS:-$repo_root/tools/pinned-release-tools.sh}
pinned_tools_test=$repo_root/tools/pinned-release-tools.test.sh
angle_stager=$repo_root/tools/stage-pinned-angle.ps1
nsis_installer=$repo_root/tools/install-pinned-nsis.ps1
windows_signer=$repo_root/tools/sign-windows-release.ps1
package_handoff=$repo_root/tools/package-manager-handoff.sh
release_flattener=$repo_root/tools/flatten-release-artifacts.sh
release_flattener_test=$repo_root/tools/flatten-release-artifacts.test.sh
appimage_packager=$repo_root/scripts/package-appimage.sh
macos_bundler=$repo_root/scripts/bundle-macos.sh
wasm_builder=$repo_root/tools/check-wasm-bundle.sh
sdk_wasm_builder=$repo_root/crates/op-web-sdk/tools/build-wasm.sh
dockerfile=${OPENPENCIL_WEB_DOCKERFILE:-$repo_root/Dockerfile.web-rust}
vscode_package=$repo_root/packages/op-vscode/package.json
bun_lock=$repo_root/packages/bun.lock
version_sync_workflow=${OPENPENCIL_VERSION_SYNC_WORKFLOW:-$repo_root/.github/workflows/version-sync.yml}
ios_workflow=$repo_root/.github/workflows/ios-app-store.yml
android_workflow=$repo_root/.github/workflows/android-release.yml
ios_workflow_checker=$repo_root/tools/check-ios-app-store-workflow.sh
android_workflow_checker=$repo_root/tools/check-android-release-workflow.sh

for file in \
    "$workflow" "$matrix_verifier" "$prebuilt_verifier" "$release_builder" "$pinned_tools" \
    "$pinned_tools_test" "$angle_stager" "$nsis_installer" "$windows_signer" "$package_handoff" \
    "$release_flattener" "$release_flattener_test" \
    "$appimage_packager" "$macos_bundler" "$wasm_builder" "$sdk_wasm_builder" \
    "$dockerfile" "$vscode_package" "$bun_lock" "$version_sync_workflow"; do
    [[ -f "$file" && ! -L "$file" ]] || {
        printf 'error: missing Rust release contract file: %s\n' "$file" >&2
        exit 1
    }
done
for file in \
    "$ios_workflow" "$android_workflow" \
    "$ios_workflow_checker" "$android_workflow_checker"; do
    [[ -f "$file" && ! -L "$file" ]] || {
        printf 'error: missing mobile release contract file: %s\n' "$file" >&2
        exit 1
    }
done
bash -n \
    "$matrix_verifier" "$prebuilt_verifier" "$release_builder" "$pinned_tools" "$pinned_tools_test" "$package_handoff" \
    "$release_flattener" "$release_flattener_test" \
    "$appimage_packager" "$macos_bundler" "$wasm_builder" "$sdk_wasm_builder"
"$release_flattener_test"
ruby - "$workflow" <<'RUBY'
require "yaml"

document = YAML.safe_load(File.read(ARGV.fetch(0)), aliases: true)
trigger = document["on"] || document[true]
manual = trigger.fetch("workflow_dispatch")
unless manual == {
    "inputs" => {
      "ios_app_store_only" => {
        "description" => "Run only the iOS App Store / TestFlight lane",
        "required" => false,
        "default" => false,
        "type" => "boolean",
      },
    },
  }
  raise "manual release must expose only the default-off iOS App Store lane selector"
end
raise "workflow permissions must default to contents:read" unless document.fetch("permissions") == {"contents" => "read"}

jobs = document.fetch("jobs")
version = jobs.fetch("version")
raise "auth gate must not access secrets or variables" if version.inspect.match?(/(?:secrets|vars)\./)
checkout = version.fetch("steps").find { |step| step["uses"] }
unless checkout.fetch("with") == {
    "fetch-depth" => 1,
    "persist-credentials" => false,
    "ref" => '${{ github.sha }}',
  }
  raise "release gate must check out the exact source without credentials"
end
unless version.fetch("outputs") == {
    "version" => '${{ steps.version.outputs.version }}',
    "source_sha" => '${{ steps.version.outputs.source_sha }}',
  }
  raise "release gate must expose only current source identity"
end
version_gate = version.fetch("steps").find { |step| step["id"] == "version" }
version_script = version_gate.fetch("run")
unless version_script.include?('"$(git rev-parse HEAD)" == "$GITHUB_SHA"') &&
    version_script.include?('"refs/heads/v$version"|"refs/tags/v$version"') &&
    version_script.include?("tools/check-op-auth-release-matrix.sh") &&
    version_script.include?("tools/check-op-auth-prebuilt.sh --require-hardened") &&
    !version_script.include?("OP_AUTH_RELEASE_EXPECTED_OPENPENCIL_REVISION") &&
    !version_script.include?("OP_AUTH_RELEASE_WORKSPACE_VERSION")
  raise "version gate must validate the source ref and adopted Auth matrix without A/S binding"
end

expected_permissions = {
  "version" => {"contents" => "read"},
  "ios-app-store" => {"contents" => "read"},
  "android-release" => {"contents" => "read"},
  "build" => {"contents" => "read"},
  "web-docker" => {"contents" => "read", "packages" => "write"},
  "sdk-packages" => {"contents" => "read"},
  "vsix" => {"contents" => "read"},
  "release-draft" => {
    "contents" => "write",
    "pull-requests" => "write",
    "id-token" => "write",
    "attestations" => "write",
  },
  "package-managers" => {"contents" => "read", "actions" => "read"},
}
raise "release job set changed without a permission review" unless jobs.keys.sort == expected_permissions.keys.sort
expected_permissions.each do |name, permissions|
  raise "#{name} permissions exceed the reviewed minimum" unless jobs.fetch(name).fetch("permissions") == permissions
end
raise "secret-free version gate must not request the release environment" if version.key?("environment")
reusable_jobs = %w[ios-app-store android-release]
reusable_jobs.each do |name|
  raise "#{name} must not set a caller-side environment" if jobs.fetch(name).key?("environment")
end
(jobs.keys - ["version"] - reusable_jobs).each do |name|
  raise "#{name} must be protected by release-production" unless jobs.fetch(name).fetch("environment") == "release-production"
end

approved_actions = {
  "actions/checkout" => "08eba0b27e820071cde6df949e0beb9ba4906955",
  "dtolnay/rust-toolchain" => "4360b52568e2003a75bf9bc1d59f33a8e3fc893c",
  "Swatinem/rust-cache" => "6323deb102c322ba6fcbdcafc7e3dddab59af2b6",
  "apple-actions/import-codesign-certs" => "63fff01cd422d4b7b855d40ca1e9d34d2de9427d",
  "actions/upload-artifact" => "ea165f8d65b6e75b540449e92b4886f43607fa02",
  "docker/login-action" => "c94ce9fb468520275223c153574b00df6fe4bcc9",
  "docker/build-push-action" => "10e90e3645eae34f1e60eeb005ba3a3d33f178e8",
  "actions/download-artifact" => "d3f86a106a0bac45b974a628896c90dbdf5c8093",
  "actions/attest-build-provenance" => "e8998f949152b193b063cb0ec769d69d929409be",
  "softprops/action-gh-release" => "3bb12739c298aeb8a4eeaf626c5b8d85266b0e65",
  "peter-evans/create-pull-request" => "5f6978faf089d4d20b00c7766989d076bb2fc7f1",
}
jobs.each do |job_name, job|
  job.fetch("steps", []).each do |step|
    next unless step["uses"]
    action, revision = step.fetch("uses").split("@", 2)
    unless revision&.match?(/\A[0-9a-f]{40}\z/)
      raise "mutable or malformed action reference: #{step.fetch("uses")}"
    end
    raise "unreviewed action: #{action}" unless approved_actions.key?(action)
    raise "unexpected action revision: #{step.fetch("uses")}" unless approved_actions.fetch(action) == revision
    next unless action == "actions/checkout"
    if job_name == "package-managers"
      if step["name"] == "Checkout release helpers without credentials"
        unless step.fetch("with") == {
            "persist-credentials" => false,
            "ref" => '${{ github.sha }}',
          }
          raise "package-manager helper checkout must use exact source without credentials"
        end
      elsif !["Checkout Homebrew tap", "Checkout Scoop bucket"].include?(step["name"])
        raise "unreviewed credential-bearing package-manager checkout"
      end
    elsif step.fetch("with", {}).fetch("persist-credentials", nil) != false
      raise "#{job_name} checkout must discard persisted credentials"
    end
  end
end

build_steps = jobs.fetch("build").fetch("steps")
cargo_bundle = build_steps.find { |step| step["name"] == "Install digest-pinned cargo-bundle" }
certificate_import = build_steps.find { |step| step["name"] == "Import codesign certificate (macos)" }
raise "missing digest-pinned cargo-bundle setup" unless cargo_bundle && certificate_import
raise "cargo-bundle setup must be macOS-only" unless cargo_bundle.fetch("if") == "runner.os == 'macOS'"
raise "cargo-bundle setup must not access secrets" if cargo_bundle.inspect.include?("secrets.")
unless cargo_bundle.fetch("run").include?("tools/pinned-release-tools.sh cargo-cli cargo-bundle")
  raise "cargo-bundle must come from the reviewed crate installer"
end
unless build_steps.index(cargo_bundle) < build_steps.index(certificate_import)
  raise "cargo-bundle must be installed before signing credentials are imported"
end

%w[sdk-packages vsix].each do |job_name|
  steps = jobs.fetch(job_name).fetch("steps")
  installer = steps.find { |step| step["name"] == "Install digest-pinned wasm-bindgen-cli" }
  raise "#{job_name} lacks the pinned wasm-bindgen installer" unless installer
  run = installer.fetch("run")
  unless run.include?("version\" != 0.2.117") &&
      run.include?("tools/pinned-release-tools.sh cargo-cli wasm-bindgen-cli")
    raise "#{job_name} does not bind wasm-bindgen-cli to the reviewed Cargo.lock version"
  end
  raise "#{job_name} wasm-bindgen setup must not access secrets" if installer.inspect.include?("secrets.")
  first_secret = steps.index { |step| step.inspect.include?("secrets.") }
  if first_secret && steps.index(installer) >= first_secret
    raise "#{job_name} must install wasm-bindgen-cli before exposing publish secrets"
  end
end
raise "release workflow must not install unverified Cargo CLIs directly" if File.read(ARGV.fetch(0)).include?("cargo install ")

web_steps = jobs.fetch("web-docker").fetch("steps")
buildx = web_steps.find { |step| step["name"] == "Install digest-pinned Buildx" }
raise "missing pinned Buildx setup" unless buildx
buildx_run = buildx.fetch("run")
%w[
  tools/pinned-release-tools.sh\ buildx
  docker\ buildx\ create
  --driver\ docker-container
  --use
  docker\ buildx\ inspect
  --bootstrap
].each { |fragment| raise "incomplete pinned Buildx setup: #{fragment}" unless buildx_run.include?(fragment.tr("\\", "")) }
unless buildx_run.include?("moby/buildkit:v0.32.2@sha256:28a898719c18a33f4e8000685287fa36fd0dd9560c6440227d3a732d79bb41d8")
  raise "BuildKit image must be immutable"
end
bun_steps = jobs.values.flat_map { |job| job.fetch("steps", []) }.select do |step|
  step["name"] == "Install digest-pinned Bun"
end
unless bun_steps.length == 2 && bun_steps.all? do |step|
    step.fetch("run") == 'tools/pinned-release-tools.sh bun "$RUNNER_TEMP/bun-1.3.14"'
  end
  raise "every Bun setup must use the digest-pinned repository installer"
end
node_steps = jobs.values.flat_map { |job| job.fetch("steps", []) }.select do |step|
  step["name"] == "Install digest-pinned Node"
end
unless node_steps.length == 2 && node_steps.all? do |step|
    step.fetch("run") == 'tools/pinned-release-tools.sh node "$RUNNER_TEMP/node-20.20.2"'
  end
  raise "SDK and VSIX publishing must use the digest-pinned repository Node/npm installer"
end

sensitive_tokens = %w[NPM_TOKEN OPENVSX_TOKEN TAP_GITHUB_TOKEN]
jobs.each do |name, job|
  leaked = sensitive_tokens & job.fetch("env", {}).keys
  raise "#{name} exposes registry/publish tokens at job scope: #{leaked.join(", ")}" unless leaked.empty?
end

sdk_publish = jobs.fetch("sdk-packages").fetch("steps").select do |step|
  step.fetch("env", {}).key?("NPM_TOKEN")
end
unless sdk_publish.length == 1 && sdk_publish.fetch(0).fetch("name") == "Publish npm packages" &&
    sdk_publish.fetch(0).fetch("env").fetch("NPM_TOKEN") == '${{ secrets.NPM_TOKEN }}'
  raise "NPM_TOKEN must be scoped to the npm publish step"
end
vsix_publish = jobs.fetch("vsix").fetch("steps").select do |step|
  step.fetch("env", {}).key?("OPENVSX_TOKEN")
end
unless vsix_publish.length == 1 && vsix_publish.fetch(0).fetch("name") == "Publish to Open VSX" &&
    vsix_publish.fetch(0).fetch("env").fetch("OPENVSX_TOKEN") == '${{ secrets.OPENVSX_TOKEN }}'
  raise "OPENVSX_TOKEN must be scoped to the Open VSX publish step"
end
tap_checkouts = jobs.fetch("package-managers").fetch("steps").select do |step|
  ["Checkout Homebrew tap", "Checkout Scoop bucket"].include?(step["name"])
end
unless tap_checkouts.length == 2 && tap_checkouts.all? do |step|
    step.fetch("with").fetch("token") == '${{ secrets.TAP_GITHUB_TOKEN }}'
  end
  raise "TAP_GITHUB_TOKEN must be scoped to the two package-manager checkouts"
end
source = File.read(ARGV.fetch(0))
{
  "NPM_TOKEN" => 1,
  "OPENVSX_TOKEN" => 1,
  "TAP_GITHUB_TOKEN" => 2,
}.each do |name, count|
  actual = source.scan(/\$\{\{\s*secrets\.#{Regexp.escape(name)}\s*\}\}/).length
  raise "unexpected #{name} secret reference count: #{actual}" unless actual == count
end

def depends_on?(jobs, name, target, seen = {})
  return false if seen[name]
  seen[name] = true
  needs = Array(jobs.fetch(name).fetch("needs", []))
  needs.include?(target) || needs.any? { |dependency| depends_on?(jobs, dependency, target, seen) }
end

(jobs.keys - ["version"]).each do |name|
  raise "#{name} can bypass the version/auth gate" unless depends_on?(jobs, name, "version")
end

ios = jobs.fetch("ios-app-store")
unless ios.fetch("uses") == "./.github/workflows/ios-app-store.yml" &&
    ios.fetch("if") == "startsWith(github.ref, 'refs/tags/v') || (github.event_name == 'workflow_dispatch' && inputs.ios_app_store_only == true)" &&
    ios.fetch("needs") == "version" &&
    ios.fetch("with") == {
      "release_sha" => '${{ github.sha }}',
      "release_ref" => '${{ github.ref }}',
    } &&
    ios.fetch("secrets") == {
      "APPLE_TEAM_ID" => '${{ secrets.APPLE_TEAM_ID }}',
      "OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_CN" =>
        '${{ secrets.OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_CN }}',
      "OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_GLOBAL" =>
        '${{ secrets.OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_GLOBAL }}',
    }
  raise "tag releases and explicit iOS-only dispatches must call the exact reusable App Store lane"
end

android = jobs.fetch("android-release")
unless android.fetch("uses") == "./.github/workflows/android-release.yml" &&
    android.fetch("if") == "startsWith(github.ref, 'refs/tags/v') && (github.event_name != 'workflow_dispatch' || inputs.ios_app_store_only == false)" &&
    android.fetch("needs") == "version" &&
    android.fetch("secrets") == {
      "OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_CN" =>
        '${{ secrets.OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_CN }}',
      "OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_GLOBAL" =>
        '${{ secrets.OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_GLOBAL }}',
    } && !android.key?("with")
  raise "formal release must call the exact reusable Android asset lane"
end

tag_only_condition = "startsWith(github.ref, 'refs/tags/v') && (github.event_name != 'workflow_dispatch' || inputs.ios_app_store_only == false)"
%w[android-release web-docker sdk-packages vsix release-draft package-managers].each do |name|
  unless jobs.fetch(name).fetch("if") == tag_only_condition
    raise "#{name} must stay disabled on a version-branch iOS-only dispatch"
  end
end

release = jobs.fetch("release-draft")
release_needs = Array(release.fetch("needs"))
raise "GitHub Release must wait for signed Android assets" unless release_needs.include?("android-release")
raise "App Store failure must not block GitHub Release assets" if release_needs.include?("ios-app-store")
expected_handoff_outputs = {
  "package_manager_artifact_id" => '${{ steps.package_manager_handoff.outputs.artifact-id }}',
  "package_manager_artifact_digest" => '${{ steps.package_manager_handoff.outputs.artifact-digest }}',
  "package_manager_manifest_sha256" => '${{ steps.package_manager_manifest.outputs.manifest_sha256 }}',
}
raise "package-manager handoff outputs changed" unless release.fetch("outputs") == expected_handoff_outputs
handoff_upload = release.fetch("steps").find { |step| step["id"] == "package_manager_handoff" }
raise "missing immutable package-manager handoff upload" unless handoff_upload
raise "package-manager handoff must fail closed" unless handoff_upload.fetch("with").fetch("if-no-files-found") == "error"
flatten = release.fetch("steps").find { |step| step["name"] == "Flatten artifacts" }
raise "missing release asset flatten gate" unless flatten
unless flatten.fetch("run") == 'tools/flatten-release-artifacts.sh dist release-files "$OP_VERSION"'
  raise "release assets must pass the repository-owned flatten gate"
end

package_steps = jobs.fetch("package-managers").fetch("steps")
handoff_download = package_steps.find { |step| step["name"] == "Download immutable same-run package-manager handoff" }
raise "missing same-run package-manager artifact download" unless handoff_download
unless handoff_download.fetch("env") == {"GH_TOKEN" => '${{ github.token }}'} &&
    handoff_download.fetch("run").include?("tools/package-manager-handoff.sh download") &&
    handoff_download.fetch("run").include?('${{ needs.release-draft.outputs.package_manager_artifact_id }}') &&
    handoff_download.fetch("run").include?('${{ needs.release-draft.outputs.package_manager_artifact_digest }}')
  raise "package-manager artifacts must be fetched and hashed by exact current-run artifact ID"
end
handoff_verify = package_steps.find { |step| step["name"] == "Verify handoff manifest and compute hashes" }
raise "missing package-manager handoff verification" unless handoff_verify
unless handoff_verify.fetch("env") == {
    "EXPECTED_ARTIFACT_DIGEST" => '${{ needs.release-draft.outputs.package_manager_artifact_digest }}',
    "EXPECTED_MANIFEST_SHA256" => '${{ needs.release-draft.outputs.package_manager_manifest_sha256 }}',
  }
  raise "package-manager handoff must carry the upload action digest"
end
unless handoff_verify.fetch("run") ==
    'tools/package-manager-handoff.sh verify release-assets "$OP_VERSION" "$EXPECTED_ARTIFACT_DIGEST" "$EXPECTED_MANIFEST_SHA256" "$GITHUB_ENV"'
  raise "package-manager handoff verification must use the reviewed repository helper"
end

build = jobs.fetch("build")
unless build.fetch("if") == "(github.event_name == 'workflow_dispatch' && inputs.ios_app_store_only == false) || (startsWith(github.ref, 'refs/tags/v') && github.event_name != 'workflow_dispatch')"
  raise "desktop builds must be disabled when manual iOS-only publication is selected"
end
job_env = build.fetch("env", {})
if job_env.key?("OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_CN") ||
    job_env.key?("OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_GLOBAL")
  raise "relay secrets must not be exposed at release-job scope"
end
build_step = build.fetch("steps").find { |step| step["name"] == "Build (host)" }
raise "missing production host build step" unless build_step
build_env = build_step.fetch("env")
%w[
  OPENPENCIL_RELEASE_TARGET
  OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_CN
  OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_GLOBAL
].each { |name| raise "missing protected #{name}" unless build_env.key?(name) }
unless build_step.fetch("run") == "scripts/build-rust-release-host.sh"
  raise "release build must use the reviewed repository-owned wrapper"
end
RUBY

require_literal() {
    grep -Fq -- "$1" "$2" || {
        printf 'error: %s is missing release contract: %s\n' "$2" "$1" >&2
        exit 1
    }
}

reject_literal() {
    if grep -Fq -- "$1" "$2"; then
        printf 'error: %s contains forbidden release contract: %s\n' "$2" "$1" >&2
        exit 1
    fi
}

require_count() {
    local expected=$1 literal=$2 file=$3 actual
    actual=$(grep -Fc -- "$literal" "$file" || true)
    [[ $actual -eq $expected ]] || {
        printf 'error: %s must contain %s copies of %s (found %s)\n' \
            "$file" "$expected" "$literal" "$actual" >&2
        exit 1
    }
}

require_literal 'tools/check-op-auth-release-matrix.test.sh' "$workflow"
require_literal 'tools/check-op-auth-release-matrix.sh' "$workflow"
require_literal 'tools/check-op-auth-prebuilt.sh --require-hardened' "$workflow"
require_literal 'tools/check-op-auth-cargo-build.test.sh' "$workflow"
require_literal 'tools/pinned-release-tools.test.sh' "$workflow"
reject_literal 'auth_artifact_ref:' "$workflow"
reject_literal 'tools/check-op-auth-artifact-commit.sh' "$workflow"
reject_literal 'tools/check-op-auth-artifact-commit.test.sh' "$workflow"
reject_literal 'OP_AUTH_RELEASE_EXPECTED_OPENPENCIL_REVISION:' "$workflow"
reject_literal 'OP_AUTH_RELEASE_WORKSPACE_VERSION:' "$workflow"
require_literal 'tools/check-collab-bootstrap-urls.py' "$release_builder"
require_literal 'cargo clean -p op-auth-bridge' "$release_builder"
require_literal 'tools/check-op-auth-cargo-build.sh' "$release_builder"
require_literal '--target "$release_target" --release --locked' "$release_builder"

bash "$pinned_tools_test"
bash "$package_handoff" --self-test
require_count 2 'tools/pinned-release-tools.sh binaryen "$GITHUB_WORKSPACE"' "$workflow"
require_literal 'tools/pinned-release-tools.sh appimage' "$workflow"
require_literal 'tools/pinned-release-tools.sh buildx "$buildx"' "$workflow"
reject_literal 'docker/setup-buildx-action@' "$workflow"
require_literal 'docker buildx create --name openpencil-release --driver docker-container' "$workflow"
require_literal '--driver-opt "image=$buildkit" --use' "$workflow"
require_literal 'moby/buildkit:v0.32.2@sha256:28a898719c18a33f4e8000685287fa36fd0dd9560c6440227d3a732d79bb41d8' "$workflow"
require_literal 'snapshot_url=https://snapshot.ubuntu.com/ubuntu/20260801T000000Z/' "$workflow"
require_literal 'Dir::Etc::sourcelist=$sources_file' "$workflow"
require_literal 'Dir::Etc::sourceparts=-' "$workflow"
require_literal '--print-uris update' "$workflow"
require_literal 'if [[ $uri != "$snapshot_url"* ]]' "$workflow"
reject_literal 'ports.ubuntu.com' "$workflow"
reject_literal 'archive.ubuntu.com' "$workflow"
require_literal 'tools/pinned-release-tools.sh skia desktop' "$workflow"
require_literal 'tools/package-manager-handoff.sh stage release-files package-manager-handoff' "$workflow"
require_literal 'tools/package-manager-handoff.sh download' "$workflow"
reject_literal 'gh release download' "$workflow"
require_literal '& tools/install-pinned-nsis.ps1 -SelfTest' "$workflow"
require_literal '& tools/stage-pinned-angle.ps1 -SelfTest' "$workflow"
reject_literal 'choco install' "$workflow"
reject_literal 'bunx ' "$workflow"
reject_literal 'npx ' "$workflow"
require_literal './node_modules/.bin/vsce package' "$workflow"
require_literal './node_modules/.bin/ovsx create-namespace' "$workflow"
require_literal './node_modules/.bin/ovsx publish' "$workflow"
require_count 1 'tools/pinned-release-tools.sh cargo-cli cargo-bundle' "$workflow"
require_count 2 'tools/pinned-release-tools.sh cargo-cli wasm-bindgen-cli' "$workflow"
reject_literal 'cargo install ' "$workflow"

require_literal 'runs-on: ubuntu-24.04' "$version_sync_workflow"
require_literal 'permissions:' "$version_sync_workflow"
require_literal 'contents: read' "$version_sync_workflow"
require_literal 'actions/checkout@08eba0b27e820071cde6df949e0beb9ba4906955' "$version_sync_workflow"
require_literal 'dtolnay/rust-toolchain@4360b52568e2003a75bf9bc1d59f33a8e3fc893c' "$version_sync_workflow"
require_literal "toolchain: '1.94'" "$version_sync_workflow"
require_literal 'tools/pinned-release-tools.test.sh' "$version_sync_workflow"
require_literal 'tools/pinned-release-tools.sh bun "$RUNNER_TEMP/bun-1.3.14"' "$version_sync_workflow"
require_literal 'tools/pinned-release-tools.sh ripgrep "$RUNNER_TEMP/ripgrep-15.2.0"' "$version_sync_workflow"
reject_literal 'setup-bun@' "$version_sync_workflow"
reject_literal 'apt-get' "$version_sync_workflow"
while IFS= read -r action; do
    [[ "$action" =~ @[0-9a-f]{40}$ ]] || {
        printf 'error: mutable version-sync action: %s\n' "$action" >&2
        exit 1
    }
done < <(sed -n 's/^[[:space:]]*uses: \([^ #]*\).*$/\1/p' "$version_sync_workflow")

require_literal '48af8a397ebd60178778bf63611dbcebe5f5e7a9be90eb9147b24b9587455778' "$pinned_tools"
require_literal 'e959f2170af4c20c552e9de3a0253704d6a9d2766e8fdb88e4d6ac4bae9388fe' "$pinned_tools"
require_literal 'ed4ce84f0d9caff66f50bcca6ff6f35aae54ce8135408b3fa33abfc3cb384eb0' "$pinned_tools"
require_literal 'f0837e7448a0c1e4e650a93bb3e85802546e60654ef287576f46c71c126a9158' "$pinned_tools"
require_literal '2fca8b443c92510f1483a883f60061ad09b46b978b2631c807cd873a47ec260d' "$pinned_tools"
require_literal '00cbdfcf917cc6c0ff6d3347d59e0ca1f7f45a6df1a428a0d6d8a78664d87444' "$pinned_tools"
require_literal '41058f8f2967385b2799764c2c281fd143392ef82221d5ffde0481a1cdbfc40e' "$pinned_tools"
require_literal 'bb3601b2899d4887512bdcaad115074750be7c212b122fa7ed4faed6c919229e' "$pinned_tools"
require_literal '33e15bcf1624b25cdd2a55813a47a2f95dbe126268203e76aa6a585d1e7b149c' "$pinned_tools"
require_literal "cargo-bundle) printf 'cargo-bundle v0.10.0" "$pinned_tools"
require_literal 'https://static.crates.io/crates/$crate_name/$crate_name-$version.crate' "$pinned_tools"
require_literal "cargo install \\" "$pinned_tools"
require_literal '--path "$source_dir" --locked --root "$install_root" --force' "$pinned_tools"
require_literal 'https://github.com/AppImage/type2-runtime/releases/download/20251108/' "$pinned_tools"
require_literal 'download_verified' "$pinned_tools"
for digest in \
    c4c5d5059ab9226aaf3d5337a8fd42ef0e42e9fe3cbc3c8da4310b4a3a1e4254 \
    fe92e66916947a4d666a24d0580434f42585853d221d2af006a52a72b55b283b \
    2587dcaf11aab680ef8637d4192fc77a507c91e3a88bebb79d7993a4fefa1d1b \
    ee77fbd0183e854e297276705e4e8685837c6c7d0304472c97145fcd8f7f2cfc \
    20ba7acf5e306b6d875863c838cb9d3c4a39a05792fb6256a3f03ddcbc1077a1 \
    6b61061c32fb7a72944e3dae63d97241271b1ac7bcaf3752cfa0c79ed37ee8b6 \
    c066658b13e257d418f647447d06eb8a83cb060b037228da838589dd863bf053 \
    4abbaea5e4e8934a6f19c5de44eaba9bf9238af4abbe57dbac5f2dc03923b182; do
    require_literal "$digest" "$pinned_tools"
done
# skia-bindings 0.97.2 strips file:// itself; Windows must leave C:/...,
# not /C:/..., for std::fs::read.
require_literal 'skia_file_url Windows' "$pinned_tools"
reject_literal 'FORCE_SKIA_BINARIES_DOWNLOAD=1' "$pinned_tools"
require_literal 'pinned-skia-binaries' "$release_builder"
require_literal 'cargo clean -p skia-bindings' "$release_builder"
require_literal '[[ -z ${FORCE_SKIA_BINARIES_DOWNLOAD:-}' "$release_builder"
require_literal 'pinned-skia-binaries = ["skia-safe/no-compile"]' \
    "$repo_root/crates/op-host-desktop/Cargo.toml"
require_literal 'pinned-skia-binaries = ["skia-safe/no-compile"]' \
    "$repo_root/crates/op-host-services/Cargo.toml"
require_literal 'pinned-skia-binaries = ["op-host-services/pinned-skia-binaries"]' \
    "$repo_root/crates/op-host-web-server/Cargo.toml"

require_literal '52bbe826b5e9d0dc779321866043d310aa8072d44ef3c05d7cdd3c4a69228fa0' "$angle_stager"
require_literal '781209a26586dcb1e545335dc451479424e94407f73cc25696f0035a31273323' "$angle_stager"
require_literal 'Assert-Sha256 $zipPath $asset.Sha256' "$angle_stager"
reject_literal 'Expand-Archive' "$angle_stager"
require_literal '4a1bbf9987e5b9b6bda4c2433af62bb79f2d9d3bd67b392f29a069ecda8c5f64' "$nsis_installer"
require_literal '3bc2b06253a7e4957111be152ac6a536e0c7478a706e19da814038db5d706495' "$nsis_installer"
require_literal 'Assert-Sha256 $installerPath $installerSha256' "$nsis_installer"
require_literal "\$packageVersion = '3.12.0'" "$nsis_installer"
require_literal '--runtime-file "$RUNTIME_FILE"' "$appimage_packager"
require_literal 'CARGO_BUNDLE_VERSION=0.10.0' "$macos_bundler"
require_literal 'CARGO_BUNDLE_HOME/bin/cargo-bundle' "$macos_bundler"
require_literal 'cargo-bundle v$CARGO_BUNDLE_VERSION' "$macos_bundler"
reject_literal 'command -v cargo-bundle' "$macos_bundler"
reject_literal 'cargo install cargo-bundle' "$macos_bundler"
require_literal 'export CARGO_BUNDLE_SKIP_BUILD=1' "$macos_bundler"
require_literal 'cargo build --release --locked' "$macos_bundler"
require_literal '--features canvaskit --release --locked' "$wasm_builder"
require_literal '--features canvaskit' "$sdk_wasm_builder"
require_literal "--release \\" "$sdk_wasm_builder"
require_literal '--locked' "$sdk_wasm_builder"

require_literal "throw 'certificate payload must not be a URL" "$windows_signer"
require_literal 'ExpectedPfxSha256' "$windows_signer"
require_literal 'ExpectedCertificateSha1' "$windows_signer"
require_literal '/tr https://timestamp.digicert.com' "$windows_signer"
require_literal 'Get-AuthenticodeSignature' "$windows_signer"
reject_literal 'Invoke-WebRequest' "$workflow"
reject_literal '/tr http://' "$workflow"

require_literal '# syntax=docker/dockerfile:1.26.0@sha256:ecfaec9ed6d810b56388c508f4121597bfbba70d41a6dfeee4d8cad5f295fc32' "$dockerfile"
require_literal "tools/pinned-release-tools.sh cargo-cli wasm-bindgen-cli \\" "$dockerfile"
require_literal '/opt/wasm-bindgen-cli-0.2.117' "$dockerfile"
require_literal 'version" != 0.2.117' "$dockerfile"
reject_literal 'cargo install wasm-bindgen-cli' "$dockerfile"
require_literal 'FROM rust:1.94-bookworm@sha256:6ae102bdbf528294bc79ad6e1fae682f6f7c2a6e6621506ba959f9685b308a55 AS builder' "$dockerfile"
require_literal 'FROM debian:bookworm-slim@sha256:abd67ffcfa541b485a3dff59865ab629aa048a6c613e639d36e7456b0b229241 AS runtime' "$dockerfile"
require_literal 'BINARYEN_SHA256=e959f2170af4c20c552e9de3a0253704d6a9d2766e8fdb88e4d6ac4bae9388fe' "$dockerfile"
require_literal 'sha256sum -c -' "$dockerfile"
require_literal 'SKIA_BINARY_SHA256=c066658b13e257d418f647447d06eb8a83cb060b037228da838589dd863bf053' "$dockerfile"
require_literal 'cargo build -p op-host-web-server --features pinned-skia-binaries --release --locked' "$dockerfile"
require_literal 'snapshot.debian.org/archive/debian/20260406T000000Z' "$dockerfile"
require_literal 'snapshot.debian.org/archive/debian/20260803T000000Z' "$dockerfile"
require_count 2 'Check-Valid-Until: no' "$dockerfile"
reject_literal '| tar -' "$dockerfile"

python3 - "$vscode_package" <<'PYTHON'
import json
import pathlib
import sys

package = json.loads(pathlib.Path(sys.argv[1]).read_text())
dev = package.get("devDependencies", {})
if dev.get("@vscode/vsce") != "3.9.2" or dev.get("ovsx") != "1.1.1":
    raise SystemExit("error: VSCE and Open VSX CLIs must use reviewed exact versions")
PYTHON
require_literal '"@vscode/vsce": ["@vscode/vsce@3.9.2"' "$bun_lock"
require_literal 'sha512-XSxMosEEDO6vLxELAHVkwmhC0qe0ijZni2jB9Rcs8kQsW4lhTDQ/wMzmwFs/buotAWSnpmUp/dRWD2ufG3UYKA==' "$bun_lock"
require_literal '"ovsx": ["ovsx@1.1.1"' "$bun_lock"
require_literal 'sha512-tklsCzvGVWKlM91Vc9U8tNnaQ+XacPJ12SWHjDaHGUJB49oMhoAULsJGeefhHebPvvckbcWbKqKIXODMZah5SA==' "$bun_lock"

line_count=$(wc -l < "$workflow" | tr -d '[:space:]')
[[ "$line_count" -le 1326 ]] || {
    printf 'error: Rust release workflow exceeds its 1326-line baseline\n' >&2
    exit 1
}

printf 'check-rust-release-auth-workflow.sh: release auth gates passed.\n'
