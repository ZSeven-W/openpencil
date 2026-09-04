import Foundation

/// One third-party sign-in method the regional SSO deployment advertises
/// (`GET /api/v1/auth/providers?channel=web_mobile`). Each deployment lists
/// only its own enabled providers, so the native login screen shows WeChat /
/// Alipay / Douyin on the mainland site and Apple / GitHub / Google on the
/// global site without any hardcoded region table.
struct SsoProviderEntry: Equatable {
    let id: String
    let displayName: String
}

enum SsoProviderList {
    /// Decodes the providers response, dropping malformed rows. Returns an
    /// empty array for structurally invalid payloads so the login screen
    /// falls back to its generic browser button.
    static func parse(_ data: Data) -> [SsoProviderEntry] {
        struct Envelope: Decodable {
            struct Row: Decodable {
                let id: String?
                let displayName: String?

                enum CodingKeys: String, CodingKey {
                    case id
                    case displayName = "display_name"
                }
            }
            let providers: [Row]
        }
        guard let envelope = try? JSONDecoder().decode(Envelope.self, from: data) else {
            return []
        }
        return envelope.providers.compactMap { row in
            guard
                let id = row.id, !id.isEmpty,
                let name = row.displayName, !name.isEmpty,
                name.count <= 64
            else { return nil }
            return SsoProviderEntry(id: id, displayName: name)
        }
    }
}
