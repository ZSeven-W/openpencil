import Foundation

@main
enum UniversalLinkTests {
    static func main() {
        acceptsProviderLinks()
        rejectsUntrustedLinks()
        print("UniversalLink tests passed")
    }

    private static func parse(_ value: String) -> UniversalLink? {
        guard let url = URL(string: value) else { return nil }
        return UniversalLink(url: url)
    }

    private static func acceptsProviderLinks() {
        for provider in UniversalLinkProvider.allCases {
            let link = parse(
                "https://op.zseven.cn/app-links/\(provider.rawValue)/callback?state=opaque"
            )
            precondition(link?.provider == provider)
        }

        precondition(
            parse("https://op.zseven.cn/app-links/wechat/")?.provider == .wechat,
            "provider root with a trailing slash must remain valid"
        )
    }

    private static func rejectsUntrustedLinks() {
        let rejected = [
            "http://op.zseven.cn/app-links/wechat/",
            "https://op.zseven.tech/app-links/wechat/",
            "https://op.zseven.cn.evil.example/app-links/wechat/",
            "https://user@op.zseven.cn/app-links/wechat/",
            "https://op.zseven.cn:443/app-links/wechat/",
            "https://op.zseven.cn/app-links/unknown/",
            "https://op.zseven.cn/app-links/",
            "https://op.zseven.cn/app-links/wechat",
            "https://op.zseven.cn/app-links%2Fwechat/callback",
            "https://op.zseven.cn/app-links/wechat/../alipay/",
            "https://op.zseven.cn/app-links/wechat/%2e%2e/alipay/",
            "https://op.zseven.cn/app-links/wechat/#fragment",
        ]

        for value in rejected {
            precondition(parse(value) == nil, "must reject \(value)")
        }
    }
}
