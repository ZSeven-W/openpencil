#!/usr/bin/env bash
set -euo pipefail

player_dir="$(cd "$(dirname "$0")/.." && pwd)"
repo_dir="$(cd "$player_dir/../.." && pwd)"
header_dir="$repo_dir/crates/op-engine-ffi/include"
fixture="$repo_dir/crates/op-editor-core/assets/scene_templates/daily-sign-card.op"

required=(
  "$player_dir/OpenPencilPlayer-Bridging-Header.h"
  "$player_dir/OpenPencilPlayer.entitlements"
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
  "$player_dir/Sources/ImageImportCoordinator.swift"
  "$player_dir/Sources/GenerationBackgroundCoordinator.swift"
  "$player_dir/Sources/GenerationBackgroundRegistration.swift"
  "$player_dir/Sources/GenerationBackgroundState.swift"
  "$player_dir/Sources/DocumentExportCoordinator.swift"
  "$player_dir/Sources/DocumentSaveCoordinator.swift"
  "$player_dir/Sources/DocumentSaveBinding.swift"
  "$player_dir/Sources/AuthStorage.swift"
  "$player_dir/Sources/DocumentStorage.swift"
  "$player_dir/Sources/DeviceLoginRequestInfo.swift"
  "$player_dir/Sources/NativeLoginViewController.swift"
  "$player_dir/Sources/AuthCodeFormViewController.swift"
  "$player_dir/Sources/RegisterViewController.swift"
  "$player_dir/Sources/AuthTheme.swift"
  "$player_dir/Sources/AccountCenterViewController.swift"
  "$player_dir/Sources/SsoAuthClient.swift"
  "$player_dir/Sources/SsoProviderList.swift"
  "$player_dir/Sources/SsoRegion.swift"
  "$player_dir/Sources/PinchZoomDelta.swift"
  "$player_dir/Sources/UniversalLink.swift"
  "$player_dir/Tests/GenerationBackgroundRegistrationTests.swift"
  "$player_dir/Tests/GenerationBackgroundStateTests.swift"
)

for path in "${required[@]}"; do
  if [[ ! -f "$path" ]]; then
    echo "missing required iOS Player source: $path" >&2
    exit 1
  fi
done

background_identifier="tech.zseven.openpencil.generation.*"
if [[ "$(plutil -extract BGTaskSchedulerPermittedIdentifiers.0 raw -o - \
  "$player_dir/Info.plist" 2>/dev/null)" != "$background_identifier" ]]; then
  echo "Info.plist must authorize the continued-processing identifier wildcard" >&2
  exit 1
fi
if [[ "$(plutil -extract UIBackgroundModes.0 raw -o - \
  "$player_dir/Info.plist" 2>/dev/null)" != "processing" ]]; then
  echo "Info.plist must enable background processing for BGTaskScheduler" >&2
  exit 1
fi

plutil -lint \
  "$player_dir/Info.plist" \
  "$player_dir/OpenPencilPlayer.entitlements" \
  "$player_dir/Resources/en.lproj/InfoPlist.strings" \
  "$player_dir/Resources/zh-Hans.lproj/InfoPlist.strings" >/dev/null
grep -Fq '"NSLocalNetworkUsageDescription"' "$player_dir/Resources/en.lproj/InfoPlist.strings"
grep -Fq '"NSLocalNetworkUsageDescription"' "$player_dir/Resources/zh-Hans.lproj/InfoPlist.strings"
grep -Fq '"backgroundGeneration.title"' "$player_dir/Resources/en.lproj/Localizable.strings"
grep -Fq '"backgroundGeneration.title"' "$player_dir/Resources/zh-Hans.lproj/Localizable.strings"

cmp "$fixture" "$player_dir/Resources/sample.op"

# Saved documents are only visible in the Files app when the generated
# Info.plist carries BOTH sharing keys.
for key in UIFileSharingEnabled LSSupportsOpeningDocumentsInPlace; do
  if [[ "$(plutil -extract "$key" raw -o - "$player_dir/Info.plist" 2>/dev/null)" != "true" ]]; then
    echo "Info.plist must set $key to true so saved documents appear in Files" >&2
    exit 1
  fi
