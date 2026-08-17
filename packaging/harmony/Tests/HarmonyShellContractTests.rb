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
  missing = (expected - exported).sort
  extra = (exported - expected).sort
  raise "NAPI surface is missing JNI functions: #{missing.join(', ')}" unless missing.empty?
  raise "NAPI surface has functions the JNI bridge lacks: #{extra.join(', ')}" unless extra.empty?
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
  onOpenLanguagePicker onOpenLoginUi onCloseLoginUi
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
raise "sign-in must report itself unavailable" unless index.include?(
  "$r('app.string.native_login_unavailable')",
)
raise "an unavailable sign-in must cancel the engine flow" unless index.match?(
  /onOpenLoginUi.*?native_login_unavailable.*?this\.host\.cancelLogin\(\)/m,
)
raise "RequestLogin must cancel the engine flow too" unless index.match?(
  /onRequestLogin.*?native_login_unavailable.*?this\.host\.cancelLogin\(\)/m,
)
raise "cancelLogin must go through the engine" unless engine_host.include?("napi.editorCancelLogin(this.engine)")
# Action 3: an engine-terminal close dismisses without cancelling again.
raise "CloseLoginWebView must dismiss the notice" unless index.match?(
  /onCloseLoginUi.*?this\.showNotice = false/m,
)
raise "an engine-terminal close must not cancel" if index.match?(
  /onCloseLoginUi[^}]*cancelLogin/m,
)
# Action 5: the account center shares the unavailable notice.
raise "the account center must report itself unavailable" unless index.match?(
  /onOpenAccountCenter.*?native_login_unavailable/m,
)
# No WebView anywhere: HarmonyOS never opens the retired login web view.
ets_sources.each do |source|
  if File.read(source).include?("@ohos.web.webview") || File.read(source).include?("Web({")
    raise "#{File.basename(source)} must keep WebView out of the login path"
  end
end

# ---- IME conduit: the empty-conduit backspace contract ---------------------

raise "the conduit must use the system input method" unless ime.include?("inputMethod.getController()")
raise "the conduit must attach/detach with engine IME focus" unless ime.include?("this.controller.attach(true,") &&
  ime.include?("this.controller.detach()")
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
# Physical keys must not double-type while the IME conduit is attached.
raise "key text must be suppressed while the IME is attached" unless read(ets_dir, "common/KeyForwarder.ets").include?(
  "if (imeAttached) {",
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

# ---- SSO origin hygiene ----------------------------------------------------

ets_sources.push(File.join(player_dir, "entry/src/main/cpp/types/libopenpencil/index.d.ts")).each do |source|
  body = File.read(source)
  if body.match?(%r{https://(?:sso\.|op\.)?zseven\.(?:cn|tech)})
    raise "#{File.basename(source)} must not hardcode an SSO/hub origin"
  end
end
raise "auth region codes must stay engine-owned" unless header.include?("OpAuthRegion_China = 0") &&
  header.include?("OpAuthRegion_Global = 1")
# HarmonyOS has no op-auth build, so the shell never configures an auth region.
raise "the shell must not configure auth without an OHOS op-auth build" if ets_sources.any? { |source|
  File.read(source).match?(/napi\.editorConfigureAuth\s*\(/)
}

puts "HarmonyOS shell contract validates"
