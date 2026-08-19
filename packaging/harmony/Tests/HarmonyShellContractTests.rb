# frozen_string_literal: true

# Source contracts for the HarmonyOS shell behaviour: the NAPI surface, the
# seven engine shell actions, the empty-conduit backspace fix, the 15-locale
# picker, the export format set, and SSO origin hygiene.

player_dir = File.expand_path("..", __dir__)
repo_dir = File.expand_path("../..", player_dir)
ets_dir = File.join(player_dir, "entry/src/main/ets")
android_dir = File.join(repo_dir, "packaging/android/app/src/main/kotlin/tech/zseven/openpencil")

def read(*parts)
  File.read(File.join(*parts))
end

ets_sources = Dir.glob(File.join(ets_dir, "**/*.ets")).sort
raise "no ArkTS sources found" if ets_sources.empty?

op_native = read(ets_dir, "common/OpNative.ets")
engine_host = read(ets_dir, "common/EngineHost.ets")
ime = read(ets_dir, "common/ImeConduit.ets")
index = read(ets_dir, "pages/Index.ets")
export_support = read(ets_dir, "common/DocumentExportSupport.ets")
document_shell = read(ets_dir, "common/DocumentShell.ets")
language = read(ets_dir, "common/EngineLanguage.ets")
declaration = read(player_dir, "entry/src/main/cpp/types/libopenpencil/index.d.ts")
header = read(repo_dir, "crates/op-engine-ffi/include/op_engine.h")

# ---- NAPI surface: OpNative.kt externals, camelCase, no `native` prefix ----

exported = declaration.scan(/^export const ([A-Za-z0-9_]+):/).flatten.sort
raise "the NAPI declaration exports nothing" if exported.empty?
raise "the surface must be bound as libopenpencil.so" unless op_native.include?("from 'libopenpencil.so'")

