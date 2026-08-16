import Foundation

/// Parsed engine verification URL (`https://<origin>/login?device_pairing=<id>`).
///
/// The native login screen signs in against `origin`'s JSON API and approves
/// `pairingID`; a URL that does not carry a pairing on an HTTPS origin is
/// rejected so the shell cancels the flow instead of presenting a screen
/// that could never approve anything.
struct DeviceLoginRequestInfo: Equatable {
    let origin: URL
    let pairingID: String
    let verificationURL: URL

    init?(verificationURL: URL) {
        guard
            let components = URLComponents(url: verificationURL, resolvingAgainstBaseURL: false),
            components.scheme?.lowercased() == "https",
            components.user == nil,
            components.password == nil,
            let host = components.host,
            !host.isEmpty,
            let pairing = components.queryItems?
                .first(where: { $0.name == "device_pairing" })?.value,
            !pairing.isEmpty,
            let origin = URL(string: "https://\(host)\(components.port.map { ":\($0)" } ?? "")")
        else { return nil }
        self.origin = origin
        self.pairingID = pairing
        self.verificationURL = verificationURL
    }
}
