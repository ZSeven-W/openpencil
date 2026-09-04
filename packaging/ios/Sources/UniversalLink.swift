import Foundation

enum UniversalLinkProvider: String, CaseIterable, Sendable {
    case wechat
    case alipay
    case douyin
}

struct UniversalLink: Equatable, Sendable {
    static let host = "op.zseven.cn"
    static let pathPrefix = "/app-links/"

    let provider: UniversalLinkProvider
    let url: URL

    init?(url: URL) {
        guard
            let components = URLComponents(url: url, resolvingAgainstBaseURL: false),
            components.scheme?.lowercased() == "https",
            components.host?.lowercased() == Self.host,
            components.user == nil,
            components.password == nil,
            components.port == nil,
            components.fragment == nil,
            components.percentEncodedPath.hasPrefix(Self.pathPrefix),
            !components.percentEncodedPath.lowercased().contains("%2f"),
            !components.percentEncodedPath.lowercased().contains("%2e")
        else {
            return nil
        }

        let suffix = components.percentEncodedPath.dropFirst(Self.pathPrefix.count)
        let segments = suffix.split(separator: "/", omittingEmptySubsequences: false)
        guard
            segments.count >= 2,
            let provider = UniversalLinkProvider(rawValue: String(segments[0])),
            !segments.contains("."),
            !segments.contains("..")
        else {
            return nil
        }

        self.provider = provider
        self.url = url
    }
}

extension Notification.Name {
    static let openPencilUniversalLink = Notification.Name(
        "tech.zseven.openpencil.universal-link"
    )
}

@MainActor
enum UniversalLinkRouter {
    @discardableResult
    static func handle(_ url: URL) -> Bool {
        guard let link = UniversalLink(url: url) else {
            return false
        }

        NotificationCenter.default.post(
            name: .openPencilUniversalLink,
            object: link
        )
        return true
    }
}
