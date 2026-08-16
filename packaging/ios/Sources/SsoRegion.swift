import Foundation

/// Regional SSO deployment this install signs in against. Mirrors the
/// engine's `OpAuthRegion` codes; both origins are pinned first-party hosts.
enum SsoRegion: String, CaseIterable {
    case china
    case global

    var origin: URL {
        switch self {
        case .china: return URL(string: "https://sso.zseven.cn")!
        case .global: return URL(string: "https://sso.zseven.tech")!
        }
    }

    /// Engine `OpAuthRegion` code. Literal values keep this file free of the
    /// bridging header so the standalone test runner can compile it; the
    /// lifecycle contract test pins them against `op_engine.h`.
    var authRegionCode: Int32 {
        switch self {
        case .china: return 0
        case .global: return 1
        }
    }

    var displayName: String {
        switch self {
        case .china:
            return NSLocalizedString(
                "sso.region.china",
                value: "Mainland China",
                comment: "SSO region name"
            )
        case .global:
            return NSLocalizedString(
                "sso.region.global",
                value: "Global",
                comment: "SSO region name"
            )
        }
    }
}

/// Region resolution: a user override always wins, then the last IP-informed
/// detection, then a device-locale default. The auth runtime initializes once
/// per process, so later changes (override or fresh detection) apply on the
/// next launch — but the very first launch WAITS for the IP probe before the
/// runtime starts, so region is IP-determined from day one.
enum SsoRegionStore {
    private static let overrideKey = "sso.region.override"
    private static let detectedKey = "sso.region.detected"

    static func resolved() -> SsoRegion {
        if let override = stored(overrideKey) { return override }
        if let detected = stored(detectedKey) { return detected }
        return localeDefault()
    }

    static func hasUserOverride() -> Bool { stored(overrideKey) != nil }

    static func saveUserOverride(_ region: SsoRegion) {
        UserDefaults.standard.set(region.rawValue, forKey: overrideKey)
    }

    /// Startup resolution for the auth runtime. With a stored answer
    /// (override or earlier detection) it completes immediately on the main
    /// thread and refreshes the detection in the background for the next
    /// launch. On a first launch it runs the IP probe first — bounded by the
    /// probe's own timeouts — and only falls back to the device locale when
    /// the probe is inconclusive, so the region really comes from the IP.
    static func resolveForStartup(completion: @escaping (SsoRegion) -> Void) {
        let finish: (SsoRegion) -> Void = { region in
            DispatchQueue.main.async { completion(region) }
        }
        if stored(overrideKey) != nil || stored(detectedKey) != nil {
            finish(resolved())
            refreshDetectedRegionAsync()
            return
        }
        detectRegion { detected in
            if let detected {
                save(detected: detected)
                finish(detected)
            } else {
                finish(localeDefault())
            }
        }
    }

    /// One IP-informed detection per launch, skipped once the user chose a
    /// region; the result applies on the next launch.
    static func refreshDetectedRegionAsync() {
        guard !hasUserOverride() else { return }
        detectRegion { detected in
            if let detected { save(detected: detected) }
        }
    }

    /// Where the IP decision actually lives: the ZSeven global gateway.
    /// Nginx 302-redirects cookie-less mainland-China browser requests on
    /// `op.zseven.tech` to `op.zseven.cn` (APNIC-derived list, no
    /// third-party geolocation), so a plain no-follow GET against the hub
    /// entry reads the gateway's own IP verdict.
    private static let regionProbeURL = URL(string: "https://op.zseven.tech/")!
    private static let mainlandRedirectHost = "op.zseven.cn"

    /// The IP probe: ask the gateway. A mainland redirect means China, a
    /// direct answer means Global, an unreachable global host also reads as
    /// mainland (typical when only the domestic site is reachable); other
    /// outcomes are inconclusive (`nil`).
    private static func detectRegion(completion: @escaping (SsoRegion?) -> Void) {
        let configuration = URLSessionConfiguration.ephemeral
        configuration.httpShouldSetCookies = false
        configuration.httpCookieAcceptPolicy = .never
        configuration.timeoutIntervalForRequest = 3
        configuration.timeoutIntervalForResource = 5
        let delegate = NoRedirectDelegate()
        let session = URLSession(
            configuration: configuration,
            delegate: delegate,
            delegateQueue: nil
        )
        var request = URLRequest(url: regionProbeURL)
        request.httpMethod = "GET"
        let task = session.dataTask(with: request) { _, response, error in
            defer { session.finishTasksAndInvalidate() }
            if let http = response as? HTTPURLResponse {
                if (300..<400).contains(http.statusCode),
                    let location = http.value(forHTTPHeaderField: "Location"),
                    let host = URL(string: location, relativeTo: request.url)?.host,
                    host.lowercased() == mainlandRedirectHost
                {
                    completion(.china)
                } else if http.statusCode < 400 {
                    completion(.global)
                } else {
                    completion(nil)
                }
                return
            }
            completion(error != nil ? .china : nil)
        }
        task.resume()
    }

    private static func save(detected region: SsoRegion) {
        UserDefaults.standard.set(region.rawValue, forKey: detectedKey)
    }

    private static func stored(_ key: String) -> SsoRegion? {
        UserDefaults.standard.string(forKey: key).flatMap(SsoRegion.init(rawValue:))
    }

    private static func localeDefault() -> SsoRegion {
        Locale.current.identifier.lowercased().contains("cn") ? .china : .global
    }

    private final class NoRedirectDelegate: NSObject, URLSessionTaskDelegate {
        func urlSession(
            _ session: URLSession,
            task: URLSessionTask,
            willPerformHTTPRedirection response: HTTPURLResponse,
            newRequest request: URLRequest,
            completionHandler: @escaping (URLRequest?) -> Void
        ) {
            completionHandler(nil)
        }
    }
}