done

# Save / Save As must go through the document picker (the engine paints no
# name dialog once the capability is declared), and the picker must still open
# on the Files-visible Documents directory.
grep -Fq 'DocumentSaveCoordinator.declareCapability(engine: created, host: self)' \
  "$player_dir/Sources/OpEngineHost.swift"
grep -Fq 'op_editor_configure_save_picker(engine, true)' \
  "$player_dir/Sources/DocumentSaveCoordinator.swift"
grep -Fq 'OpShellAction_SaveDocument.rawValue' "$player_dir/Sources/OpEngineHost.swift"
grep -Fq 'picker.directoryURL = DocumentStorage.prepare()' \
  "$player_dir/Sources/DocumentSaveCoordinator.swift"
# Only a reported destination write may mark the document saved.
grep -Fq 'op_editor_commit_save' "$player_dir/Sources/DocumentSaveCoordinator.swift"
grep -Fq 'op_editor_cancel_save' "$player_dir/Sources/DocumentSaveCoordinator.swift"

# The touch shape picker must cross the shell boundary exactly once and return
# bounded bytes to Rust, where collaboration permission is checked again.
grep -Fq 'OpShellAction_ImportImageOrSvg.rawValue' "$player_dir/Sources/OpEngineHost.swift"
grep -Fq 'op_editor_import_image_or_svg' "$player_dir/Sources/ImageImportCoordinator.swift"
grep -Fq 'BoundedDocumentReader.read' "$player_dir/Sources/ImageImportCoordinator.swift"
grep -Fq 'allowsMultipleSelection = false' "$player_dir/Sources/ImageImportCoordinator.swift"
grep -Fq 'modalPresentationStyle = .formSheet' "$player_dir/Sources/ImageImportCoordinator.swift"
grep -Fq '"imageImport.error.title"' "$player_dir/Resources/en.lproj/Localizable.strings"
grep -Fq '"imageImport.error.title"' "$player_dir/Resources/zh-Hans.lproj/Localizable.strings"
grep -Fq '"imageImport.error.body"' "$player_dir/Resources/en.lproj/Localizable.strings"
grep -Fq '"imageImport.error.body"' "$player_dir/Resources/zh-Hans.lproj/Localizable.strings"

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
%w[BackgroundTasks.framework CoreFoundation.framework Metal.framework QuartzCore.framework Security.framework UIKit.framework].each do |framework|
  raise "#{framework} dependency missing" unless frameworks.include?(framework)
end
raise "native login must not link WebKit" if frameworks.include?("WebKit.framework")
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
info_properties = target.fetch("info").fetch("properties")
%w[UIFileSharingEnabled LSSupportsOpeningDocumentsInPlace].each do |key|
  raise "#{key} must be declared so saved documents appear in Files" unless info_properties[key] == true
