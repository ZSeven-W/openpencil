# frozen_string_literal: true

# Source contracts for the HarmonyOS project skeleton: identity, device
# coverage, the XComponent <-> native-library binding, the lifecycle-only
# render contract, and the resource wiring. DevEco/hvigor cannot run on this
# host, so these checks are the local build gate.

require "json"

player_dir = File.expand_path("..", __dir__)
repo_dir = File.expand_path("../..", player_dir)
ets_dir = File.join(player_dir, "entry/src/main/ets")

def parse_json5(path)
  raw = File.read(path)
  # Strip whole-line // comments and trailing commas, then parse as JSON.
  stripped = raw.lines.reject { |line| line.strip.start_with?("//") }.join
  stripped = stripped.gsub(/,(\s*[}\]])/, '\1')
  JSON.parse(stripped)
end

def read(*parts)
  File.read(File.join(*parts))
end

# ---- Identity --------------------------------------------------------------

app = parse_json5(File.join(player_dir, "AppScope/app.json5"))
raise "bundleName must be tech.zseven.openpencil" unless app["app"]["bundleName"] == "tech.zseven.openpencil"
raise "app label must come from the shared string resource" unless app["app"]["label"] == "$string:app_name"
raise "app icon must be declared" unless app["app"]["icon"] == "$media:app_icon"
raise "HarmonyOS 5 floor is API 12" unless app["app"]["minAPIVersion"] == 12

app_strings = JSON.parse(read(player_dir, "AppScope/resources/base/element/string.json"))
app_name = app_strings["string"].find { |entry| entry["name"] == "app_name" }
raise "app_name must be exactly OpenPencil" unless app_name && app_name["value"] == "OpenPencil"

icon = File.binread(File.join(player_dir, "AppScope/resources/base/media/app_icon.png"))
raise "app icon must be a PNG" unless icon.start_with?("\x89PNG\r\n\x1a\n".b)

# ---- Module: one app for phone + tablet + 2in1 -----------------------------

module_json = parse_json5(File.join(player_dir, "entry/src/main/module.json5"))["module"]
raise "deviceTypes drifted" unless module_json["deviceTypes"] == %w[phone tablet 2in1]
raise "entry module type must be entry" unless module_json["type"] == "entry"
raise "mainElement must be EntryAbility" unless module_json["mainElement"] == "EntryAbility"

ability = module_json["abilities"].first
raise "EntryAbility missing" unless ability && ability["name"] == "EntryAbility"
raise "ability entry source drifted" unless ability["srcEntry"] == "./ets/entryability/EntryAbility.ets"
raise "ability must be exported as the launcher entry" unless ability["exported"] == true
# Android declares no screenOrientation and handles the config change itself:
# the window follows the user's rotation setting on every device type.
raise "orientation must stay unspecified" unless ability["orientation"] == "unspecified"
raise "launcher skill missing" unless ability["skills"].any? { |skill|
  skill["entities"].include?("entity.system.home") && skill["actions"].include?("action.system.home")
}
raise "INTERNET permission must be requested for remote images" unless module_json["requestPermissions"].any? { |permission|
  permission["name"] == "ohos.permission.INTERNET"
}

pages = JSON.parse(read(player_dir, "entry/src/main/resources/base/profile/main_pages.json"))
raise "pages profile must route to pages/Index" unless pages["src"] == ["pages/Index"]

# ---- Build profile ---------------------------------------------------------

build_profile = parse_json5(File.join(player_dir, "build-profile.json5"))
product = build_profile["app"]["products"].first
raise "compatibleSdkVersion must target API 12" unless product["compatibleSdkVersion"] == "5.0.0(12)"
raise "runtimeOS must be HarmonyOS" unless product["runtimeOS"] == "HarmonyOS"
raise "signing must stay an unchecked-in placeholder" unless build_profile["app"]["signingConfigs"].empty?
raise "entry module must be registered" unless build_profile["modules"].any? { |mod| mod["name"] == "entry" }
raise "app hvigor task set drifted" unless read(player_dir, "hvigorfile.ts").include?("appTasks")
raise "entry hvigor task set drifted" unless read(player_dir, "entry/hvigorfile.ts").include?("hapTasks")

entry_package = parse_json5(File.join(player_dir, "entry/oh-package.json5"))
raise "the native module type package must be wired" unless entry_package["dependencies"]["libopenpencil.so"] ==
  "file:./src/main/cpp/types/libopenpencil"

# ---- XComponent <-> libopenpencil.so binding -------------------------------

index = read(ets_dir, "pages/Index.ets")
raise "the surface must be an XComponent" unless index.include?("XComponent({")
raise "XComponent must bind libopenpencil.so by library name" unless index.include?("libraryname: 'openpencil'")
raise "XComponent must be a surface type" unless index.include?("type: XComponentType.SURFACE")
raise "the XComponent id must be a single shared constant" unless index.include?("const SURFACE_ID = 'openpencil_surface'") &&
  index.include?("id: SURFACE_ID")
