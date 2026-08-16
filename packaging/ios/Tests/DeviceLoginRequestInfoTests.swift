import Foundation

@main
enum DeviceLoginRequestInfoTests {
    static func main() {
        acceptsCanonicalVerificationURLs()
        rejectsNonPairingAndNonHTTPSURLs()
        parsesRegionalProviderLists()
        print("DeviceLoginRequestInfo tests passed")
    }

    private static func parse(_ value: String) -> DeviceLoginRequestInfo? {
        guard let url = URL(string: value) else { return nil }
        return DeviceLoginRequestInfo(verificationURL: url)
    }

    private static func acceptsCanonicalVerificationURLs() {
        let info = parse("https://sso.zseven.cn/login?device_pairing=pair-123")
        precondition(info != nil, "canonical device-pairing URL must parse")
        precondition(info?.origin.absoluteString == "https://sso.zseven.cn")
        precondition(info?.pairingID == "pair-123")

        let withPort = parse("https://sso.example.test:8443/login?device_pairing=x")
        precondition(withPort?.origin.absoluteString == "https://sso.example.test:8443")

        let extraQuery = parse(
            "https://sso.zseven.tech/login?utm=app&device_pairing=abc"
        )
        precondition(extraQuery?.pairingID == "abc", "extra query params must not hide the pairing")
    }

    private static func rejectsNonPairingAndNonHTTPSURLs() {
        precondition(parse("https://sso.zseven.cn/login") == nil, "missing pairing must reject")
        precondition(
            parse("https://sso.zseven.cn/login?device_pairing=") == nil,
            "empty pairing must reject"
        )
        precondition(
            parse("http://sso.zseven.cn/login?device_pairing=x") == nil,
            "plain HTTP must reject"
        )
        precondition(
            parse("https://user:pw@sso.zseven.cn/login?device_pairing=x") == nil,
            "userinfo URLs must reject"
        )
    }

    private static func parsesRegionalProviderLists() {
        let global = SsoProviderList.parse(Data("""
        {"providers":[
            {"id":"apple","display_name":"Apple","icon":"apple","channel":"web_mobile","start_url":"/x"},
            {"id":"github","display_name":"GitHub","icon":"github","channel":"web_mobile","start_url":"/x"},
            {"id":"google","display_name":"Google","icon":"google","channel":"web_mobile","start_url":"/x"}
        ]}
        """.utf8))
        precondition(
            global.map(\.displayName) == ["Apple", "GitHub", "Google"],
            "global providers must parse in order"
        )

        let mainland = SsoProviderList.parse(Data("""
        {"providers":[
            {"id":"wechat","display_name":"微信"},
            {"id":"alipay","display_name":"支付宝"},
            {"id":"douyin","display_name":"抖音"},
            {"id":"","display_name":"broken"},
            {"id":"nameless"}
        ]}
        """.utf8))
        precondition(
            mainland.map(\.id) == ["wechat", "alipay", "douyin"],
            "malformed rows must drop without hiding valid providers"
        )

        precondition(SsoProviderList.parse(Data("not json".utf8)).isEmpty)
        precondition(SsoProviderList.parse(Data("{}".utf8)).isEmpty)
    }
}