end
expected_background_ids = ["tech.zseven.openpencil.generation.*"]
raise "continued-processing wildcard drifted" unless info_properties["BGTaskSchedulerPermittedIdentifiers"] == expected_background_ids
raise "background processing mode missing" unless info_properties["UIBackgroundModes"] == ["processing"]
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
raise "generated project must link BackgroundTasks" unless project.include?("BackgroundTasks.framework")
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
grep -Fq -- "-framework BackgroundTasks" "$player_dir/README.md"
grep -Fq -- "UIDocumentPickerViewController" "$player_dir/Sources/OpPlayerView.swift"
grep -Fq -- "BoundedDocumentReader.read" "$player_dir/Sources/OpPlayerView.swift"
grep -Fq -- "maximumBytes = 32 * 1024 * 1024" "$player_dir/Sources/BoundedDocumentReader.swift"
grep -Fq -- "op_editor_take_shell_action" "$player_dir/Sources/OpEngineHost.swift"
grep -Fq -- "op_editor_open_document" "$player_dir/Sources/OpEngineHost.swift"
grep -Fq -- "OpShellAction_ExportDocument.rawValue" "$player_dir/Sources/OpEngineHost.swift"
grep -Fq -- "documentExportCoordinator.cancelForTeardown()" "$player_dir/Sources/OpEngineHost.swift"
grep -Fq -- "op_editor_copy_export_file_name" "$player_dir/Sources/DocumentExportCoordinator.swift"
grep -Fq -- "op_editor_export_to_path" "$player_dir/Sources/DocumentExportCoordinator.swift"
grep -Fq -- "op_editor_cancel_export" "$player_dir/Sources/DocumentExportCoordinator.swift"
grep -Fq -- "UUID().uuidString" "$player_dir/Sources/DocumentExportCoordinator.swift"
grep -Fq -- "UIDocumentPickerViewController(forExporting: [stagedFile], asCopy: true)" "$player_dir/Sources/DocumentExportCoordinator.swift"
grep -Fq -- "FileManager.default.removeItem(at: directory)" "$player_dir/Sources/DocumentExportCoordinator.swift"
grep -Fq -- "desc.storage_root_ptr" "$player_dir/Sources/OpEngineHost.swift"
grep -Fq -- "desc.storage_root_len" "$player_dir/Sources/OpEngineHost.swift"
grep -Fq -- "desc.documents_root_ptr" "$player_dir/Sources/OpEngineHost.swift"
grep -Fq -- "desc.documents_root_len" "$player_dir/Sources/OpEngineHost.swift"
grep -Fq -- "for: .documentDirectory" "$player_dir/Sources/DocumentStorage.swift"
# The user's own documents belong in iCloud/iTunes backups; only the
# private auth root opts out.
if grep -Fq -- "isExcludedFromBackup" "$player_dir/Sources/DocumentStorage.swift"; then
  echo "saved documents must stay in device backups" >&2
  exit 1
fi
grep -Fq -- "callbacks.credential_load = nil" "$player_dir/Sources/OpEngineHost.swift"
grep -Fq -- "callbacks.credential_store_if_absent = nil" "$player_dir/Sources/OpEngineHost.swift"
grep -Fq -- "FileProtectionType.completeUntilFirstUserAuthentication" "$player_dir/Sources/AuthStorage.swift"
grep -Fq -- "isExcludedFromBackup = true" "$player_dir/Sources/AuthStorage.swift"
grep -Fq -- 'if editorMode && explicitDocName == nil' "$player_dir/Sources/OpEngineHost.swift"
grep -Fq -- 'document = Data()' "$player_dir/Sources/OpEngineHost.swift"
grep -Fq -- 'let docName = explicitDocName ?? "ppt-demo"' "$player_dir/Sources/OpEngineHost.swift"
grep -Fq -- '.ignoresSafeArea(.all, edges: .all)' "$player_dir/Sources/OpPlayerApp.swift"
grep -Fq -- '.onOpenURL { url in' "$player_dir/Sources/OpPlayerApp.swift"
grep -Fq -- 'UniversalLinkRouter.handle(url)' "$player_dir/Sources/OpPlayerApp.swift"
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

