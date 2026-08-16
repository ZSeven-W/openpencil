#!/usr/bin/env bash
set -euo pipefail

player_dir="$(cd "$(dirname "$0")/.." && pwd)"
repo_dir="$(cd "$player_dir/../.." && pwd)"
header_dir="$repo_dir/crates/op-engine-ffi/include"
fixture="$repo_dir/crates/op-editor-core/assets/scene_templates/daily-sign-card.op"

required=(
  "$player_dir/OpenPencilPlayer-Bridging-Header.h"
  "$player_dir/project.yml"
  "$player_dir/README.md"
  "$player_dir/Assets.xcassets/AppIcon.appiconset/AppIcon-1024.png"
  "$player_dir/Assets.xcassets/AppIcon.appiconset/Contents.json"
  "$player_dir/Resources/ppt-demo.op"
  "$player_dir/Resources/sample.op"
  "$player_dir/Resources/en.lproj/InfoPlist.strings"
  "$player_dir/Resources/zh-Hans.lproj/InfoPlist.strings"
  "$player_dir/Sources/OpPlayerApp.swift"
  "$player_dir/Sources/OpPlayerView.swift"
  "$player_dir/Sources/OpEngineHost.swift"
  "$player_dir/Sources/AuthStorage.swift"
  "$player_dir/Sources/EmbeddedLoginRequest.swift"
  "$player_dir/Sources/EmbeddedLoginWebViewController.swift"
  "$player_dir/Sources/PinchZoomDelta.swift"
)

for path in "${required[@]}"; do
  if [[ ! -f "$path" ]]; then
    echo "missing required iOS Player source: $path" >&2
    exit 1
  fi
done

plutil -lint \
  "$player_dir/Resources/en.lproj/InfoPlist.strings" \
  "$player_dir/Resources/zh-Hans.lproj/InfoPlist.strings" >/dev/null
grep -Fq '"NSLocalNetworkUsageDescription"' "$player_dir/Resources/en.lproj/InfoPlist.strings"
grep -Fq '"NSLocalNetworkUsageDescription"' "$player_dir/Resources/zh-Hans.lproj/InfoPlist.strings"

cmp "$fixture" "$player_dir/Resources/sample.op"

while IFS= read -r source; do
  lines="$(wc -l < "$source" | tr -d ' ')"
  if (( lines > 800 )); then
    echo "$source has $lines lines; new files are capped at 800" >&2
    exit 1
  fi
done < <(find "$player_dir/Sources" -name '*.swift' -type f -print)

