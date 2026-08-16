import Foundation

/// Typed failure from the SSO JSON API (or the transport underneath it).
struct SsoAuthError: Error {
    /// Backend error code (`invalid_credentials`, …) or a transport-level
    /// sentinel (`network`, `protocol`).
    let code: String
    let message: String

    var localizedText: String {
        switch code {
        case "invalid_credentials":
            return NSLocalizedString(
                "nativeLogin.error.invalidCredentials",
                value: "Incorrect email or password.",
                comment: "Login failure"
            )
        case "rate_limited":
            return NSLocalizedString(
                "nativeLogin.error.rateLimited",
                value: "Too many attempts. Try again in a moment.",
                comment: "Login failure"
            )
        case "network":
            return NSLocalizedString(
                "nativeLogin.error.network",
                value: "Network error. Check your connection and try again.",
                comment: "Login failure"
            )
        default:
            return message.isEmpty
                ? NSLocalizedString(
                    "nativeLogin.error.generic",
                    value: "Sign-in failed. Please try again.",
                    comment: "Login failure"
                )
                : message
        }
    }
}

/// Minimal JSON client for the native login flow against one regional SSO
/// origin. Uses an ephemeral session so the short-lived web session cookie
/// obtained for the device approval never persists on disk; the durable
/// credential remains the Rust runtime's device token.
final class SsoAuthClient {
    private let origin: URL
    private let session: URLSession

    init(origin: URL) {
        self.origin = origin
        let configuration = URLSessionConfiguration.ephemeral
        configuration.timeoutIntervalForRequest = 15
        configuration.httpCookieAcceptPolicy = .always
        configuration.httpShouldSetCookies = true
        session = URLSession(configuration: configuration)
    }

    /// `POST /api/v1/auth/password-login`; success stores the session cookie
    /// inside this client's ephemeral cookie jar.
    func passwordLogin(
        email: String,
        password: String,
        completion: @escaping (Result<Void, SsoAuthError>) -> Void
    ) {
        post(
            path: "/api/v1/auth/password-login",
            body: ["email": email, "password": password],
            completion: completion
        )
    }

    /// `POST /api/v1/auth/email-codes` — sends a verification code for
    /// registration (`purpose: "register"`) or password recovery
    /// (`purpose: "password_reset"`), localized to the device language.
    func sendEmailCode(
        email: String,
        purpose: String,
        completion: @escaping (Result<Void, SsoAuthError>) -> Void
    ) {
        let locale = Locale.preferredLanguages.first?.hasPrefix("zh") == true ? "zh-CN" : "en"
        post(
            path: "/api/v1/auth/email-codes",
            body: ["email": email, "purpose": purpose, "locale": locale],
            completion: completion
        )
    }

    /// `POST /api/v1/auth/register` — creates the account; the caller then
    /// signs in with the same credentials to approve the device pairing.
    func register(
        email: String,
        code: String,
        password: String,
        completion: @escaping (Result<Void, SsoAuthError>) -> Void
    ) {
        post(
            path: "/api/v1/auth/register",
            body: ["email": email, "verification_code": code, "password": password],
            completion: completion
        )
    }

    /// `POST /api/v1/auth/password-reset` — sets a new password; the caller
    /// then signs in with it to approve the device pairing.
    func resetPassword(
        email: String,
        code: String,
        newPassword: String,
        completion: @escaping (Result<Void, SsoAuthError>) -> Void
    ) {
        post(
            path: "/api/v1/auth/password-reset",
            body: ["email": email, "verification_code": code, "new_password": newPassword],
            completion: completion
        )
    }

    /// `GET /api/v1/auth/providers?channel=web_mobile` — the third-party
    /// sign-in methods this regional deployment advertises (WeChat / Alipay /
    /// Douyin on the mainland site, Apple / GitHub / Google on the global
    /// site). Failures resolve to an empty list so the login screen keeps
    /// its generic browser button.
    func fetchProviders(completion: @escaping ([SsoProviderEntry]) -> Void) {
        var components = URLComponents(
            url: origin.appendingPathComponent("/api/v1/auth/providers"),
            resolvingAgainstBaseURL: false
        )
        components?.queryItems = [URLQueryItem(name: "channel", value: "web_mobile")]
        guard let url = components?.url else {
            DispatchQueue.main.async { completion([]) }
            return
        }
        let task = session.dataTask(with: url) { data, response, _ in
            let providers: [SsoProviderEntry]
            if let data,
                let http = response as? HTTPURLResponse,
                (200..<300).contains(http.statusCode)
            {
                providers = SsoProviderList.parse(data)
            } else {
                providers = []
            }
            DispatchQueue.main.async { completion(providers) }
        }
        task.resume()
    }

    /// `POST /api/v1/device/login/approve` with the logged-in session cookie —
    /// approves the pairing the engine's device flow is polling.
    func approvePairing(
        pairingID: String,
        completion: @escaping (Result<Void, SsoAuthError>) -> Void
    ) {
        post(
            path: "/api/v1/device/login/approve",
            body: ["pairing_id": pairingID],
            completion: completion
        )
    }

    private func post(
        path: String,
        body: [String: String],
        completion: @escaping (Result<Void, SsoAuthError>) -> Void
    ) {
        var request = URLRequest(url: origin.appendingPathComponent(path))
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = try? JSONSerialization.data(withJSONObject: body)
        let finish: (Result<Void, SsoAuthError>) -> Void = { result in
            DispatchQueue.main.async { completion(result) }
        }
        let task = session.dataTask(with: request) { data, response, error in
            if error != nil {
                finish(.failure(SsoAuthError(code: "network", message: "")))
                return
            }
            guard let http = response as? HTTPURLResponse else {
                finish(.failure(SsoAuthError(code: "protocol", message: "")))
                return
            }
            if (200..<300).contains(http.statusCode) {
                finish(.success(()))
                return
            }
            finish(.failure(Self.decodeError(data: data, status: http.statusCode)))
        }
        task.resume()
    }

    private static func decodeError(data: Data?, status: Int) -> SsoAuthError {
        struct Envelope: Decodable {
            struct Body: Decodable {
                let code: String
                let message: String
            }
            let error: Body
        }
        if let data, let envelope = try? JSONDecoder().decode(Envelope.self, from: data) {
            return SsoAuthError(code: envelope.error.code, message: envelope.error.message)
        }
        if status == 429 {
            return SsoAuthError(code: "rate_limited", message: "")
        }
        return SsoAuthError(code: "protocol", message: "")
    }
}
