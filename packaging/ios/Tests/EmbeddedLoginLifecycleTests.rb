# frozen_string_literal: true

view_source = File.read(ARGV.fetch(0))
browser_source = File.read(ARGV.fetch(1))
storage_source = File.read(ARGV.fetch(2))

show = view_source[/func showEmbeddedLogin\b.*?(?=\n    \/\/\/ Rust is authoritative)/m]
raise "embedded-login presentation method missing" unless show
raise "duplicate login action must be idempotent" unless show.include?(
  "guard embeddedLoginController == nil else { return }"
)
failure = show[/guard\s+!didTearDown,.*?else \{(.*?)\n        \}/m, 1]
raise "failed login presentation must cancel the Rust flow" unless failure&.include?(
  "host.cancelEmbeddedLogin()"
)

finish = browser_source[/func finishFromHost\b.*?(?=\n    \/\/\/ Called during engine teardown)/m]
raise "terminal close must clear deferred external-browser work" unless finish&.include?(
  "pendingExternalURL = nil"
)
cancel = browser_source[/func cancelButtonPressed\b.*?(?=\n    private func showNavigationError)/m]
raise "user close must clear deferred external-browser work" unless cancel&.include?(
  "pendingExternalURL = nil"
)
external = browser_source[/private func openPendingExternalURL\b.*?(?=\n    \})/m]
raise "deferred Safari open must stop after terminal close" unless external&.include?(
  "guard !isFinishing"
)
unless browser_source.scan("pendingExternalURL = request.initialURL").length == 2
  raise "external fallback must restart from verification_uri in both navigation paths"
end
if browser_source.include?("pendingExternalURL = url")
  raise "identity-provider redirect URL must never be opened directly"
end
unless browser_source.include?("directlyUserInitiated || userNavigationChainActive")
  raise "same-origin user navigation must carry authority through an OAuth redirect"
end
%w[.linkActivated .formSubmitted .formResubmitted].each do |navigation_type|
  unless browser_source.include?(navigation_type)
    raise "#{navigation_type} must count as an explicit user navigation"
  end
end
finish_navigation = browser_source[/func webView\(_ webView: WKWebView, didFinish.*?\n    \}/m]
unless finish_navigation&.include?("userNavigationChainActive = false")
  raise "completed same-origin navigation must expire external fallback authority"
end

raise "auth storage must be excluded from backup" unless storage_source.include?(
  "resourceValues.isExcludedFromBackup = true"
)
unless storage_source.scan("completeUntilFirstUserAuthentication").length >= 2
  raise "auth storage must retain the required iOS data-protection class"
end

puts "iOS embedded-login lifecycle contract validates"