if File.exist?(File.join(android_dir, "OpNative.kt"))
  kotlin = read(android_dir, "OpNative.kt")
  expected = kotlin.scan(/external fun native([A-Za-z0-9_]+)\(/).flatten.map do |name|
    name[0].downcase + name[1..]
  end.sort
  # OHOS-only entry points have no JNI twin: the XComponent surface model and
  # the 2in1 / PC form factor (mouse, wheel, hardware keys, desktop chrome)
  # simply do not exist on the Kotlin player. Everything else must match.
  ohos_only = %w[editorHover editorSetTouchChrome editorWheel keyEvent keyModifiers
                 setImeConduitAttached]
  missing = (expected - exported).sort
  extra = (exported - expected - ohos_only).sort
  raise "NAPI surface is missing JNI functions: #{missing.join(', ')}" unless missing.empty?
  raise "NAPI surface has functions the JNI bridge lacks: #{extra.join(', ')}" unless extra.empty?
  stale = (ohos_only - exported).sort
  raise "OHOS-only allowlist names nothing declared: #{stale.join(', ')}" unless stale.empty?
end

# The Surface argument becomes the XComponent id on OHOS.
raise "attachSurface must take the XComponent id" unless declaration.match?(
  /attachSurface: \(engine: number, xcomponentId: string\)/,
)
raise "resume must take the XComponent id" unless declaration.match?(
  /resume: \(engine: number, xcomponentId: string \| null\)/,
)

called = ets_sources.flat_map { |source| File.read(source).scan(/\bnapi\.([A-Za-z0-9_]+)\s*\(/).flatten }.uniq.sort
unknown = (called - exported).sort
raise "ArkTS calls undeclared NAPI functions: #{unknown.join(', ')}" unless unknown.empty?

# ---- Shell actions 1-7 -----------------------------------------------------

action_codes = {
  "OPEN_DOCUMENT" => ["OpShellAction_OpenDocument = 1", 1],
  "OPEN_LOGIN_WEBVIEW" => ["OpShellAction_OpenLoginWebView = 2", 2],
  "CLOSE_LOGIN_WEBVIEW" => ["OpShellAction_CloseLoginWebView = 3", 3],
  "EXPORT_DOCUMENT" => ["OpShellAction_ExportDocument = 4", 4],
  "OPEN_ACCOUNT_CENTER" => ["OpShellAction_OpenAccountCenter = 5", 5],
  "REQUEST_LOGIN" => ["OpShellAction_RequestLogin = 6", 6],
  "OPEN_LANGUAGE_PICKER" => ["OpShellAction_OpenLanguagePicker = 7", 7],
  # The TopBar's painted traffic lights; desktop-class shells only.
  "WINDOW_CLOSE" => ["OpShellAction_WindowClose = 8", 8],
  "WINDOW_MINIMIZE" => ["OpShellAction_WindowMinimize = 9", 9],
  "WINDOW_ZOOM" => ["OpShellAction_WindowZoom = 10", 10],
}.freeze

action_codes.each do |name, (header_line, code)|
  raise "header shell action drifted: #{header_line}" unless header.include?(header_line)
  raise "OpShellAction.#{name} must be pinned to #{code}" unless op_native.match?(
    /#{name} = #{code},/,
  )
  raise "shell action #{name} must be handled" unless engine_host.include?("case OpShellAction.#{name}:")
end
raise "OpShellAction.NONE must be 0" unless op_native.include?("NONE = 0,")
raise "the drain must consume actions until the queue is empty" unless engine_host.include?(
  "napi.editorTakeShellAction(this.engine)",
)
raise "an unknown action must be logged, not silently dropped" unless engine_host.include?(
  "unknown editor shell action",
)

# Every action reaches a main-thread sink handler.
%w[
  onOpenDocument onExportDocument onOpenAccountCenter onRequestLogin
  onOpenLanguagePicker onOpenLoginUi onCloseLoginUi onWindowControl
].each do |handler|
  raise "sink handler #{handler} missing from EngineHost" unless engine_host.include?(handler)
  raise "sink handler #{handler} missing from the page" unless index.include?(handler)
end

# Action 1 / 4: the pickers.
raise "OpenDocument must use DocumentViewPicker" unless document_shell.include?("new picker.DocumentViewPicker(context)")
raise "OpenDocument must filter .op/.pen" unless document_shell.include?("options.fileSuffixFilters = ['.op', '.pen']")
raise "ExportDocument must use the save picker" unless document_shell.include?("new picker.DocumentSaveOptions()")
raise "ExportDocument must offer the engine-derived name" unless document_shell.include?("options.newFileNames = [filename]")
raise "a cancelled save must discard the frozen export" unless document_shell.match?(
  /result\.length === 0.*?host\.cancelExport\(\)/m,
)

# Action 2 / 6: the login URL comes only from native, and refusing the flow
# cancels it through the same call the Android shell uses on rejection.
raise "login URL must come from native" unless engine_host.include?("napi.editorTakeLoginUrl(this.engine)")
raise "a URL-less login action must cancel the flow" unless engine_host.match?(
  /login action had no URL.*?napi\.editorCancelLogin\(this\.engine\)/m,
)
raise "cancelLogin must go through the engine" unless engine_host.include?("napi.editorCancelLogin(this.engine)")
raise "a build without an auth backend must still say so" unless index.include?(
  "$r('app.string.native_login_unavailable')",
)
raise "an unavailable sign-in must cancel the engine flow" unless index.match?(
  /native_login_unavailable.*?this\.host\.cancelLogin\(\)/m,
)
# Action 3: an engine-terminal close dismisses without cancelling again.
raise "CloseLoginWebView must dismiss the login UI" unless index.match?(
  /onCloseLoginUi.*?this\.showLogin = false/m,
)
raise "an engine-terminal close must not cancel" if index.match?(
  /onCloseLoginUi[^}]*cancelLogin/m,
)
# No WebView anywhere: HarmonyOS never opens the retired login web view.
ets_sources.each do |source|
  if File.read(source).include?("@ohos.web.webview") || File.read(source).include?("Web({")
    raise "#{File.basename(source)} must keep WebView out of the login path"
  end
end

# ---- Window controls: the engine's traffic lights own the window -----------

window_bridge = read(ets_dir, "common/WindowBridge.ets")
raise "minimize must reach the platform window" unless window_bridge.include?("win.minimize()")
raise "zoom must toggle maximise and restore" unless window_bridge.match?(
  /expanded \? win\.recover\(\) : win\.maximize\(\)/,
)
# A layout-full-screen window reports FULL_SCREEN once maximised, so a
# MAXIMIZE-only test makes the green dot a one-way trip.
raise "the expanded test must accept FULL_SCREEN too" unless window_bridge.include?(
  "status === window.WindowStatusType.FULL_SCREEN",
)
raise "close must terminate the ability, not just hide the window" unless index.include?(
  "this.context.terminateSelf()",
)
# The system's floating title buttons overlap the engine's own right-hand
# cluster once the decor bar is hidden, so they are retired outright.
raise "the system title buttons must be hidden on 2in1" unless read(
  ets_dir, "entryability/EntryAbility.ets"
).include?("WindowBridge.shared.hideTitleButtons(mainWindow)")
raise "hiding must go through setWindowTitleButtonVisible" unless window_bridge.include?(
  "setWindowTitleButtonVisible(false, false, false)",
)
raise "any residual button width must be measurable" unless window_bridge.include?(
  "win.getTitleButtonRect()",
)

# ---- IME proxy: composition + key arbitration on 2in1 ----------------------
#
# `inputMethod.attach` is REFUSED for a SURFACE XComponent (12800009 on the
# HarmonyOS 6 PC emulator), so the shell gives the input framework a real
# editable component to serve and relays the result to the engine.

proxy = read(ets_dir, "common/ImeProxyBridge.ets")
raise "the page must host a focusable IME proxy" unless index.include?("TextInput({ controller: this.imeProxyController })") &&
  index.include?(".id(IME_PROXY_ID)")
raise "the proxy must be invisible, not hidden" unless index.include?(".opacity(0)") &&
  index.include?(".width(1)")
raise "engine text focus must move ArkUI focus to the proxy" unless index.match?(
  /onImeFocusChanged.*?focusControl\.requestFocus\(IME_PROXY_ID\)/m,
)
raise "losing engine focus must return focus to the surface" unless index.match?(
  /onImeFocusChanged.*?focusControl\.requestFocus\(SURFACE_ID\)/m,
)
raise "committed IME text must reach the engine" unless proxy.include?("napi.editorText(engine, added)")
raise "a composition commit must replace the preedit" unless proxy.include?(
  "napi.editorImeCommit(engine, added)",
)
raise "a live composition must reach the engine as preedit" unless proxy.include?(
  "napi.editorImePreedit(engine, previewText, caret, caret)",
)
# KEY ARBITRATION. Both halves are load-bearing and were each observed broken:
# the TextInput swallows Backspace (so `onKeyEvent` never sees it), and
# claiming Backspace during composition makes pinyin uncorrectable.
raise "keys must be taken before the IME, not after" unless index.include?(".onKeyPreIme(")
raise "ArkTS must not double-handle keys through onKeyEvent" if index.match?(
  /IME_PROXY_ID[\s\S]{0,900}?\.onKeyEvent\(/m,
)
raise "a composing IME must keep every key" unless proxy.match?(
  /if \(this\.composing\) \{\s*\n\s*return false;/m,
)
raise "a printable key must start the composition window" unless proxy.include?(
  "this.composing = true;",
)
raise "any IME report must end the composition window" unless proxy.include?(
  "this.composing = false;",
)
raise "bare modifiers must not arm the composition window" unless proxy.include?(
  "ImeProxyBridge.isModifierKey(event.keyCode)",
)
raise "the editor key table must stay single-sourced in the engine" unless proxy.include?(
  "napi.keyEvent(engine, event.keyCode, modifiers)",
)
raise "preedit offsets must be converted to bytes" unless proxy.include?(
  "ImeProxyBridge.utf8Length(previewText, previewText.length)",
)

# ---- IME conduit: the empty-conduit backspace contract ---------------------

raise "the conduit must use the system input method" unless ime.include?("inputMethod.getController()")
raise "the conduit must attach/detach with engine IME focus" unless ime.include?("this.controller.attach(this.showKeyboard,") &&
  ime.include?("this.controller.detach()")
# Composition is the whole reason the conduit still exists on 2in1: the
# native key channel injects a US-QWERTY table with no preedit, so pinyin
# can only reach the engine through these two events.
raise "the conduit must forward composition preedit" unless ime.match?(
  /'setPreviewText'.*?napi\.editorImePreedit\(engine, text, caret, caret\)/m,
)
raise "an abandoned composition must be cleared" unless ime.match?(
  /'finishTextPreview'.*?napi\.editorImePreedit\(engine, '', 0, 0\)/m,
)
raise "a commit must replace the composition, not append to it" unless ime.match?(
  /this\.previewActive.*?napi\.editorImeCommit\(engine, text\)/m,
)
raise "conduit preedit offsets must be converted to bytes" unless ime.include?(
  "ImeConduit.utf8Length(text, range.end)",
)
# PROVE-THEN-SUPPRESS: an `attach` that resolves against an inert IME must
# not disable the only text path that works on this form factor.
raise "native text must stay on until the IME delivers" if ime.match?(
  /this\.attached = true;\s*\n(?:\s*(?:\/\/[^\n]*)?\n)*\s*napi\.setImeConduitAttached\(true\)/m,
)
raise "the conduit must suppress native text only once proven" unless ime.match?(
  /private markProven\([\s\S]{0,500}?napi\.setImeConduitAttached\(true\);/m,
)
raise "there must be exactly one place that suppresses the native path" unless
  ime.scan("napi.setImeConduitAttached(true)").length == 1
raise "the empty-conduit backspace contract must be documented" unless ime.include?(
  "EMPTY-CONDUIT BACKSPACE CONTRACT",
)
raise "backspace must be forwarded from deleteLeft itself" unless ime.match?(
  /'deleteLeft'.*?napi\.editorKey\(engine, OpKey\.BACKSPACE\)/m,
)
raise "forward delete must be forwarded from deleteRight" unless ime.match?(
  /'deleteRight'.*?napi\.editorKey\(engine, OpKey\.DELETE\)/m,
)
# The conduit holds NO text, so nothing may gate the key on a local buffer.
raise "the conduit must not keep a local text buffer" if ime.match?(/private\s+\w*[Bb]uffer\w*\s*:/)
delete_left_block = ime[/this\.controller\.on\('deleteLeft'.*?\n    \}\);/m]
raise "deleteLeft handler not found" if delete_left_block.nil?
raise "deleteLeft must not gate backspace on emptiness" if delete_left_block.match?(
  /length\s*(===|==)\s*0|isEmpty|text\.length/,
)
raise "surrounding text must be reported so IMEs still emit deleteLeft" unless ime.match?(
  /'getLeftTextOfCursor'.*?ImeConduit\.placeholder\(length\)/m,
)
raise "the engine must own IME focus, not the shell" unless engine_host.include?(
  "napi.editorImeFocused(this.engine)",
)
# Physical keys must not double-type while the IME conduit is attached. The
# rule moved into the native key channel (`xcomponent.rs`), so the conduit's
# only job is to report its own attachment state.
raise "detaching must release the native text path" unless ime.include?(
  "napi.setImeConduitAttached(false)",
)
# The conduit attaches on every form factor; only the soft-keyboard request
# differs, because a 2in1 has no on-screen keyboard to raise.
raise "a desktop-class shell must suppress the soft keyboard" unless engine_host.include?(
  "this.ime.setShowKeyboard(false)",
)
raise "IME focus must drive attach on every form factor" unless engine_host.match?(
  /this\.imeFocused = focused;.*?if \(focused\) \{\s*\n\s*this\.ime\.show\(\);/m,
)

# ---- Locale picker: exactly the engine's 15 locales ------------------------

codes = language.scan(/new EngineLocale\('([^']+)', '([^']+)'\)/)
raise "the locale table must have exactly 15 entries (found #{codes.length})" unless codes.length == 15
expected_codes = %w[en-US zh-CN zh-TW ja ko fr es de pt ru hi tr th vi id]
raise "locale codes drifted from op_i18n::Locale::ALL" unless codes.map(&:first) == expected_codes

if File.exist?(File.join(android_dir, "EngineLanguage.kt"))
  android_codes = read(android_dir, "EngineLanguage.kt").scan(/"([a-zA-Z-]+)" to "([^"]+)"/)
  raise "HarmonyOS locale codes must match Android" unless codes.map(&:first) == android_codes.map(&:first)
  raise "HarmonyOS locale names must match Android" unless codes.map(&:last) == android_codes.map(&:last)
end

raise "the picker must apply the choice through the engine" unless index.include?("this.host.applyLocale(code)")
raise "the choice must be persisted" unless index.include?("EngineLanguage.savePreference(this.context, code)")
raise "the persisted locale must re-apply after engine create" unless index.match?(
  /this\.engineStarted = true;\s*\n\s*this\.restorePersistedLocale\(\);/m,
)
raise "the locale must be persisted with Preferences" unless language.include?("preferences.getPreferences(context, PREFS)")

# ---- Export formats: PNG / JPEG / SVG / PDF, never WebP --------------------

raise "export suffixes drifted" unless export_support.include?(
  "static readonly SAVE_SUFFIXES: string[] = ['.png', '.jpg', '.jpeg', '.svg', '.pdf'];",
)
%w[image/png image/jpeg image/svg+xml application/pdf].each do |mime|
  raise "export MIME #{mime} missing" unless export_support.include?(mime)
end
# No code path may offer WebP; only prose may mention why it is absent.
ets_sources.each do |source|
  if File.read(source).match?(%r{\.webp|image/webp|['"]webp['"]}i)
    raise "#{File.basename(source)} must keep WebP hidden (no mobile Skia encoder)"
  end
end
raise "the WebP omission must stay documented" unless export_support.include?(
  "WebP is deliberately ABSENT",
)
raise "the engine export filename must be validated before staging" unless document_shell.include?(
  "DocumentExportSupport.validatedFilename(host.exportFileName())",
)
raise "the export must be staged app-privately, then copied" unless document_shell.include?(
  "host.exportToPath(stagedPath)",
)

# ---- SSO: region, origins, and the lazy configure --------------------------

region = read(ets_dir, "common/SsoRegion.ets")
auth_runtime = read(ets_dir, "common/AuthRuntime.ets")
auth_client = read(ets_dir, "common/SsoAuthClient.ets")
device_login = read(ets_dir, "common/DeviceLoginRequest.ets")
browser = read(ets_dir, "common/BrowserLauncher.ets")
login_page = read(ets_dir, "common/NativeLoginPage.ets")
account_center = read(ets_dir, "common/AccountCenterSheet.ets")
code_form = read(ets_dir, "common/AuthCodeForm.ets")

# ORIGIN HYGIENE. `SsoRegion.ets` is the single place allowed to name a
# first-party host — the same discipline `SsoRegion.kt` holds on Android.
# Everywhere else an origin arrives at runtime (the region store, or the
# engine's own verification URL).
origin_owner = File.join(ets_dir, "common/SsoRegion.ets")
ets_sources.push(File.join(player_dir, "entry/src/main/cpp/types/libopenpencil/index.d.ts")).each do |source|
  next if source == origin_owner

  body = File.read(source)
  if body.match?(%r{https://(?:sso\.|op\.)?zseven\.(?:cn|tech)})
    raise "#{File.basename(source)} must not hardcode an SSO/hub origin"
  end
end
raise "auth region codes must stay engine-owned" unless header.include?("OpAuthRegion_China = 0") &&
  header.include?("OpAuthRegion_Global = 1")
raise "the region table must carry both deployments" unless region.include?(
  "new SsoRegion('CHINA', 'https://sso.zseven.cn', 0)",
) && region.include?("new SsoRegion('GLOBAL', 'https://sso.zseven.tech', 1)")

# The IP probe target and the mainland redirect host must match Android's,
# so both shells read the SAME gateway verdict.
raise "the region probe must ask the global gateway" unless region.include?(
  "const REGION_PROBE_URL = 'https://op.zseven.tech/'",
)
raise "the mainland redirect host drifted" unless region.include?(
  "const MAINLAND_REDIRECT_HOST = 'op.zseven.cn'",
)
if File.exist?(File.join(android_dir, "SsoRegion.kt"))
  kotlin_region = read(android_dir, "SsoRegion.kt")
  raise "the probe URL must match the Android shell" unless kotlin_region.include?(
    'REGION_PROBE_URL = "https://op.zseven.tech/"',
  )
  raise "the redirect host must match the Android shell" unless kotlin_region.include?(
    'MAINLAND_REDIRECT_HOST = "op.zseven.cn"',
  )
  %w[https://sso.zseven.cn https://sso.zseven.tech].each do |origin|
    raise "region origin #{origin} drifted from Android" unless kotlin_region.include?(origin)
  end
end
raise "an unreachable global host must read as mainland" unless region.match?(
  /probeRegion\(\)[\s\S]{0,2000}?\} catch \(error\) \{[\s\S]{0,400}?return SsoRegion\.CHINA;/m,
)
# Resolution order: override, then the last detection, then the locale.
resolved_body = region[/resolved\(\): SsoRegion \{.*?\n  \}/m]
raise "resolved() not found" if resolved_body.nil?
raise "a user override must win over detection" unless resolved_body.match?(
  /this\.overrideKey[\s\S]*?this\.detectedKey[\s\S]*?localeDefault\(\)/m,
)
raise "the region must be persisted with Preferences" unless region.include?(
  "preferences.getPreferences(this.context, PREFS)",
)
raise "detection must be skipped once the user chose" unless region.match?(
  /refreshDetectedRegionAsync\(\): void \{\s*\n\s*if \(this\.hasUserOverride\(\)/m,
)

# LAZY CONFIGURE (`AndroidAuthRuntime.kt`): a fresh install defers the
# configure until the first sign-in so the region can still change; a
# returning user configures at startup so the session restores.
raise "auth must be configured through the engine" unless engine_host.include?(
  "napi.editorConfigureAuth(this.engine, storageDir, deviceName, appVersion, region)",
)
raise "the runtime must configure at most once per process" unless auth_runtime.include?(
  "if (this.configured || !host.isEditorMode()",
)
raise "a fresh install must defer the configure" unless auth_runtime.match?(
  /if \(!this\.hasPersistedCredential\(\)\) \{[\s\S]{0,400}?refreshDetectedRegionAsync\(\);\s*\n\s*return;/m,
)
raise "a returning user must configure at startup" unless auth_runtime.match?(
  /this\.configured = true;[\s\S]{0,200}?resolveForStartup\(\);[\s\S]{0,120}?configureNow/m,
)
raise "startLogin must configure before beginning the flow" unless auth_runtime.match?(
  /if \(!this\.configured\)[\s\S]{0,300}?configureNow\(host, region\);[\s\S]{0,120}?host\.beginLogin\(\)/m,
)
raise "a build with no auth backend must answer NotReady" unless auth_runtime.include?(
  "export const STATUS_NOT_READY = 10;",
)
raise "credentials must live in their own private directory" unless auth_runtime.include?(
  "`${context.filesDir}/auth`",
)
raise "the auth directory must not be the engine storage root" if auth_runtime.include?(
  "${context.filesDir}/config",
)
raise "the shell must configure auth exactly once at startup" unless index.include?(
  "this.auth.configureAtStartup(this.host)",
)

# DEVICE SPLIT: a 2in1 signs in through the SYSTEM BROWSER, phones and tablets
# through the native page. Both are load-bearing and mutually exclusive.
raise "the desktop-class test must gate the login presentation" unless index.match?(
  /if \(this\.host\.isDesktopClass\(\)\) \{\s*\n\s*this\.openBrowserForLogin\(request\);\s*\n\s*return;/m,
)
raise "the mobile path must open the native login page" unless index.match?(
  /this\.loginPairingId = request\.pairingId;[\s\S]{0,200}?this\.showLogin = true;/m,
)
raise "the native login page must be built only when shown" unless index.match?(
  /if \(this\.showLogin\) \{\s*\n\s*NativeLoginPage\(\{/m,
)
raise "the browser launcher must use an implicit view want" unless browser.include?(
  "action: 'ohos.want.action.viewData'",
) && browser.include?("entities: ['entity.system.browsable']")
raise "only https links may be handed to a browser" unless browser.include?(
  "if (!url.startsWith('https://'))",
)
raise "the desktop-class flag must come from the engine host" unless engine_host.include?(
  "isDesktopClass(): boolean",
)
# The engine's DESKTOP chrome starts its own pairing and never emits
# RequestLogin, so a 2in1 has to configure eagerly.
raise "a 2in1 must configure at startup" unless auth_runtime.match?(
  /if \(host\.isDesktopClass\(\)\) \{[\s\S]{0,300}?configureNow\(host, region\);/m,
)

# The verification URL is the ONLY source of the pairing origin.
raise "the verification URL must be parsed, not trusted" unless device_login.include?(
  "if (parsed.protocol.toLowerCase() !== 'https:')",
)
raise "a URL carrying userinfo must be rejected" unless device_login.include?(
  "parsed.username.length > 0 || parsed.password.length > 0",
)
raise "a URL without a pairing must be rejected" unless device_login.match?(
  /device_pairing.*?return null;/m,
)
raise "a rejected verification URL must cancel the flow" unless index.match?(
  /rejected the login request.*?this\.host\.cancelLogin\(\)/m,
)
raise "the auth client must be bound to the pairing origin" unless login_page.include?(
  "this.client = new SsoAuthClient(this.origin);",
)

# MEMORY-ONLY COOKIES: the short-lived web session must never touch disk.
raise "the cookie jar must be an in-memory Map" unless auth_client.include?(
  "private readonly cookies: Map<string, string> = new Map<string, string>();",
)
raise "the memory-only cookie rule must stay documented" unless auth_client.include?(
  "COOKIES ARE MEMORY-ONLY",
)
raise "session cookies must never be persisted" if auth_client.match?(
  /preferences|fileIo|@kit\.ArkData|@kit\.CoreFileKit/,
)
# Nothing in the auth path may log a credential or a token.
[auth_client, login_page, code_form, auth_runtime, account_center].each do |source|
  if source.match?(/hilog\.\w+\([^)]*(?:password|token|cookie|verification_code)/i)
    raise "the auth path must never log a credential"
  end
end
raise "the device token must stay inside the engine" if ets_sources.any? { |source|
  File.read(source).match?(/deviceToken|device_token/)
}

# The SSO JSON API surface must match the Android client route for route.
{
  "password-login" => "/api/v1/auth/password-login",
  "email-codes" => "/api/v1/auth/email-codes",
  "register" => "/api/v1/auth/register",
  "password-reset" => "/api/v1/auth/password-reset",
  "approve" => "/api/v1/device/login/approve",
}.each do |name, route|
  raise "SSO route #{name} missing" unless auth_client.include?("'#{route}'")
end
raise "providers must be fetched for the mobile channel" unless auth_client.include?(
  "/api/v1/auth/providers?channel=web_mobile",
)
if File.exist?(File.join(android_dir, "SsoAuthClient.kt"))
  kotlin_client = read(android_dir, "SsoAuthClient.kt")
  %w[
    /api/v1/auth/password-login /api/v1/auth/email-codes /api/v1/auth/register
    /api/v1/auth/password-reset /api/v1/device/login/approve
  ].each do |route|
    raise "SSO route #{route} drifted from Android" unless kotlin_client.include?(route)
  end
end

# The native login page mirrors the ZSeven design + the register/reset forms.
raise "the login page must carry the ZSeven logo" unless login_page.include?(
  "$r('app.media.zseven_logo')",
)
%w[
  native_login_welcome native_login_email_label native_login_password_label
  native_login_sign_in native_login_forgot_password native_login_register_now
  native_login_region
].each do |key|
  raise "the login page must render #{key}" unless login_page.include?("app.string.#{key}")
end
raise "the region row must toggle the deployment" unless login_page.match?(
  /toggleRegion\(\): void \{[\s\S]{0,400}?saveUserOverride\(next\)/m,
)
raise "a region switch must offer the restart" unless index.include?(
  "$r('app.string.sso_region_restart_note')",
)
raise "the password rules must mirror the backend policy" unless read(
  ets_dir, "common/AuthTheme.ets"
).match?(/value\.length >= 12 && value\.length <= 50/)
%w[register reset].each do |mode|
  raise "the #{mode} form must exist" unless code_form.include?("app.string.#{mode}_title")
end
raise "the code form must run both email-code purposes" unless code_form.include?(
  "this.isRegister() ? 'register' : 'password_reset'",
)
raise "a finished form must approve the page's pairing" unless code_form.include?(
  "await this.onApprovePairing()",
)

# Action 5: the account center is native and reads the ENGINE snapshot.
raise "the account center must read the engine snapshot" unless index.include?(
  "AccountSnapshot.parse(this.host.accountSnapshot())",
)
raise "a signed-out engine must start a sign-in instead" unless index.match?(
  /if \(snapshot === null \|\| !snapshot\.signedIn\) \{\s*\n\s*this\.requestLogin\(\);/m,
)
raise "sign out must go through the engine" unless engine_host.include?("napi.editorSignOut(this.engine)")
raise "the account center must offer sign out" unless account_center.include?(
  "app.string.account_center_sign_out",
)
raise "signing out must be confirmed" unless index.include?(
  "$r('app.string.account_center_sign_out_confirm')",
)
raise "the account page must be region-scoped" unless account_center.include?(
  "`${store.resolved().origin}/account`",
)
# A PC gets a light card, touch chrome the full sheet.
raise "the account center presentation must follow the form factor" unless index.include?(
  "compact: this.host.isDesktopClass(),",
)

puts "HarmonyOS shell contract validates"
