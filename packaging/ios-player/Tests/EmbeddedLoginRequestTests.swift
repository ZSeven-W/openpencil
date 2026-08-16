import Foundation

private func expect(_ condition: @autoclosure () -> Bool, _ message: String) {
    guard condition() else {
        FileHandle.standardError.write(Data("FAIL: \(message)\n".utf8))
        exit(1)
    }
}

@main
private enum EmbeddedLoginRequestTests {
    static func main() {
        rejectsInsecureAndCredentialedInitialURLs()
        defaultsToExactInitialOrigin()
        supportsExplicitOriginsWithoutWildcardMatching()
        externalFallbackRequiresAUserActivatedSecureLink()
        validatesEveryAllowlistEntry()
        keepsStoragePolicyExplicit()
        print("EmbeddedLoginRequest tests passed")
    }

    private static func request(_ value: String) -> EmbeddedLoginRequest? {
        guard let url = URL(string: value) else { return nil }
        return EmbeddedLoginRequest(initialURL: url)
    }

    private static func rejectsInsecureAndCredentialedInitialURLs() {
        expect(request("http://login.example/start") == nil, "HTTP must be rejected")
        expect(request("file:///tmp/login.html") == nil, "file URLs must be rejected")
        expect(request("javascript:alert(1)") == nil, "script URLs must be rejected")
        expect(request("https://user:secret@login.example/start") == nil, "userinfo must be rejected")
        expect(request("https://login..example/start") == nil, "invalid hosts must be rejected")
    }

    private static func defaultsToExactInitialOrigin() {
        let value = request("https://Login.Example/start?device=1")
        expect(value != nil, "valid HTTPS request should be accepted")
        expect(
            value?.allowsTopLevelNavigation(to: URL(string: "https://login.example/approve")!) == true,
            "same origin path should be allowed"
        )
        expect(
            value?.allowsTopLevelNavigation(to: URL(string: "https://login.example:443/approve")!) == true,
            "default and explicit HTTPS ports should match"
        )
        expect(
            value?.allowsTopLevelNavigation(to: URL(string: "https://login.example.evil.test/")!) == false,
            "suffix-confusable host must be rejected"
        )
        expect(
            value?.allowsTopLevelNavigation(to: URL(string: "https://sub.login.example/")!) == false,
            "subdomains require explicit authorization"
        )
        expect(
            value?.allowsTopLevelNavigation(to: URL(string: "https://login.example:8443/")!) == false,
            "alternate ports require explicit authorization"
        )
    }

    private static func supportsExplicitOriginsWithoutWildcardMatching() {
        let value = EmbeddedLoginRequest(
            flowID: 42,
            initialURL: URL(string: "https://login.example/start")!,
            allowedOriginURLs: [URL(string: "https://accounts.identity.example/oauth")!]
        )
        expect(value?.flowID == 42, "flow ID should round-trip")
        expect(
            value?.allowsTopLevelNavigation(
                to: URL(string: "https://accounts.identity.example/consent")!
            ) == true,
            "explicit identity origin should be allowed"
        )
        expect(
            value?.allowsTopLevelNavigation(
                to: URL(string: "https://evil.accounts.identity.example/consent")!
            ) == false,
            "allowlist entries must not authorize subdomains"
        )
        expect(
            value?.allowsSubframeNavigation(to: URL(string: "https://challenge.example/frame")!) == true,
            "secure third-party challenge frames should remain usable"
        )
        expect(
            value?.allowsSubframeNavigation(to: URL(string: "data:text/html,test")!) == false,
            "active non-HTTPS subframes must be blocked"
        )
    }

    private static func validatesEveryAllowlistEntry() {
        let value = EmbeddedLoginRequest(
            initialURL: URL(string: "https://login.example/start")!,
            allowedOriginURLs: [URL(string: "http://accounts.identity.example")!]
        )
        expect(value == nil, "one invalid allowlist entry must reject the whole request")
    }

    private static func externalFallbackRequiresAUserActivatedSecureLink() {
        let value = request("https://login.example/start")!
        let external = URL(string: "https://accounts.google.com/signin")!
        let disposition = value.topLevelDisposition(to: external, userInitiated: true)
        expect(
            disposition == .openInitialURLExternally,
            "a user-tapped external HTTPS link should restart verification in the system browser"
        )
        let systemBrowserURL = disposition == .openInitialURLExternally ? value.initialURL : external
        expect(
            systemBrowserURL == value.initialURL,
            "fallback must open verification_uri, never the provider redirect URL"
        )
        expect(
            value.topLevelDisposition(to: external, userInitiated: false) == .reject,
            "an external redirect must not escape the embedded origin"
        )
        expect(
            value.topLevelDisposition(
                to: URL(string: "custom-login://accounts")!,
                userInitiated: true
            ) == .reject,
            "custom schemes must not reach UIApplication.open"
        )
        expect(
            value.topLevelDisposition(
                to: URL(string: "https://login.example/approve")!,
                userInitiated: false
            ) == .allowEmbedded,
            "same-origin redirects should remain in the WebView"
        )
    }

    private static func keepsStoragePolicyExplicit() {
        let persistent = request("https://login.example/start")
        expect(persistent?.dataStorePolicy == .persistent, "persistent cookies should be default")
        let ephemeral = EmbeddedLoginRequest(
            initialURL: URL(string: "https://login.example/start")!,
            dataStorePolicy: .ephemeral
        )
        expect(ephemeral?.dataStorePolicy == .ephemeral, "ephemeral policy should round-trip")
    }
}