# FACTUAL pointer timestamps: every live editor press/move/release carries
# UITouch.timestamp*1000 into the `_at` C ABI, and every synthetic Cancel
# (touchesCancelled, long-press / geometry / two-finger takeovers) carries
# the same monotonic boot-uptime domain via CACurrentMediaTime*1000.
grep -Fq -- 'editorPressAt(x: point.x, y: point.y, timeMs: touchTimestampMs(touch))' "$player_dir/Sources/OpPlayerView.swift"
grep -Fq -- 'editorMoveAt(x: point.x, y: point.y, timeMs: touchTimestampMs(touch))' "$player_dir/Sources/OpPlayerView.swift"
grep -Fq -- 'editorReleaseAt(x: point.x, y: point.y, timeMs: touchTimestampMs(touch))' "$player_dir/Sources/OpPlayerView.swift"
grep -Fq -- 'editorCancelGestureAt(timeMs: OpEngineHost.syntheticCancelNowMs())' "$player_dir/Sources/OpPlayerView.swift"
grep -Fq -- 'op_editor_press_at(engine, Float(x), Float(y), timeMs)' "$player_dir/Sources/OpEngineHost+Pointer.swift"
grep -Fq -- 'op_editor_move_at(engine, Float(x), Float(y), timeMs)' "$player_dir/Sources/OpEngineHost+Pointer.swift"
grep -Fq -- 'op_editor_release_at(engine, Float(x), Float(y), timeMs)' "$player_dir/Sources/OpEngineHost+Pointer.swift"
grep -Fq -- 'op_editor_cancel_gesture_at(engine, timeMs)' "$player_dir/Sources/OpEngineHost+Pointer.swift"
grep -Fq -- 'UInt64((touch.timestamp * 1_000).rounded(.down))' "$player_dir/Sources/OpPlayerView.swift"
grep -Fq -- 'CACurrentMediaTime() * 1_000' "$player_dir/Sources/OpEngineHost+Pointer.swift"
# The generic viewer pointer route must carry the touch timestamp too.
grep -Fq -- 'timeMs: touchTimestampMs(touch)' "$player_dir/Sources/OpPlayerView.swift"
grep -Fq -- 'prefersLightIcons ? .dark : .light' "$player_dir/Sources/OpPlayerView.swift"
grep -Fq -- 'window.overrideUserInterfaceStyle = systemChromeStyle' "$player_dir/Sources/OpPlayerView.swift"

background_coordinator="$player_dir/Sources/GenerationBackgroundCoordinator.swift"
grep -Fq -- 'op_has_background_work(engine, &active)' "$background_coordinator"
grep -Fq -- 'op_background_tick(engine, OpEngineHost.nowMilliseconds(), &active)' \
  "$background_coordinator"
grep -Fq -- 'op_cancel_background_work(engine)' "$background_coordinator"
grep -Fq -- 'BGContinuedProcessingTaskRequest(' "$background_coordinator"
grep -Fq -- 'request.strategy = .fail' "$background_coordinator"
grep -Fq -- 'task.progress.totalUnitCount = -1' "$background_coordinator"
grep -Fq -- 'task.progress.completedUnitCount = 0' "$background_coordinator"
grep -Fq -- 'beginBackgroundTask(' "$background_coordinator"
if grep -Fq -- 'op_frame(' "$background_coordinator"; then
  echo "background generation must never invoke the GPU frame pump" >&2
  exit 1
fi

ruby - "$player_dir/Sources/OpEngineHost.swift" "$background_coordinator" \
  "$player_dir/Sources/GenerationBackgroundState.swift" \
  "$player_dir/Sources/GenerationBackgroundRegistration.swift" <<'RUBY'