ruby - "$player_dir/project.yml" <<'RUBY'
require "yaml"
project = YAML.safe_load(File.read(ARGV.fetch(0)), aliases: true)
target = project.fetch("targets").fetch("OpenPencilPlayer")
raise "OpenPencilPlayer must be an iOS application" unless target["platform"] == "iOS" && target["type"] == "application"
raise "bundle-id prefix must be tech.zseven" unless project.fetch("options").fetch("bundleIdPrefix") == "tech.zseven"
settings = target.fetch("settings").fetch("base")
raise "bundle identifier must be tech.zseven.openpencil" unless settings.fetch("PRODUCT_BUNDLE_IDENTIFIER") == "tech.zseven.openpencil"
raise "deployment target must be iOS 15+" unless settings.fetch("IPHONEOS_DEPLOYMENT_TARGET").to_f >= 15.0
raise "display name must be OpenPencil" unless settings.fetch("INFOPLIST_KEY_CFBundleDisplayName") == "OpenPencil"
raise "export-compliance exemption must stay source-controlled" unless settings.fetch("INFOPLIST_KEY_ITSAppUsesNonExemptEncryption") == "NO"
raise "exempt builds must not carry an export-compliance code" if settings.key?("INFOPLIST_KEY_ITSEncryptionExportComplianceCode")
iphone_orientations = settings.fetch("INFOPLIST_KEY_UISupportedInterfaceOrientations").split
ipad_orientations = settings.fetch("INFOPLIST_KEY_UISupportedInterfaceOrientations_iPad").split
expected_ipad_orientations = %w[
  UIInterfaceOrientationPortrait
  UIInterfaceOrientationPortraitUpsideDown
  UIInterfaceOrientationLandscapeLeft
  UIInterfaceOrientationLandscapeRight
]
raise "iPhone orientation contract drifted" unless iphone_orientations == expected_ipad_orientations.reject { |value| value.end_with?("UpsideDown") }
raise "iPad multitasking requires all four orientations" unless ipad_orientations == expected_ipad_orientations
local_network_usage = settings.fetch("INFOPLIST_KEY_NSLocalNetworkUsageDescription")
raise "manual LAN collaboration requires a local-network usage description" unless local_network_usage.include?("collaboration")
raise "relay-only mobile builds must not declare Bonjour discovery" if settings.key?("INFOPLIST_KEY_NSBonjourServices")
raise "system appearance must remain runtime-controlled" if settings.key?("INFOPLIST_KEY_UIUserInterfaceStyle")
raise "AppIcon catalog setting missing" unless settings.fetch("ASSETCATALOG_COMPILER_APPICON_NAME") == "AppIcon"
raise "bridging header setting missing" unless settings.key?("SWIFT_OBJC_BRIDGING_HEADER")
raise "op_engine.h search path missing" unless settings.fetch("HEADER_SEARCH_PATHS").to_s.include?("op-engine-ffi/include")
raise "device staticlib search path missing" unless settings.fetch("LIBRARY_SEARCH_PATHS[sdk=iphoneos*]").include?("aarch64-apple-ios/release")
raise "simulator staticlib search path missing" unless settings.fetch("LIBRARY_SEARCH_PATHS[sdk=iphonesimulator*]").include?("aarch64-apple-ios-sim/release")
frameworks = target.fetch("dependencies").map { |entry| entry["sdk"] }.compact
%w[CoreFoundation.framework Metal.framework QuartzCore.framework WebKit.framework Security.framework UIKit.framework].each do |framework|
  raise "#{framework} dependency missing" unless frameworks.include?(framework)
end
raise "optional auth archive must be empty by default" unless settings.fetch("OP_AUTH_ARCHIVE") == ""
raise "final link must consume the explicit auth archive setting" unless settings.fetch("OTHER_LDFLAGS").include?("$(OP_AUTH_ARCHIVE)")
raise "final link must consume the exact engine archive setting" unless settings.fetch("OTHER_LDFLAGS").include?("$(OP_ENGINE_ARCHIVE)")
configs = target.fetch("settings").fetch("configs")
debug = configs.fetch("Debug")
release = configs.fetch("Release")
raise "Debug simulator must link the static debug engine" unless debug.fetch("OP_ENGINE_ARCHIVE[sdk=iphonesimulator*]").end_with?("aarch64-apple-ios-sim/debug/libop_engine_ffi.a")
raise "Debug device must link the static debug engine" unless debug.fetch("OP_ENGINE_ARCHIVE[sdk=iphoneos*]").end_with?("aarch64-apple-ios/debug/libop_engine_ffi.a")
raise "Release simulator must link the static release engine" unless release.fetch("OP_ENGINE_ARCHIVE[sdk=iphonesimulator*]").end_with?("aarch64-apple-ios-sim/release/libop_engine_ffi.a")
raise "Release device must link the static release engine" unless release.fetch("OP_ENGINE_ARCHIVE[sdk=iphoneos*]").end_with?("aarch64-apple-ios/release/libop_engine_ffi.a")
scripts = target.fetch("preBuildScripts")
auth_gate = scripts.find { |script| script["name"] == "Validate optional op-auth archive" }
raise "mobile auth link gate missing" unless auth_gate
raise "mobile auth link gate must call the repository verifier" unless auth_gate.fetch("script").include?("check-mobile-auth-link-input.sh")
RUBY

pbxproj="$player_dir/OpenPencilPlayer.xcodeproj/project.pbxproj"
if [[ -f "$pbxproj" ]]; then
  ruby - "$pbxproj" <<'RUBY'
