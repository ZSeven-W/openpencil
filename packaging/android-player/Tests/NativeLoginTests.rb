# frozen_string_literal: true

# Source contracts for the native (WebView-free) SSO login and account
# surfaces on Android: engine-owned URLs, single cancellation, region
# hygiene, and the retirement of the login WebView.

root = File.expand_path("../app/src/main/kotlin/tech/zseven/openpencil", __dir__)
overlay = File.read(File.join(root, "NativeLoginOverlay.kt"))
account = File.read(File.join(root, "AccountCenterOverlay.kt"))
region = File.read(File.join(root, "SsoRegion.kt"))
request = File.read(File.join(root, "DeviceLoginRequest.kt"))
activity = File.read(File.join(root, "MainActivity.kt"))
surface = File.read(File.join(root, "OpSurfaceView.kt"))
runtime = File.read(File.join(root, "AndroidAuthRuntime.kt"))
client = File.read(File.join(root, "SsoAuthClient.kt"))
header = File.read(File.expand_path(
  "../../../crates/op-engine-ffi/include/op_engine.h", __dir__
))

# The login URL comes only from native; the shell never manufactures one.
raise "login URL must come from native" unless surface.include?(
  "OpNative.nativeEditorTakeLoginUrl(engine)",
)
raise "login screen must parse the engine verification URL" unless overlay.include?(
  "DeviceLoginRequest.parse(verificationUrl)",
)
[overlay, request, activity, surface, runtime].each do |source|
  if source.match?(%r{https://(?:sso\.)?zseven\.(?:cn|tech)})
    raise "regional SSO origins must live only in SsoRegion"
  end
end

# Rejected/duplicate requests and user cancel keep the Rust flow honest.
raise "unsafe verification URL must cancel native login" unless activity.match?(
  /onRequestRejected.*?surfaceView\.cancelLogin\(\)/m,
)
raise "duplicate show must be idempotent" unless overlay.include?(
  "if (request?.verificationUrl == parsed.verificationUrl) return true",
)
raise "user cancel must report exactly once" unless overlay.include?("cancellationReported")
raise "engine-terminal close must not cancel" unless overlay.include?(
  "fun dismissFromNative()",
)

# WebView is fully retired from the login path.
raise "WebView must stay out of the login path" if overlay.include?("WebView")
raise "login WebView overlay must stay deleted" if File.exist?(
  File.join(root, "LoginWebViewOverlay.kt"),
)

# Third-party providers hand off to the system browser at the engine URL.
raise "provider hand-off must reuse the verification URL" unless overlay.include?(
  "openExternal(request?.verificationUrl)",
)

# Region codes are pinned to the C header; probing consults the ZSeven
# gateway itself (nginx geo routing on the hub entry), stays redirect-blind,
# waits for the probe on a first launch, and a user override suppresses
# detection.
raise "region code 0 must be China" unless region.include?('CHINA("https://sso.zseven.cn", 0)')
raise "region code 1 must be Global" unless region.include?('GLOBAL("https://sso.zseven.tech", 1)')
raise "region probe must ask the hub gateway" unless region.include?(
  'REGION_PROBE_URL = "https://op.zseven.tech/"',
)
raise "mainland verdict must match the hub redirect" unless region.include?(
  'MAINLAND_REDIRECT_HOST = "op.zseven.cn"',
)
raise "first launch must wait for the IP verdict" unless region.include?(
  "fun resolveForStartup",
)
raise "auth runtime must configure from the startup resolution" unless runtime.include?(
  "regionStore.resolveForStartup",
)
raise "provider list must come from the pairing origin" unless overlay.include?(
  "fetchProviders",
)
raise "provider cards must reuse the verification URL" unless overlay.include?(
  "AuthUi.providerCard"
)

# Lazy region lock: fresh installs defer auth configuration to the first
# sign-in; RequestLogin configures-then-begins through one shared runtime.
runtime_new = File.read(File.join(root, "AndroidAuthRuntime.kt"))
raise "fresh installs must defer auth configuration" unless runtime_new.include?(
  "if (!hasPersistedCredential()) {"
)
raise "sign-in must configure before beginning" unless runtime_new.include?(
  "fun startLogin"
)
raise "region switch must offer an immediate restart" unless overlay.include?(
  "fun restartApp"
)
raise "header China region drifted" unless header.include?("OpAuthRegion_China = 0")
raise "header Global region drifted" unless header.include?("OpAuthRegion_Global = 1")
raise "region probe must not follow redirects" unless region.include?(
  "connection.instanceFollowRedirects = false",
)
raise "user region override must suppress detection" unless region.include?(
  "if (hasUserOverride()) return",
)
raise "auth runtime must pass the resolved region" unless runtime.include?(
  "region.authRegionCode",
)

# The approval session cookie stays in memory; sign-out goes through the
# engine so the device token is revoked by the runtime that owns it.
raise "SSO cookies must stay in a per-client jar" unless client.include?(
  "CookieManager(null, CookiePolicy.ACCEPT_ALL)",
)
raise "account sign-out must go through the engine" unless account.include?("onSignOut()") &&
  activity.include?("surfaceView.signOutAccount()")

puts "Android native login contract validates"