host = File.read(ARGV.fetch(0))
coordinator = File.read(ARGV.fetch(1))
state = File.read(ARGV.fetch(2))
registration_model = File.read(ARGV.fetch(3))
suspend = host[/private func suspendForBackground\b.*?(?=\n    private func resumeFromForeground)/m]
raise "iOS background suspend path missing" unless suspend
detach = suspend.index("op_suspend(engine)")
background = suspend.index("generationBackgroundCoordinator.didEnterBackground()")
raise "Metal must detach before background ticks begin" unless detach && background && detach < background
resume = host[/private func resumeFromForeground\b.*?(?=\n    \/\/\/ `needs_redraw`)/m]
raise "iOS foreground resume path missing" unless resume
stop = resume.index("generationBackgroundCoordinator.willEnterForeground()")
attach = resume.index("op_resume(engine, &desc)")
raise "background ticks must stop before Metal reattaches" unless stop && attach && stop < attach
expiration = state[/mutating func protectionExpired\b.*?(?=\n    mutating func teardown)/m]
raise "background expiration state missing" unless expiration
stop_pump = expiration.index("stopPumpIfNeeded()")
cancel_work = expiration.index("cancelWorkIfNeeded()")
finish_task = expiration.index(".finishProtection(success: false)")
raise "expiration must stop pump, cancel engine work, then finish the task" unless stop_pump && cancel_work && finish_task && stop_pump < cancel_work && cancel_work < finish_task
teardown = state[/mutating func teardown\b.*?(?=\n    private mutating func reconcilePump)/m]
raise "background teardown state missing" unless teardown
stop_pump = teardown.index("stopPumpIfNeeded()")
cancel_work = teardown.index("cancelWorkIfNeeded()")
finish_task = teardown.index(".finishProtection(success: false)")
raise "teardown must stop pump, cancel engine work, then finish the task" unless stop_pump && cancel_work && finish_task && stop_pump < cancel_work && cancel_work < finish_task
raise "foreground frames must observe generation work" unless host[/func displayLinkDidFire\b.*?(?=\n    \/\/\/ Keeps native)/m]&.include?("generationBackgroundCoordinator.observeEngineWork()")
release = host[/func editorRelease\b.*?(?=\n    func editorCancelGesture)/m]
raise "pointer release must observe newly submitted generation work" unless release&.include?("generationBackgroundCoordinator.observeEngineWork()")
raise "continued identifier prefix must match the permitted wildcard" unless coordinator.include?('continuedIdentifierPrefix = "tech.zseven.openpencil.generation."')
raise "each generation must use a unique identifier suffix" unless coordinator.include?("UUID().uuidString.lowercased()")
request = coordinator[/private func requestContinuedProtection\b.*?(?=\n    @available\(iOS 26\.0, \*\)\n    private func handleContinuedTask)/m]
raise "continued-processing request path missing" unless request
register_task = request.index("scheduler.register(")
submit_task = request.index("try scheduler.submit(request)")
mark_submitted = request.index("registration.markSubmitted()")
start_handoff = request.index("guard startApplicationProtection(")
unless register_task && submit_task && mark_submitted && start_handoff &&
       register_task < submit_task && submit_task < mark_submitted && mark_submitted < start_handoff
  raise "continued request must register, submit, record submission, then acquire its finite handoff"
end
raise "registration must capture only weak owner plus generation identity" unless request.include?("[weak self, registration]")
raise "late handlers without an owner need the recorded terminal result" unless request.include?("registration.completion ?? false")
raise "submission and handoff failures must close the same registration" unless request.scan("failContinuedProtection(registration: registration)").length >= 2
handler = coordinator[/private func handleContinuedTask\b.*?(?=\n    private func expireContinuedTask)/m]
raise "continued handler path missing" unless handler
raise "handler must bind the system task's exact identifier" unless handler.include?("deliveredIdentifier: task.identifier")
raise "handler must require the exact current generation identity" unless handler.include?("continuedRegistration === registration && continuedTask == nil")
task_owner = handler.index("continuedTask = task")
end_handoff = handler.index("endApplicationProtection(token: registration.token)")
start_protection = handler.index("apply(state.protectionStarted())")
unless task_owner && end_handoff && start_protection && task_owner < end_handoff && end_handoff < start_protection
  raise "continued handler must own the task, end the handoff, then reconcile protection"
end
raise "unknown generation progress must use Foundation indeterminate state" unless handler.include?("task.progress.totalUnitCount = -1") && handler.include?("task.progress.completedUnitCount = 0")
raise "continued progress must never be synthesized from elapsed time" if coordinator.match?(/systemUptime|lastProgressUpdate|advanceProgress/)
failure = coordinator[/private func failContinuedProtection\b.*?(?=\n    private func requestFallbackProtection)/m]
raise "continued failure path missing" unless failure
state_failure = failure.index("apply(state.protectionFailed())")
release_handoff = failure.index("endApplicationProtection(token: registration.token)")
terminalize = failure.index("registration.finish(success: false)")
cancel_request = failure.index("taskRequestWithIdentifier: registration.identifier")
unless state_failure && release_handoff && terminalize && cancel_request &&
       state_failure < release_handoff && release_handoff < terminalize && terminalize < cancel_request
  raise "failure must stop the pump, end handoff, terminalize, then cancel the exact request"