project = File.read(ARGV.fetch(0))
display_names = project.scan(/^\s*INFOPLIST_KEY_CFBundleDisplayName = (?:"([^"]+)"|([^;]+));$/).map { |quoted, plain| quoted || plain }
raise "generated project has stale display-name settings" unless display_names == ["OpenPencil", "OpenPencil"]
encryption_declarations = project.scan(/^\s*INFOPLIST_KEY_ITSAppUsesNonExemptEncryption = (?:"([^"]+)"|([^;]+));$/).map { |quoted, plain| quoted || plain }
raise "generated project has stale export-compliance settings" unless encryption_declarations == ["NO", "NO"]
raise "generated project must not contain an export-compliance code" if project.include?("INFOPLIST_KEY_ITSEncryptionExportComplianceCode")
ipad_orientations = project.scan(/^\s*INFOPLIST_KEY_UISupportedInterfaceOrientations_iPad = "([^"]+)";/).flatten
expected_ipad_orientations = "UIInterfaceOrientationPortrait UIInterfaceOrientationPortraitUpsideDown UIInterfaceOrientationLandscapeLeft UIInterfaceOrientationLandscapeRight"
raise "generated project must preserve all iPad multitasking orientations" unless ipad_orientations == [expected_ipad_orientations, expected_ipad_orientations]
RUBY
fi

ruby - "$player_dir/Assets.xcassets/AppIcon.appiconset" <<'RUBY'
require "json"

icon_dir = ARGV.fetch(0)
contents = JSON.parse(File.read(File.join(icon_dir, "Contents.json")))
image = contents.fetch("images").find { |entry| entry["filename"] == "AppIcon-1024.png" }
raise "1024px universal iOS AppIcon entry missing" unless image
raise "AppIcon must target iOS" unless image["platform"] == "ios"
raise "AppIcon must be universal" unless image["idiom"] == "universal"
raise "AppIcon must declare 1024x1024" unless image["size"] == "1024x1024"

png = File.binread(File.join(icon_dir, "AppIcon-1024.png"))
raise "AppIcon is not a PNG" unless png.start_with?("\x89PNG\r\n\x1a\n".b)
width, height, bit_depth, color_type = png.byteslice(16, 13).unpack("NNCCC")
raise "AppIcon must be 1024x1024" unless width == 1024 && height == 1024
raise "AppIcon must be 8-bit opaque RGB" unless bit_depth == 8 && color_type == 2
RUBY

grep -Fq -- "-sdk iphonesimulator26.4" "$player_dir/README.md"
grep -Fq -- "-destination 'platform=iOS Simulator,id=<sim-id>'" "$player_dir/README.md"
grep -Fq -- "aarch64-apple-ios-sim/release/libop_engine_ffi.a -lc++" "$player_dir/README.md"
grep -Fq -- "aarch64-apple-ios/release/libop_engine_ffi.a -lc++" "$player_dir/README.md"
grep -Fq -- "-framework Metal" "$player_dir/README.md"
grep -Fq -- "-framework CoreFoundation" "$player_dir/README.md"
grep -Fq -- "-framework Security" "$player_dir/README.md"
grep -Fq -- "UIDocumentPickerViewController" "$player_dir/Sources/OpPlayerView.swift"
grep -Fq -- "BoundedDocumentReader.read" "$player_dir/Sources/OpPlayerView.swift"
grep -Fq -- "maximumBytes = 32 * 1024 * 1024" "$player_dir/Sources/BoundedDocumentReader.swift"
grep -Fq -- "op_editor_take_shell_action" "$player_dir/Sources/OpEngineHost.swift"
grep -Fq -- "op_editor_open_document" "$player_dir/Sources/OpEngineHost.swift"
grep -Fq -- "desc.storage_root_ptr" "$player_dir/Sources/OpEngineHost.swift"
grep -Fq -- "desc.storage_root_len" "$player_dir/Sources/OpEngineHost.swift"
grep -Fq -- "callbacks.credential_load = nil" "$player_dir/Sources/OpEngineHost.swift"
grep -Fq -- "callbacks.credential_store_if_absent = nil" "$player_dir/Sources/OpEngineHost.swift"
grep -Fq -- "FileProtectionType.completeUntilFirstUserAuthentication" "$player_dir/Sources/AuthStorage.swift"
grep -Fq -- "isExcludedFromBackup = true" "$player_dir/Sources/AuthStorage.swift"
grep -Fq -- 'return "ppt-demo"' "$player_dir/Sources/OpEngineHost.swift"
grep -Fq -- '.ignoresSafeArea(.all, edges: .all)' "$player_dir/Sources/OpPlayerApp.swift"
grep -Fq -- 'viewportConvergence.signal' "$player_dir/Sources/OpPlayerView.swift"
grep -Fq -- 'viewportConvergence.displayFrame' "$player_dir/Sources/OpPlayerView.swift"
grep -Fq -- 'geometryGate.isPending || !suppressedTouches.isEmpty' "$player_dir/Sources/OpPlayerView.swift"
grep -Fq -- 'suppressedTouches.suppress(touchIDs.keys)' "$player_dir/Sources/OpPlayerView.swift"
grep -Fq -- 'safeArea: UIEdgeInsets(' "$player_dir/Sources/OpPlayerView.swift"
grep -Fq -- 'op_resize_with_safe_area(' "$player_dir/Sources/OpEngineHost.swift"
if grep -Fq -- 'op_set_safe_area(engine' "$player_dir/Sources/OpEngineHost.swift"; then
  echo "iOS viewport updates must not split size and safe-area mutations" >&2
  exit 1