raise "the engine must be created against the same surface id" unless index.match?(
  /this\.host\.create\(\s*SURFACE_ID/m,
)
raise "surface lifecycle must drive attach" unless index.match?(/\.onLoad\(.*?this\.host\.attach\(\)/m)
raise "surface teardown must suspend before the platform reclaims it" unless index.match?(
  /\.onDestroy\(.*?this\.host\.suspend\(\)/m,
)
raise "page teardown must destroy the engine" unless index.include?("this.host.destroy()")

# ArkTS owns a COALESCING vsync-aligned frame pump: displaySync ticks paint
# at most one frame per refresh, `framePending` merges every redraw request
# (engine callbacks and raw input), and the engine's timed wakes ride the
# same tick. Verified on the HarmonyOS 6 emulator: a per-event immediate
# paint saturates the main thread and drops frames.
engine_host = read(ets_dir, "common/EngineHost.ets")
raise "frame pump must be vsync-aligned" unless engine_host.include?("displaySync.create()")
raise "frame pump must coalesce redraw requests" unless engine_host.include?("framePending")
raise "engine timed wakes must ride the pump" unless engine_host.include?("nextWakeAtMs")
frame_calls = Dir.glob(File.join(ets_dir, "**/*.ets")).sum do |source|
  File.read(source).scan(/napi\.frame\s*\(/).length
end
raise "exactly one frame call site (the pump tick), found #{frame_calls}" unless frame_calls == 1

# ---- Input coverage: touch, mouse, keyboard, focus -------------------------

raise "touch input must be forwarded" unless index.include?(".onTouch(")
raise "mouse input must be forwarded for 2in1/PC" unless index.include?(".onMouse(")
raise "hover must be handled for 2in1/PC" unless index.include?(".onHover(")
raise "the surface must take focus for keyboard input" unless index.include?(".focusable(true)") &&
  index.include?(".defaultFocus(true)")
# A SURFACE XComponent consumes hardware keys on the NATIVE channel, so ArkTS
# `onKeyEvent` never fires while it holds focus: the key path lives in
# `xcomponent.rs`. A second ArkTS forwarder would double every keystroke.
xcomponent = read(repo_dir, "crates/op-engine-napi/src/xcomponent.rs")
raise "physical keys must be forwarded on the native channel" unless xcomponent.include?(
  "OH_NativeXComponent_RegisterKeyEventCallback",
)
raise "the native key path must carry the live modifier bitmask" unless xcomponent.include?(
  "MODIFIERS.load(Ordering::Relaxed)",
)
raise "ArkTS must not forward keys as well" if Dir.glob(File.join(ets_dir, "**/*.ets")).any? { |source|
  File.read(source).include?(".onKeyEvent(")
}
# Typing into an engine input on 2in1: no soft keyboard ever appears, so the
# native channel injects printable characters itself while the IME conduit is
# detached (`setImeConduitAttached`).
raise "printable keys must reach the engine as text" unless xcomponent.include?(
  "crate::action::printable_char(code, modifiers)",
)
raise "native text injection must be gated on engine IME focus" unless xcomponent.include?(
  "if ime && !conduit {",
)
raise "the shell must report its IME conduit state to native" unless engine_host.include?(
  "napi.setImeConduitAttached(false)",
)

pointer_router = read(ets_dir, "common/PointerRouter.ets")
raise "the shell must forward raw pointers, not interpret gestures" unless pointer_router.include?(
  "Gestures are interpreted BY THE ENGINE",
)
raise "two-finger takeover must cancel the press ladder first" unless pointer_router.match?(
  /this\.active\.size === 2.*?editorCancelGesture.*?editorBeginTransform/m,
)
raise "pinch must reuse the shared wheel-delta conversion" unless pointer_router.include?(
  "PinchZoomDelta.wheelDelta(this.lastPinchDistance, distance)",
)
raise "a cancelled stream must never run the release ladder" unless pointer_router.include?(
  "!this.longPressFired && !this.releaseSuppressed",
)

# ---- Safe area / viewport --------------------------------------------------

window_bridge = read(ets_dir, "common/WindowBridge.ets")
raise "the window must be laid out full screen" unless window_bridge.include?("setWindowLayoutFullScreen(true)")
raise "system bars must stay visible and transparent" unless window_bridge.include?("setWindowSystemBarEnable(['status', 'navigation'])") &&
  window_bridge.include?("statusBarColor: '#00000000'")
raise "the safe area must union system bars, cutout, and the indicator" unless window_bridge.include?("AvoidAreaType.TYPE_SYSTEM") &&
  window_bridge.include?("AvoidAreaType.TYPE_CUTOUT") &&
  window_bridge.include?("AvoidAreaType.TYPE_NAVIGATION_INDICATOR")
raise "the keyboard must stay a separate occlusion channel" unless window_bridge.include?("AvoidAreaType.TYPE_KEYBOARD")
raise "window resizes must republish the viewport" unless window_bridge.include?("'windowSizeChange'")
raise "avoid-area changes must republish the viewport" unless window_bridge.include?("'avoidAreaChange'")
raise "the ability must attach the window bridge" unless read(ets_dir, "entryability/EntryAbility.ets").include?(
  "WindowBridge.shared.attach(mainWindow)",
)
raise "bounds, density, and insets must be published atomically" unless engine_host.include?("napi.resizeWithSafeArea(")
raise "insets must not be sent independently of bounds" if engine_host.include?("napi.setSafeArea(")
raise "the IME must not resize the editor viewport" unless engine_host.include?("napi.setKeyboard(")
raise "a geometry transition must cancel live gestures" unless engine_host.match?(
  /beginGeometryUpdate\(\).*?this\.pointerRouter\.cancelAll\(\)/m,
)

# ---- Resources -------------------------------------------------------------

base_strings = JSON.parse(read(player_dir, "entry/src/main/resources/base/element/string.json"))["string"]
en_strings = JSON.parse(read(player_dir, "entry/src/main/resources/en_US/element/string.json"))["string"]
zh_strings = JSON.parse(read(player_dir, "entry/src/main/resources/zh_CN/element/string.json"))["string"]
base_keys = base_strings.map { |entry| entry["name"] }.sort
raise "en_US strings must mirror the base key set" unless en_strings.map { |entry| entry["name"] }.sort == base_keys
raise "zh-CN strings must mirror the base key set" unless zh_strings.map { |entry| entry["name"] }.sort == base_keys
raise "zh-CN strings must actually be translated" unless zh_strings.any? { |entry| entry["value"].match?(/\p{Han}/) }

referenced = Dir.glob(File.join(ets_dir, "**/*.ets")).flat_map do |source|
  File.read(source).scan(/\$r\('app\.string\.([A-Za-z0-9_]+)'\)/).flatten
end
missing = (referenced.uniq - base_keys).sort
raise "unresolved string resources: #{missing.join(', ')}" unless missing.empty?

# module.json5 may also reference AppScope-level strings (app_name).
app_scope_keys = app_strings["string"].map { |entry| entry["name"] }
module_referenced = File.read(File.join(player_dir, "entry/src/main/module.json5")).scan(/\$string:([A-Za-z0-9_]+)/).flatten
module_missing = (module_referenced.uniq - base_keys - app_scope_keys).sort
raise "unresolved module string resources: #{module_missing.join(', ')}" unless module_missing.empty?

# Media: every `$r('app.media.*')` must resolve to a real PNG, and the SSO
# brand marks must stay in step with the Android shell's drawable set.
media_dir = File.join(player_dir, "entry/src/main/resources/base/media")
media_keys = Dir.glob(File.join(media_dir, "*")).map { |path| File.basename(path, ".*") }
media_referenced = Dir.glob(File.join(ets_dir, "**/*.ets")).flat_map do |source|
  File.read(source).scan(/\$r\('app\.media\.([A-Za-z0-9_]+)'\)/).flatten
end
media_missing = (media_referenced.uniq - media_keys).sort
raise "unresolved media resources: #{media_missing.join(', ')}" unless media_missing.empty?

android_drawable = File.join(repo_dir, "packaging/android/app/src/main/res/drawable-nodpi")
if File.directory?(android_drawable)
  providers = Dir.glob(File.join(android_drawable, "provider_*.png")).map { |path| File.basename(path) }
  (providers + ["zseven_logo.png"]).each do |asset|
    path = File.join(media_dir, asset)
    raise "SSO asset #{asset} missing from the HarmonyOS media set" unless File.exist?(path)
    raise "SSO asset #{asset} must be a PNG" unless File.binread(path).start_with?("\x89PNG\r\n\x1a\n".b)
  end
end

colors = JSON.parse(read(player_dir, "entry/src/main/resources/base/element/color.json"))["color"]
raise "start window background color missing" unless colors.any? { |entry| entry["name"] == "start_window_background" }
raise "entry media icon must be a PNG" unless File.binread(
  File.join(player_dir, "entry/src/main/resources/base/media/app_icon.png"),
).start_with?("\x89PNG\r\n\x1a\n".b)

# ---- House rules -----------------------------------------------------------

Dir.glob(File.join(ets_dir, "**/*.ets")).each do |source|
  lines = File.read(source).lines.length
  raise "#{File.basename(source)} exceeds the 800-line cap (#{lines})" if lines > 800
end

readme = read(player_dir, "README.md")
raise "README must document where the engine library lands" unless readme.include?("entry/libs/arm64-v8a/")
raise "README must point at the Rust OHOS build script" unless readme.include?("scripts/build-ohos.sh")
raise "README must document the signing placeholder" unless readme.match?(/signing/i)
raise "README must carry a LIMITATIONS section" unless readme.include?("## Limitations")

puts "HarmonyOS project contract validates"