end
finish = coordinator[/private func finishProtection\b.*?\n    \}\n\}/m]
raise "completion path missing" unless finish
raise "completion must clear the current generation identity" unless finish.include?("continuedRegistration = nil")
raise "pending continued requests must cancel their unique identifier" unless finish.include?("taskRequestWithIdentifier: registration.identifier")
raise "every terminal completion must release a pending handoff" unless finish.include?("endApplicationProtection()")
raise "successful terminal progress must be truthful" unless finish.include?("totalUnitCount = 1") && finish.include?("completedUnitCount = 1")
raise "registration identity must compose its identifier from the token" unless registration_model.include?("identifier = identifierPrefix + token")
raise "old handlers must observe terminal completion before current identity" unless registration_model.index("if let completion") < registration_model.index("guard isCurrentRegistration")
raise "handler identity must include exact identifier equality" unless registration_model.include?("deliveredIdentifier == identifier")
raise "continued tasks must run their handler on the owner queue" unless coordinator.include?("using: DispatchQueue.main")
raise "continued tasks must not request background GPU" if coordinator.include?("requiredResources = .gpu")
RUBY

ruby - "$player_dir/OpenPencilPlayer.entitlements" <<'RUBY'
source = File.read(ARGV.fetch(0))
raise "Associated Domains entitlement missing" unless source.include?(
  "<key>com.apple.developer.associated-domains</key>"
)
raise "canonical OpenPencil applinks domain missing" unless source.include?(
  "<string>applinks:op.zseven.cn</string>"
)
raise "redirecting global domain must not be associated" if source.include?(
  "applinks:op.zseven.tech"
)
RUBY

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
create = source[/private func createAndAttach\b.*?(?=\n    \/\/\/ Registers every bundled)/m]
raise "iOS engine create path missing" unless create
prepare = create.index("let storageURL = AuthStorage.prepare()")
root = create.index("desc.storage_root_ptr")
call = create.index("return op_create(&desc, &created)")
raise "private storage must be prepared and bound before op_create" unless prepare && root && call && prepare < root && root < call
documents_prepare = create.index("DocumentStorage.prepare()")
documents_root = create.index("desc.documents_root_ptr")
raise "the visible documents root must be prepared and bound before op_create" unless documents_prepare && documents_root && documents_prepare < documents_root && documents_root < call
raise "auth must reuse the create-time storage root" unless create.include?("configureMobileAuth(engine: created, storageURL: storageURL)")
RUBY

ruby - "$player_dir/Sources/OpEngineHost.swift" \
  "$player_dir/Sources/DocumentExportCoordinator.swift" <<'RUBY'
host = File.read(ARGV.fetch(0))
coordinator = File.read(ARGV.fetch(1))

drain = host[/func drainShellActions\b.*?(?=\n    \/\/\/ Polls the engine)/m]
raise "iOS export shell-action branch missing" unless drain
action = drain.index("OpShellAction_ExportDocument.rawValue")
defer_to_uikit = drain.index("DispatchQueue.main.async", action || 0)
begin_export = drain.index("documentExportCoordinator.beginExport()", action || 0)
raise "export presentation must leave the editor ABI stack" unless action && defer_to_uikit && begin_export && action < defer_to_uikit && defer_to_uikit < begin_export

begin_method = coordinator[/func beginExport\(\).*?(?=\n    func cancelForTeardown)/m]
raise "iOS export coordinator missing" unless begin_method
filename = begin_method.index("copyExportFilename")
stage = begin_method.index("makeStagedFileURL")
write = begin_method.index("op_editor_export_to_path")
present = begin_method.index("presentDocumentPicker")
raise "export must name, stage, write, then present" unless filename && stage && write && present && filename < stage && stage < write && write < present

