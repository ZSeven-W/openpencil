# frozen_string_literal: true

root = File.expand_path("../app/src/main/kotlin/tech/zseven/openpencil", __dir__)
overlay = File.read(File.join(root, "LoginWebViewOverlay.kt"))
policy = File.read(File.join(root, "LoginWebViewPolicy.kt"))
activity = File.read(File.join(root, "MainActivity.kt"))
surface = File.read(File.join(root, "OpSurfaceView.kt"))
runtime = File.read(File.join(root, "AndroidAuthRuntime.kt"))

raise "login URL must come from native" unless surface.include?(
  "OpNative.nativeEditorTakeLoginUrl(engine)",
)
raise "platform login URL must not be hard-coded" if [overlay, policy, activity, surface].any? do |source|
  source.match?(%r{https://(?:sso\.)?zseven\.cn})
end
raise "unsafe initial URL must cancel native login" unless activity.match?(
  /onRequestRejected.*?surfaceView\.cancelLogin\(\)/m,
)

raise "main-frame provider fallback needs an explicit gesture" unless overlay.include?(
  "hasGesture = request.hasGesture()",
)
raise "server redirects need a bounded user navigation chain" unless policy.match?(
  /class LoginNavigationChain.*?isRedirect && userInitiatedChain.*?pageFinished\(\).*?userInitiatedChain = false/m,
)
raise "fallback must restart the verification URI" unless overlay.include?(
  "openExternalHttps(policy.initialUrl)",
)
raise "mixed content must be disabled" unless overlay.include?(
  "MIXED_CONTENT_NEVER_ALLOW",
)
raise "file and content access must be disabled" unless overlay.match?(
  /allowFileAccess = false.*?allowContentAccess = false/m,
)
raise "TLS failures must not proceed" unless overlay.match?(
  /onReceivedSslError.*?handler\.cancel\(\)/m,
)
raise "WebView permissions must fail closed" unless overlay.match?(
  /onPermissionRequest.*?request\.deny\(\)/m,
)
raise "login WebView must not expose a JavaScript bridge" if overlay.include?(
  "addJavascriptInterface",
)

raise "system back must be owned while login is visible" unless activity.match?(
  /OnBackPressedCallback\(false\).*?loginWebView\.handleBack\(\)/m,
)
raise "user close must cancel the Rust flow" unless activity.include?(
  "onCanceled = { surfaceView.cancelLogin() }",
)
raise "native success must close without cancel" unless surface.match?(
  /SHELL_ACTION_CLOSE_LOGIN_WEBVIEW.*?closeLoginWebViewHandler/m,
)
raise "WebView lifecycle must follow the Activity" unless activity.match?(
  /onPause\(\).*?loginWebView\.onPause\(\).*?onResume\(\).*?loginWebView\.onResume\(\).*?onDestroy\(\).*?loginWebView\.destroy\(\)/m,
)
raise "login storage must be private and excluded from backup" unless runtime.include?(
  'File(context.noBackupFilesDir, "auth")',
)
raise "device label must not use a persistent Android identifier" unless runtime.include?(
  "Build.MODEL",
)

puts "Android embedded login WebView contract validates"