fi
grep -Fq -- 'keyboardLayoutGuide.followsUndockedKeyboard = false' "$player_dir/Sources/OpPlayerView.swift"
grep -Fq -- 'op_prefers_light_system_icons(engine' "$player_dir/Sources/OpEngineHost.swift"
grep -Fq -- 'op_editor_begin_transform(engine' "$player_dir/Sources/OpEngineHost.swift"
grep -Fq -- 'host.editorBeginTransform(x: lastMidpoint.x, y: lastMidpoint.y)' "$player_dir/Sources/OpPlayerView.swift"
grep -Fq -- 'prefersLightIcons ? .dark : .light' "$player_dir/Sources/OpPlayerView.swift"
grep -Fq -- 'window.overrideUserInterfaceStyle = systemChromeStyle' "$player_dir/Sources/OpPlayerView.swift"

ruby - "$player_dir/Sources/OpPlayerView.swift" <<'RUBY'
source = File.read(ARGV.fetch(0))
ended = source[/private func editorTouchEnded\b.*?(?=\n    private func resetEditorTouchTracking)/m]
raise "editor touch-end routing missing" unless ended
suppress = ended.index("suppressedTouches.suppress(touchIDs.keys)")
reset = ended.index("resetEditorTouchTracking()")
raise "two-finger tail must be suppressed before tracking resets" unless suppress && reset && suppress < reset
raise "two-finger tail must not re-arm single-finger release" if ended.include?("primaryTouchKey = remainingKey")

touches_ended = source[/override func touchesEnded\b.*?(?=\n    override func touchesCancelled)/m]
raise "editor touchesEnded routing missing" unless touches_ended
route = touches_ended.index("editorTouchEnded(touch, key: key)")
finish = touches_ended.index("suppressedTouches.finish([key])")
raise "same-batch terminal touch must clear suppression" unless route && finish && route < finish
RUBY

ruby - "$player_dir/Sources/OpEngineHost.swift" <<'RUBY'
source = File.read(ARGV.fetch(0))
create = source[/private func createAndAttach\b.*?(?=\n    \/\/\/ Installs the mobile auth runtime)/m]
raise "iOS engine create path missing" unless create
prepare = create.index("let storageURL = AuthStorage.prepare()")
root = create.index("desc.storage_root_ptr")
call = create.index("return op_create(&desc, &created)")
raise "private storage must be prepared and bound before op_create" unless prepare && root && call && prepare < root && root < call
raise "auth must reuse the create-time storage root" unless create.include?("configureMobileAuth(engine: created, storageURL: storageURL)")
RUBY

ruby "$repo_dir/packaging/mobile-editor-handoff/Tests/TouchCancelRoutingTests.rb" \
  "$player_dir/Sources/OpPlayerView.swift" \
  "$repo_dir/packaging/android-player/app/src/main/kotlin/tech/zseven/openpencil/OpSurfaceView.kt"
