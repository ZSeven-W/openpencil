import Foundation

/// Cookie/storage ownership for one embedded sign-in flow.
enum EmbeddedLoginDataStorePolicy: Equatable {
    /// Reuse the app's WebKit cookie jar so a returning user does not have to
    /// authenticate with the identity provider on every OpenPencil launch.
    case persistent
    /// Keep cookies and website data in memory for this WebView only.
    case ephemeral
}

/// A platform-neutral description of the WebView the Rust host asked iOS to
/// present. The Swift shell never owns a production login URL: every request
/// must be injected by the engine for the active auth flow.
struct EmbeddedLoginRequest: Equatable {
    let flowID: UInt64?
    let initialURL: URL
    let allowedOrigins: Set<EmbeddedLoginOrigin>
    let dataStorePolicy: EmbeddedLoginDataStorePolicy

    /// Returns `nil` rather than weakening navigation policy when the engine
    /// provides an invalid URL or allowlist entry.
    init?(
        flowID: UInt64? = nil,
        initialURL: URL,
        allowedOriginURLs: [URL] = [],
        dataStorePolicy: EmbeddedLoginDataStorePolicy = .persistent
    ) {
        guard let initialOrigin = EmbeddedLoginOrigin(url: initialURL) else { return nil }
        var origins: Set<EmbeddedLoginOrigin> = [initialOrigin]
        for url in allowedOriginURLs {
            guard let origin = EmbeddedLoginOrigin(url: url) else { return nil }
            origins.insert(origin)
        }
        self.flowID = flowID
        self.initialURL = initialURL
        self.allowedOrigins = origins
        self.dataStorePolicy = dataStorePolicy
    }

    func allowsTopLevelNavigation(to url: URL) -> Bool {
        guard let origin = EmbeddedLoginOrigin(url: url) else { return false }
        return allowedOrigins.contains(origin)
    }

    /// Secure third-party frames are allowed because identity providers often
    /// embed challenges or account pickers from another origin. They cannot
    /// become the top-level page without passing the strict origin allowlist.
    func allowsSubframeNavigation(to url: URL) -> Bool {
        EmbeddedLoginOrigin(url: url) != nil
    }
}

/// Result for a main-frame navigation request. A user-tapped link to another
/// HTTPS origin leaves the embedded browser via the system browser; redirects
/// and scripts cannot silently escape the exact-origin policy.
enum EmbeddedLoginNavigationDisposition: Equatable {
    case allowEmbedded
    case openInitialURLExternally
    case reject
}

extension EmbeddedLoginRequest {
    func topLevelDisposition(to url: URL, userInitiated: Bool) -> EmbeddedLoginNavigationDisposition {
        if allowsTopLevelNavigation(to: url) { return .allowEmbedded }
        guard userInitiated, EmbeddedLoginOrigin(url: url) != nil else { return .reject }
        return .openInitialURLExternally
    }
}

/// Exact HTTPS origin. Paths never broaden an allowlist entry, and host
/// comparisons are case-insensitive without suffix or wildcard matching.
struct EmbeddedLoginOrigin: Hashable {
    let host: String
    let port: Int

    init?(url: URL) {
        guard
            url.baseURL == nil,
            let components = URLComponents(url: url, resolvingAgainstBaseURL: false),
            components.scheme?.lowercased() == "https",
            components.user == nil,
            components.password == nil,
            let rawHost = components.host,
            !rawHost.isEmpty,
            !rawHost.hasPrefix("."),
            !rawHost.hasSuffix("."),
            !rawHost.contains(".."),
            rawHost.unicodeScalars.allSatisfy({ scalar in
                scalar.isASCII && (
                    CharacterSet.alphanumerics.contains(scalar)
                        || scalar == "."
                        || scalar == "-"
                        || scalar == ":"
                )
            })
        else { return nil }

        let normalizedPort = components.port ?? 443
        guard (1...65_535).contains(normalizedPort) else { return nil }
        host = rawHost.lowercased()
        port = normalizedPort
    }
}