teardown = coordinator[/func cancelForTeardown\(\).*?(?=\n    private func copyExportFilename)/m]
raise "export teardown cleanup missing" unless teardown&.include?("cancelPendingEngineRequest()") && teardown.include?("cleanupStagingDirectory()")
raise "picker success cleanup missing" unless coordinator.include?("didPickDocumentsAt urls: [URL]") && coordinator.include?("finishPicker()")
raise "picker cancellation cleanup missing" unless coordinator.include?("documentPickerWasCancelled")
RUBY

ruby - "$player_dir/Sources/OpEngineHost.swift" \
  "$player_dir/Sources/DocumentSaveCoordinator.swift" <<'RUBY'
host = File.read(ARGV.fetch(0))
coordinator = File.read(ARGV.fetch(1))

drain = host[/func drainShellActions\b.*?(?=\n    \/\/\/ Polls the engine)/m]
raise "iOS save shell-action branch missing" unless drain
action = drain.index("OpShellAction_SaveDocument.rawValue")
defer_to_uikit = drain.index("DispatchQueue.main.async", action || 0)
begin_save = drain.index("documentSaveCoordinator.beginSave()", action || 0)
raise "save presentation must leave the editor ABI stack" unless action && defer_to_uikit && begin_save && action < defer_to_uikit && defer_to_uikit < begin_save
raise "save teardown must cancel the pending engine request" unless host.include?("documentSaveCoordinator.cancelForTeardown()")

begin_method = coordinator[/func beginSave\(\).*?(?=\n    \/\/\/ Teardown)/m]
raise "iOS save coordinator missing" unless begin_method
filename = begin_method.index("copySaveFilename")
stage_url = begin_method.index("makeStagedFileURL")
write = begin_method.index("op_editor_stage_save_to_path")
target = begin_method.index("copySaveTarget")
rewrite = begin_method.index("rewrite(staged:")
present = begin_method.index("presentPicker(for: staged)")
raise "save must name, stage, then write the canonical bytes" unless filename && stage_url && write && filename < stage_url && stage_url < write
raise "a bound destination must be rewritten before the picker is considered" unless target && rewrite && present && write < target && target < rewrite && rewrite < present

# Only a reported destination write may mark the document saved, and the
# engine must always be told how a round trip ended.
raise "save commit missing" unless coordinator.include?("op_editor_commit_save")
raise "save cancellation missing" unless coordinator.include?("op_editor_cancel_save")
raise "picker cancellation must release the engine's pending save" unless coordinator[/func documentPickerWasCancelled.*?\n    \}/m]&.include?("cancelPendingEngineRequest(failed: false)")
raise "a bookmark that cannot be made must not mark the document saved" unless coordinator[/private func bind\(pickedURL.*?(?=\n    \/\/ MARK:)/m]&.include?("cancelPendingEngineRequest(failed: true)")
RUBY

ruby - "$player_dir/Sources/OpEngineHost.swift" \
  "$player_dir/Sources/ImageImportCoordinator.swift" <<'RUBY'
host = File.read(ARGV.fetch(0))
coordinator = File.read(ARGV.fetch(1))

drain = host[/func drainShellActions\b.*?(?=\n    \/\/\/ Polls the engine)/m]
raise "iOS image-import shell-action branch missing" unless drain
action = drain.index("OpShellAction_ImportImageOrSvg.rawValue")
defer_to_uikit = drain.index("DispatchQueue.main.async", action || 0)
begin_import = drain.index("imageImportCoordinator.beginImport()", action || 0)
unless action && defer_to_uikit && begin_import && action < defer_to_uikit && defer_to_uikit < begin_import
  raise "image picker presentation must leave the editor ABI stack"
end
raise "image-import teardown missing" unless host.include?("imageImportCoordinator.cancelForTeardown()")

begin_method = coordinator[/func beginImport\(\).*?(?=\n    \/\/\/ Teardown)/m]
raise "image-import coordinator missing" unless begin_method
%w[png jpeg gif webp svg].each do |kind|
  raise "#{kind} picker type missing" unless begin_method.downcase.include?(kind)
end
raise "image picker must be single selection" unless begin_method.include?("allowsMultipleSelection = false")
raise "image picker must be a bounded form sheet" unless begin_method.include?("modalPresentationStyle = .formSheet")

read = coordinator.index("BoundedDocumentReader.read")
return_bytes = coordinator.index("op_editor_import_image_or_svg")
raise "bounded read must finish before bytes cross the ABI" unless read && return_bytes && read < return_bytes
raise "picker cancellation must retire UIKit ownership" unless coordinator[/func documentPickerWasCancelled.*?\n    \}/m]&.include?("finishPicker()")
teardown = coordinator[/func cancelForTeardown\(\).*?(?=\n    private func readPickedFile)/m]
raise "image-import teardown must invalidate worker completion" unless teardown&.include?("activeReadToken = nil")
raise "image-import teardown must detach and dismiss the picker" unless teardown&.include?("picker.delegate = nil") && teardown.include?("picker.dismiss(animated: false)")
raise "collaboration rejection must rely on the engine notice" unless coordinator.include?("status != OpStatus_Busy")
RUBY

