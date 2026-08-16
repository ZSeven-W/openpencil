# frozen_string_literal: true

# Source contracts for the native (WebView-free) SSO login and account
# surfaces: OpPlayerView presentation glue, the login screen itself, region
# handling, and auth storage hygiene.

view_source = File.read(ARGV.fetch(0))
login_source = File.read(ARGV.fetch(1))
storage_source = File.read(ARGV.fetch(2))
region_source = File.read(ARGV.fetch(3))
header_source = File.read(ARGV.fetch(4))

show = view_source[/func showNativeLogin\b.*?(?=\n    \/\/\/ Rust is authoritative)/m]
raise "native-login presentation method missing" unless show
raise "duplicate login action must be idempotent" unless show.include?(
  "guard nativeLoginController == nil else { return }"
)
raise "failed login presentation must cancel the Rust flow" unless show.scan(
  "host.cancelLoginFlow()"
).length >= 1
raise "engine-terminal close path missing" unless view_source.include?(
  "func closeNativeLoginFromHost()"
)

# The login screen must never manufacture its own login URL: origin and
# pairing come only from the engine-provided verification URL.
raise "login screen must parse the engine verification URL" unless login_source.include?(
  "DeviceLoginRequestInfo(verificationURL: verificationURL)"
)
if login_source.match?(%r{https://(?:sso\.)?zseven\.(?:cn|tech)})
  raise "platform login URL must not be hard-coded in the login screen"
end

# User cancel reports exactly once and Rust stays authoritative for success.
cancel = login_source[/@objc private func cancelPressed\b.*?\n    \}/m]
raise "user cancel must guard duplicate reports" unless cancel&.include?("cancellationReported")
raise "approval success must wait for the engine close action" unless login_source.include?(
  "self.statusLabel.isHidden = false"
)
finish = login_source[/func finishFromHost\b.*?\n    \}/m]
raise "host-driven dismissal missing" unless finish&.include?("dismiss(animated: animated)")

# Third-party providers stay inside the app: an in-app Safari sheet opens the
# engine-provided verification URL deep-linked to the tapped provider — never
# an embedded WebView, never an external-browser bounce.
raise "provider hand-off must reuse verification_uri" unless login_source.include?(
  "var url = verificationURL"
)
raise "provider tap must deep-link the tapped provider" unless login_source.include?(
  'URLQueryItem(name: "provider", value: providerID)'
)
raise "provider sign-in must stay in-app" unless login_source.include?(
  "present(SFSafariViewController(url: url), animated: true)"
)
raise "WebKit must stay out of the login path" if login_source.include?("import WebKit")

# Apple sign-in is SDK-native: the tapped card runs the system
# AuthenticationServices sheet, exchanges the nonce-bound identity token at
# the SSO native-login endpoint, then approves the pairing directly. A user
# cancel returns to the screen; other failures fall back to the in-app web
# flow.
raise "apple card must run the native sheet" unless login_source.include?(
  'if providerID == "apple" {'
)
raise "apple token must exchange at native-login" unless login_source.include?(
  'providerID: "apple",'
) && login_source.include?("self.client.nativeLogin(")
raise "native apple success must approve the pairing" unless login_source.include?(
  "self.approvePairing()"
)

# Region codes are literals for standalone-test compilability; pin them to
# the C header so they cannot drift.
raise "region code 0 must be China" unless region_source.include?("case .china: return 0")
raise "region code 1 must be Global" unless region_source.include?("case .global: return 1")
raise "header China region drifted" unless header_source.include?("OpAuthRegion_China = 0")
raise "header Global region drifted" unless header_source.include?("OpAuthRegion_Global = 1")

# Region probing consults the ZSeven gateway itself (nginx geo routing on
# the hub entry), stays cookie-less and redirect-blind, waits for the probe
# on a first launch, and a user override always wins.
raise "region probe must ask the hub gateway" unless region_source.include?(
  'URL(string: "https://op.zseven.tech/")'
)
raise "mainland verdict must match the hub redirect" unless region_source.include?(
  'mainlandRedirectHost = "op.zseven.cn"'
)
raise "region probe must not send cookies" unless region_source.include?(
  "configuration.httpShouldSetCookies = false"
)
raise "region probe must not follow redirects" unless region_source.include?(
  "completionHandler(nil)"
)
raise "first launch must wait for the IP verdict" unless region_source.include?(
  "static func resolveForStartup"
)
raise "user region override must suppress detection" unless region_source.include?(
  "guard !hasUserOverride() else { return }"
)

# Third-party methods are region-accurate: fetched from the pairing origin,
# never hardcoded per region in the login screen.
raise "provider list must come from the pairing origin" unless login_source.include?(
  "client.fetchProviders"
)

raise "auth storage must be excluded from backup" unless storage_source.include?(
  "resourceValues.isExcludedFromBackup = true"
)

# The IME conduit starts (and stays) empty for plain typing, so backspace
# must be forwarded from deleteBackward itself — the delegate path never
# fires on an empty text view and engine inputs could type but not delete.
view_full = File.read(File.expand_path("../Sources/OpPlayerView.swift", __dir__))
raise "empty-conduit backspace must forward to the engine" unless view_full.include?(
  "imeTextView.onEmptyDeleteBackward"
)
raise "the conduit must override deleteBackward" unless view_full.include?(
  "override func deleteBackward()"
)

puts "iOS native login lifecycle contract validates"