ruby "$repo_dir/packaging/mobile-editor-handoff/Tests/PinchZoomRoutingTests.rb" \
  "$player_dir/Sources/OpPlayerView.swift" \
  "$repo_dir/packaging/android-player/app/src/main/kotlin/tech/zseven/openpencil/OpSurfaceView.kt"
ruby "$repo_dir/packaging/mobile-editor-handoff/Tests/BundledPptDemoTests.rb"

sdk="$(xcrun --sdk iphonesimulator --show-sdk-path)"
target="arm64-apple-ios15.0-simulator"
module_cache="${TMPDIR:-/tmp}/op-ios-player-module-cache"
mkdir -p "$module_cache"
export CLANG_MODULE_CACHE_PATH="$module_cache"

xcrun clang \
  -target "$target" \
  -isysroot "$sdk" \
  -fsyntax-only \
  -x objective-c \
  -I "$header_dir" \
  "$player_dir/OpenPencilPlayer-Bridging-Header.h"

xcrun swiftc \
  -typecheck \
  -warnings-as-errors \
  -parse-as-library \
  -target "$target" \
  -sdk "$sdk" \
  -import-objc-header "$player_dir/OpenPencilPlayer-Bridging-Header.h" \
  -module-cache-path "$module_cache" \
  -Xcc -I \
  -Xcc "$header_dir" \
  "$player_dir"/Sources/*.swift

reader_test_dir="$(mktemp -d "${TMPDIR:-/tmp}/op-bounded-document-reader-tests.XXXXXX")"
trap 'rm -rf "$reader_test_dir"' EXIT
reader_test="$reader_test_dir/runner"
xcrun swiftc \
  -warnings-as-errors \
  -parse-as-library \
  "$player_dir/Sources/BoundedDocumentReader.swift" \
  "$player_dir/Tests/BoundedDocumentReaderTests.swift" \
  -o "$reader_test"
"$reader_test"

keyboard_test="$reader_test_dir/keyboard-occlusion-runner"
xcrun swiftc \
  -warnings-as-errors \
  -parse-as-library \
  "$player_dir/Sources/KeyboardOcclusion.swift" \
  "$player_dir/Tests/KeyboardOcclusionTests.swift" \
  -o "$keyboard_test"
"$keyboard_test"

viewport_test="$reader_test_dir/viewport-change-runner"
xcrun swiftc \
  -warnings-as-errors \
  -parse-as-library \
  "$player_dir/Sources/ViewportInsets.swift" \
  "$player_dir/Sources/ViewportChange.swift" \
  "$player_dir/Tests/ViewportChangeTests.swift" \
  -o "$viewport_test"
"$viewport_test"

insets_test="$reader_test_dir/viewport-insets-runner"
xcrun swiftc \
  -warnings-as-errors \
  -parse-as-library \
  "$player_dir/Sources/ViewportInsets.swift" \
  "$player_dir/Tests/ViewportInsetsTests.swift" \
  -o "$insets_test"
"$insets_test"

pinch_test="$reader_test_dir/pinch-zoom-delta-runner"
xcrun swiftc \
  -warnings-as-errors \
  -parse-as-library \
  "$player_dir/Sources/PinchZoomDelta.swift" \
  "$player_dir/Tests/PinchZoomDeltaTests.swift" \
  -o "$pinch_test"
"$pinch_test"

embedded_login_test="$reader_test_dir/embedded-login-request-runner"
xcrun swiftc \
  -warnings-as-errors \
  -parse-as-library \
  "$player_dir/Sources/EmbeddedLoginRequest.swift" \
  "$player_dir/Tests/EmbeddedLoginRequestTests.swift" \
  -o "$embedded_login_test"
"$embedded_login_test"

if [[ -f "$player_dir/Tests/EmbeddedLoginLifecycleTests.rb" ]]; then
  ruby "$player_dir/Tests/EmbeddedLoginLifecycleTests.rb" \
    "$player_dir/Sources/OpPlayerView.swift" \
    "$player_dir/Sources/EmbeddedLoginWebViewController.swift" \
    "$player_dir/Sources/AuthStorage.swift"
fi

echo "iOS Player sources and ABI imports validate"