sdk="$(xcrun --sdk iphonesimulator --show-sdk-path)"
target="arm64-apple-ios15.0-simulator"
module_cache="${TMPDIR:-/tmp}/op-ios-module-cache"
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

ime_selection_test="$reader_test_dir/ime-selection-offsets-runner"
xcrun swiftc \
  -warnings-as-errors \
  -parse-as-library \
  "$player_dir/Sources/ImeSelectionOffsets.swift" \
  "$player_dir/Tests/ImeSelectionOffsetsTests.swift" \
  -o "$ime_selection_test"
"$ime_selection_test"

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

device_login_test="$reader_test_dir/device-login-request-runner"
xcrun swiftc \
  -warnings-as-errors \
  -parse-as-library \
  "$player_dir/Sources/DeviceLoginRequestInfo.swift" \
  "$player_dir/Sources/SsoProviderList.swift" \
  "$player_dir/Tests/DeviceLoginRequestInfoTests.swift" \
  -o "$device_login_test"
"$device_login_test"

save_binding_test="$reader_test_dir/document-save-binding-runner"
xcrun swiftc \
  -warnings-as-errors \
  -parse-as-library \
  "$player_dir/Sources/DocumentSaveBinding.swift" \
  "$player_dir/Tests/DocumentSaveBindingTests.swift" \
  -o "$save_binding_test"
"$save_binding_test"

universal_link_test="$reader_test_dir/universal-link-runner"
xcrun swiftc \
  -warnings-as-errors \
  -parse-as-library \
  "$player_dir/Sources/UniversalLink.swift" \
  "$player_dir/Tests/UniversalLinkTests.swift" \
  -o "$universal_link_test"
"$universal_link_test"

background_state_test="$reader_test_dir/generation-background-state-runner"
xcrun swiftc \
  -warnings-as-errors \
  -parse-as-library \
  "$player_dir/Sources/GenerationBackgroundState.swift" \
  "$player_dir/Tests/GenerationBackgroundStateTests.swift" \
  -o "$background_state_test"
"$background_state_test"

background_registration_test="$reader_test_dir/generation-background-registration-runner"
xcrun swiftc \
  -warnings-as-errors \
  -parse-as-library \
  "$player_dir/Sources/GenerationBackgroundRegistration.swift" \
  "$player_dir/Tests/GenerationBackgroundRegistrationTests.swift" \
  -o "$background_registration_test"
"$background_registration_test"

ruby "$player_dir/Tests/NativeLoginLifecycleTests.rb" \
  "$player_dir/Sources/OpPlayerView+Login.swift" \
  "$player_dir/Sources/NativeLoginViewController.swift" \
  "$player_dir/Sources/AuthStorage.swift" \
  "$player_dir/Sources/SsoRegion.swift" \
  "$header_dir/op_engine.h"

echo "iOS Player sources and ABI imports validate"
